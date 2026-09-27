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

#[constant]
pub const GRADUATION_SEED: &[u8] = b"graduation";

/// riff PDA used as the graduated coin's Raydium pool address. Only riff can
/// sign for it, so nobody can create the graduation pool first.
#[constant]
pub const POOL_SEED: &[u8] = b"pool";

/// Rent allowance on top of Raydium's pool-creation fee that the graduation
/// caller fronts; whatever isn't spent is refunded in the same transaction.
pub const GRADUATION_RENT_ALLOWANCE: u64 = 100_000_000;

/// Raydium CPMM (constant-product) program. Apache-2.0.
/// Mainnet by default; the `devnet` feature switches to Raydium's devnet
/// deployment (the fee tier is chosen per deployment in the config).
#[cfg(not(feature = "devnet"))]
pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
#[cfg(feature = "devnet")]
pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb");
/// Raydium's pool-creation fee receiver (a wrapped-SOL token account).
#[cfg(not(feature = "devnet"))]
pub const RAYDIUM_CREATE_POOL_FEE_RECEIVER: Pubkey =
    pubkey!("DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8");
#[cfg(feature = "devnet")]
pub const RAYDIUM_CREATE_POOL_FEE_RECEIVER: Pubkey =
    pubkey!("3oE58BKVt8KuYkGxx8zBojugnymWmBiyafWgMrnb6eYy");
pub const RAYDIUM_AUTH_SEED: &[u8] = b"vault_and_lp_mint_auth_seed";
pub const RAYDIUM_POOL_LP_MINT_SEED: &[u8] = b"pool_lp_mint";
pub const RAYDIUM_POOL_VAULT_SEED: &[u8] = b"pool_vault";
pub const RAYDIUM_OBSERVATION_SEED: &[u8] = b"observation";
/// Anchor discriminator of Raydium's `initialize`: sha256("global:initialize")[..8].
pub const RAYDIUM_INITIALIZE_DISCRIMINATOR: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
/// Byte offset of `create_pool_fee: u64` in Raydium's `AmmConfig` account
/// (8 discriminator + bump u8 + disable_create_pool bool + index u16 +
/// trade/protocol/fund fee rates 3 x u64).
pub const RAYDIUM_AMM_CONFIG_CREATE_POOL_FEE_OFFSET: usize = 36;

/// Hours of trade volume each coin keeps, one bucket per hour (rolling 24h volume).
pub const VOLUME_HOURS: usize = 24;

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards against the two clusters' addresses being swapped. CI runs
    /// this with and without `--features devnet`.
    #[test]
    fn raydium_addresses_match_the_cluster() {
        let (program, receiver) = if cfg!(feature = "devnet") {
            (
                "DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb",
                "3oE58BKVt8KuYkGxx8zBojugnymWmBiyafWgMrnb6eYy",
            )
        } else {
            (
                "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
                "DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8",
            )
        };
        assert_eq!(RAYDIUM_CPMM_PROGRAM_ID.to_string(), program);
        assert_eq!(RAYDIUM_CREATE_POOL_FEE_RECEIVER.to_string(), receiver);
    }
}
