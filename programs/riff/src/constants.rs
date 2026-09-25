use anchor_lang::prelude::*;

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

#[constant]
pub const COIN_SEED: &[u8] = b"coin";

#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

/// Highest total trading fee the config will accept (10%).
#[constant]
pub const MAX_TRADE_FEE_BPS: u16 = 1_000;

/// Highest creator launch-buy cap the config will accept (10% of supply).
#[constant]
pub const MAX_CREATOR_BUY_BPS: u16 = 1_000;

/// Basis-point denominator (100%).
#[constant]
pub const BPS_DENOMINATOR: u16 = 10_000;

#[constant]
pub const COIN_DECIMALS: u8 = 6;

/// 1,000,000,000 whole tokens, in base units. Minted once at creation; never more.
#[constant]
pub const COIN_TOTAL_SUPPLY: u64 = 1_000_000_000 * 10u64.pow(COIN_DECIMALS as u32);

// Token metadata limits (bytes).
pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 10;
pub const MAX_URI_LEN: usize = 200;

// Unclaimed artist identity limits (bytes).
pub const MAX_ARTIST_ID_LEN: usize = 64;
pub const MAX_ARTIST_NAME_LEN: usize = 64;
