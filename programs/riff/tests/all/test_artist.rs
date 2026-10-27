//! M-01: artist claim, artist fee withdrawal, charity sweep, and verifier
//! rotation.
//!
//! Where the artist's share of each trade fee goes:
//! - unclaimed, window open: held for the artist
//! - claimed (at any time): to the artist from then on
//! - unclaimed, window closed: to the charity, along with anything held

use {
    crate::common::*, anchor_lang::prelude::Pubkey, riff::error::ErrorCode,
    solana_keypair::Keypair, solana_signer::Signer,
};

struct Launch {
    env: Env,
    mint: Pubkey,
    artist: Keypair,
    alice: Keypair,
}

fn launch() -> Launch {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 500 * SOL);
    let artist = funded_keypair(&mut env.svm);
    Launch {
        env,
        mint,
        artist,
        alice,
    }
}

fn coin(l: &Launch) -> riff::Coin {
    fetch(&l.env.svm, &coin_address(&l.mint))
}

/// Alice buys `sol`; the admin pays the network fee.
fn buy(l: &mut Launch, sol: u64) {
    let payer = l.env.admin.insecure_clone();
    let ix = buy_ix(&l.alice.pubkey(), &l.mint, sol, 0);
    send(&mut l.env.svm, ix, &[&payer, &l.alice]).unwrap();
}

/// Artist share of a `sol` buy on the current curve.
fn artist_share(l: &Launch, sol: u64) -> u64 {
    let q = riff::curve::quote_buy(&coin(l).reserves(), TOTAL_FEE_BPS, sol).unwrap();
    riff::curve::split_fee(q.fee, ARTIST_FEE_BPS, CREATOR_FEE_BPS, PROTOCOL_FEE_BPS).artist
}

#[allow(clippy::result_large_err)]
fn claim(l: &mut Launch) -> litesvm::types::TransactionResult {
    let artist = l.artist.insecure_clone();
    claim_artist(&mut l.env, &artist, &l.mint)
}

#[allow(clippy::result_large_err)]
fn withdraw(l: &mut Launch, who: &Keypair) -> litesvm::types::TransactionResult {
    let payer = l.env.admin.insecure_clone();
    let ix = withdraw_artist_fees_ix(&who.pubkey(), &l.mint);
    send(&mut l.env.svm, ix, &[&payer, who])
}

#[allow(clippy::result_large_err)]
fn sweep(l: &mut Launch) -> litesvm::types::TransactionResult {
    let stranger = funded_keypair(&mut l.env.svm);
    let ix = sweep_charity_fees_ix(&l.env.charity, &l.mint);
    send(&mut l.env.svm, ix, &[&stranger])
}

fn close_claim_window(l: &mut Launch) {
    let c = coin(l);
    let now = l
        .env
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp;
    advance_time(&mut l.env.svm, c.claim_deadline - now + 1);
}

/// The coin must always hold exactly rent + curve SOL + every fee balance.
fn assert_solvent(l: &Launch) {
    let key = coin_address(&l.mint);
    let c = coin(l);
    let len = l.env.svm.get_account(&key).unwrap().data.len();
    let rent = l.env.svm.minimum_balance_for_rent_exemption(len);
    assert_eq!(
        lamports(&l.env.svm, &key),
        rent + c.real_sol_reserves
            + c.artist_fees
            + c.charity_fees
            + c.creator_fees
            + c.protocol_fees
    );
}

// ------------------------------------------------------------ claim in window

