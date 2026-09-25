pub mod constants;
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

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        artist_fee_share_bps: u16,
        claim_window_secs: i64,
    ) -> Result<()> {
        crate::instructions::initialize_config::handle_initialize_config(
            ctx,
            artist_fee_share_bps,
            claim_window_secs,
        )
    }

    pub fn create_coin(ctx: Context<CreateCoin>, args: CreateCoinArgs) -> Result<()> {
        crate::instructions::create_coin::handle_create_coin(ctx, args)
    }
}
