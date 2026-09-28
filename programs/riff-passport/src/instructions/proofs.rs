use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::ProofRecorded,
    instructions::shared::spotify_id,
    reclaim::{self, ReclaimProof},
    state::{PassportConfig, ProofBuffer, ProofKind, ProofRecord},
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

/// A proof riff's verifier checked off-chain (YouTube login, the Spotify
/// profile code, a website's DNS). The verifier's co-signature is the
/// attestation; the wallet's signature binds the proof to that wallet.
pub fn handle_record_proof(
    ctx: Context<RecordProof>,
    artist_id: String,
    kind: ProofKind,
    source_hash: [u8; 32],
) -> Result<()> {
    spotify_id(&artist_id)?;
    require!(
        kind != ProofKind::SpotifyForArtists,
        PassportError::OnChainOnly
    );
    write_record(
        &mut ctx.accounts.proof_record,
        artist_id,
        ctx.accounts.wallet.key(),
        kind,
        source_hash,
        false,
        ctx.bumps.proof_record,
    )
}

fn write_record(
    record: &mut ProofRecord,
    artist_id: String,
    wallet: Pubkey,
    kind: ProofKind,
    source_hash: [u8; 32],
    on_chain: bool,
    bump: u8,
) -> Result<()> {
    record.artist_id = artist_id.clone();
    record.wallet = wallet;
    record.kind = kind;
    record.source_hash = source_hash;
    record.verified_at = Clock::get()?.unix_timestamp;
    record.on_chain = on_chain;
    record.bump = bump;
    emit!(ProofRecorded {
        artist_id,
        wallet,
        kind,
        on_chain
    });
    Ok(())
}

#[derive(Accounts)]
pub struct WriteBuffer<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + ProofBuffer::INIT_SPACE,
        seeds = [BUFFER_SEED, wallet.key().as_ref()],
        bump
    )]
    pub buffer: Account<'info, ProofBuffer>,
    pub system_program: Program<'info, System>,
}

/// Uploads part of a Reclaim proof. Writing at offset 0 starts over.
pub fn handle_write_buffer(ctx: Context<WriteBuffer>, offset: u32, bytes: Vec<u8>) -> Result<()> {
    let buffer = &mut ctx.accounts.buffer;
    buffer.wallet = ctx.accounts.wallet.key();
    let start = offset as usize;
    let end = start
        .checked_add(bytes.len())
        .ok_or(error!(PassportError::BufferOverflow))?;
    require!(end <= MAX_BUFFER_LEN, PassportError::BufferOverflow);
    if start == 0 {
        buffer.data.clear();
    }
    require!(start <= buffer.data.len(), PassportError::BufferOverflow);
    if buffer.data.len() < end {
        buffer.data.resize(end, 0);
    }
    buffer.data[start..end].copy_from_slice(&bytes);
    Ok(())
}

#[derive(Accounts)]
#[instruction(artist_id: String)]
pub struct ProveSpotifyForArtists<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(mut, close = wallet, seeds = [BUFFER_SEED, wallet.key().as_ref()], bump, has_one = wallet)]
    pub buffer: Account<'info, ProofBuffer>,
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + ProofRecord::INIT_SPACE,
        seeds = [PROOF_SEED, artist_id.as_bytes(), wallet.key().as_ref(), &[ProofKind::SpotifyForArtists as u8]],
        bump
    )]
    pub proof_record: Account<'info, ProofRecord>,
    pub system_program: Program<'info, System>,
}

/// Verifies the uploaded Reclaim proof from Spotify for Artists on-chain:
/// no trust in riff's servers.
pub fn handle_prove_spotify_for_artists(
    ctx: Context<ProveSpotifyForArtists>,
    artist_id: String,
) -> Result<()> {
    let spotify = spotify_id(&artist_id)?.to_string();
    let proof = ReclaimProof::deserialize(&mut &ctx.accounts.buffer.data[..])
        .map_err(|_| error!(PassportError::ReclaimMalformed))?;
    let now = Clock::get()?.unix_timestamp;
    let wallet = ctx.accounts.wallet.key();
    let identifier = reclaim::verify(&proof, &ctx.accounts.config, &wallet, &spotify, now)?;
    write_record(
        &mut ctx.accounts.proof_record,
        artist_id,
        wallet,
        ProofKind::SpotifyForArtists,
        identifier,
        true,
        ctx.bumps.proof_record,
    )
}
