//! The running totals each coin keeps for the app: lifetime artist share,
//! peak price and hourly volume for the last 24 hours.
mod common;

use {
    anchor_lang::prelude::Pubkey,
    common::*,
    riff::{events::Trade, VOLUME_HOURS},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

const HOUR: i64 = 3600;

fn coin(env: &Env, mint: &Pubkey) -> riff::Coin {
    fetch(&env.svm, &coin_address(mint))
}

/// Sends a trade and returns its Trade event.
fn trade(
    env: &mut Env,
    trader: &Keypair,
    ix: anchor_lang::solana_program::instruction::Instruction,
) -> Trade {
    let payer = env.admin.insecure_clone();
    let meta = send(&mut env.svm, ix, &[&payer, trader]).unwrap();
    events::<Trade>(&meta.logs).remove(0)
}

fn bucket(hour: i64) -> usize {
    hour.rem_euclid(VOLUME_HOURS as i64) as usize
}

/// Rolling 24-hour volume as the app computes it at `now`.
fn volume_24h(c: &riff::Coin, now: i64) -> u64 {
    let now_hour = now.div_euclid(HOUR);
    (0..VOLUME_HOURS as i64)
        .map(|k| c.volume_hour - k)
        .filter(|h| *h > now_hour - VOLUME_HOURS as i64)
        .map(|h| c.volume_hourly[bucket(h)])
        .sum()
}

#[test]
fn a_new_coin_starts_with_empty_totals() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let c = coin(&env, &mint);
    assert_eq!(c.artist_fees_total, 0);
    assert_eq!(c.peak_virtual_sol, INITIAL_VIRTUAL_SOL);
    assert_eq!(c.volume_hour, START_TIME.div_euclid(HOUR));
    assert_eq!(c.volume_hourly, [0; VOLUME_HOURS]);
}

#[test]
fn trades_add_to_artist_share_volume_and_peak() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);

    let bought = trade(&mut env, &alice, buy_ix(&alice.pubkey(), &mint, 2 * SOL, 0));
    let after_buy = coin(&env, &mint);
    assert_eq!(after_buy.peak_virtual_sol, after_buy.virtual_sol_reserves);
    assert!(after_buy.peak_virtual_sol > INITIAL_VIRTUAL_SOL);

    let sold = trade(
        &mut env,
        &alice,
        sell_ix(&alice.pubkey(), &mint, bought.token_amount / 2, 0),
    );
    let c = coin(&env, &mint);
    assert_eq!(c.artist_fees_total, bought.artist_fee + sold.artist_fee);
    assert_eq!(
        c.volume_hourly[bucket(c.volume_hour)],
        bought.sol_amount + sold.sol_amount
    );
    assert_eq!(
        volume_24h(&c, START_TIME),
        bought.sol_amount + sold.sol_amount
    );
    // A sell lowers the price but not the peak.
    assert!(c.virtual_sol_reserves < c.peak_virtual_sol);
    assert_eq!(c.peak_virtual_sol, after_buy.peak_virtual_sol);
}

#[test]
fn the_launch_buy_counts() {
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let mut args = coin_args();
    args.creator_buy_sol = SOL / 2;
    let meta = send(
        &mut env.svm,
        create_coin_ix(&creator.pubkey(), &mint.pubkey(), args),
        &[&creator, &mint],
    )
    .unwrap();
    let launch = events::<Trade>(&meta.logs).remove(0);
    let c = coin(&env, &mint.pubkey());
    assert_eq!(c.artist_fees_total, launch.artist_fee);
    assert_eq!(volume_24h(&c, START_TIME), launch.sol_amount);
    assert_eq!(c.peak_virtual_sol, c.virtual_sol_reserves);
}

#[test]
fn the_artist_share_counts_even_when_it_goes_to_charity() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    advance_time(&mut env.svm, CLAIM_WINDOW_SECS + 1);

    let t = trade(&mut env, &alice, buy_ix(&alice.pubkey(), &mint, SOL, 0));
    let c = coin(&env, &mint);
    assert_eq!(c.charity_fees, t.artist_fee);
    assert_eq!(c.artist_fees, 0);
    assert_eq!(c.artist_fees_total, t.artist_fee);
}

#[test]
fn volume_rolls_over_by_the_hour() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 100 * SOL);
    let h0 = START_TIME.div_euclid(HOUR);
    let buy = |env: &mut Env, sol: u64| {
        trade(env, &alice, buy_ix(&alice.pubkey(), &mint, sol, 0)).sol_amount
    };

    let a = buy(&mut env, SOL);
    advance_time(&mut env.svm, HOUR);
    let b = buy(&mut env, 2 * SOL);
    let c = coin(&env, &mint);
    assert_eq!(c.volume_hour, h0 + 1);
    assert_eq!(c.volume_hourly[bucket(h0)], a);
    assert_eq!(c.volume_hourly[bucket(h0 + 1)], b);
    assert_eq!(volume_24h(&c, START_TIME + HOUR), a + b);

    // 23 hours later the first hour has dropped out of the window, and its
    // bucket is reused for the new hour.
    advance_time(&mut env.svm, 23 * HOUR);
    let now = START_TIME + 24 * HOUR;
    assert_eq!(
        volume_24h(&coin(&env, &mint), now),
        b,
        "read before any new trade"
    );
    let d = buy(&mut env, 3 * SOL);
    let c = coin(&env, &mint);
    assert_eq!(c.volume_hour, h0 + 24);
    assert_eq!(c.volume_hourly[bucket(h0 + 24)], d);
    assert_eq!(c.volume_hourly[bucket(h0 + 1)], b);
    assert_eq!(volume_24h(&c, now), b + d);

    // After a quiet day and more, only the newest trade remains.
    advance_time(&mut env.svm, 30 * HOUR);
    let e = buy(&mut env, SOL);
    let c = coin(&env, &mint);
    assert_eq!(c.volume_hourly.iter().sum::<u64>(), e);
    assert_eq!(volume_24h(&c, now + 30 * HOUR), e);
    // Nobody traded for 24 hours: the window is empty again.
    assert_eq!(volume_24h(&c, now + 54 * HOUR), 0);
}
