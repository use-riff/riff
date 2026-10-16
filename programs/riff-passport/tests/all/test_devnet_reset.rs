//! The devnet-only reset (built with `--features devnet`). Run with:
//!   make build-passport-devnet && cargo test -p riff-passport --features devnet --test all test_devnet_reset

use crate::common::*;
use anchor_lang::{
    prelude::Pubkey,
    solana_program::instruction::{AccountMeta, Instruction},
};
use riff_passport::{error::PassportError as E, ProofKind};
use sha2::{Digest, Sha256};
use solana_keypair::Keypair;
use solana_signer::Signer;

/// riff and the devnet build of the passport program, configured.
fn devnet_setup() -> Env {
    let mut env = setup();
    let bytes = std::fs::read(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../devnet/deploy/riff_passport.so"
    ))
    .expect(
        "target/devnet/deploy/riff_passport.so is missing: run `make build-passport-devnet` first",
    );
    env.svm().add_program(riff_passport::ID, &bytes).unwrap();
    env
}

fn reset_ix(admin: &Pubkey) -> Instruction {
    let passport = passport_address(ARTIST_ID);
    let discriminator = &Sha256::digest(b"global:reset_passport")[..8];
    Instruction::new_with_bytes(
        riff_passport::ID,
        discriminator,
        vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(config_address(), false),
            AccountMeta::new(passport, false),
            AccountMeta::new(vault_address(&passport), false),
        ],
    )
}

#[test]
fn the_admin_resets_a_passport_and_the_artist_gets_a_new_one() {
    let mut env = devnet_setup();
    let (wallet, _) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    let vault = vault_address(&passport);
    let coin = coin_with_fees(&mut env, 2);
    let verifier = env.verifier();
    let claim = claim_coin_ix(&env, &wallet.pubkey(), &coin);
    send(env.svm(), claim, &[&wallet, &verifier]).unwrap();

    // Only riff's admin can reset.
    let stranger = funded_keypair(env.svm());
    assert_error(
        send(env.svm(), reset_ix(&stranger.pubkey()), &[&stranger]),
        E::NotAdmin,
    );
    let admin = env.riff.admin.insecure_clone();
    send(env.svm(), reset_ix(&admin.pubkey()), &[&admin]).unwrap();
    assert!(env
        .svm()
        .get_account(&passport)
        .is_none_or(|a| a.lamports == 0));
    assert!(env
        .svm()
        .get_account(&vault)
        .is_none_or(|a| a.lamports == 0));

    // The artist proves it again and gets a passport at the same address.
    next_slot(env.svm());
    let again = TestPasskey::new();
    issue(
        &mut env,
        &wallet,
        &again,
        &[ProofKind::SpotifyEmail, ProofKind::Distributor],
    )
    .unwrap();
    let p: riff_passport::Passport = fetch(&env.riff.svm, &passport);
    assert_eq!(p.passkey, again.public());

    // The coin claimed before still pays the (new) vault.
    let payer = Keypair::new();
    env.svm().airdrop(&payer.pubkey(), SOL).unwrap();
    send(env.svm(), collect_ix(&coin), &[&payer]).unwrap();
    assert!(lamports(&env.riff.svm, &vault) > 0);
}