#[test]
fn artist_claiming_in_window_gets_held_and_future_fees() {
    let mut l = launch();
    let held = artist_share(&l, 10 * SOL);
    buy(&mut l, 10 * SOL);
    assert_eq!(coin(&l).artist_fees, held);
    assert_eq!(coin(&l).charity_fees, 0);

    let meta = claim(&mut l).unwrap();
    let claimed = events::<riff::events::ArtistClaimed>(&meta.logs);
    assert_eq!(claimed.len(), 1);
    assert!(!claimed[0].late);
    assert_eq!(claimed[0].forfeited_to_charity, 0);
    assert_eq!(coin(&l).artist, Some(l.artist.pubkey()));

    // Everything held before the claim is the artist's.
    let before = lamports(&l.env.svm, &l.artist.pubkey());
    let artist = l.artist.insecure_clone();
    withdraw(&mut l, &artist).unwrap();
    assert_eq!(lamports(&l.env.svm, &l.artist.pubkey()), before + held);
    assert_eq!(coin(&l).artist_fees, 0);

    // Fees keep flowing to the artist, even long after the window.
    close_claim_window(&mut l);
    let later = artist_share(&l, 5 * SOL);
    buy(&mut l, 5 * SOL);
    assert_eq!(coin(&l).artist_fees, later);
    assert_eq!(coin(&l).charity_fees, 0);
    withdraw(&mut l, &artist).unwrap();
    assert_eq!(
        lamports(&l.env.svm, &l.artist.pubkey()),
        before + held + later
    );
    assert_solvent(&l);
}

// ------------------------------------------------------ unclaimed → charity

#[test]
fn nothing_goes_to_charity_while_the_window_is_open() {
    let mut l = launch();
    buy(&mut l, 10 * SOL);
    let held = coin(&l).artist_fees;

    assert_riff_error(sweep(&mut l), ErrorCode::NoFeesToWithdraw);
    assert_eq!(coin(&l).artist_fees, held, "still held for the artist");
    assert_eq!(lamports(&l.env.svm, &l.env.charity), CHARITY_START);
}

#[test]
fn the_window_closes_exactly_at_the_deadline() {
    let mut l = launch();
    let deadline = coin(&l).claim_deadline;
    let now = l
        .env
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp;

    // At the deadline itself: still the artist's.
    advance_time(&mut l.env.svm, deadline - now);
    buy(&mut l, SOL);
    assert!(coin(&l).artist_fees > 0);
    assert_eq!(coin(&l).charity_fees, 0);

    // One second later: the charity's.
    advance_time(&mut l.env.svm, 1);
    let held = coin(&l).artist_fees;
    buy(&mut l, SOL);
    assert_eq!(coin(&l).artist_fees, held);
    assert!(coin(&l).charity_fees > 0);
}

#[test]
fn unclaimed_fees_go_to_charity_for_the_life_of_the_coin() {
    let mut l = launch();
    let held = artist_share(&l, 10 * SOL);
    buy(&mut l, 10 * SOL);
    close_claim_window(&mut l);

    // Held fees are forfeited to the charity on the first sweep.
    let meta = sweep(&mut l).unwrap();
    assert_eq!(lamports(&l.env.svm, &l.env.charity), CHARITY_START + held);
    let swept = events::<riff::events::CharityFeesSwept>(&meta.logs);
    assert_eq!(swept[0].amount, held);
    assert_eq!(coin(&l).artist_fees, 0);

    // New fees go straight to the charity balance, sweep after sweep.
    let next = artist_share(&l, 7 * SOL);
    buy(&mut l, 7 * SOL);
    assert_eq!(coin(&l).artist_fees, 0);
    assert_eq!(coin(&l).charity_fees, next);
    sweep(&mut l).unwrap();
    assert_eq!(
        lamports(&l.env.svm, &l.env.charity),
        CHARITY_START + held + next
    );
    assert_solvent(&l);
}

// --------------------------------------------------------------- late claim

