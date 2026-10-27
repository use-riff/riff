use anchor_lang::prelude::*;

use crate::state::{EndorsementStatus, ProofKind};

#[event]
pub struct ProofRecorded {
    pub artist_id: String,
    pub wallet: Pubkey,
    pub kind: ProofKind,
}

#[event]
pub struct PassportIssued {
    pub passport: Pubkey,
    pub artist_id: String,
    pub wallet: Pubkey,
    pub proofs: Vec<ProofKind>,
}

#[event]
pub struct ProofsAdded {
    pub passport: Pubkey,
    pub proofs: Vec<ProofKind>,
}

#[event]
pub struct WalletChanged {
    pub passport: Pubkey,
    pub old_wallet: Pubkey,
    pub new_wallet: Pubkey,
}

#[event]
pub struct PasskeyChanged {
    pub passport: Pubkey,
}

#[event]
pub struct EndorsementChanged {
    pub passport: Pubkey,
    pub mint: Pubkey,
    pub status: EndorsementStatus,
}

#[event]
pub struct RecoveryRequested {
    pub passport: Pubkey,
    pub new_wallet: Pubkey,
    pub effective_at: i64,
}

#[event]
pub struct RecoveryVetoed {
    pub passport: Pubkey,
    pub by_passkey: bool,
    pub by_proof: Option<ProofKind>,
}

#[event]
pub struct RecoveryFinalized {
    pub passport: Pubkey,
    pub new_wallet: Pubkey,
}

#[event]
pub struct PassportRevoked {
    pub passport: Pubkey,
    pub reason: String,
}

#[event]
pub struct PassportReissued {
    pub passport: Pubkey,
    pub wallet: Pubkey,
    pub proofs: Vec<ProofKind>,
}

#[event]
pub struct CoinClaimed {
    pub passport: Pubkey,
    pub mint: Pubkey,
}

#[event]
pub struct VaultWithdrawn {
    pub passport: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub with_passkey: bool,
}
