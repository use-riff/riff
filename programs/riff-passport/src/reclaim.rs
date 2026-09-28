//! Reclaim (zkTLS) proofs, verified on-chain.
//!
//! A Reclaim attestor watches the artist's own TLS session with Spotify for
//! Artists and signs a claim about what the site returned, here the Spotify
//! artist ID the logged-in account manages. The attestor signs, Ethereum
//! style (EIP-191, secp256k1), the text
//! `identifier \n owner \n timestamp \n epoch`, where the identifier is the
//! keccak256 of `provider \n parameters \n context`. This program recomputes
//! the identifier from the claim, recovers the signer and checks it's a
//! trusted attestor, then reads the context: the provider must be riff's,
//! the proof must be made for this wallet, and the extracted artist ID must
//! match.
//!
//! Why the context can be trusted: the attestor itself overwrites
//! `providerHash` with the hash of the request it checked (URL, method,
//! response matches and redactions), and writes `extractedParameters` from
//! what it extracted. Pinning the provider hash therefore pins where the
//! artist ID came from. For that to work, riff's provider must use a Spotify
//! for Artists request whose URL is the same for every artist (the list of
//! artists the account manages), so the hash is too.

use anchor_lang::prelude::*;

use crate::{error::PassportError, state::PassportConfig};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ReclaimProof {
    pub provider: String,
    pub parameters: String,
    /// The claim's context, canonicalized (keys sorted) as Reclaim hashes it.
    pub context: String,
    pub owner: [u8; 20],
    pub timestamp_s: u32,
    pub epoch: u32,
    /// r || s || v, as Reclaim attestors sign.
    pub signature: [u8; 65],
}

/// What the proof's context must say for riff to accept it.
pub fn context_message(wallet: &Pubkey) -> String {
    format!("riff-passport:{wallet}")
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

/// The claim identifier Reclaim computes from the claim info.
pub fn identifier(proof: &ReclaimProof) -> [u8; 32] {
    solana_keccak_hasher::hashv(&[
        proof.provider.as_bytes(),
        b"\n",
        proof.parameters.as_bytes(),
        b"\n",
        proof.context.as_bytes(),
    ])
    .to_bytes()
}

/// The Ethereum address that signed the claim.
pub fn signer(proof: &ReclaimProof, identifier: &[u8; 32]) -> Result<[u8; 20]> {
    let data = format!(
        "0x{}\n0x{}\n{}\n{}",
        hex(identifier),
        hex(&proof.owner),
        proof.timestamp_s,
        proof.epoch
    );
    let prefix = format!("\x19Ethereum Signed Message:\n{}", data.len());
    let digest = solana_keccak_hasher::hashv(&[prefix.as_bytes(), data.as_bytes()]).to_bytes();
    let v = proof.signature[64];
    let recovery_id = if v >= 27 { v - 27 } else { v };
    let key =
        solana_secp256k1_recover::secp256k1_recover(&digest, recovery_id, &proof.signature[..64])
            .map_err(|_| error!(PassportError::ReclaimUntrustedAttestor))?;
    let hash = solana_keccak_hasher::hash(&key.to_bytes()).to_bytes();
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    Ok(address)
}

/// Verifies the proof for `wallet` and the Spotify artist `spotify_id`.
/// Returns the claim identifier, which becomes the proof's source hash.
pub fn verify(
    proof: &ReclaimProof,
    config: &PassportConfig,
    wallet: &Pubkey,
    spotify_id: &str,
    now: i64,
) -> Result<[u8; 32]> {
    let identifier = identifier(proof);
    let signer = signer(proof, &identifier)?;
    require!(
        config.attestors.contains(&signer),
        PassportError::ReclaimUntrustedAttestor
    );

    let issued = proof.timestamp_s as i64;
    require!(
        issued <= now + 300 && now - issued <= config.proof_max_age,
        PassportError::ProofExpired
    );

    let context: serde_json::Value = serde_json::from_str(&proof.context)
        .map_err(|_| error!(PassportError::ReclaimMalformed))?;
    let provider_hash = format!("0x{}", hex(&config.reclaim_provider_hash));
    require!(
        context["providerHash"]
            .as_str()
            .map(str::to_ascii_lowercase)
            == Some(provider_hash),
        PassportError::ReclaimWrongProvider
    );
    require!(
        context["contextMessage"].as_str() == Some(context_message(wallet).as_str()),
        PassportError::ReclaimWrongWallet
    );
    require!(
        context["extractedParameters"]["artistId"].as_str() == Some(spotify_id),
        PassportError::ReclaimWrongArtist
    );
    Ok(identifier)
}