#[test]
fn late_claim_redirects_only_future_fees() {
    let mut l = launch();
    let in_window = artist_share(&l, 10 * SOL);
    buy(&mut l, 10 * SOL);
    close_claim_window(&mut l);
    let after_window = artist_share(&l, 4 * SOL);
    buy(&mut l, 4 * SOL);

    let meta = claim(&mut l).unwrap();
    let claimed = events::<riff::events::ArtistClaimed>(&meta.logs);
    assert!(claimed[0].late);
    assert_eq!(claimed[0].forfeited_to_charity, in_window);
    // What was already the charity's stays the charity's.
    let c = coin(&l);
    assert_eq!(c.artist_fees, 0);
    assert_eq!(c.charity_fees, in_window + after_window);

    // From the claim on, fees go to the artist.
    let after_claim = artist_share(&l, 6 * SOL);
    buy(&mut l, 6 * SOL);
    assert_eq!(coin(&l).artist_fees, after_claim);
    assert_eq!(coin(&l).charity_fees, in_window + after_window);

    let before = lamports(&l.env.svm, &l.artist.pubkey());
    let artist = l.artist.insecure_clone();
    withdraw(&mut l, &artist).unwrap();
    assert_eq!(
        lamports(&l.env.svm, &l.artist.pubkey()),
        before + after_claim
    );
    sweep(&mut l).unwrap();
    assert_eq!(
        lamports(&l.env.svm, &l.env.charity),
        CHARITY_START + in_window + after_window
    );
    assert_solvent(&l);
}

// ------------------------------------------------------------ claim security

#[test]
fn claim_needs_the_verifier_cosignature() {
    let mut l = launch();
    let artist_id = coin(&l).artist_id;
    let impostor_verifier = Keypair::new();

    // Signed by some other key posing as the verifier.
    let ix = claim_artist_ix(
        &l.artist.pubkey(),
        &impostor_verifier.pubkey(),
        &l.mint,
        &artist_id,
    );
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&l.artist, &impostor_verifier]),
        ErrorCode::NotVerifier,
    );

    // The real verifier's address, but without its signature.
    let mut ix = claim_artist_ix(
        &l.artist.pubkey(),
        &l.env.verifier.pubkey(),
        &l.mint,
        &artist_id,
    );
    ix.accounts[1].is_signer = false;
    assert!(send(&mut l.env.svm, ix, &[&l.artist]).is_err());

    assert_eq!(coin(&l).artist, None);
}

#[test]
fn claim_needs_the_artist_signature() {
    let mut l = launch();
    let artist_id = coin(&l).artist_id;
    let verifier = l.env.verifier.insecure_clone();
    let mut ix = claim_artist_ix(&l.artist.pubkey(), &verifier.pubkey(), &l.mint, &artist_id);
    ix.accounts[0].is_signer = false;
    assert!(send(&mut l.env.svm, ix, &[&verifier]).is_err());
    assert_eq!(coin(&l).artist, None);
}

#[test]
fn claim_must_name_the_coins_artist() {
    let mut l = launch();
    let verifier = l.env.verifier.insecure_clone();
    // A co-signature meant for a different artist's coin.
    let ix = claim_artist_ix(
        &l.artist.pubkey(),
        &verifier.pubkey(),
        &l.mint,
        "someone-else",
    );
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&l.artist, &verifier]),
        ErrorCode::ArtistIdMismatch,
    );
}

#[test]
fn a_coin_can_only_be_claimed_once() {
    let mut l = launch();
    claim(&mut l).unwrap();
    let rival = funded_keypair(&mut l.env.svm);
    assert_riff_error(
        claim_artist(&mut l.env, &rival, &l.mint.clone()),
        ErrorCode::AlreadyClaimed,
    );
    assert_eq!(coin(&l).artist, Some(l.artist.pubkey()));
}

#[test]
fn only_the_claimed_artist_can_withdraw() {
    let mut l = launch();
    buy(&mut l, 5 * SOL);
    let stranger = funded_keypair(&mut l.env.svm);

    // Before any claim, nobody can.
    let artist = l.artist.insecure_clone();
    assert_riff_error(withdraw(&mut l, &artist), ErrorCode::NotArtist);

    claim(&mut l).unwrap();
    assert_riff_error(withdraw(&mut l, &stranger), ErrorCode::NotArtist);
    assert!(coin(&l).artist_fees > 0);
    withdraw(&mut l, &artist).unwrap();
}

