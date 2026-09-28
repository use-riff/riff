use anchor_lang::prelude::*;

use crate::{
    error::PassportError,
    state::{ProofKind, ProofRecord},
};

/// The Spotify ID inside a riff artist ID ("spotify:<22 characters>").
pub fn spotify_id(artist_id: &str) -> Result<&str> {
    let id = artist_id
        .strip_prefix("spotify:")
        .ok_or(error!(PassportError::InvalidArtistId))?;
    require!(
        id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()),
        PassportError::InvalidArtistId
    );
    Ok(id)
}

/// The proof records among `accounts`, each checked to belong to
/// `artist_id` and `wallet` and to be recent. One per kind.
pub fn fresh_proofs<'info>(
    accounts: &'info [AccountInfo<'info>],
    artist_id: &str,
    wallet: &Pubkey,
    now: i64,
    max_age: i64,
) -> Result<Vec<(Pubkey, ProofRecord)>> {
    let mut out: Vec<(Pubkey, ProofRecord)> = Vec::with_capacity(accounts.len());
    for info in accounts {
        let record = Account::<ProofRecord>::try_from(info)?;
        require!(
            record.artist_id == artist_id && record.wallet == *wallet,
            PassportError::ProofMismatch
        );
        require!(
            now - record.verified_at <= max_age,
            PassportError::ProofExpired
        );
        require!(
            !out.iter().any(|(_, r)| r.kind == record.kind),
            PassportError::ProofMismatch
        );
        out.push((info.key(), record.into_inner()));
    }
    Ok(out)
}

pub fn kinds(records: &[(Pubkey, ProofRecord)]) -> Vec<ProofKind> {
    records.iter().map(|(_, r)| r.kind).collect()
}
