use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    token_2022::Token2022,
};

use crate::{dynamic_bonding_curve, error::RiffDbcError, instructions::shared, *};

/// After the claim window, sends a coin's unclaimed artist fees to music
/// charity. Anyone can call it, as often as fees come in.
#[derive(Accounts)]
pub struct SweepCharity<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [COIN_SEED, base_mint.key().as_ref()], bump = coin.bump)]
    pub coin: Box<Account<'info, Coin>>,
    /// CHECK: PDA, the pool creator.
    #[account(seeds = [ESCROW_SEED, base_mint.key().as_ref()], bump = coin.escrow_bump)]
    pub escrow: UncheckedAccount<'info>,
    #[account(mut, seeds = [ARTIST_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub artist_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: the escrow's account for the coin (see create_coin).
    #[account(mut)]
    pub escrow_base_account: UncheckedAccount<'info>,
    #[account(mut, token::mint = quote_mint, constraint = charity_quote_account.owner == config.charity @ RiffDbcError::WrongReceiver)]
    pub charity_quote_account: Box<Account<'info, TokenAccount>>,
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

pub fn handle_sweep_charity<'info>(ctx: Context<'info, SweepCharity<'info>>) -> Result<()> {
    let a = ctx.accounts;
    require!(a.coin.artist.is_none(), RiffDbcError::AlreadyClaimed);
    require!(
        Clock::get()?.unix_timestamp > a.coin.claim_deadline,
        RiffDbcError::ClaimWindowOpen
    );

    let mint = a.base_mint.key();
    let escrow_bump = [a.coin.escrow_bump];
    let escrow_signer = shared::escrow_seeds(&mint, &escrow_bump);
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
        &escrow_signer,
    )?;
    a.coin.artist_fees_total += collected;
    shared::empty_artist_vault(
        &a.token_quote_program.to_account_info(),
        &a.artist_vault,
        &a.charity_quote_account.to_account_info(),
        &a.escrow.to_account_info(),
        &escrow_signer,
    )?;
    Ok(())
}
