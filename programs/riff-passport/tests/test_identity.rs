mod common;

use common::*;
use riff_passport::{error::PassportError as E, EndorsementStatus, PasskeyAction, ProofKind};
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn issues_a_passport_with_two_sources_including_a_strong_one() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport: riff_passport::Passport = fetch(&env.riff.svm, &passport_address(ARTIST_ID));
    assert_eq!(passport.artist_id, ARTIST_ID);
    assert_eq!(passport.wallet, wallet.pubkey());
    assert_eq!(passport.passkey, passkey.public());
    assert_eq!(passport.proofs.len(), 2);
    assert_eq!(passport.nonce, 1);
    let vault: riff_passport::Vault =
        fetch(&env.riff.svm, &vault_address(&passport_address(ARTIST_ID)));
    assert_eq!(vault.passport, passport_address(ARTIST_ID));
}

#[test]
fn refuses_too_few_or_too_weak_proofs() {
    for kinds in [
        vec![ProofKind::SpotifyProfileCode],            // one proof
        vec![ProofKind::YouTube, ProofKind::Instagram], // nothing strong
        vec![ProofKind::SpotifyProfileCode, ProofKind::SpotifyEmail], // one source (Spotify)
    ] {
        let mut env = setup();
        let wallet = funded_keypair(env.svm());
        assert_error(
            issue(&mut env, &wallet, &TestPasskey::new(), &kinds),
            E::NotEnoughProofs,
        );
    }
}

#[test]
fn proofs_need_the_verifier_and_count_only_for_their_own_wallet() {
    let mut env = setup();
    let wallet = funded_keypair(env.svm());
    let fake_verifier = Keypair::new();
    let mut ix = record_proof_ix(&env, &wallet.pubkey(), ARTIST_ID, ProofKind::YouTube);
    ix.accounts[1].pubkey = fake_verifier.pubkey();
    assert_error(
        send(env.svm(), ix, &[&wallet, &fake_verifier]),
        E::NotVerifier,
    );

    // The verifier can't stand in for the on-chain Spotify for Artists proof.
    assert_error(
        record_proof(&mut env, &wallet, ProofKind::SpotifyForArtists),
        E::OnChainOnly,
    );

    // Someone else's proofs don't help this wallet.
    let other = funded_keypair(env.svm());
    for kind in DEFAULT_KINDS {
        record_proof(&mut env, &other, kind).unwrap();
    }
    let records: Vec<_> = DEFAULT_KINDS
        .iter()
        .map(|k| proof_address(ARTIST_ID, &other.pubkey(), *k))
        .collect();
    let passkey = TestPasskey::new();
    let (proof, pre) = passkey.approve(
        &passport_address(ARTIST_ID),
        0,
        &PasskeyAction::Register {
            passkey: passkey.public(),
        },
    );
    let ix = issue_ix(&wallet.pubkey(), passkey.public(), proof, &records);
    assert_error(
        send_many(env.svm(), &[pre, ix], &[&wallet]),
        E::ProofMismatch,
    );
}

#[test]
fn stale_proofs_dont_count() {
    let mut env = setup();
    let wallet = funded_keypair(env.svm());
    for kind in DEFAULT_KINDS {
        record_proof(&mut env, &wallet, kind).unwrap();
    }
    advance_time(env.svm(), PROOF_MAX_AGE + 1);
    let records: Vec<_> = DEFAULT_KINDS
        .iter()
        .map(|k| proof_address(ARTIST_ID, &wallet.pubkey(), *k))
        .collect();
    let passkey = TestPasskey::new();
    let (proof, pre) = passkey.approve(
        &passport_address(ARTIST_ID),
        0,
        &PasskeyAction::Register {
            passkey: passkey.public(),
        },
    );
    let ix = issue_ix(&wallet.pubkey(), passkey.public(), proof, &records);
    assert_error(
        send_many(env.svm(), &[pre, ix], &[&wallet]),
        E::ProofExpired,
    );
}

