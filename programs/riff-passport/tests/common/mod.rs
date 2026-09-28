#![allow(dead_code, clippy::result_large_err)]
//! Test helpers: riff and the passport program in LiteSVM, passkeys that
//! sign like a browser does, and a stand-in Reclaim attestor.

#[path = "../../../riff/tests/common/mod.rs"]
pub mod riff_env;

use anchor_lang::{
    prelude::{Clock, Pubkey},
    solana_program::{bpf_loader_upgradeable, instruction::Instruction, system_program},
    InstructionData, ToAccountMetas,
};
use base64::Engine;
use litesvm::types::TransactionResult;
use p256::ecdsa::{signature::Signer as _, Signature as P256Signature, SigningKey as P256Key};
use riff_passport::{PasskeyAction, PasskeyProof, ProofKind};
use sha2::{Digest, Sha256};
use solana_keypair::Keypair;
use solana_signer::Signer;

#[allow(unused_imports)]
pub use riff_env::{
    advance_time, fetch, funded_keypair, lamports, next_slot, send, send_many, SOL,
};

pub const ARTIST_ID: &str = "spotify:4Z8W4fKeB5YxbusRsdQVPb";
pub const SPOTIFY_ID: &str = "4Z8W4fKeB5YxbusRsdQVPb";
pub const RP_ID: &str = "riffpad.fun";
pub const RECOVERY_DELAY: i64 = 72 * 60 * 60;
pub const FREE_PER_DAY: u64 = SOL / 10;
pub const PROOF_MAX_AGE: i64 = 7 * 24 * 60 * 60;

pub struct Env {
    pub riff: riff_env::Env,
}

impl Env {
    pub fn svm(&mut self) -> &mut litesvm::LiteSVM {
        &mut self.riff.svm
    }
    pub fn verifier(&self) -> Keypair {
        self.riff.verifier.insecure_clone()
    }
}

pub fn passport_program_data() -> Pubkey {
    Pubkey::find_program_address(&[riff_passport::ID.as_ref()], &bpf_loader_upgradeable::ID).0
}
pub fn config_address() -> Pubkey {
    Pubkey::find_program_address(&[riff_passport::CONFIG_SEED], &riff_passport::ID).0
}
pub fn passport_address(artist_id: &str) -> Pubkey {
    Pubkey::find_program_address(
        &[riff_passport::PASSPORT_SEED, artist_id.as_bytes()],
        &riff_passport::ID,
    )
    .0
}
pub fn vault_address(passport: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[riff_passport::VAULT_SEED, passport.as_ref()],
        &riff_passport::ID,
    )
    .0
}
pub fn proof_address(artist_id: &str, wallet: &Pubkey, kind: ProofKind) -> Pubkey {
    Pubkey::find_program_address(
        &[
            riff_passport::PROOF_SEED,
            artist_id.as_bytes(),
            wallet.as_ref(),
            &[kind as u8],
        ],
        &riff_passport::ID,
    )
    .0
}
pub fn endorsement_address(passport: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            riff_passport::ENDORSEMENT_SEED,
            passport.as_ref(),
            mint.as_ref(),
        ],
        &riff_passport::ID,
    )
    .0
}

pub fn config_params(env: &Env) -> riff_passport::PassportConfigParams {
    riff_passport::PassportConfigParams {
        verifier: env.riff.verifier.pubkey(),
        rp_id_hash: Sha256::digest(RP_ID.as_bytes()).into(),
        recovery_delay: RECOVERY_DELAY,
        free_withdraw_per_day: FREE_PER_DAY,
        proof_max_age: PROOF_MAX_AGE,
    }
}

/// riff and the passport program deployed and configured.
pub fn setup() -> Env {
    let mut riff = riff_env::setup();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/riff_passport.so"
    ));
    riff.svm.add_program(riff_passport::ID, bytes).unwrap();
    // Make riff's admin the passport program's upgrade authority too.
    let mut account = riff.svm.get_account(&passport_program_data()).unwrap();
    account.data[12] = 1;
    account.data[13..45].copy_from_slice(riff.admin.pubkey().as_ref());
    riff.svm
        .set_account(passport_program_data(), account)
        .unwrap();
    riff_env::initialize_config(&mut riff);

    let mut env = Env { riff };
    let admin = env.riff.admin.insecure_clone();
    let ix = ix(
        riff_passport::instruction::InitializeConfig {
            params: config_params(&env),
        },
        riff_passport::accounts::InitializeConfig {
            admin: admin.pubkey(),
            config: config_address(),
            program: riff_passport::ID,
            program_data: passport_program_data(),
            system_program: system_program::ID,
        },
    );
    send(env.svm(), ix, &[&admin]).unwrap();
    env
}

pub fn ix(data: impl InstructionData, accounts: impl ToAccountMetas) -> Instruction {
    Instruction::new_with_bytes(
        riff_passport::ID,
        &data.data(),
        accounts.to_account_metas(None),
    )
}

