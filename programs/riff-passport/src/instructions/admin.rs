use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::PassportRevoked,
    program::RiffPassport,
    state::{Passport, PassportConfig},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PassportConfigParams {
    pub verifier: Pubkey,
    pub attestors: Vec<[u8; 20]>,
    pub reclaim_provider_hash: [u8; 32],
    pub rp_id_hash: [u8; 32],
    pub recovery_delay: i64,
    pub free_withdraw_per_day: u64,
    pub proof_max_age: i64,
}

impl PassportConfigParams {
    fn apply(self, config: &mut PassportConfig) -> Result<()> {
        require!(
            !self.attestors.is_empty() && self.attestors.len() <= MAX_ATTESTORS,
            PassportError::InvalidConfig
        );
        require!(
            self.recovery_delay > 0 && self.proof_max_age > 0,
            PassportError::InvalidConfig
        );
        config.verifier = self.verifier;
        config.attestors = self.attestors;
        config.reclaim_provider_hash = self.reclaim_provider_hash;
        config.rp_id_hash = self.rp_id_hash;
        config.recovery_delay = self.recovery_delay;
        config.free_withdraw_per_day = self.free_withdraw_per_day;
        config.proof_max_age = self.proof_max_age;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Must be the program's upgrade authority, so nobody can front-run the config.
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + PassportConfig::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, RiffPassport>,
    #[account(
        constraint = program_data.upgrade_authority_address == Some(admin.key())
            @ PassportError::NotUpgradeAuthority
    )]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_config(
    ctx: Context<InitializeConfig>,
    params: PassportConfigParams,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.bump = ctx.bumps.config;
    params.apply(config)
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ PassportError::NotAdmin)]
    pub config: Account<'info, PassportConfig>,
}

pub fn handle_update_config(
    ctx: Context<UpdateConfig>,
    params: PassportConfigParams,
) -> Result<()> {
    params.apply(&mut ctx.accounts.config)
}

#[derive(Accounts)]
pub struct RevokePassport<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ PassportError::NotAdmin)]
    pub config: Account<'info, PassportConfig>,
    #[account(mut, seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()], bump = passport.bump)]
    pub passport: Account<'info, Passport>,
}

/// For a compromised or wrongly issued passport. Public: the reason is logged.
pub fn handle_revoke_passport(ctx: Context<RevokePassport>, reason: String) -> Result<()> {
    require!(reason.len() <= 200, PassportError::InvalidConfig);
    ctx.accounts.passport.revoked = true;
    emit!(PassportRevoked {
        passport: ctx.accounts.passport.key(),
        reason
    });
    Ok(())
}
