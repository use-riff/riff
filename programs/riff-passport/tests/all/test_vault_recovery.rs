use crate::common::*;
use anchor_lang::solana_program::instruction::AccountMeta;
use riff_passport::{error::PassportError as E, PasskeyAction, ProofKind};
use solana_keypair::Keypair;
use solana_signer::Signer;

fn claimed_coin(env: &mut Env, wallet: &Keypair) -> anchor_lang::prelude::Pubkey {
    let mint = coin_with_fees(env, 3);
    let verifier = env.verifier();
    let ix = claim_coin_ix(env, &wallet.pubkey(), &mint);
    send(env.svm(), ix, &[wallet, &verifier]).unwrap();
    mint
}

#[test]
fn the_vault_is_the_coins_artist_and_collects_its_fees() {
    let mut env = setup();
    let (wallet, _) = issued(&mut env);
    let mint = claimed_coin(&mut env, &wallet);
    let vault = vault_address(&passport_address(ARTIST_ID));
    let coin: riff::Coin = fetch(&env.riff.svm, &crate::common::riff_env::coin_address(&mint));
    assert_eq!(coin.artist, Some(vault));
    let fees = coin.artist_fees;
    assert!(fees > 0);

    let before = lamports(&env.riff.svm, &vault);
    let payer = funded_keypair(env.svm());
    send(env.svm(), collect_ix(&mint), &[&payer]).unwrap();
    assert_eq!(lamports(&env.riff.svm, &vault), before + fees);
}

#[test]
fn claiming_needs_the_passport_wallet_and_riffs_verifier() {
    let mut env = setup();
    let (wallet, _) = issued(&mut env);
    let mint = coin_with_fees(&mut env, 1);
    let stranger = funded_keypair(env.svm());
    let verifier = env.verifier();
    let ix = claim_coin_ix(&env, &stranger.pubkey(), &mint);
    assert_error(
        send(env.svm(), ix, &[&stranger, &verifier]),
        E::NotPassportWallet,
    );
    let fake = Keypair::new();
    let mut ix = claim_coin_ix(&env, &wallet.pubkey(), &mint);
    ix.accounts[1].pubkey = fake.pubkey();
    let res = send(env.svm(), ix, &[&wallet, &fake]);
    crate::common::riff_env::assert_riff_error(res, riff::error::ErrorCode::NotVerifier);
}

#[test]
fn a_stolen_wallet_can_only_take_the_small_daily_amount() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    let vault = vault_address(&passport);
    env.svm().airdrop(&vault, 5 * SOL).unwrap();

    // Up to the free daily amount: the wallet alone.
    send(
        env.svm(),
        withdraw_ix(&wallet.pubkey(), FREE_PER_DAY, None),
        &[&wallet],
    )
    .unwrap();
    // Anything more today: the passkey too.
    assert_error(
        send(
            env.svm(),
            withdraw_ix(&wallet.pubkey(), 1, None),
            &[&wallet],
        ),
        E::PasskeyRequired,
    );
    let amount = 2 * SOL;
    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::Withdraw {
            to: wallet.pubkey(),
            amount,
        },
    );
    let before = lamports(&env.riff.svm, &wallet.pubkey());
    send_many(
        env.svm(),
        &[pre, withdraw_ix(&wallet.pubkey(), amount, Some(proof))],
        &[&wallet],
    )
    .unwrap();
    assert!(lamports(&env.riff.svm, &wallet.pubkey()) > before + amount - SOL / 100);

    // A new day resets the free amount.
    advance_time(env.svm(), 24 * 60 * 60);
    send(
        env.svm(),
        withdraw_ix(&wallet.pubkey(), FREE_PER_DAY, None),
        &[&wallet],
    )
    .unwrap();

    // The vault keeps its rent; it can't be emptied below that.
    let everything = lamports(&env.riff.svm, &vault);
    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::Withdraw {
            to: wallet.pubkey(),
            amount: everything,
        },
    );
    assert_error(
        send_many(
            env.svm(),
            &[pre, withdraw_ix(&wallet.pubkey(), everything, Some(proof))],
            &[&wallet],
        ),
        E::InsufficientVault,
    );
}

