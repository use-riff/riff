use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{error::RiffDbcError, program::RiffDbc, *};

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Must be the program's upgrade authority, so nobody can front-run
    /// deployment and claim the config.
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + Config::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, Config>,
    /// CHECK: PDA that signs as the DBC config's fee claimer.
    #[account(seeds = [FEE_AUTHORITY_SEED], bump)]
    pub fee_authority: UncheckedAccount<'info>,
    /// Where the partner share lands before it's split.
    #[account(
        init,
        payer = admin,
        seeds = [FEES_VAULT_SEED],
        bump,
        token::mint = quote_mint,
        token::authority = fee_authority,
        token::token_program = token_program,
    )]
    pub fees_vault: Account<'info, TokenAccount>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub quote_mint: Account<'info, Mint>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, RiffDbc>,
    #[account(
        constraint = program_data.upgrade_authority_address == Some(admin.key())
            @ RiffDbcError::NotUpgradeAuthority
    )]
    pub program_data: Account<'info, ProgramData>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>, params: ConfigParams) -> Result<()> {
    require!(
        params.launcher_share_bps as u64 <= BPS,
        RiffDbcError::InvalidShare
    );
    ctx.accounts.config.set_inner(Config {
        admin: ctx.accounts.admin.key(),
        verifier: params.verifier,
        charity: params.charity,
        treasury: params.treasury,
        dbc_config: params.dbc_config,
        launcher_share_bps: params.launcher_share_bps,
        claim_window_secs: params.claim_window_secs,
        bump: ctx.bumps.config,
        fee_authority_bump: ctx.bumps.fee_authority,
    });
    Ok(())
}
