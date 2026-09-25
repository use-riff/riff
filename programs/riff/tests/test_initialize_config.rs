mod common;

use {common::*, riff::error::ErrorCode, solana_signer::Signer};

#[test]
fn initializes_config() {
    let mut env = setup();
    initialize_config(&mut env);

    let config: riff::Config = fetch(&env.svm, &config_address());
    assert_eq!(config.admin, env.admin.pubkey());
    assert_eq!(config.treasury, env.treasury);
    assert_eq!(config.artist_fee_bps, ARTIST_FEE_BPS);
    assert_eq!(config.creator_fee_bps, CREATOR_FEE_BPS);
    assert_eq!(config.protocol_fee_bps, PROTOCOL_FEE_BPS);
    assert_eq!(config.total_fee_bps(), 100);
    assert_eq!(config.claim_window_secs, CLAIM_WINDOW_SECS);
    assert_eq!(config.initial_virtual_sol_reserves, INITIAL_VIRTUAL_SOL);
    assert_eq!(config.curve_token_supply, CURVE_TOKEN_SUPPLY);
    assert_eq!(config.max_creator_buy_bps, MAX_CREATOR_BUY_BPS);
    assert_eq!(config.max_creator_buy_tokens(), 30_000_000 * TOKEN);
}

#[test]
fn derives_virtual_token_reserves_from_supply_split() {
    let mut env = setup();
    initialize_config(&mut env);

    let config: riff::Config = fetch(&env.svm, &config_address());
    assert_eq!(config.initial_virtual_token_reserves, INITIAL_VIRTUAL_TOKEN);
    assert_eq!(
        Some(config.initial_virtual_token_reserves),
        riff::curve::graduation_virtual_token_reserves(riff::COIN_TOTAL_SUPPLY, CURVE_TOKEN_SUPPLY)
    );
}

#[test]
fn rejects_non_upgrade_authority() {
    let mut env = setup();
    let impostor = funded_keypair(&mut env.svm);
    let ix = initialize_config_ix(&impostor.pubkey(), config_params(env.treasury));

    assert_riff_error(
        send(&mut env.svm, ix, &[&impostor]),
        ErrorCode::NotUpgradeAuthority,
    );
    assert!(env.svm.get_account(&config_address()).is_none());
}

/// Each case must be rejected with its error; the config must not be created.
fn assert_rejected(set: impl Fn(&mut riff::ConfigParams), expected: ErrorCode) {
    let mut env = setup();
    let mut params = config_params(env.treasury);
    set(&mut params);
    let ix = initialize_config_ix(&env.admin.pubkey(), params);
    assert_riff_error(send(&mut env.svm, ix, &[&env.admin]), expected);
    assert!(env.svm.get_account(&config_address()).is_none());
}

#[test]
fn rejects_total_fee_over_10_percent() {
    assert_rejected(
        |p| {
            p.artist_fee_bps = 500;
            p.creator_fee_bps = 300;
            p.protocol_fee_bps = 201;
        },
        ErrorCode::InvalidTradeFee,
    );
    // Each part alone is fine, but they can't overflow a u16 sum either.
    assert_rejected(
        |p| {
            p.artist_fee_bps = u16::MAX;
            p.creator_fee_bps = u16::MAX;
        },
        ErrorCode::InvalidTradeFee,
    );
}

#[test]
fn rejects_creator_buy_cap_over_10_percent() {
    assert_rejected(
        |p| p.max_creator_buy_bps = 1_001,
        ErrorCode::InvalidCreatorBuyCap,
    );
}

#[test]
fn accepts_boundary_values() {
    let mut env = setup();
    let mut params = config_params(env.treasury);
    params.artist_fee_bps = 500;
    params.creator_fee_bps = 300;
    params.protocol_fee_bps = 200;
    params.max_creator_buy_bps = 1_000;
    params.claim_window_secs = 1;
    let ix = initialize_config_ix(&env.admin.pubkey(), params);
    send(&mut env.svm, ix, &[&env.admin]).unwrap();
}

#[test]
fn rejects_non_positive_claim_window() {
    assert_rejected(|p| p.claim_window_secs = 0, ErrorCode::InvalidClaimWindow);
    assert_rejected(|p| p.claim_window_secs = -1, ErrorCode::InvalidClaimWindow);
}

#[test]
fn rejects_bad_curve_params() {
    let bad = ErrorCode::InvalidCurveParams;
    assert_rejected(|p| p.initial_virtual_sol_reserves = 0, bad);
    assert_rejected(|p| p.curve_token_supply = 0, bad);
    // Half or less can't match the graduation price; all leaves no reserve.
    assert_rejected(|p| p.curve_token_supply = riff::COIN_TOTAL_SUPPLY / 2, bad);
    assert_rejected(|p| p.curve_token_supply = riff::COIN_TOTAL_SUPPLY, bad);
    assert_rejected(|p| p.curve_token_supply = riff::COIN_TOTAL_SUPPLY + 1, bad);
}

#[test]
fn cannot_initialize_twice() {
    let mut env = setup();
    initialize_config(&mut env);

    let mut params = config_params(env.treasury);
    params.artist_fee_bps = 0;
    let ix = initialize_config_ix(&env.admin.pubkey(), params);
    assert!(send(&mut env.svm, ix, &[&env.admin]).is_err());

    let config: riff::Config = fetch(&env.svm, &config_address());
    assert_eq!(config.artist_fee_bps, ARTIST_FEE_BPS);
}
