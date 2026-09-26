use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    token_2022::Token2022,
    token_interface::{transfer_checked, Mint, TokenAccount, TransferChecked},
};

use crate::{
    constants::*,
    curve::{quote_buy, quote_sell, split_fee, BuyQuote, FeeSplit},
    error::ErrorCode,
    events::{CurveCompleted, Trade},
    payout::pay_from_coin,
    state::{Coin, Config},
};

/// Accounts shared by `buy` and `sell`.
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(
        mut,
        seeds = [COIN_SEED, mint.key().as_ref()],
        bump = coin.bump,
        has_one = mint,
        has_one = vault,
    )]
    pub coin: Box<Account<'info, Coin>>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = trader,
        token::token_program = token_program,
    )]
    pub trader_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

/// Spend up to `max_sol_in` lamports (fee included) on tokens, receiving at
/// least `min_tokens_out`. The final buy is trimmed to what the curve has left.
pub fn handle_buy(ctx: Context<Swap>, max_sol_in: u64, min_tokens_out: u64) -> Result<()> {
    let a = ctx.accounts;
    // Closed in the launch slot: otherwise buys bundled into the launch
    // transaction (by the creator or any wallet they control) would land
    // before anyone else could trade, bypassing the creator-buy cap.
    require!(
        Clock::get()?.slot > a.coin.created_slot,
        ErrorCode::TradingNotOpen
    );
    let (quote, fees) = settle_buy(&a.config, &mut a.coin, max_sol_in)?;
    require!(
        quote.tokens_out >= min_tokens_out,
        ErrorCode::SlippageExceeded
    );
    pay_for_buy(
        &a.system_program,
        &a.trader.to_account_info(),
        &a.coin.to_account_info(),
        &quote,
    )?;
    release_tokens(
        &a.token_program,
        &a.vault,
        &a.mint,
        &a.trader_token_account.to_account_info(),
        &a.coin,
        quote.tokens_out,
    )?;
    emit_trade(
        &a.coin,
        a.trader.key(),
        true,
        quote.sol_to_curve,
        quote.tokens_out,
        &fees,
    )
}

/// Sell `token_amount` tokens back to the curve, receiving at least
/// `min_sol_out` lamports after fees.
pub fn handle_sell(ctx: Context<Swap>, token_amount: u64, min_sol_out: u64) -> Result<()> {
    let a = ctx.accounts;
    let coin = &mut a.coin;
    require!(!coin.complete, ErrorCode::CurveComplete);

    let reserves = coin.reserves();
    let quote = quote_sell(&reserves, a.config.total_fee_bps(), token_amount)
        .ok_or(ErrorCode::AmountTooSmall)?;
    require!(quote.sol_out >= min_sol_out, ErrorCode::SlippageExceeded);
    let fees = split_fee(
        quote.fee,
        a.config.artist_fee_bps,
        a.config.creator_fee_bps,
        a.config.protocol_fee_bps,
    );
    let next = reserves
        .apply_sell(token_amount, &quote)
        .ok_or(ErrorCode::MathOverflow)?;
    coin.set_reserves(next);
    accrue_fees(coin, &fees)?;

    transfer_checked(
        CpiContext::new(
            a.token_program.key(),
            TransferChecked {
                from: a.trader_token_account.to_account_info(),
                mint: a.mint.to_account_info(),
                to: a.vault.to_account_info(),
                authority: a.trader.to_account_info(),
            },
        ),
        token_amount,
        COIN_DECIMALS,
    )?;

    // Paid out of the curve's SOL (already reduced above). All three fees
    // are simply left behind on the coin.
    pay_from_coin(coin, &a.trader.to_account_info(), quote.sol_out)?;

    emit_trade(
        coin,
        a.trader.key(),
        false,
        quote.sol_from_curve,
        token_amount,
        &fees,
    )
}

