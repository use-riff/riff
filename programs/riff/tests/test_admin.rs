//! Two-step admin handover, e.g. from the deploy key to a Squads multisig.

mod common;

use {
    anchor_lang::prelude::Pubkey, common::*, riff::error::ErrorCode, solana_keypair::Keypair,
    solana_signer::Signer,
};

fn config(env: &Env) -> riff::Config {
    fetch(&env.svm, &config_address())
}

/// A harmless admin-only action: rotate the verifier to `key`.
#[allow(clippy::result_large_err)]
fn rotate_verifier(
    env: &mut Env,
    admin: &Keypair,
    key: Pubkey,
) -> litesvm::types::TransactionResult {
    let ix = update_config_ix(
        &admin.pubkey(),
        riff::UpdateConfigParams {
            verifier: Some(key),
            ..Default::default()
        },
    );
    send(&mut env.svm, ix, &[admin])
}

fn setup_with_config() -> (Env, Keypair) {
    let mut env = setup();
    initialize_config(&mut env);
    let admin = env.admin.insecure_clone();
    (env, admin)
}

#[test]
fn handover_completes_only_when_the_new_admin_accepts() {
    let (mut env, old) = setup_with_config();
    let new = funded_keypair(&mut env.svm);

    let meta = send(
        &mut env.svm,
        transfer_admin_ix(&old.pubkey(), &new.pubkey()),
        &[&old],
    )
    .unwrap();
    assert_eq!(
        events::<riff::events::AdminTransferProposed>(&meta.logs)[0].pending_admin,
        new.pubkey()
    );
    // Proposed, not transferred: the old admin still rules, the new one can't yet.
    assert_eq!(config(&env).admin, old.pubkey());
    assert_eq!(config(&env).pending_admin, Some(new.pubkey()));
    assert_riff_error(
        rotate_verifier(&mut env, &new, Pubkey::new_unique()),
        ErrorCode::NotAdmin,
    );
    rotate_verifier(&mut env, &old, Pubkey::new_unique()).unwrap();

    let meta = send(&mut env.svm, accept_admin_ix(&new.pubkey()), &[&new]).unwrap();
    let transferred = &events::<riff::events::AdminTransferred>(&meta.logs)[0];
    assert_eq!(transferred.previous_admin, old.pubkey());
    assert_eq!(transferred.admin, new.pubkey());
    assert_eq!(config(&env).admin, new.pubkey());
    assert_eq!(config(&env).pending_admin, None);

    // The old admin has lost every power; the new one has them.
    assert_riff_error(
        rotate_verifier(&mut env, &old, Pubkey::new_unique()),
        ErrorCode::NotAdmin,
    );
    assert_riff_error(
        send(
            &mut env.svm,
            transfer_admin_ix(&old.pubkey(), &old.pubkey()),
            &[&old],
        ),
        ErrorCode::NotAdmin,
    );
    rotate_verifier(&mut env, &new, Pubkey::new_unique()).unwrap();
}

#[test]
fn only_the_admin_can_propose() {
    let (mut env, _) = setup_with_config();
    let intruder = funded_keypair(&mut env.svm);
    assert_riff_error(
        send(
            &mut env.svm,
            transfer_admin_ix(&intruder.pubkey(), &intruder.pubkey()),
            &[&intruder],
        ),
        ErrorCode::NotAdmin,
    );
    assert_eq!(config(&env).pending_admin, None);
}

#[test]
fn only_the_proposed_admin_can_accept() {
    let (mut env, old) = setup_with_config();
    let stranger = funded_keypair(&mut env.svm);
    // Nothing proposed yet.
    assert_riff_error(
        send(
            &mut env.svm,
            accept_admin_ix(&stranger.pubkey()),
            &[&stranger],
        ),
        ErrorCode::NotPendingAdmin,
    );

    let new = funded_keypair(&mut env.svm);
    send(
        &mut env.svm,
        transfer_admin_ix(&old.pubkey(), &new.pubkey()),
        &[&old],
    )
    .unwrap();
    assert_riff_error(
        send(
            &mut env.svm,
            accept_admin_ix(&stranger.pubkey()),
            &[&stranger],
        ),
        ErrorCode::NotPendingAdmin,
    );
    assert_eq!(config(&env).admin, old.pubkey());
}

#[test]
fn a_new_proposal_replaces_the_old_one() {
    let (mut env, old) = setup_with_config();
    let (first, second) = (funded_keypair(&mut env.svm), funded_keypair(&mut env.svm));
    send(
        &mut env.svm,
        transfer_admin_ix(&old.pubkey(), &first.pubkey()),
        &[&old],
    )
    .unwrap();
    send(
        &mut env.svm,
        transfer_admin_ix(&old.pubkey(), &second.pubkey()),
        &[&old],
    )
    .unwrap();

    assert_riff_error(
        send(&mut env.svm, accept_admin_ix(&first.pubkey()), &[&first]),
        ErrorCode::NotPendingAdmin,
    );
    send(&mut env.svm, accept_admin_ix(&second.pubkey()), &[&second]).unwrap();
    assert_eq!(config(&env).admin, second.pubkey());
}

#[test]
fn cannot_propose_the_all_zero_address() {
    let (mut env, old) = setup_with_config();
    assert_riff_error(
        send(
            &mut env.svm,
            transfer_admin_ix(&old.pubkey(), &Pubkey::default()),
            &[&old],
        ),
        ErrorCode::InvalidAddress,
    );
}
