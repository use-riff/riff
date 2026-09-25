use anchor_lang::prelude::*;

use crate::{
    constants::*, error::ErrorCode, events::ConfigInitialized, program::Riff, state::Config,
};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Must be the program's upgrade authority, so nobody can front-run
    /// deployment and claim the config.
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Riff>,
    #[account(
        constraint = program_data.upgrade_authority_address == Some(admin.key())
            @ ErrorCode::NotUpgradeAuthority
    )]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_config(
    ctx: Context<InitializeConfig>,
    artist_fee_share_bps: u16,
    claim_window_secs: i64,
) -> Result<()> {
    require!(
        artist_fee_share_bps <= BPS_DENOMINATOR,
        ErrorCode::InvalidArtistFeeShare
    );
    require!(claim_window_secs > 0, ErrorCode::InvalidClaimWindow);

    ctx.accounts.config.set_inner(Config {
        admin: ctx.accounts.admin.key(),
        artist_fee_share_bps,
        claim_window_secs,
        bump: ctx.bumps.config,
    });

    emit!(ConfigInitialized {
        admin: ctx.accounts.admin.key(),
        artist_fee_share_bps,
        claim_window_secs,
    });
    Ok(())
}
