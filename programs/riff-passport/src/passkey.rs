//! Passkeys (WebAuthn, P-256) as a second factor, checked by Solana itself.
//!
//! The browser signs `authenticatorData || sha256(clientDataJSON)` with the
//! passkey. The transaction carries that signature in an instruction for
//! Solana's secp256r1 precompile, which fails the whole transaction if the
//! signature is wrong. This program then checks that the precompile verified
//! *this* passkey over *this* action: the challenge inside clientDataJSON
//! must be the hash of the action and the passport's nonce.

use anchor_lang::prelude::*;
use solana_instructions_sysvar::load_instruction_at_checked;

use crate::{constants::CHALLENGE_DOMAIN, error::PassportError, state::Passkey};

pub const SECP256R1_PROGRAM_ID: Pubkey = pubkey!("Secp256r1SigVerify1111111111111111111111111");

/// What the passkey approved. Hashed into the challenge.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PasskeyAction {
    /// Proves the artist holds a new passkey before it's stored.
    Register {
        passkey: Passkey,
    },
    SetWallet {
        new_wallet: Pubkey,
    },
    SetPasskey {
        new_passkey: Passkey,
    },
    Endorse {
        mint: Pubkey,
    },
    Withdraw {
        to: Pubkey,
        amount: u64,
    },
    AddProofs {
        records: Vec<Pubkey>,
    },
    Veto {
        new_wallet: Pubkey,
    },
}

/// The parts of a WebAuthn assertion the program needs; the signature and
/// public key travel in the precompile instruction.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PasskeyProof {
    pub authenticator_data: Vec<u8>,
    pub client_data_json: Vec<u8>,
}

/// The 32-byte challenge the passkey must sign for `action`.
pub fn challenge(passport: &Pubkey, nonce: u64, action: &PasskeyAction) -> Result<[u8; 32]> {
    let mut encoded = Vec::new();
    action.serialize(&mut encoded)?;
    let action = encoded;
    Ok(solana_sha256_hasher::hashv(&[
        CHALLENGE_DOMAIN,
        passport.as_ref(),
        &nonce.to_le_bytes(),
        &action,
    ])
    .to_bytes())
}

const FLAG_USER_PRESENT: u8 = 0x01;
const FLAG_USER_VERIFIED: u8 = 0x04;

/// Checks that this transaction proves `passkey` signed `expected` on the riff site.
pub fn verify(
    instructions: &AccountInfo,
    passkey: &Passkey,
    rp_id_hash: &[u8; 32],
    expected: &[u8; 32],
    proof: &PasskeyProof,
) -> Result<()> {
    let auth = &proof.authenticator_data;
    require!(auth.len() >= 37, PassportError::PasskeyMalformed);
    require!(
        auth[..32] == rp_id_hash[..],
        PassportError::PasskeyWrongSite
    );
    let flags = auth[32];
    require!(
        flags & FLAG_USER_PRESENT != 0 && flags & FLAG_USER_VERIFIED != 0,
        PassportError::PasskeyNotVerified
    );

    let client: serde_json::Value = serde_json::from_slice(&proof.client_data_json)
        .map_err(|_| error!(PassportError::PasskeyMalformed))?;
    require!(
        client["type"].as_str() == Some("webauthn.get"),
        PassportError::PasskeyMalformed
    );
    let signed = client["challenge"]
        .as_str()
        .and_then(base64url_decode)
        .ok_or(error!(PassportError::PasskeyMalformed))?;
    require!(
        signed[..] == expected[..],
        PassportError::PasskeyWrongChallenge
    );

    let mut message = auth.clone();
    message.extend_from_slice(&solana_sha256_hasher::hash(&proof.client_data_json).to_bytes());
    require!(
        precompile_verified(instructions, passkey, &message)?,
        PassportError::PasskeyMissing
    );
    Ok(())
}

/// Whether a secp256r1 precompile instruction in this transaction verified
/// `passkey` over `message`. Only entries whose data sits in the precompile
/// instruction itself count, so offsets can't point somewhere misleading.
fn precompile_verified(
    instructions: &AccountInfo,
    passkey: &Passkey,
    message: &[u8],
) -> Result<bool> {
    let mut index = 0usize;
    while let Ok(ix) = load_instruction_at_checked(index, instructions) {
        index += 1;
        if ix.program_id != SECP256R1_PROGRAM_ID {
            continue;
        }
        let data = &ix.data;
        let count = *data.first().unwrap_or(&0) as usize;
        for i in 0..count {
            let at = 2 + i * 14;
            let Some(offsets) = data.get(at..at + 14) else {
                break;
            };
            let read = |j: usize| u16::from_le_bytes([offsets[j], offsets[j + 1]]);
            let (sig_ix, key_off, key_ix, msg_off, msg_len, msg_ix) = (
                read(2),
                read(4) as usize,
                read(6),
                read(8) as usize,
                read(10) as usize,
                read(12),
            );
            if sig_ix != u16::MAX || key_ix != u16::MAX || msg_ix != u16::MAX {
                continue;
            }
            if data.get(key_off..key_off + 33) == Some(&passkey[..])
                && data.get(msg_off..msg_off + msg_len) == Some(message)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Unpadded base64url, as WebAuthn writes challenges.
pub fn base64url_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'=' => break,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The same vectors are in riff-site's client tests, so the browser and
    /// the program agree on every challenge byte.
    #[test]
    fn challenge_vectors() {
        let passport = Pubkey::new_from_array([1; 32]);
        let withdraw = PasskeyAction::Withdraw {
            to: Pubkey::new_from_array([2; 32]),
            amount: 5,
        };
        assert_eq!(
            hex(&challenge(&passport, 7, &withdraw).unwrap()),
            "ddc9265a889d722217397f0938acd066a8768e63e9ba4e7a5b4e62d7e47d9a0d"
        );
        let register = PasskeyAction::Register { passkey: [3; 33] };
        assert_eq!(
            hex(&challenge(&passport, 0, &register).unwrap()),
            "e4eabeee9c2653c111ed24e5ce0bad8e3fe19c3ee299ed781993437a2c219a0e"
        );
        let add = PasskeyAction::AddProofs {
            records: vec![
                Pubkey::new_from_array([4; 32]),
                Pubkey::new_from_array([5; 32]),
            ],
        };
        assert_eq!(
            hex(&challenge(&passport, 2, &add).unwrap()),
            "a2db009f1680c1a5e24852e0299d1d3d788f69f911d1b54748fa2237dc4845d5"
        );
    }

    #[test]
    fn decodes_base64url() {
        assert_eq!(base64url_decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(base64url_decode("_-8").unwrap(), vec![0xff, 0xef]);
        assert!(base64url_decode("a+b").is_none());
    }
}