#[test]
fn one_passport_per_artist() {
    let mut env = setup();
    issued(&mut env);
    let other = funded_keypair(env.svm());
    assert!(issue(&mut env, &other, &TestPasskey::new(), &DEFAULT_KINDS).is_err());
}

// ---- Spotify for Artists, verified on-chain

#[test]
fn verifies_a_spotify_for_artists_proof_on_chain() {
    let mut env = setup();
    let wallet = funded_keypair(env.svm());
    let t = now(&mut env) as u32;
    let proof = reclaim_proof(
        &env.attestor,
        canonical_context(&wallet.pubkey(), SPOTIFY_ID, &PROVIDER_HASH),
        t,
    );
    prove_spotify(&mut env, &wallet, &proof).unwrap();
    let record: riff_passport::ProofRecord = fetch(
        &env.riff.svm,
        &proof_address(ARTIST_ID, &wallet.pubkey(), ProofKind::SpotifyForArtists),
    );
    assert!(record.on_chain);
    assert_eq!(record.kind, ProofKind::SpotifyForArtists);
    // The buffer is closed and its rent refunded.
    assert!(env
        .svm()
        .get_account(&buffer_address(&wallet.pubkey()))
        .is_none_or(|a| a.lamports == 0));

    // With YouTube, that's a passport.
    record_proof(&mut env, &wallet, ProofKind::YouTube).unwrap();
    let records = [
        proof_address(ARTIST_ID, &wallet.pubkey(), ProofKind::SpotifyForArtists),
        proof_address(ARTIST_ID, &wallet.pubkey(), ProofKind::YouTube),
    ];
    let passkey = TestPasskey::new();
    let (p, pre) = passkey.approve(
        &passport_address(ARTIST_ID),
        0,
        &PasskeyAction::Register {
            passkey: passkey.public(),
        },
    );
    send_many(
        env.svm(),
        &[
            pre,
            issue_ix(&wallet.pubkey(), passkey.public(), p, &records),
        ],
        &[&wallet],
    )
    .unwrap();
}

#[test]
fn rejects_forged_or_mismatched_reclaim_proofs() {
    let mut env = setup();
    let wallet = funded_keypair(env.svm());
    let t = now(&mut env) as u32;
    let good_ctx = canonical_context(&wallet.pubkey(), SPOTIFY_ID, &PROVIDER_HASH);

    // Signed by someone who isn't a trusted attestor.
    let stranger = k256::ecdsa::SigningKey::random(&mut rand_core::OsRng);
    assert_error(
        prove_spotify(
            &mut env,
            &wallet,
            &reclaim_proof(&stranger, good_ctx.clone(), t),
        ),
        E::ReclaimUntrustedAttestor,
    );

    // A real attestor signature, but the claim was edited afterwards.
    let mut tampered = reclaim_proof(&env.attestor, good_ctx.clone(), t);
    tampered.parameters = tampered.parameters.replace("GET", "PUT");
    assert_error(
        prove_spotify(&mut env, &wallet, &tampered),
        E::ReclaimUntrustedAttestor,
    );

    // Made for another wallet: can't be replayed.
    let thief = Keypair::new();
    let ctx = canonical_context(&thief.pubkey(), SPOTIFY_ID, &PROVIDER_HASH);
    let proof = reclaim_proof(&env.attestor, ctx, t);
    assert_error(
        prove_spotify(&mut env, &wallet, &proof),
        E::ReclaimWrongWallet,
    );

    // Another artist's Spotify for Artists account.
    let ctx = canonical_context(&wallet.pubkey(), "0TnOYISbd1XYRBk9myaseg", &PROVIDER_HASH);
    let proof = reclaim_proof(&env.attestor, ctx, t);
    assert_error(
        prove_spotify(&mut env, &wallet, &proof),
        E::ReclaimWrongArtist,
    );

    // Another Reclaim provider (one that could extract anything).
    let ctx = canonical_context(&wallet.pubkey(), SPOTIFY_ID, &[9u8; 32]);
    let proof = reclaim_proof(&env.attestor, ctx, t);
    assert_error(
        prove_spotify(&mut env, &wallet, &proof),
        E::ReclaimWrongProvider,
    );

    // Too old.
    let old = t - PROOF_MAX_AGE as u32 - 10;
    let proof = reclaim_proof(&env.attestor, good_ctx, old);
    assert_error(prove_spotify(&mut env, &wallet, &proof), E::ProofExpired);
}

