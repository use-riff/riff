pub mod constants;
pub mod curve;
pub mod error;
pub mod events;
pub mod instructions;
pub mod payout;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("59MehWKuM1t6u3LAD4nyEg3kBw1HKq5EbS4ByKsosqtV");

#[program]
pub mod riff {
    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConfig>, params: ConfigParams) -> Result<()> {
        crate::instructions::initialize_config::handle_initialize_config(ctx, params)
    }

    pub fn create_coin(ctx: Context<CreateCoin>, args: CreateCoinArgs) -> Result<()> {
        crate::instructions::create_coin::handle_create_coin(ctx, args)
    }

    pub fn buy(ctx: Context<Swap>, max_sol_in: u64, min_tokens_out: u64) -> Result<()> {
        crate::instructions::trade::handle_buy(ctx, max_sol_in, min_tokens_out)
    }

    pub fn sell(ctx: Context<Swap>, token_amount: u64, min_sol_out: u64) -> Result<()> {
        crate::instructions::trade::handle_sell(ctx, token_amount, min_sol_out)
    }

    pub fn withdraw_creator_fees(ctx: Context<WithdrawCreatorFees>) -> Result<()> {
        crate::instructions::withdraw_creator_fees::handle_withdraw_creator_fees(ctx)
    }

    pub fn collect_protocol_fees(ctx: Context<CollectProtocolFees>) -> Result<()> {
        crate::instructions::collect_protocol_fees::handle_collect_protocol_fees(ctx)
    }

    pub fn claim_artist(ctx: Context<ClaimArtist>, artist_id: String) -> Result<()> {
        crate::instructions::claim_artist::handle_claim_artist(ctx, artist_id)
    }

    pub fn withdraw_artist_fees(ctx: Context<WithdrawArtistFees>) -> Result<()> {
        crate::instructions::withdraw_artist_fees::handle_withdraw_artist_fees(ctx)
    }

    pub fn sweep_charity_fees(ctx: Context<SweepCharityFees>) -> Result<()> {
        crate::instructions::sweep_charity_fees::handle_sweep_charity_fees(ctx)
    }

    pub fn update_config(ctx: Context<UpdateConfig>, params: UpdateConfigParams) -> Result<()> {
        crate::instructions::update_config::handle_update_config(ctx, params)
    }

    pub fn graduate(ctx: Context<Graduate>) -> Result<()> {
        crate::instructions::graduate::handle_graduate(ctx)
    }

    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        crate::instructions::transfer_admin::handle_transfer_admin(ctx, new_admin)
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        crate::instructions::transfer_admin::handle_accept_admin(ctx)
    }
}