/// Prices a buy against the coin's curve and records it on the coin. The
/// caller then moves the SOL and tokens.
pub(crate) fn settle_buy(
    config: &Config,
    coin: &mut Coin,
    max_sol_in: u64,
) -> Result<(BuyQuote, FeeSplit)> {
    require!(!coin.complete, ErrorCode::CurveComplete);
    let reserves = coin.reserves();
    let quote = quote_buy(&reserves, config.total_fee_bps(), max_sol_in)
        .ok_or(ErrorCode::AmountTooSmall)?;
    let fees = split_fee(
        quote.fee,
        config.artist_fee_bps,
        config.creator_fee_bps,
        config.protocol_fee_bps,
    );
    let next = reserves.apply_buy(&quote).ok_or(ErrorCode::MathOverflow)?;
    coin.set_reserves(next);
    coin.complete = next.real_token == 0;
    accrue_fees(coin, &fees)?;
    Ok((quote, fees))
}

/// Buyer pays the curve SOL and every fee to the coin account. Fees wait
/// there until the artist, creator, or protocol collects them.
pub(crate) fn pay_for_buy<'info>(
    system_program: &Program<'info, System>,
    buyer: &AccountInfo<'info>,
    coin: &AccountInfo<'info>,
    quote: &BuyQuote,
) -> Result<()> {
    pay(system_program, buyer, coin, quote.total_cost)
}

/// Sends tokens out of the vault, signed by the coin PDA.
pub(crate) fn release_tokens<'info>(
    token_program: &Program<'info, Token2022>,
    vault: &InterfaceAccount<'info, TokenAccount>,
    mint: &InterfaceAccount<'info, Mint>,
    to: &AccountInfo<'info>,
    coin: &Account<'info, Coin>,
    amount: u64,
) -> Result<()> {
    let mint_key = mint.key();
    let signer_seeds: &[&[&[u8]]] = &[&[COIN_SEED, mint_key.as_ref(), &[coin.bump]]];
    transfer_checked(
        CpiContext::new_with_signer(
            token_program.key(),
            TransferChecked {
                from: vault.to_account_info(),
                mint: mint.to_account_info(),
                to: to.clone(),
                authority: coin.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
        COIN_DECIMALS,
    )
}

pub(crate) fn emit_trade(
    coin: &Account<Coin>,
    trader: Pubkey,
    is_buy: bool,
    sol_amount: u64,
    token_amount: u64,
    fees: &FeeSplit,
) -> Result<()> {
    emit!(Trade {
        coin: coin.key(),
        mint: coin.mint,
        trader,
        is_buy,
        sol_amount,
        token_amount,
        artist_fee: fees.artist,
        creator_fee: fees.creator,
        protocol_fee: fees.protocol,
        virtual_sol_reserves: coin.virtual_sol_reserves,
        virtual_token_reserves: coin.virtual_token_reserves,
        timestamp: Clock::get()?.unix_timestamp,
    });
    // Only a buy can sell out the curve, and trading stops once it has.
    if is_buy && coin.complete {
        emit!(CurveCompleted {
            coin: coin.key(),
            mint: coin.mint,
            real_sol_reserves: coin.real_sol_reserves,
        });
    }
    Ok(())
}

fn accrue_fees(coin: &mut Coin, fees: &FeeSplit) -> Result<()> {
    coin.artist_fees = coin
        .artist_fees
        .checked_add(fees.artist)
        .ok_or(ErrorCode::MathOverflow)?;
    coin.creator_fees = coin
        .creator_fees
        .checked_add(fees.creator)
        .ok_or(ErrorCode::MathOverflow)?;
    coin.protocol_fees = coin
        .protocol_fees
        .checked_add(fees.protocol)
        .ok_or(ErrorCode::MathOverflow)?;
    Ok(())
}

fn pay<'info>(
    system_program: &Program<'info, System>,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    transfer(
        CpiContext::new(
            system_program.key(),
            Transfer {
                from: from.clone(),
                to: to.clone(),
            },
        ),
        lamports,
    )
}
