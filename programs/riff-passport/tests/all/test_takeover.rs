//! A passport made from a hacked inbox: it waits out a probation before it
//! can vouch for a coin or move money, and riff can give it back to the real
//! artist without losing what was claimed into its vault.

use crate::common::*;
use anchor_lang::{prelude::Pubkey, solana_program::instruction::Instruction};
use riff_passport::{error::PassportError as E, PasskeyAction, ProofKind};
use solana_keypair::Keypair;
use solana_signer::Signer;

fn endorse(env: &Env, wallet: &Pubkey, passkey: &TestPasskey, mint: Pubkey) -> [Instruction; 2] {
    let (proof, pre) = passkey.approve(
        &passport_address(ARTIST_ID),
        nonce(env),
        &PasskeyAction::Endorse { mint },
    );
    let ix = ix(
        riff_passport::instruction::Endorse {
            mint,
            passkey_proof: proof,
        },
        endorsement_accounts(wallet, &mint),
    );
    [pre, ix]
}

fn withdraw(env: &Env, wallet: &Pubkey, passkey: &TestPasskey, amount: u64) -> [Instruction; 2] {
    let (proof, pre) = passkey.approve(
        &passport_address(ARTIST_ID),
        nonce(env),
        &PasskeyAction::Withdraw {
            to: *wallet,
            amount,
        },
    );
    [pre, withdraw_ix(wallet, amount, Some(proof))]
}

/// The artist's proofs from `wallet`, ready to pass to issue or reissue.
fn proofs_for(env: &mut Env, wallet: &Keypair, kinds: &[ProofKind]) -> Vec<Pubkey> {
    kinds
        .iter()
        .map(|k| {
            record_proof(env, wallet, *k).unwrap();
            proof_address(ARTIST_ID, &wallet.pubkey(), *k)
        })
        .collect()
}

#[allow(clippy::result_large_err)]
fn reissue(
    env: &mut Env,
    admin: &Keypair,
    wallet: &Keypair,
    passkey: &TestPasskey,
    records: &[Pubkey],
) -> litesvm::types::TransactionResult {
    let (proof, pre) = passkey.approve(
        &passport_address(ARTIST_ID),
        nonce(env),
        &PasskeyAction::Register {
            passkey: passkey.public(),
        },
    );
    let ix = reissue_ix(
        &admin.pubkey(),
        &wallet.pubkey(),
        passkey.public(),
        proof,
        records,
    );
    send_many(env.svm(), &[pre, ix], &[admin, wallet])
}

#[test]
fn a_new_passport_cant_endorse_or_withdraw_until_its_probation_ends() {
    let mut env = setup();
    let wallet = funded_keypair(env.svm());
    let passkey = TestPasskey::new();
    issue(&mut env, &wallet, &passkey, &DEFAULT_KINDS).unwrap();
    let passport = passport_address(ARTIST_ID);
    let vault = vault_address(&passport);
    let mint = Keypair::new().pubkey();

    let ixs = endorse(&env, &wallet.pubkey(), &passkey, mint);
    assert_error(send_many(env.svm(), &ixs, &[&wallet]), E::OnProbation);
    env.svm().airdrop(&vault, SOL).unwrap();
    let small = withdraw_ix(&wallet.pubkey(), 1000, None);
    assert_error(send(env.svm(), small.clone(), &[&wallet]), E::OnProbation);

    // Saying "not mine" is always safe, and claiming only moves money into the vault.
    let disavow = ix(
        riff_passport::instruction::Disavow { mint },
        endorsement_accounts(&wallet.pubkey(), &mint),
    );
    send(env.svm(), disavow, &[&wallet]).unwrap();
    let coin = coin_with_fees(&mut env, 1);
    let verifier = env.verifier();
    let claim = claim_coin_ix(&env, &wallet.pubkey(), &coin);
    send(env.svm(), claim, &[&wallet, &verifier]).unwrap();

    // One second before the end: still waiting. Then both work.
    advance_time(env.svm(), RECOVERY_DELAY - 1);
    assert_error(send(env.svm(), small.clone(), &[&wallet]), E::OnProbation);
    advance_time(env.svm(), 1);
    send(env.svm(), small, &[&wallet]).unwrap();
    let ixs = endorse(&env, &wallet.pubkey(), &passkey, mint);
    send_many(env.svm(), &ixs, &[&wallet]).unwrap();
}

