//! Prototype: riff's artist fees on top of Meteora's Dynamic Bonding Curve.
//!
//! Meteora runs the curve, trading and graduation, so riff coins trade in
//! Jupiter, Axiom and other apps from their first trade. riff keeps what
//! makes it riff:
//!
//! - Each coin's pool is created with a riff-owned **escrow** as its
//!   creator, so the creator share of every trade (the artist's share) is
//!   held for the artist until they verify.
//! - riff's **fee authority** is the config's fee claimer: the partner share
//!   of every trade is collected per coin and split between the person who
//!   launched it and riff's treasury.
//! - When the artist verifies, their held fees are paid out and the pool's
//!   creator becomes the artist, so from then on Meteora pays them directly.
//! - If the claim window ends first, held fees go to music charity.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("gATYd3vfHCPCrhFupj8HxVX78syyXNfPSbYRsttm9RF");
declare_program!(dynamic_bonding_curve);

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

pub const MAX_ARTIST_ID_LEN: usize = 64;
pub const MAX_ARTIST_NAME_LEN: usize = 64;
pub const BPS: u64 = 10_000;

#[program]
pub mod riff_dbc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: ConfigParams) -> Result<()> {
        instructions::initialize::handle_initialize(ctx, params)
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

    pub fn claim_artist<'info>(ctx: Context<'info, ClaimArtist<'info>>) -> Result<()> {
        instructions::claim_artist::handle_claim_artist(ctx)
    }

    pub fn sweep_charity<'info>(ctx: Context<'info, SweepCharity<'info>>) -> Result<()> {
        instructions::sweep_charity::handle_sweep_charity(ctx)
    }
}
