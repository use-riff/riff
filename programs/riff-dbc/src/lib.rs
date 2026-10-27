//! riff's artist fees on top of Meteora's Dynamic Bonding Curve.
//!
//! Meteora runs the curve, trading and graduation, so riff coins trade in
//! Jupiter, Axiom and other apps from their first trade. riff keeps what
//! makes it riff:
//!
//! - Each coin's pool is created with a riff-owned **escrow** as its
//!   creator, for good. The creator share of every trade (the artist's
//!   share), and after graduation the creator side's LP fees, go through it.
//! - riff's **fee authority** is the config's fee claimer: the partner share
//!   of every trade is collected per coin and split between the person who
//!   launched it and riff's treasury.
//! - The artist claims with their **Artist Passport**: from then on their
//!   share is paid in SOL to the passport's vault, which a lost or stolen
//!   wallet can't take with it.
//! - If the claim window ends first, held fees go to music charity.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("gATYd3vfHCPCrhFupj8HxVX78syyXNfPSbYRsttm9RF");
declare_program!(dynamic_bonding_curve);
declare_program!(cp_amm);

pub mod error;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

pub const CONFIG_SEED: &[u8] = b"config";
pub const COIN_SEED: &[u8] = b"coin";
pub const ESCROW_SEED: &[u8] = b"escrow";
pub const FEE_AUTHORITY_SEED: &[u8] = b"fee_authority";
pub const FEES_VAULT_SEED: &[u8] = b"fees_vault";
pub const ARTIST_VAULT_SEED: &[u8] = b"artist_vault";
/// A WSOL account that lives for one instruction, to unwrap the artist's
/// fees into SOL for their passport vault.
pub const UNWRAP_SEED: &[u8] = b"unwrap";

pub const MAX_ARTIST_ID_LEN: usize = 64;
pub const MAX_ARTIST_NAME_LEN: usize = 64;
pub const BPS: u64 = 10_000;

#[program]
pub mod riff_dbc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: ConfigParams) -> Result<()> {
        instructions::initialize::handle_initialize(ctx, params)
    }

    pub fn update_config(ctx: Context<UpdateConfig>, params: ConfigUpdate) -> Result<()> {
        instructions::update_config::handle_update_config(ctx, params)
    }

    pub fn create_coin<'info>(
        ctx: Context<'info, CreateCoin<'info>>,
        args: CreateCoinArgs,
    ) -> Result<()> {
        instructions::create_coin::handle_create_coin(ctx, args)
    }

    pub fn collect_fees<'info>(ctx: Context<'info, CollectFees<'info>>) -> Result<()> {
        instructions::collect_fees::handle_collect_fees(ctx)
    }

    pub fn collect_lp_fees<'info>(ctx: Context<'info, CollectLpFees<'info>>) -> Result<()> {
        instructions::collect_lp_fees::handle_collect_lp_fees(ctx)
    }

    pub fn claim_artist<'info>(ctx: Context<'info, ClaimArtist<'info>>) -> Result<()> {
        instructions::claim_artist::handle_claim_artist(ctx)
    }

    pub fn pay_artist<'info>(ctx: Context<'info, PayArtist<'info>>) -> Result<()> {
        instructions::pay_artist::handle_pay_artist(ctx)
    }

    pub fn sweep_charity<'info>(ctx: Context<'info, SweepCharity<'info>>) -> Result<()> {
        instructions::sweep_charity::handle_sweep_charity(ctx)
    }
}
