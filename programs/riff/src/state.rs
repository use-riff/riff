use anchor_lang::prelude::*;

use crate::constants::*;

/// Protocol-wide settings. Singleton PDA at `[CONFIG_SEED]`.
#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    /// Share of protocol fees owed to a coin's artist, in basis points.
    pub artist_fee_share_bps: u16,
    /// How long after a coin's creation its artist may claim it, in seconds.
    pub claim_window_secs: i64,
    pub bump: u8,
}

/// One launched coin. PDA at `[COIN_SEED, mint]`.
///
/// The artist is recorded only as text until they claim the coin; until then
/// `artist` is `None` and nothing implies the artist is affiliated with it.
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
    pub bump: u8,
    pub vault_bump: u8,
}
