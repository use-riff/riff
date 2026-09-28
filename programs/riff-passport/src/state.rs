use anchor_lang::prelude::*;

use crate::constants::*;

#[account]
#[derive(InitSpace)]
pub struct PassportConfig {
    pub admin: Pubkey,
    /// riff's verification service. It attests proofs checked off-chain
    /// (YouTube, the Spotify profile code, a website) and co-signs claims.
    pub verifier: Pubkey,
    /// Ethereum addresses of the Reclaim attestors whose signatures count.
    #[max_len(MAX_ATTESTORS)]
    pub attestors: Vec<[u8; 20]>,
    /// Reclaim's hash of riff's Spotify for Artists provider: only proofs
    /// from that provider count.
    pub reclaim_provider_hash: [u8; 32],
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
    /// A Reclaim (zkTLS) proof from the artist's Spotify for Artists login,
    /// verified by this program.
    SpotifyForArtists,
    /// A one-time code the artist put on their Spotify profile.
    SpotifyProfileCode,
    /// A DKIM-signed email from Spotify or a distributor (zkEmail).
    SpotifyEmail,
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
            Self::SpotifyForArtists | Self::SpotifyProfileCode | Self::SpotifyEmail => {
                Strength::Strong
            }
            Self::Website | Self::YouTube => Strength::Medium,
            Self::Instagram | Self::TikTok | Self::X => Strength::Weak,
        }
    }

    /// Proofs from the same place count once: three Spotify proofs are still one source.
    pub fn source(self) -> u8 {
        match self {
            Self::SpotifyForArtists | Self::SpotifyProfileCode | Self::SpotifyEmail => 0,
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

/// Enough proof for a passport: 2 or more sources, at least one strong.
pub fn proofs_suffice(kinds: &[ProofKind]) -> bool {
    let mut sources = 0u8;
    for kind in kinds {
        sources |= 1 << kind.source();
    }
    sources.count_ones() >= 2 && kinds.iter().any(|k| k.strength() == Strength::Strong)
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
    /// Hash of what was checked (a channel ID, a domain, a Reclaim claim).
    pub source_hash: [u8; 32],
    pub verified_at: i64,
    /// Checked by this program itself, not attested by riff's verifier.
    pub on_chain: bool,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub struct ProofSummary {
    pub kind: ProofKind,
    pub source_hash: [u8; 32],
    pub verified_at: i64,
    pub on_chain: bool,
}

impl From<&ProofRecord> for ProofSummary {
    fn from(r: &ProofRecord) -> Self {
        Self {
            kind: r.kind,
            source_hash: r.source_hash,
            verified_at: r.verified_at,
            on_chain: r.on_chain,
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

/// A Reclaim proof, uploaded in pieces before it's verified.
#[account]
#[derive(InitSpace)]
pub struct ProofBuffer {
    pub wallet: Pubkey,
    #[max_len(MAX_BUFFER_LEN)]
    pub data: Vec<u8>,
}