pub fn assert_error(res: TransactionResult, expected: riff_passport::error::PassportError) {
    let err = res.expect_err("transaction should have failed");
    let code = u32::from(expected);
    assert!(
        format!("{:?}", err.err).contains(&format!("Custom({code})")),
        "expected {expected:?} ({code}), got {:?}\n{}",
        err.err,
        err.meta.logs.join("\n")
    );
}

// ---- proofs

pub fn record_proof_ix(
    env: &Env,
    wallet: &Pubkey,
    artist_id: &str,
    kind: ProofKind,
) -> Instruction {
    ix(
        riff_passport::instruction::RecordProof {
            artist_id: artist_id.into(),
            kind,
            source_hash: [kind as u8; 32],
        },
        riff_passport::accounts::RecordProof {
            wallet: *wallet,
            verifier: env.riff.verifier.pubkey(),
            config: config_address(),
            proof_record: proof_address(artist_id, wallet, kind),
            system_program: system_program::ID,
        },
    )
}

/// riff's verifier attests a proof for `wallet`.
pub fn record_proof(env: &mut Env, wallet: &Keypair, kind: ProofKind) -> TransactionResult {
    let verifier = env.verifier();
    let ix = record_proof_ix(env, &wallet.pubkey(), ARTIST_ID, kind);
    send(env.svm(), ix, &[wallet, &verifier])
}

pub fn now(env: &mut Env) -> i64 {
    env.svm().get_sysvar::<Clock>().unix_timestamp
}

// ---- passkeys

pub struct TestPasskey {
    pub key: P256Key,
    pub rp_id: String,
    pub verified: bool,
}

impl TestPasskey {
    pub fn new() -> Self {
        Self {
            key: P256Key::random(&mut rand_core::OsRng),
            rp_id: RP_ID.into(),
            verified: true,
        }
    }

    pub fn public(&self) -> [u8; 33] {
        self.key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .unwrap()
    }

    /// What a browser returns from navigator.credentials.get() for `challenge`,
    /// and the secp256r1 precompile instruction that checks the signature.
    pub fn sign(&self, challenge: &[u8; 32]) -> (PasskeyProof, Instruction) {
        let mut authenticator_data = Sha256::digest(self.rp_id.as_bytes()).to_vec();
        authenticator_data.push(if self.verified { 0x05 } else { 0x01 });
        authenticator_data.extend_from_slice(&1u32.to_be_bytes());
        let challenge_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(challenge);
        let client_data_json = format!(
            r#"{{"type":"webauthn.get","challenge":"{challenge_b64}","origin":"https://{}","crossOrigin":false}}"#,
            self.rp_id
        )
        .into_bytes();
        let mut message = authenticator_data.clone();
        message.extend_from_slice(&Sha256::digest(&client_data_json));
        let sig: P256Signature = self.key.sign(&message);
        let sig = sig.normalize_s().unwrap_or(sig);
        let ix = secp256r1_ix(&self.public(), &sig.to_bytes(), &message);
        (
            PasskeyProof {
                authenticator_data,
                client_data_json,
            },
            ix,
        )
    }

    pub fn approve(
        &self,
        passport: &Pubkey,
        nonce: u64,
        action: &PasskeyAction,
    ) -> (PasskeyProof, Instruction) {
        self.sign(&riff_passport::passkey::challenge(passport, nonce, action).unwrap())
    }
}

/// A secp256r1 precompile instruction verifying one signature, with all data inline.
pub fn secp256r1_ix(pubkey: &[u8; 33], signature: &[u8], message: &[u8]) -> Instruction {
    let key_off = 16u16;
    let sig_off = key_off + 33;
    let msg_off = sig_off + 64;
    let mut data = vec![1u8, 0];
    for v in [
        sig_off,
        u16::MAX,
        key_off,
        u16::MAX,
        msg_off,
        message.len() as u16,
        u16::MAX,
    ] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(pubkey);
    data.extend_from_slice(signature);
    data.extend_from_slice(message);
    Instruction::new_with_bytes(riff_passport::passkey::SECP256R1_PROGRAM_ID, &data, vec![])
}

pub fn nonce(env: &Env) -> u64 {
    fetch::<riff_passport::Passport>(&env.riff.svm, &passport_address(ARTIST_ID)).nonce
}

pub fn instructions_sysvar() -> Pubkey {
    solana_instructions_sysvar::ID
}

// ---- issuing

