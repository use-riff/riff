use anchor_lang::prelude::*;

use crate::{error::RiffDbcError, *};

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(address = config.admin)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
}

pub fn handle_update_config(ctx: Context<UpdateConfig>, params: ConfigUpdate) -> Result<()> {
    validate(params.launcher_share_bps, params.claim_window_secs)?;
    let config = &mut ctx.accounts.config;
    config.admin = params.admin;
    config.verifier = params.verifier;
    config.charity = params.charity;
    config.treasury = params.treasury;
    config.launcher_share_bps = params.launcher_share_bps;
    config.max_launch_buy = params.max_launch_buy;
    config.claim_window_secs = params.claim_window_secs;
    Ok(())
}

pub fn validate(launcher_share_bps: u16, claim_window_secs: i64) -> Result<()> {
    require!(launcher_share_bps as u64 <= BPS, RiffDbcError::InvalidShare);
    require!(claim_window_secs > 0, RiffDbcError::InvalidClaimWindow);
    Ok(())
}