// ------------------------------------------------------------ charity wallet

#[test]
fn sweep_only_pays_the_configured_charity() {
    let mut l = launch();
    buy(&mut l, 5 * SOL);
    close_claim_window(&mut l);
    let thief = Pubkey::new_unique();
    l.env.svm.airdrop(&thief, CHARITY_START).unwrap();

    let stranger = funded_keypair(&mut l.env.svm);
    let ix = sweep_charity_fees_ix(&thief, &l.mint);
    assert!(send(&mut l.env.svm, ix, &[&stranger]).is_err());
    assert_eq!(lamports(&l.env.svm, &thief), CHARITY_START);
}

#[test]
fn sweep_refuses_to_leave_an_unfunded_charity_below_rent_minimum() {
    let mut env = setup();
    // A fresh multisig vault that has never received SOL.
    env.charity = Pubkey::find_program_address(&[b"charity"], &Pubkey::new_unique()).0;
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 50 * SOL);
    let artist = funded_keypair(&mut env.svm);
    let mut l = Launch {
        env,
        mint,
        artist,
        alice,
    };
    let rent_min = l.env.svm.minimum_balance_for_rent_exemption(0);

    // 0.1 SOL buy: 500,000 lamports of artist share, below the minimum.
    buy(&mut l, SOL / 10);
    close_claim_window(&mut l);
    assert!(coin(&l).artist_fees < rent_min);
    assert_riff_error(sweep(&mut l), ErrorCode::CharityNotRentExempt);
    assert!(l.env.svm.get_account(&l.env.charity).is_none());

    buy(&mut l, SOL);
    sweep(&mut l).unwrap();
    assert!(lamports(&l.env.svm, &l.env.charity) >= rent_min);
    assert_solvent(&l);
}

// ------------------------------------------------------------ update_config

#[test]
fn admin_can_rotate_a_leaked_verifier() {
    let mut l = launch();
    let old = l.env.verifier.insecure_clone();
    let new = Keypair::new();
    let admin = l.env.admin.insecure_clone();
    let ix = update_config_ix(
        &admin.pubkey(),
        riff::UpdateConfigParams {
            verifier: Some(new.pubkey()),
            ..Default::default()
        },
    );
    let meta = send(&mut l.env.svm, ix, &[&admin]).unwrap();
    assert_eq!(
        events::<riff::events::ConfigUpdated>(&meta.logs)[0].verifier,
        new.pubkey()
    );

    // The old key can no longer vouch for anyone.
    let artist_id = coin(&l).artist_id;
    let ix = claim_artist_ix(&l.artist.pubkey(), &old.pubkey(), &l.mint, &artist_id);
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&l.artist, &old]),
        ErrorCode::NotVerifier,
    );

    l.env.verifier = new;
    claim(&mut l).unwrap();
}

#[test]
fn admin_can_move_treasury_and_charity() {
    let mut l = launch();
    buy(&mut l, 10 * SOL);
    close_claim_window(&mut l);
    let (treasury, charity) = (Pubkey::new_unique(), Pubkey::new_unique());
    l.env.svm.airdrop(&treasury, TREASURY_START).unwrap();
    l.env.svm.airdrop(&charity, CHARITY_START).unwrap();
    let admin = l.env.admin.insecure_clone();
    let ix = update_config_ix(
        &admin.pubkey(),
        riff::UpdateConfigParams {
            treasury: Some(treasury),
            charity: Some(charity),
            verifier: None,
            raydium_amm_config: None,
            claim_window_secs: None,
        },
    );
    send(&mut l.env.svm, ix, &[&admin]).unwrap();
    let config: riff::Config = fetch(&l.env.svm, &config_address());
    assert_eq!(config.verifier, l.env.verifier.pubkey(), "unchanged");

    // Payouts follow the new addresses, and only the new addresses.
    let stranger = funded_keypair(&mut l.env.svm);
    let old_charity = l.env.charity;
    let ix = sweep_charity_fees_ix(&old_charity, &l.mint);
    assert!(send(&mut l.env.svm, ix, &[&stranger]).is_err());
    l.env.charity = charity;
    sweep(&mut l).unwrap();
    assert!(lamports(&l.env.svm, &charity) > CHARITY_START);
    let ix = collect_protocol_fees_ix(&treasury, &l.mint);
    send(&mut l.env.svm, ix, &[&stranger]).unwrap();
    assert!(lamports(&l.env.svm, &treasury) > TREASURY_START);
}

