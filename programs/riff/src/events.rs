use anchor_lang::prelude::*;

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub charity: Pubkey,
    pub verifier: Pubkey,
    pub artist_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub claim_window_secs: i64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_virtual_token_reserves: u64,
    pub curve_token_supply: u64,
    pub max_creator_buy_bps: u16,
}

#[event]
pub struct CoinCreated {
    pub coin: Pubkey,
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub artist_id: String,
    pub artist_name: String,
    pub claim_deadline: i64,
    /// Tokens the creator bought at launch (0 if none).
    pub creator_buy_tokens: u64,
    /// SOL the creator paid for them, fees included.
    pub creator_buy_sol: u64,
}

#[event]
pub struct Trade {
    pub coin: Pubkey,
    pub mint: Pubkey,
    pub trader: Pubkey,
    pub is_buy: bool,
    /// SOL into (buy) or out of (sell) the curve's reserves, before fees.
    pub sol_amount: u64,
    pub token_amount: u64,
    pub artist_fee: u64,
    pub creator_fee: u64,
    pub protocol_fee: u64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub timestamp: i64,
}

#[event]
pub struct CurveCompleted {
    pub coin: Pubkey,
    pub mint: Pubkey,
    pub real_sol_reserves: u64,
}

#[event]
pub struct CreatorFeesWithdrawn {
    pub coin: Pubkey,
    pub creator: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ProtocolFeesCollected {
    pub coin: Pubkey,
    pub treasury: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ConfigUpdated {
    pub treasury: Pubkey,
    pub charity: Pubkey,
    pub verifier: Pubkey,
}

#[event]
pub struct ArtistClaimed {
    pub coin: Pubkey,
    pub artist: Pubkey,
    pub artist_id: String,
    /// Claimed after the claim window closed.
    pub late: bool,
    /// Fees held for the artist that went to the charity instead because
    /// the claim was late.
    pub forfeited_to_charity: u64,
}

#[event]
pub struct ArtistFeesWithdrawn {
    pub coin: Pubkey,
    pub artist: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CharityFeesSwept {
    pub coin: Pubkey,
    pub charity: Pubkey,
    pub amount: u64,
}
