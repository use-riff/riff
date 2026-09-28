use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Mint, Token, TokenAccount, Transfer},
    token_2022::Token2022,
};

use crate::{dynamic_bonding_curve, error::RiffDbcError, instructions::shared, *};

/// Collects a coin's fees from Meteora. Anyone can call it (a crank).
///
/// - The partner share is split between the coin's launcher and riff.
/// - Until the artist claims, the creator share goes to the coin's artist
///   vault. After they claim, Meteora pays the artist directly.
#[derive(Accounts)]
pub struct CollectFees<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [COIN_SEED, base_mint.key().as_ref()], bump = coin.bump)]
    pub coin: Box<Account<'info, Coin>>,
    /// CHECK: PDA, the pool creator until the artist claims.
    #[account(seeds = [ESCROW_SEED, base_mint.key().as_ref()], bump = coin.escrow_bump)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK: PDA, the DBC config's fee claimer.
    #[account(seeds = [FEE_AUTHORITY_SEED], bump = config.fee_authority_bump)]
    pub fee_authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [FEES_VAULT_SEED], bump)]
    pub fees_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [ARTIST_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub artist_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: the escrow's account for the coin (see create_coin).
    #[account(mut)]
    pub escrow_base_account: UncheckedAccount<'info>,
    #[account(mut, token::mint = quote_mint, constraint = launcher_quote_account.owner == coin.launcher @ RiffDbcError::WrongReceiver)]
    pub launcher_quote_account: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint, constraint = treasury_quote_account.owner == config.treasury @ RiffDbcError::WrongReceiver)]
    pub treasury_quote_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: riff's DBC config.
    #[account(address = config.dbc_config)]
    pub dbc_config: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: must be this coin's pool.
    #[account(mut, address = coin.pool @ RiffDbcError::WrongPool)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: this coin's mint.
    pub base_mint: UncheckedAccount<'info>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub quote_mint: Box<Account<'info, Mint>>,
    /// CHECK: checked by Meteora.
    pub dbc_event_authority: UncheckedAccount<'info>,
    pub dbc_program: Program<'info, dynamic_bonding_curve::program::DynamicBondingCurve>,
    pub token_quote_program: Program<'info, Token>,
    pub token_base_program: Program<'info, Token2022>,
}

pub fn handle_collect_fees<'info>(ctx: Context<'info, CollectFees<'info>>) -> Result<()> {
    let a = ctx.accounts;
    let fee_authority_bump = [a.config.fee_authority_bump];
    let fee_signer: &[&[u8]] = &[FEE_AUTHORITY_SEED, &fee_authority_bump];

    // Partner share: this pool's only, so it can be credited to this coin.
    let before = a.fees_vault.amount;
    dynamic_bonding_curve::cpi::claim_trading_fee(
        CpiContext::new_with_signer(
            a.dbc_program.key(),
            dynamic_bonding_curve::cpi::accounts::ClaimTradingFee {
                pool_authority: a.pool_authority.to_account_info(),
                config: a.dbc_config.to_account_info(),
                pool: a.pool.to_account_info(),
                token_a_account: a.escrow_base_account.to_account_info(),
                token_b_account: a.fees_vault.to_account_info(),
                base_vault: a.base_vault.to_account_info(),
                quote_vault: a.quote_vault.to_account_info(),
                base_mint: a.base_mint.to_account_info(),
                quote_mint: a.quote_mint.to_account_info(),
                fee_claimer: a.fee_authority.to_account_info(),
                token_base_program: a.token_base_program.to_account_info(),
                token_quote_program: a.token_quote_program.to_account_info(),
                event_authority: a.dbc_event_authority.to_account_info(),
                program: a.dbc_program.to_account_info(),
            },
            &[fee_signer],
        ),
        u64::MAX,
        u64::MAX,
    )?;
    a.fees_vault.reload()?;
    let partner = a.fees_vault.amount - before;
    let to_launcher = (partner as u128 * a.config.launcher_share_bps as u128 / BPS as u128) as u64;
    for (to, amount) in [
        (a.launcher_quote_account.to_account_info(), to_launcher),
        (
            a.treasury_quote_account.to_account_info(),
            partner - to_launcher,
        ),
    ] {
        if amount > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    a.token_quote_program.key(),
                    Transfer {
                        from: a.fees_vault.to_account_info(),
                        to,
                        authority: a.fee_authority.to_account_info(),
                    },
                    &[fee_signer],
                ),
                amount,
            )?;
        }
    }
    a.coin.launcher_fees_total += to_launcher;

    // Creator share, while riff's escrow is still the creator.
    if a.coin.artist.is_none() {
        let mint = a.base_mint.key();
        let escrow_bump = [a.coin.escrow_bump];
        let collected = shared::collect_creator_fees(
            &a.dbc_program.to_account_info(),
            &a.pool_authority.to_account_info(),
            &a.pool.to_account_info(),
            &a.escrow_base_account.to_account_info(),
            &mut a.artist_vault,
            &a.base_vault.to_account_info(),
            &a.quote_vault.to_account_info(),
            &a.base_mint.to_account_info(),
            &a.quote_mint.to_account_info(),
            &a.escrow.to_account_info(),
            &a.token_base_program.to_account_info(),
            &a.token_quote_program.to_account_info(),
            &a.dbc_event_authority.to_account_info(),
            &shared::escrow_seeds(&mint, &escrow_bump),
        )?;
        a.coin.artist_fees_total += collected;
    }
    Ok(())
}