#[test]
fn only_the_admin_can_update_config() {
    let mut l = launch();
    let intruder = funded_keypair(&mut l.env.svm);
    let ix = update_config_ix(
        &intruder.pubkey(),
        riff::UpdateConfigParams {
            verifier: Some(intruder.pubkey()),
            ..Default::default()
        },
    );
    assert_riff_error(send(&mut l.env.svm, ix, &[&intruder]), ErrorCode::NotAdmin);
    let config: riff::Config = fetch(&l.env.svm, &config_address());
    assert_eq!(config.verifier, l.env.verifier.pubkey());
}

#[test]
fn config_rejects_the_all_zero_address() {
    let mut l = launch();
    let admin = l.env.admin.insecure_clone();
    let ix = update_config_ix(
        &admin.pubkey(),
        riff::UpdateConfigParams {
            charity: Some(Pubkey::default()),
            ..Default::default()
        },
    );
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&admin]),
        ErrorCode::InvalidAddress,
    );

    let mut env = setup();
    let mut params = config_params(&env);
    params.verifier = Pubkey::default();
    let ix = initialize_config_ix(&env.admin.pubkey(), params);
    let admin = env.admin.insecure_clone();
    assert_riff_error(send(&mut env.svm, ix, &[&admin]), ErrorCode::InvalidAddress);
}

#[test]
fn admin_can_change_the_claim_window_for_future_coins() {
    let mut l = launch();
    let old_deadline = coin(&l).claim_deadline;
    assert_eq!(old_deadline, START_TIME + CLAIM_WINDOW_SECS);

    let admin = l.env.admin.insecure_clone();
    let week = 7 * 24 * 60 * 60;
    let ix = update_config_ix(
        &admin.pubkey(),
        riff::UpdateConfigParams {
            claim_window_secs: Some(week),
            ..Default::default()
        },
    );
    let meta = send(&mut l.env.svm, ix, &[&admin]).unwrap();
    assert_eq!(
        events::<riff::events::ConfigUpdated>(&meta.logs)[0].claim_window_secs,
        week
    );
    let config: riff::Config = fetch(&l.env.svm, &config_address());
    assert_eq!(config.claim_window_secs, week);

    // A coin launched now gets the new window...
    advance_time(&mut l.env.svm, 100);
    let creator = funded_keypair(&mut l.env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), coin_args());
    send(&mut l.env.svm, ix, &[&creator, &mint]).unwrap();
    let new_coin: riff::Coin = fetch(&l.env.svm, &coin_address(&mint.pubkey()));
    assert_eq!(new_coin.claim_deadline, START_TIME + 100 + week);

    // ...and the existing coin keeps the deadline it launched with.
    assert_eq!(coin(&l).claim_deadline, old_deadline);
}

#[test]
fn claim_window_must_stay_positive() {
    let mut l = launch();
    let admin = l.env.admin.insecure_clone();
    for bad in [0, -1] {
        let ix = update_config_ix(
            &admin.pubkey(),
            riff::UpdateConfigParams {
                claim_window_secs: Some(bad),
                ..Default::default()
            },
        );
        assert_riff_error(
            send(&mut l.env.svm, ix, &[&admin]),
            ErrorCode::InvalidClaimWindow,
        );
    }
    let config: riff::Config = fetch(&l.env.svm, &config_address());
    assert_eq!(config.claim_window_secs, CLAIM_WINDOW_SECS);
}
