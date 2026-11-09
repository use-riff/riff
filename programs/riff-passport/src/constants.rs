use anchor_lang::prelude::*;

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";
#[constant]
pub const PASSPORT_SEED: &[u8] = b"passport";
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";
#[constant]
pub const PROOF_SEED: &[u8] = b"proof";
#[constant]
pub const ENDORSEMENT_SEED: &[u8] = b"endorse";
#[constant]
pub const PENDING_ADMIN_SEED: &[u8] = b"pending_admin";

/// riff artist IDs: "spotify:" plus a 22-character Spotify ID.
pub const MAX_ARTIST_ID_LEN: usize = 32;
/// Proofs listed on a passport.
pub const MAX_PASSPORT_PROOFS: usize = 8;
/// A day, for the free withdrawal allowance.
pub const WITHDRAW_WINDOW_SECS: i64 = 24 * 60 * 60;
/// Domain separator for passkey challenges, so a signature for riff can't be replayed elsewhere.
pub const CHALLENGE_DOMAIN: &[u8] = b"riff-passport:v1";

// Bounds on the config, so the admin can't switch off the protections the
// passport promises (the time-lock, probation and the passkey on large
// withdrawals) with a config change. Changing them takes a program upgrade.

/// The shortest recovery time-lock and probation the config may set.
/// Devnet tests use minutes; mainnet plans 72 hours.
#[cfg(not(feature = "devnet"))]
pub const MIN_RECOVERY_DELAY: i64 = 24 * 60 * 60;
#[cfg(feature = "devnet")]
pub const MIN_RECOVERY_DELAY: i64 = 60;
/// The most that may leave a vault per day without the passkey (1 SOL).
pub const MAX_FREE_WITHDRAW_PER_DAY: u64 = 1_000_000_000;
/// The oldest a proof may be to issue, reissue or recover a passport.
pub const MAX_PROOF_MAX_AGE: i64 = 30 * 24 * 60 * 60;
