use anchor_lang::{prelude::*, Discriminator};
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    dynamic_bonding_curve::{
        self,
        accounts::PoolConfig,
        types::{LiquidityVestingInfo, LockedVestingConfig},
    },
    error::RiffDbcError,
    instructions::update_config,
    program::RiffDbc,
    *,
};

#[derive(Accounts)]
#[instruction(params: ConfigParams)]
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
    /// CHECK: riff's Meteora config, `params.dbc_config`; its settings are
    /// checked in the handler (see `check_dbc_config`).
    #[account(address = params.dbc_config)]
    pub dbc_config: UncheckedAccount<'info>,
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
    update_config::validate(params.launcher_share_bps, params.claim_window_secs)?;
    check_dbc_config(&ctx.accounts.dbc_config, &ctx.accounts.fee_authority.key())?;
    ctx.accounts.config.set_inner(Config {
        admin: ctx.accounts.admin.key(),
        verifier: params.verifier,
        charity: params.charity,
        treasury: params.treasury,
        dbc_config: params.dbc_config,
        launcher_share_bps: params.launcher_share_bps,
        max_launch_buy: params.max_launch_buy,
        claim_window_secs: params.claim_window_secs,
        bump: ctx.bumps.config,
        fee_authority_bump: ctx.bumps.fee_authority,
    });
    Ok(())
}

// Meteora's enum values (its SDK: CollectFeeMode, MigrationOption,
// MigratedCollectFeeMode, TokenType, TokenAuthorityOption).
const COLLECT_FEE_QUOTE: u8 = 0;
const MIGRATE_TO_DAMM_V2: u8 = 1;
const TOKEN_2022: u8 = 1;
const TOKEN_AUTHORITY_IMMUTABLE: u8 = 1;

/// The Meteora config can never change once riff's config names it, and
/// every coin is created with it. So it's checked here for what riff's
/// instructions rely on:
///
/// - riff's fee authority claims the partner share, in WSOL;
/// - fees are charged in SOL only, before and after graduation (riff only
///   routes the SOL side; base-token fees would be stuck in the escrow's
///   account);
/// - graduation goes to DAMM v2 (what `collect_lp_fees` collects from), with
///   all liquidity in the two permanently locked positions;
/// - nothing else accrues to the escrow or the fee authority that riff has no
///   instruction to claim: no migration fee, locked vesting, liquidity
///   vesting or pool creation fee;
/// - coins are Token-2022 with immutable metadata.
fn check_dbc_config(info: &AccountInfo, fee_authority: &Pubkey) -> Result<()> {
    use core::mem::{offset_of, size_of};
    require_keys_eq!(
        *info.owner,
        dynamic_bonding_curve::ID,
        RiffDbcError::InvalidDbcConfig
    );
    let data = info.try_borrow_data()?;
    require!(
        data.len() >= 8 + size_of::<PoolConfig>() && data[..8] == *PoolConfig::DISCRIMINATOR,
        RiffDbcError::InvalidDbcConfig
    );
    let field = |offset: usize, len: usize| &data[8 + offset..8 + offset + len];
    let byte = |offset: usize| data[8 + offset];
    let u64_at = |offset: usize| u64::from_le_bytes(field(offset, 8).try_into().unwrap());
    let vesting = offset_of!(PoolConfig, locked_vesting_config);
    let vesting_percentage = offset_of!(LiquidityVestingInfo, vesting_percentage);
    let ok = field(offset_of!(PoolConfig, quote_mint), 32)
        == anchor_spl::token::spl_token::native_mint::ID.as_ref()
        && field(offset_of!(PoolConfig, fee_claimer), 32) == fee_authority.as_ref()
        && byte(offset_of!(PoolConfig, collect_fee_mode)) == COLLECT_FEE_QUOTE
        && byte(offset_of!(PoolConfig, migrated_collect_fee_mode)) == COLLECT_FEE_QUOTE
        && byte(offset_of!(PoolConfig, migration_option)) == MIGRATE_TO_DAMM_V2
        && byte(offset_of!(PoolConfig, partner_liquidity_percentage)) == 0
        && byte(offset_of!(PoolConfig, creator_liquidity_percentage)) == 0
        && byte(offset_of!(PoolConfig, partner_liquidity_vesting_info) + vesting_percentage) == 0
        && byte(offset_of!(PoolConfig, creator_liquidity_vesting_info) + vesting_percentage) == 0
        && byte(offset_of!(PoolConfig, migration_fee_percentage)) == 0
        && byte(offset_of!(PoolConfig, creator_migration_fee_percentage)) == 0
        && u64_at(vesting + offset_of!(LockedVestingConfig, amount_per_period)) == 0
        && u64_at(vesting + offset_of!(LockedVestingConfig, cliff_unlock_amount)) == 0
        && u64_at(offset_of!(PoolConfig, pool_creation_fee)) == 0
        && byte(offset_of!(PoolConfig, token_type)) == TOKEN_2022
        && byte(offset_of!(PoolConfig, token_update_authority)) == TOKEN_AUTHORITY_IMMUTABLE;
    require!(ok, RiffDbcError::InvalidDbcConfig);
    Ok(())
}