#[test]
fn riff_and_the_real_artist_take_back_a_squatted_passport() {
    let mut env = setup();
    // Someone with the artist's inbox gets the passport first and claims a coin.
    let squatter = funded_keypair(env.svm());
    let squatter_key = TestPasskey::new();
    issue(&mut env, &squatter, &squatter_key, &DEFAULT_KINDS).unwrap();
    let passport = passport_address(ARTIST_ID);
    let vault = vault_address(&passport);
    let coin = coin_with_fees(&mut env, 3);
    let verifier = env.verifier();
    let claim = claim_coin_ix(&env, &squatter.pubkey(), &coin);
    send(env.svm(), claim, &[&squatter, &verifier]).unwrap();
    let payer = funded_keypair(env.svm());
    send(env.svm(), collect_ix(&coin), &[&payer]).unwrap();
    let held = lamports(&env.riff.svm, &vault);

    // Still on probation, so nothing could leave before riff stepped in.
    revoke(&mut env);

    let artist = funded_keypair(env.svm());
    let artist_key = TestPasskey::new();
    let records = proofs_for(&mut env, &artist, &DEFAULT_KINDS);
    let admin = env.riff.admin.insecure_clone();

    // riff's admin must sign: the artist alone can't take a revoked passport.
    let stranger = funded_keypair(env.svm());
    assert_error(
        reissue(&mut env, &stranger, &artist, &artist_key, &records),
        E::NotAdmin,
    );
    reissue(&mut env, &admin, &artist, &artist_key, &records).unwrap();

    let p: riff_passport::Passport = fetch(&env.riff.svm, &passport);
    assert_eq!(p.wallet, artist.pubkey());
    assert_eq!(p.passkey, artist_key.public());
    assert!(!p.revoked);
    assert!(p.recovery.is_none());
    // Same vault: what was claimed is the artist's, and the coin still pays it.
    assert_eq!(lamports(&env.riff.svm, &vault), held);
    let c: riff::Coin = fetch(&env.riff.svm, &crate::common::riff_env::coin_address(&coin));
    assert_eq!(c.artist, Some(vault));

    // The squatter is out, even with their passkey.
    let ixs = withdraw(&env, &squatter.pubkey(), &squatter_key, 1000);
    assert_error(
        send_many(env.svm(), &ixs, &[&squatter]),
        E::NotPassportWallet,
    );

    // The reissued passport starts a new probation, then the artist is paid.
    let ixs = withdraw(&env, &artist.pubkey(), &artist_key, SOL / 2);
    assert_error(send_many(env.svm(), &ixs, &[&artist]), E::OnProbation);
    advance_time(env.svm(), RECOVERY_DELAY);
    let available = held - env.riff.svm.minimum_balance_for_rent_exemption(8 + 32);
    let before = lamports(&env.riff.svm, &artist.pubkey());
    let ixs = withdraw(&env, &artist.pubkey(), &artist_key, available);
    send_many(env.svm(), &ixs, &[&artist]).unwrap();
    assert!(lamports(&env.riff.svm, &artist.pubkey()) > before + available - SOL / 100);
}

#[test]
fn only_a_revoked_passport_can_be_reissued() {
    let mut env = setup();
    issued(&mut env);
    let artist = funded_keypair(env.svm());
    let artist_key = TestPasskey::new();
    let records = proofs_for(&mut env, &artist, &DEFAULT_KINDS);
    let admin = env.riff.admin.insecure_clone();
    assert_error(
        reissue(&mut env, &admin, &artist, &artist_key, &records),
        E::NotRevoked,
    );
}

#[test]
fn reissuing_needs_the_same_proof_as_issuing() {
    let mut env = setup();
    issued(&mut env);
    revoke(&mut env);
    let artist = funded_keypair(env.svm());
    let artist_key = TestPasskey::new();
    let admin = env.riff.admin.insecure_clone();
    // One source only: not enough, even with riff's admin.
    let records = proofs_for(&mut env, &artist, &[ProofKind::SpotifyEmail]);
    assert_error(
        reissue(&mut env, &admin, &artist, &artist_key, &records),
        E::NotEnoughProofs,
    );
    // Another wallet's proofs don't count for this one.
    let other = funded_keypair(env.svm());
    let theirs = proofs_for(&mut env, &other, &DEFAULT_KINDS);
    assert_error(
        reissue(&mut env, &admin, &artist, &artist_key, &theirs),
        E::ProofMismatch,
    );
}
