use anchor_lang::prelude::*;

use crate::constants::*;

#[account]
#[derive(InitSpace)]
pub struct PassportConfig {
    pub admin: Pubkey,
    /// riff's verification service. It checks proofs off-chain (a Spotify
    /// for Artists email, the Spotify bio code, YouTube, a website), attests
    /// them here, and co-signs riff claims.
    pub verifier: Pubkey,
    /// sha256 of the passkey site (the WebAuthn relying party, e.g. "riffpad.fun").
    pub rp_id_hash: [u8; 32],
    /// How long a recovery waits before it can be finalized, in seconds.
    pub recovery_delay: i64,
    /// How much may leave a vault per day without the passkey, in lamports.
    pub free_withdraw_per_day: u64,
    /// How recent proofs must be to issue or recover a passport, in seconds.
    pub proof_max_age: i64,
    pub bump: u8,
}

/// The ways an artist can prove who they are.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ProofKind {
    /// An email from Spotify for Artists about the artist, with Spotify's
    /// DKIM signature checked.
    SpotifyEmail,
    /// A code, tied to the wallet, that the artist put in their Spotify bio.
    /// Only the artist's Spotify for Artists account can edit the bio.
    SpotifyProfileCode,
    /// A DNS record on the artist's official website.
    Website,
    /// Google login to the artist's official YouTube channel.
    YouTube,
    Instagram,
    TikTok,
    X,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Strength {
    Weak,
    Medium,
    Strong,
}

impl ProofKind {
    pub fn strength(self) -> Strength {
        match self {
            Self::SpotifyEmail | Self::SpotifyProfileCode => Strength::Strong,
            Self::Website | Self::YouTube => Strength::Medium,
            Self::Instagram | Self::TikTok | Self::X => Strength::Weak,
        }
    }

    /// Proofs from the same place count once: both Spotify proofs come down
    /// to the same Spotify for Artists account, so they're one source.
    pub fn source(self) -> u8 {
        match self {
            Self::SpotifyEmail | Self::SpotifyProfileCode => 0,
            Self::Website => 1,
            Self::YouTube => 2,
            Self::Instagram => 3,
            Self::TikTok => 4,
            Self::X => 5,
        }
    }

    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

/// Enough proof for a passport (or a recovery). Either:
/// - both Spotify proofs, the DKIM-checked Spotify for Artists email and the
///   wallet-bound bio code: two different strong checks, even though they
///   come from one Spotify for Artists account; or
/// - proofs from 2 or more sources, at least one of them strong.
///
/// Kinds are counted once each, so repeating a proof never adds to it.
pub fn proofs_suffice(kinds: &[ProofKind]) -> bool {
    let both_spotify =
        kinds.contains(&ProofKind::SpotifyEmail) && kinds.contains(&ProofKind::SpotifyProfileCode);
    let mut sources = 0u8;
    for kind in kinds {
        sources |= 1 << kind.source();
    }
    let two_sources_one_strong =
        sources.count_ones() >= 2 && kinds.iter().any(|k| k.strength() == Strength::Strong);
    both_spotify || two_sources_one_strong
}

/// One verified proof that `wallet` speaks for `artist_id`. Proofs are kept
/// per wallet, so nobody can block an artist by proving first.
#[account]
#[derive(InitSpace)]
pub struct ProofRecord {
    #[max_len(MAX_ARTIST_ID_LEN)]
    pub artist_id: String,
    pub wallet: Pubkey,
    pub kind: ProofKind,
    /// Hash of what was checked (the email's signature, the bio code, a channel ID, a domain).
    pub source_hash: [u8; 32],
    pub verified_at: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub struct ProofSummary {
    pub kind: ProofKind,
    pub source_hash: [u8; 32],
    pub verified_at: i64,
}

impl From<&ProofRecord> for ProofSummary {
    fn from(r: &ProofRecord) -> Self {
        Self {
            kind: r.kind,
            source_hash: r.source_hash,
            verified_at: r.verified_at,
        }
    }
}

/// A compressed P-256 public key: a passkey.
pub type Passkey = [u8; 33];

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub struct Recovery {
    pub new_wallet: Pubkey,
    pub new_passkey: [u8; 33],
    pub requested_at: i64,
    pub effective_at: i64,
}

/// An artist's verified identity. Keyed by artist ID, so there's one per artist.
#[account]
#[derive(InitSpace)]
pub struct Passport {
    #[max_len(MAX_ARTIST_ID_LEN)]
    pub artist_id: String,
    pub wallet: Pubkey,
    pub passkey: [u8; 33],
    #[max_len(MAX_PASSPORT_PROOFS)]
    pub proofs: Vec<ProofSummary>,
    /// Counts passkey actions; part of every challenge, so no signature works twice.
    pub nonce: u64,
    pub recovery: Option<Recovery>,
    pub revoked: bool,
    pub issued_at: i64,
    pub withdraw_window_start: i64,
    pub withdrawn_in_window: u64,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Passport {
    pub fn has_kind(&self, kind: ProofKind) -> bool {
        self.proofs.iter().any(|p| p.kind == kind)
    }

    /// Adds or refreshes a proof.
    pub fn upsert_proof(&mut self, proof: ProofSummary) -> Result<()> {
        if let Some(existing) = self.proofs.iter_mut().find(|p| p.kind == proof.kind) {
            *existing = proof;
            return Ok(());
        }
        require!(
            self.proofs.len() < MAX_PASSPORT_PROOFS,
            crate::error::PassportError::TooManyProofs
        );
        self.proofs.push(proof);
        Ok(())
    }
}

/// Holds the artist's fees from every coin they claimed through the passport.
/// It is the "artist" on those coins, and only this program can pay out of it.
#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub passport: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum EndorsementStatus {
    Endorsed,
    Disavowed,
}

/// The artist's word on one coin, on any launchpad.
#[account]
#[derive(InitSpace)]
pub struct Endorsement {
    pub passport: Pubkey,
    pub mint: Pubkey,
    pub status: EndorsementStatus,
    pub updated_at: i64,
    pub bump: u8,
}
