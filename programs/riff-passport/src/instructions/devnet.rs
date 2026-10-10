//! Devnet only: compiled into the devnet build, never into the mainnet one.

use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    state::{Passport, PassportConfig, Vault},
};

#[derive(Accounts)]
pub struct ResetPassport<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ PassportError::NotAdmin)]
    pub config: Account<'info, PassportConfig>,
    #[account(
        mut,
        seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()],
        bump = passport.bump,
        close = admin,
    )]
    pub passport: Account<'info, Passport>,
    #[account(
        mut,
        seeds = [VAULT_SEED, passport.key().as_ref()],
        bump = passport.vault_bump,
        close = admin,
    )]
    pub vault: Account<'info, Vault>,
}

/// Closes a passport and its vault so the artist can be issued a new one,
/// for demos and tests on devnet (on mainnet a passport is only ever revoked
/// and reissued). The vault's test SOL goes to the admin. Coins claimed into
/// the vault keep pointing at its address, which a new passport for the same
/// artist gets again; endorsements from before don't count for the new one.
pub fn handle_reset_passport(ctx: Context<ResetPassport>) -> Result<()> {
    msg!(
        "devnet: passport {} for {} reset",
        ctx.accounts.passport.key(),
        ctx.accounts.passport.artist_id
    );
    Ok(())
}
