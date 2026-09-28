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
pub const BUFFER_SEED: &[u8] = b"buffer";

/// riff artist IDs: "spotify:" plus a 22-character Spotify ID.
pub const MAX_ARTIST_ID_LEN: usize = 32;
/// Proofs listed on a passport.
pub const MAX_PASSPORT_PROOFS: usize = 8;
/// Reclaim attestors the config trusts at once.
pub const MAX_ATTESTORS: usize = 8;
/// A Reclaim proof, written to a buffer in pieces because it's too big for one transaction.
pub const MAX_BUFFER_LEN: usize = 6_000;
/// A day, for the free withdrawal allowance.
pub const WITHDRAW_WINDOW_SECS: i64 = 24 * 60 * 60;
/// Domain separator for passkey challenges, so a signature for riff can't be replayed elsewhere.
pub const CHALLENGE_DOMAIN: &[u8] = b"riff-passport:v1";