// ---- recovery

#[allow(clippy::result_large_err)]
fn request_recovery(
    env: &mut Env,
    new_wallet: &Keypair,
    new_passkey: &TestPasskey,
    kinds: &[ProofKind],
) -> litesvm::types::TransactionResult {
    for kind in kinds {
        record_proof(env, new_wallet, *kind).unwrap();
    }
    let passport = passport_address(ARTIST_ID);
    let (proof, pre) = new_passkey.approve(
        &passport,
        nonce(env),
        &PasskeyAction::Register {
            passkey: new_passkey.public(),
        },
    );
    let mut request = ix(
        riff_passport::instruction::RequestRecovery {
            new_passkey: new_passkey.public(),
            passkey_proof: proof,
        },
        riff_passport::accounts::RequestRecovery {
            new_wallet: new_wallet.pubkey(),
            config: config_address(),
            passport,
            instructions: instructions_sysvar(),
        },
    );
    for kind in kinds {
        request.accounts.push(AccountMeta::new_readonly(
            proof_address(ARTIST_ID, &new_wallet.pubkey(), *kind),
            false,
        ));
    }
    send_many(env.svm(), &[pre, request], &[new_wallet])
}

fn veto_ix(
    vetoer: &anchor_lang::prelude::Pubkey,
    proof: Option<riff_passport::PasskeyProof>,
    guardian: Option<anchor_lang::prelude::Pubkey>,
) -> anchor_lang::solana_program::instruction::Instruction {
    let mut veto = ix(
        riff_passport::instruction::VetoRecovery {
            passkey_proof: proof,
        },
        riff_passport::accounts::VetoRecovery {
            vetoer: *vetoer,
            config: config_address(),
            passport: passport_address(ARTIST_ID),
            instructions: instructions_sysvar(),
        },
    );
    if let Some(g) = guardian {
        veto.accounts.push(AccountMeta::new_readonly(g, false));
    }
    veto
}

fn finalize_ix() -> anchor_lang::solana_program::instruction::Instruction {
    ix(
        riff_passport::instruction::FinalizeRecovery {},
        riff_passport::accounts::FinalizeRecovery {
            passport: passport_address(ARTIST_ID),
        },
    )
}

#[test]
fn recovers_a_lost_wallet_and_passkey_after_the_time_lock() {
    let mut env = setup();
    let (_lost_wallet, _lost_passkey) = issued(&mut env);
    let new_wallet = funded_keypair(env.svm());
    let new_passkey = TestPasskey::new();
    request_recovery(&mut env, &new_wallet, &new_passkey, &DEFAULT_KINDS).unwrap();

    let anyone = funded_keypair(env.svm());
    assert_error(
        send(env.svm(), finalize_ix(), &[&anyone]),
        E::RecoveryLocked,
    );
    advance_time(env.svm(), RECOVERY_DELAY);
    send(env.svm(), finalize_ix(), &[&anyone]).unwrap();
    let passport: riff_passport::Passport = fetch(&env.riff.svm, &passport_address(ARTIST_ID));
    assert_eq!(passport.wallet, new_wallet.pubkey());
    assert_eq!(passport.passkey, new_passkey.public());
    assert!(passport.recovery.is_none());
}

#[test]
fn a_hacked_instagram_is_not_enough_to_start_a_recovery() {
    let mut env = setup();
    issued(&mut env);
    let hacker = funded_keypair(env.svm());
    let res = request_recovery(
        &mut env,
        &hacker,
        &TestPasskey::new(),
        &[ProofKind::Instagram, ProofKind::X],
    );
    assert_error(res, E::NotEnoughProofs);
}

