use anchor_lang::prelude::*;

use crate::{constants::*, curve::Reserves};

/// Protocol-wide settings. Singleton PDA at `[CONFIG_SEED]`.
#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    /// Receives the protocol's share of trading fees. Must stay funded above
    /// the rent-exempt minimum or small fee transfers to it fail.
    pub treasury: Pubkey,
    /// Trading fee rates, each in basis points of a trade's SOL amount. Their
    /// sum is the total fee charged.
    pub artist_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    /// How long after a coin's creation its artist may claim it, in seconds.
    pub claim_window_secs: i64,
    /// Starting virtual reserves for every new coin's curve.
    pub initial_virtual_sol_reserves: u64,
    /// Derived from the supply split so the graduation pool opens at the
    /// curve's final price; see `curve::graduation_virtual_token_reserves`.
    pub initial_virtual_token_reserves: u64,
    /// Tokens sellable on the curve; the rest of the supply stays in the
    /// vault as the graduation reserve.
    pub curve_token_supply: u64,
    /// Most the creator may buy at launch, in basis points of total supply.
    pub max_creator_buy_bps: u16,
    pub bump: u8,
}

impl Config {
    pub fn total_fee_bps(&self) -> u16 {
        // Each is validated so the sum stays <= MAX_TRADE_FEE_BPS.
        self.artist_fee_bps + self.creator_fee_bps + self.protocol_fee_bps
    }

    pub fn max_creator_buy_tokens(&self) -> u64 {
        // bps <= MAX_CREATOR_BUY_BPS, so this can't overflow.
        COIN_TOTAL_SUPPLY / BPS_DENOMINATOR as u64 * self.max_creator_buy_bps as u64
    }
}

/// One launched coin. PDA at `[COIN_SEED, mint]`.
///
/// The artist is recorded only as text until they claim the coin; until then
/// `artist` is `None` and nothing implies the artist is affiliated with it.
///
/// Holds the curve's SOL and the unpaid artist and creator fees as lamports,
/// on top of its own rent.
#[account]
#[derive(InitSpace)]
pub struct Coin {
    pub mint: Pubkey,
    /// Program-owned token account holding the full supply.
    pub vault: Pubkey,
    pub creator: Pubkey,
    #[max_len(MAX_ARTIST_ID_LEN)]
    pub artist_id: String,
    #[max_len(MAX_ARTIST_NAME_LEN)]
    pub artist_name: String,
    /// Set when the artist claims the coin.
    pub artist: Option<Pubkey>,
    pub created_at: i64,
    /// Last moment the artist may claim, fixed at creation from the config's
    /// claim window so later config changes don't move it.
    pub claim_deadline: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    /// Artist fees accrued and not yet paid out, in lamports.
    pub artist_fees: u64,
    /// Creator fees accrued and not yet withdrawn, in lamports.
    pub creator_fees: u64,
    /// Every curve token has been sold; trading stops until graduation.
    pub complete: bool,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Coin {
    pub fn reserves(&self) -> Reserves {
        Reserves {
            virtual_sol: self.virtual_sol_reserves,
            virtual_token: self.virtual_token_reserves,
            real_token: self.real_token_reserves,
            real_sol: self.real_sol_reserves,
        }
    }

    pub fn set_reserves(&mut self, r: Reserves) {
        self.virtual_sol_reserves = r.virtual_sol;
        self.virtual_token_reserves = r.virtual_token;
        self.real_token_reserves = r.real_token;
        self.real_sol_reserves = r.real_sol;
    }
}
