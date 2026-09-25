use anchor_lang::prelude::*;

use crate::{
    constants::*, curve::graduation_virtual_token_reserves, error::ErrorCode,
    events::ConfigInitialized, program::Riff, state::Config,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfigParams {
    pub treasury: Pubkey,
    /// Fee rates in basis points of each trade's SOL amount.
    pub artist_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub claim_window_secs: i64,
    pub initial_virtual_sol_reserves: u64,
    /// Tokens sold on the curve. The graduation reserve is the rest of the
    /// supply, and the starting virtual token reserves are derived from both.
    pub curve_token_supply: u64,
    /// Creator launch-buy cap, in basis points of total supply.
    pub max_creator_buy_bps: u16,
}

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
    params: ConfigParams,
) -> Result<()> {
    let total_fee_bps = params.artist_fee_bps as u32
        + params.creator_fee_bps as u32
        + params.protocol_fee_bps as u32;
    require!(
        total_fee_bps <= MAX_TRADE_FEE_BPS as u32,
        ErrorCode::InvalidTradeFee
    );
    require!(params.claim_window_secs > 0, ErrorCode::InvalidClaimWindow);
    require!(
        params.max_creator_buy_bps <= MAX_CREATOR_BUY_BPS,
        ErrorCode::InvalidCreatorBuyCap
    );
    require!(
        params.initial_virtual_sol_reserves > 0,
        ErrorCode::InvalidCurveParams
    );
    let initial_virtual_token_reserves =
        graduation_virtual_token_reserves(COIN_TOTAL_SUPPLY, params.curve_token_supply)
            .ok_or(ErrorCode::InvalidCurveParams)?;

    ctx.accounts.config.set_inner(Config {
        admin: ctx.accounts.admin.key(),
        treasury: params.treasury,
        artist_fee_bps: params.artist_fee_bps,
        creator_fee_bps: params.creator_fee_bps,
        protocol_fee_bps: params.protocol_fee_bps,
        claim_window_secs: params.claim_window_secs,
        initial_virtual_sol_reserves: params.initial_virtual_sol_reserves,
        initial_virtual_token_reserves,
        curve_token_supply: params.curve_token_supply,
        max_creator_buy_bps: params.max_creator_buy_bps,
        bump: ctx.bumps.config,
    });

    emit!(ConfigInitialized {
        admin: ctx.accounts.admin.key(),
        treasury: params.treasury,
        artist_fee_bps: params.artist_fee_bps,
        creator_fee_bps: params.creator_fee_bps,
        protocol_fee_bps: params.protocol_fee_bps,
        claim_window_secs: params.claim_window_secs,
        initial_virtual_sol_reserves: params.initial_virtual_sol_reserves,
        initial_virtual_token_reserves,
        curve_token_supply: params.curve_token_supply,
        max_creator_buy_bps: params.max_creator_buy_bps,
    });
    Ok(())
}
