use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{error::RiffDbcError, instructions::shared, *};

/// Pays a claimed coin's collected artist fees to the artist's passport
/// vault, in SOL. Anyone can call it: the money can only go to the vault.
#[derive(Accounts)]
pub struct PayArtist<'info> {
    /// Pays the rent of `unwrap`, and gets it back.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [COIN_SEED, base_mint.key().as_ref()], bump = coin.bump)]
    pub coin: Box<Account<'info, Coin>>,
    /// CHECK: the coin's artist, a passport vault.
    #[account(
        mut,
        constraint = coin.artist == Some(passport_vault.key()) @ RiffDbcError::WrongArtistVault,
    )]
    pub passport_vault: UncheckedAccount<'info>,
    /// CHECK: PDA, the artist vault's authority.
    #[account(seeds = [ESCROW_SEED, base_mint.key().as_ref()], bump = coin.escrow_bump)]
    pub escrow: UncheckedAccount<'info>,
    #[account(mut, seeds = [ARTIST_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub artist_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = payer,
        seeds = [UNWRAP_SEED, base_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub unwrap: Box<Account<'info, TokenAccount>>,
    /// CHECK: this coin's mint.
    pub base_mint: UncheckedAccount<'info>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub quote_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_pay_artist<'info>(ctx: Context<'info, PayArtist<'info>>) -> Result<()> {
    let a = ctx.accounts;
    let mint = a.base_mint.key();
    let escrow_bump = [a.coin.escrow_bump];
    shared::pay_artist_sol(
        &a.token_program.to_account_info(),
        &a.system_program.to_account_info(),
        &a.artist_vault,
        &a.unwrap.to_account_info(),
        &a.payer.to_account_info(),
        &a.passport_vault.to_account_info(),
        &a.escrow.to_account_info(),
        &shared::escrow_seeds(&mint, &escrow_bump),
    )?;
    Ok(())
}