// ---- passkeys

#[test]
fn passkey_must_sign_this_exact_action_on_this_site() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    let mint = Keypair::new().pubkey();
    let endorse = |proof| {
        ix(
            riff_passport::instruction::Endorse {
                mint,
                passkey_proof: proof,
            },
            endorsement_accounts(&wallet.pubkey(), &mint),
        )
    };

    // No passkey signature at all: a stolen wallet alone can't endorse.
    let (proof, _) = passkey.approve(&passport, nonce(&env), &PasskeyAction::Endorse { mint });
    assert_error(
        send(env.svm(), endorse(proof), &[&wallet]),
        E::PasskeyMissing,
    );

    // Another passkey.
    let impostor = TestPasskey::new();
    let (proof, pre) = impostor.approve(&passport, nonce(&env), &PasskeyAction::Endorse { mint });
    assert_error(
        send_many(env.svm(), &[pre, endorse(proof)], &[&wallet]),
        E::PasskeyMissing,
    );

    // The right passkey, but it approved a different coin.
    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::Endorse {
            mint: Keypair::new().pubkey(),
        },
    );
    assert_error(
        send_many(env.svm(), &[pre, endorse(proof)], &[&wallet]),
        E::PasskeyWrongChallenge,
    );

    // Signed on a phishing site.
    let phished = TestPasskey {
        key: passkey.key.clone(),
        rp_id: "riffpad-claim.fun".into(),
        verified: true,
    };
    let (proof, pre) = phished.approve(&passport, nonce(&env), &PasskeyAction::Endorse { mint });
    assert_error(
        send_many(env.svm(), &[pre, endorse(proof)], &[&wallet]),
        E::PasskeyWrongSite,
    );

    // Without Face ID or a fingerprint.
    let unverified = TestPasskey {
        key: passkey.key.clone(),
        rp_id: RP_ID.into(),
        verified: false,
    };
    let (proof, pre) = unverified.approve(&passport, nonce(&env), &PasskeyAction::Endorse { mint });
    assert_error(
        send_many(env.svm(), &[pre, endorse(proof)], &[&wallet]),
        E::PasskeyNotVerified,
    );

    // Correct: endorsed.
    let (proof, pre) = passkey.approve(&passport, nonce(&env), &PasskeyAction::Endorse { mint });
    send_many(
        env.svm(),
        &[pre.clone(), endorse(proof.clone())],
        &[&wallet],
    )
    .unwrap();
    let e: riff_passport::Endorsement =
        fetch(&env.riff.svm, &endorsement_address(&passport, &mint));
    assert_eq!(e.status, EndorsementStatus::Endorsed);

    // The same signature can't be used twice.
    next_slot(env.svm());
    assert_error(
        send_many(env.svm(), &[pre, endorse(proof)], &[&wallet]),
        E::PasskeyWrongChallenge,
    );
}

#[test]
fn disavowing_needs_only_the_wallet() {
    let mut env = setup();
    let (wallet, _) = issued(&mut env);
    let mint = Keypair::new().pubkey();
    let disavow = ix(
        riff_passport::instruction::Disavow { mint },
        endorsement_accounts(&wallet.pubkey(), &mint),
    );
    send(env.svm(), disavow, &[&wallet]).unwrap();
    let e: riff_passport::Endorsement = fetch(
        &env.riff.svm,
        &endorsement_address(&passport_address(ARTIST_ID), &mint),
    );
    assert_eq!(e.status, EndorsementStatus::Disavowed);

    // Nobody else can.
    let stranger = funded_keypair(env.svm());
    let ix = ix(
        riff_passport::instruction::Disavow { mint },
        endorsement_accounts(&stranger.pubkey(), &mint),
    );
    assert_error(send(env.svm(), ix, &[&stranger]), E::NotPassportWallet);
}