#[test]
fn the_artist_vetoes_with_their_passkey_or_a_guardian_proof() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    let attacker = funded_keypair(env.svm());
    request_recovery(&mut env, &attacker, &TestPasskey::new(), &DEFAULT_KINDS).unwrap();

    // The attacker can't veto with a wallet alone, and a stolen wallet can't either.
    assert_error(
        send(env.svm(), veto_ix(&wallet.pubkey(), None, None), &[&wallet]),
        E::NotAGuardian,
    );

    // The old passkey vetoes.
    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::Veto {
            new_wallet: attacker.pubkey(),
        },
    );
    send_many(
        env.svm(),
        &[pre, veto_ix(&wallet.pubkey(), Some(proof), None)],
        &[&wallet],
    )
    .unwrap();
    assert!(fetch::<riff_passport::Passport>(&env.riff.svm, &passport)
        .recovery
        .is_none());

    // Again, and this time the artist's YouTube vetoes: a fresh YouTube proof
    // (made after the request) from whatever wallet the artist has at hand.
    advance_time(env.svm(), 10);
    let attacker2 = funded_keypair(env.svm());
    request_recovery(&mut env, &attacker2, &TestPasskey::new(), &DEFAULT_KINDS).unwrap();
    // An old proof doesn't count: it has to show control now.
    let stale_guardian = proof_address(ARTIST_ID, &wallet.pubkey(), ProofKind::YouTube);
    assert_error(
        send(
            env.svm(),
            veto_ix(&wallet.pubkey(), None, Some(stale_guardian)),
            &[&wallet],
        ),
        E::NotAGuardian,
    );
    advance_time(env.svm(), 10);
    let phone = funded_keypair(env.svm());
    record_proof(&mut env, &phone, ProofKind::YouTube).unwrap();
    let guardian = proof_address(ARTIST_ID, &phone.pubkey(), ProofKind::YouTube);
    send(
        env.svm(),
        veto_ix(&phone.pubkey(), None, Some(guardian)),
        &[&phone],
    )
    .unwrap();
    assert!(fetch::<riff_passport::Passport>(&env.riff.svm, &passport)
        .recovery
        .is_none());

    // A kind the passport doesn't have (Instagram) isn't a guardian.
    advance_time(env.svm(), 10);
    let attacker3 = funded_keypair(env.svm());
    request_recovery(&mut env, &attacker3, &TestPasskey::new(), &DEFAULT_KINDS).unwrap();
    advance_time(env.svm(), 10);
    record_proof(&mut env, &phone, ProofKind::Instagram).unwrap();
    let not_guardian = proof_address(ARTIST_ID, &phone.pubkey(), ProofKind::Instagram);
    assert_error(
        send(
            env.svm(),
            veto_ix(&phone.pubkey(), None, Some(not_guardian)),
            &[&phone],
        ),
        E::NotAGuardian,
    );
}

#[test]
fn only_one_recovery_at_a_time() {
    let mut env = setup();
    issued(&mut env);
    let a = funded_keypair(env.svm());
    request_recovery(&mut env, &a, &TestPasskey::new(), &DEFAULT_KINDS).unwrap();
    let b = funded_keypair(env.svm());
    assert_error(
        request_recovery(&mut env, &b, &TestPasskey::new(), &DEFAULT_KINDS),
        E::RecoveryPending,
    );
}

#[test]
fn a_revoked_passport_can_do_nothing() {
    let mut env = setup();
    let (wallet, _) = issued(&mut env);
    let admin = env.riff.admin.insecure_clone();
    let revoke = ix(
        riff_passport::instruction::RevokePassport {
            reason: "issued to the wrong person".into(),
        },
        riff_passport::accounts::RevokePassport {
            admin: admin.pubkey(),
            config: config_address(),
            passport: passport_address(ARTIST_ID),
        },
    );
    let stranger = funded_keypair(env.svm());
    let mut not_admin = revoke.clone();
    not_admin.accounts[0].pubkey = stranger.pubkey();
    assert_error(send(env.svm(), not_admin, &[&stranger]), E::NotAdmin);
    send(env.svm(), revoke, &[&admin]).unwrap();

    env.svm()
        .airdrop(&vault_address(&passport_address(ARTIST_ID)), SOL)
        .unwrap();
    assert_error(
        send(
            env.svm(),
            withdraw_ix(&wallet.pubkey(), 1000, None),
            &[&wallet],
        ),
        E::Revoked,
    );
    let mint = coin_with_fees(&mut env, 1);
    let verifier = env.verifier();
    let ix = claim_coin_ix(&env, &wallet.pubkey(), &mint);
    assert_error(send(env.svm(), ix, &[&wallet, &verifier]), E::Revoked);
}