pub fn issue_ix(
    wallet: &Pubkey,
    passkey: [u8; 33],
    proof: PasskeyProof,
    records: &[Pubkey],
) -> Instruction {
    let passport = passport_address(ARTIST_ID);
    let mut ix = ix(
        riff_passport::instruction::IssuePassport {
            artist_id: ARTIST_ID.into(),
            passkey,
            passkey_proof: proof,
        },
        riff_passport::accounts::IssuePassport {
            wallet: *wallet,
            config: config_address(),
            passport,
            vault: vault_address(&passport),
            instructions: instructions_sysvar(),
            system_program: system_program::ID,
        },
    );
    ix.accounts.extend(
        records.iter().map(|r| {
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(*r, false)
        }),
    );
    ix
}

/// Issues a passport from the given proof kinds (all attested by the verifier).
pub fn issue(
    env: &mut Env,
    wallet: &Keypair,
    passkey: &TestPasskey,
    kinds: &[ProofKind],
) -> TransactionResult {
    for kind in kinds {
        record_proof(env, wallet, *kind).unwrap();
    }
    let records: Vec<Pubkey> = kinds
        .iter()
        .map(|k| proof_address(ARTIST_ID, &wallet.pubkey(), *k))
        .collect();
    let passport = passport_address(ARTIST_ID);
    let (proof, pre) = passkey.approve(
        &passport,
        0,
        &PasskeyAction::Register {
            passkey: passkey.public(),
        },
    );
    let ix = issue_ix(&wallet.pubkey(), passkey.public(), proof, &records);
    send_many(env.svm(), &[pre, ix], &[wallet])
}

pub const DEFAULT_KINDS: [ProofKind; 2] = [ProofKind::SpotifyProfileCode, ProofKind::YouTube];

/// A passport issued to a fresh wallet with a fresh passkey.
pub fn issued(env: &mut Env) -> (Keypair, TestPasskey) {
    let wallet = funded_keypair(env.svm());
    let passkey = TestPasskey::new();
    issue(env, &wallet, &passkey, &DEFAULT_KINDS).unwrap();
    (wallet, passkey)
}

pub fn act_accounts(wallet: &Pubkey) -> riff_passport::accounts::PasskeyAct {
    riff_passport::accounts::PasskeyAct {
        wallet: *wallet,
        config: config_address(),
        passport: passport_address(ARTIST_ID),
        instructions: instructions_sysvar(),
    }
}

pub fn endorsement_accounts(
    wallet: &Pubkey,
    mint: &Pubkey,
) -> riff_passport::accounts::SetEndorsement {
    let passport = passport_address(ARTIST_ID);
    riff_passport::accounts::SetEndorsement {
        wallet: *wallet,
        config: config_address(),
        passport,
        endorsement: endorsement_address(&passport, mint),
        instructions: instructions_sysvar(),
        system_program: system_program::ID,
    }
}

// ---- riff coins

/// A riff coin for the passport's artist, with some trades so it has artist fees.
pub fn coin_with_fees(env: &mut Env, buys: u64) -> Pubkey {
    let creator = funded_keypair(env.svm());
    let mint = Keypair::new();
    let mut args = riff_env::coin_args();
    args.artist_id = ARTIST_ID.into();
    let ix = riff_env::create_coin_ix(&creator.pubkey(), &mint.pubkey(), args);
    send(env.svm(), ix, &[&creator, &mint]).unwrap();
    next_slot(env.svm());
    let mint = mint.pubkey();
    let buyer = riff_env::trader(env.svm(), &mint, 1_000 * SOL);
    for _ in 0..buys {
        let ix = riff_env::buy_ix(&buyer.pubkey(), &mint, 10 * SOL, 1);
        send(env.svm(), ix, &[&buyer]).unwrap();
    }
    mint
}

pub fn claim_coin_ix(env: &Env, wallet: &Pubkey, mint: &Pubkey) -> Instruction {
    let passport = passport_address(ARTIST_ID);
    ix(
        riff_passport::instruction::ClaimCoin {},
        riff_passport::accounts::ClaimCoin {
            wallet: *wallet,
            verifier: env.riff.verifier.pubkey(),
            passport,
            vault: vault_address(&passport),
            riff_config: riff_env::config_address(),
            coin: riff_env::coin_address(mint),
            riff_program: riff::ID,
        },
    )
}

pub fn collect_ix(mint: &Pubkey) -> Instruction {
    let passport = passport_address(ARTIST_ID);
    ix(
        riff_passport::instruction::CollectFees {},
        riff_passport::accounts::CollectFees {
            passport,
            vault: vault_address(&passport),
            coin: riff_env::coin_address(mint),
            riff_program: riff::ID,
        },
    )
}

pub fn withdraw_ix(wallet: &Pubkey, amount: u64, proof: Option<PasskeyProof>) -> Instruction {
    let passport = passport_address(ARTIST_ID);
    ix(
        riff_passport::instruction::Withdraw {
            amount,
            passkey_proof: proof,
        },
        riff_passport::accounts::WithdrawVault {
            wallet: *wallet,
            config: config_address(),
            passport,
            vault: vault_address(&passport),
            instructions: instructions_sysvar(),
        },
    )
}