#[test]
fn changes_wallet_and_passkey_with_the_passkey() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    let new_wallet = funded_keypair(env.svm());

    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::SetWallet {
            new_wallet: new_wallet.pubkey(),
        },
    );
    let set_wallet = ix(
        riff_passport::instruction::SetWallet {
            passkey_proof: proof,
        },
        riff_passport::accounts::SetWallet {
            act: act_accounts(&wallet.pubkey()),
            new_wallet: new_wallet.pubkey(),
        },
    );
    send_many(env.svm(), &[pre, set_wallet], &[&wallet, &new_wallet]).unwrap();
    assert_eq!(
        fetch::<riff_passport::Passport>(&env.riff.svm, &passport).wallet,
        new_wallet.pubkey()
    );

    // New passkey: the old one approves, the new one proves it's held.
    let next = TestPasskey::new();
    let n = nonce(&env);
    let (old_proof, pre_old) = passkey.approve(
        &passport,
        n,
        &PasskeyAction::SetPasskey {
            new_passkey: next.public(),
        },
    );
    let (new_proof, pre_new) = next.approve(
        &passport,
        n,
        &PasskeyAction::Register {
            passkey: next.public(),
        },
    );
    let set_passkey = ix(
        riff_passport::instruction::SetPasskey {
            new_passkey: next.public(),
            old_proof,
            new_proof,
        },
        act_accounts(&new_wallet.pubkey()),
    );
    send_many(env.svm(), &[pre_old, pre_new, set_passkey], &[&new_wallet]).unwrap();
    assert_eq!(
        fetch::<riff_passport::Passport>(&env.riff.svm, &passport).passkey,
        next.public()
    );
}

#[test]
fn adding_proofs_needs_the_passkey() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    record_proof(&mut env, &wallet, ProofKind::Website).unwrap();
    let record = proof_address(ARTIST_ID, &wallet.pubkey(), ProofKind::Website);
    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::AddProofs {
            records: vec![record],
        },
    );
    let mut add = ix(
        riff_passport::instruction::AddProofs {
            passkey_proof: proof.clone(),
        },
        act_accounts(&wallet.pubkey()),
    );
    add.accounts
        .push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(record, false));
    assert_error(send(env.svm(), add.clone(), &[&wallet]), E::PasskeyMissing);
    send_many(env.svm(), &[pre, add], &[&wallet]).unwrap();
    assert_eq!(
        fetch::<riff_passport::Passport>(&env.riff.svm, &passport)
            .proofs
            .len(),
        3
    );
}

#[test]
fn solana_itself_rejects_a_forged_passkey_signature() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    let mint = Keypair::new().pubkey();
    let (proof, pre) = passkey.approve(&passport, nonce(&env), &PasskeyAction::Endorse { mint });
    // Same key and message, but the signature bytes are forged.
    let mut forged = pre.clone();
    forged.data[16 + 33 + 5] ^= 0xff;
    let endorse = ix(
        riff_passport::instruction::Endorse {
            mint,
            passkey_proof: proof,
        },
        endorsement_accounts(&wallet.pubkey(), &mint),
    );
    let err =
        send_many(env.svm(), &[forged, endorse.clone()], &[&wallet]).expect_err("forged signature");
    assert!(
        format!("{:?}", err.err).contains("InstructionError(0"),
        "the precompile (instruction 0) must fail: {:?}",
        err.err
    );
    send_many(env.svm(), &[pre, endorse], &[&wallet]).unwrap();
}
