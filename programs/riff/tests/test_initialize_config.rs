mod common;

use {common::*, riff::error::ErrorCode, solana_signer::Signer};

#[test]
fn initializes_config() {
    let mut env = setup();
    initialize_config(&mut env);

    let config: riff::Config = fetch(&env.svm, &config_address());
    assert_eq!(config.admin, env.admin.pubkey());
    assert_eq!(config.artist_fee_share_bps, ARTIST_FEE_SHARE_BPS);
    assert_eq!(config.claim_window_secs, CLAIM_WINDOW_SECS);
}

#[test]
fn rejects_non_upgrade_authority() {
    let mut env = setup();
    let impostor = funded_keypair(&mut env.svm);
    let ix = initialize_config_ix(&impostor.pubkey(), ARTIST_FEE_SHARE_BPS, CLAIM_WINDOW_SECS);

    assert_riff_error(
        send(&mut env.svm, ix, &[&impostor]),
        ErrorCode::NotUpgradeAuthority,
    );
    assert!(env.svm.get_account(&config_address()).is_none());
}

#[test]
fn rejects_fee_share_over_100_percent() {
    let mut env = setup();
    let ix = initialize_config_ix(&env.admin.pubkey(), 10_001, CLAIM_WINDOW_SECS);
    assert_riff_error(
        send(&mut env.svm, ix, &[&env.admin]),
        ErrorCode::InvalidArtistFeeShare,
    );

    // Exactly 100% is allowed.
    let ix = initialize_config_ix(&env.admin.pubkey(), 10_000, CLAIM_WINDOW_SECS);
    send(&mut env.svm, ix, &[&env.admin]).unwrap();
}

#[test]
fn rejects_non_positive_claim_window() {
    let mut env = setup();
    for window in [0, -1] {
        let ix = initialize_config_ix(&env.admin.pubkey(), ARTIST_FEE_SHARE_BPS, window);
        assert_riff_error(
            send(&mut env.svm, ix, &[&env.admin]),
            ErrorCode::InvalidClaimWindow,
        );
    }
}

#[test]
fn cannot_initialize_twice() {
    let mut env = setup();
    initialize_config(&mut env);

    let ix = initialize_config_ix(&env.admin.pubkey(), 0, 1);
    assert!(send(&mut env.svm, ix, &[&env.admin]).is_err());

    let config: riff::Config = fetch(&env.svm, &config_address());
    assert_eq!(config.artist_fee_share_bps, ARTIST_FEE_SHARE_BPS);
}
