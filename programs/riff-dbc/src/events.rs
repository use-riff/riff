//! What indexers and riffpad.fun's ledger read. All amounts are lamports.
use anchor_lang::prelude::*;

#[event]
pub struct CoinCreated {
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub launcher: Pubkey,
    pub artist_id: String,
    pub claim_deadline: i64,
}

/// `lp` is true for LP fees of the graduated pool, false for the curve's
/// trading fees.
#[event]
pub struct FeesCollected {
    pub mint: Pubkey,
    pub lp: bool,
    pub to_launcher: u64,
    pub to_treasury: u64,
    pub to_artist_vault: u64,
}

#[event]
pub struct ArtistClaimed {
    pub mint: Pubkey,
    pub passport: Pubkey,
    pub passport_vault: Pubkey,
    pub late: bool,
    pub paid_to_artist: u64,
    pub sent_to_charity: u64,
}

#[event]
pub struct ArtistPaid {
    pub mint: Pubkey,
    pub passport_vault: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CharitySwept {
    pub mint: Pubkey,
    pub amount: u64,
}
