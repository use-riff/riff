//! Artist Passport: a public, hack-resistant identity for musicians on Solana.
//!
//! An artist proves who they are with at least two independent proofs, one
//! of them strong (a zero-knowledge proof from Spotify for Artists, checked
//! here on-chain). Every sensitive action then needs a second factor that
//! Solana itself checks: a passkey. The artist's riff earnings belong to the
//! passport's vault, and any app can read which coins the artist endorses.

pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod passkey;
pub mod reclaim;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use passkey::{PasskeyAction, PasskeyProof};
pub use reclaim::ReclaimProof;
pub use state::*;

declare_id!("2nke6euXvAnbtdtcbbk8N2Z3SRLuR7i67VYmSdcY5kwj");

#[program]
pub mod riff_passport {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        params: PassportConfigParams,
    ) -> Result<()> {
        admin::handle_initialize_config(ctx, params)
    }

    pub fn update_config(ctx: Context<UpdateConfig>, params: PassportConfigParams) -> Result<()> {
        admin::handle_update_config(ctx, params)
    }

    pub fn revoke_passport(ctx: Context<RevokePassport>, reason: String) -> Result<()> {
        admin::handle_revoke_passport(ctx, reason)
    }

    pub fn record_proof(
        ctx: Context<RecordProof>,
        artist_id: String,
        kind: ProofKind,
        source_hash: [u8; 32],
    ) -> Result<()> {
        proofs::handle_record_proof(ctx, artist_id, kind, source_hash)
    }

    pub fn write_buffer(ctx: Context<WriteBuffer>, offset: u32, bytes: Vec<u8>) -> Result<()> {
        proofs::handle_write_buffer(ctx, offset, bytes)
    }

    pub fn prove_spotify_for_artists(
        ctx: Context<ProveSpotifyForArtists>,
        artist_id: String,
    ) -> Result<()> {
        proofs::handle_prove_spotify_for_artists(ctx, artist_id)
    }

    pub fn issue_passport<'info>(
        ctx: Context<'info, IssuePassport<'info>>,
        artist_id: String,
        passkey: Passkey,
        passkey_proof: PasskeyProof,
    ) -> Result<()> {
        passport::handle_issue_passport(ctx, artist_id, passkey, passkey_proof)
    }

    pub fn add_proofs<'info>(
        ctx: Context<'info, PasskeyAct<'info>>,
        passkey_proof: PasskeyProof,
    ) -> Result<()> {
        passport::handle_add_proofs(ctx, passkey_proof)
    }

    pub fn set_wallet(ctx: Context<SetWallet>, passkey_proof: PasskeyProof) -> Result<()> {
        passport::handle_set_wallet(ctx, passkey_proof)
    }

    pub fn set_passkey(
        ctx: Context<PasskeyAct>,
        new_passkey: Passkey,
        old_proof: PasskeyProof,
        new_proof: PasskeyProof,
    ) -> Result<()> {
        passport::handle_set_passkey(ctx, new_passkey, old_proof, new_proof)
    }

    pub fn endorse(
        ctx: Context<SetEndorsement>,
        mint: Pubkey,
        passkey_proof: PasskeyProof,
    ) -> Result<()> {
        passport::handle_endorse(ctx, mint, passkey_proof)
    }

    pub fn disavow(ctx: Context<SetEndorsement>, mint: Pubkey) -> Result<()> {
        passport::handle_disavow(ctx, mint)
    }

    pub fn request_recovery<'info>(
        ctx: Context<'info, RequestRecovery<'info>>,
        new_passkey: Passkey,
        passkey_proof: PasskeyProof,
    ) -> Result<()> {
        recovery::handle_request_recovery(ctx, new_passkey, passkey_proof)
    }

    pub fn veto_recovery<'info>(
        ctx: Context<'info, VetoRecovery<'info>>,
        passkey_proof: Option<PasskeyProof>,
    ) -> Result<()> {
        recovery::handle_veto_recovery(ctx, passkey_proof)
    }

    pub fn finalize_recovery(ctx: Context<FinalizeRecovery>) -> Result<()> {
        recovery::handle_finalize_recovery(ctx)
    }

    pub fn claim_coin(ctx: Context<ClaimCoin>) -> Result<()> {
        vault::handle_claim_coin(ctx)
    }

    pub fn collect_fees(ctx: Context<CollectFees>) -> Result<()> {
        vault::handle_collect_fees(ctx)
    }

    pub fn withdraw(
        ctx: Context<WithdrawVault>,
        amount: u64,
        passkey_proof: Option<PasskeyProof>,
    ) -> Result<()> {
        vault::handle_withdraw(ctx, amount, passkey_proof)
    }
}
