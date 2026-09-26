use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    events::{AdminTransferProposed, AdminTransferred},
    state::Config,
};

/// Step 1 of 2: the current admin proposes a new admin (e.g. a Squads
/// multisig vault). Nothing changes until the new admin accepts, so a
/// mistyped address can't lock everyone out. Proposing again replaces the
/// proposal.
#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ ErrorCode::NotAdmin,
    )]
    pub config: Account<'info, Config>,
}

pub fn handle_transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
    require_keys_neq!(new_admin, Pubkey::default(), ErrorCode::InvalidAddress);
    ctx.accounts.config.pending_admin = Some(new_admin);
    emit!(AdminTransferProposed {
        admin: ctx.accounts.admin.key(),
        pending_admin: new_admin,
    });
    Ok(())
}

/// Step 2 of 2: the proposed admin signs to take over.
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub pending_admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = config.pending_admin == Some(pending_admin.key()) @ ErrorCode::NotPendingAdmin,
    )]
    pub config: Account<'info, Config>,
}

pub fn handle_accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let previous_admin = config.admin;
    config.admin = ctx.accounts.pending_admin.key();
    config.pending_admin = None;
    emit!(AdminTransferred {
        previous_admin,
        admin: config.admin,
    });
    Ok(())
}
