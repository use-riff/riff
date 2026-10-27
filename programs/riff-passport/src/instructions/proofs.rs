use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::ProofRecorded,
    instructions::shared::spotify_id,
    state::{PassportConfig, ProofKind, ProofRecord},
};

#[derive(Accounts)]
#[instruction(artist_id: String, kind: ProofKind)]
pub struct RecordProof<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(address = config.verifier @ PassportError::NotVerifier)]
    pub verifier: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + ProofRecord::INIT_SPACE,
        seeds = [PROOF_SEED, artist_id.as_bytes(), wallet.key().as_ref(), &[kind as u8]],
        bump
    )]
    pub proof_record: Account<'info, ProofRecord>,
    pub system_program: Program<'info, System>,
}

/// A proof riff's verifier checked off-chain: Spotify's signature on a
/// Spotify for Artists email, the wallet's code in the Spotify bio, a
/// YouTube login, a website's DNS. The verifier's co-signature is the
/// attestation; the wallet's signature binds the proof to that wallet.
pub fn handle_record_proof(
    ctx: Context<RecordProof>,
    artist_id: String,
    kind: ProofKind,
    source_hash: [u8; 32],
) -> Result<()> {
    spotify_id(&artist_id)?;
    let wallet = ctx.accounts.wallet.key();
    let record = &mut ctx.accounts.proof_record;
    record.artist_id = artist_id.clone();
    record.wallet = wallet;
    record.kind = kind;
    record.source_hash = source_hash;
    record.verified_at = Clock::get()?.unix_timestamp;
    record.bump = ctx.bumps.proof_record;
    emit!(ProofRecorded {
        artist_id,
        wallet,
        kind
    });
    Ok(())
}
