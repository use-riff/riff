use anchor_lang::prelude::*;

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
    pub artist_fee_share_bps: u16,
    pub claim_window_secs: i64,
}

#[event]
pub struct CoinCreated {
    pub coin: Pubkey,
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub artist_id: String,
    pub artist_name: String,
    pub claim_deadline: i64,
}
