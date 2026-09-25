pub mod constants;
pub mod curve;
pub mod error;
pub mod events;
pub mod instructions;
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
}
