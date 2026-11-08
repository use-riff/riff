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
    require!(
        claim_window_secs > 0 && claim_window_secs <= MAX_CLAIM_WINDOW_SECS,
        RiffDbcError::InvalidClaimWindow
    );
    Ok(())
}

/// The first step of handing over the admin. Proposing again replaces the
/// proposal; the admin stays until the new key accepts.
#[derive(Accounts)]
pub struct ProposeAdmin<'info> {
    #[account(mut, address = config.admin)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + PendingAdmin::INIT_SPACE,
        seeds = [PENDING_ADMIN_SEED],
        bump
    )]
    pub pending_admin: Account<'info, PendingAdmin>,
    pub system_program: Program<'info, System>,
}

pub fn handle_propose_admin(ctx: Context<ProposeAdmin>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.pending_admin.set_inner(PendingAdmin {
        new_admin,
        bump: ctx.bumps.pending_admin,
    });
    Ok(())
}

/// The second step: the proposed key signs, so a typo can't hand the config
/// to a key nobody holds. The proposal's rent goes to the new admin.
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    #[account(mut, address = pending_admin.new_admin @ RiffDbcError::NotProposedAdmin)]
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [PENDING_ADMIN_SEED], bump = pending_admin.bump, close = new_admin)]
    pub pending_admin: Account<'info, PendingAdmin>,
}

pub fn handle_accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
    ctx.accounts.config.admin = ctx.accounts.new_admin.key();
    Ok(())
}
