mod common;

use {
    anchor_lang::prelude::Pubkey, common::*, riff::error::ErrorCode, solana_keypair::Keypair,
    solana_signer::Signer,
};

struct Launch {
    env: Env,
    creator: Keypair,
    mint: Pubkey,
}

/// Config from `set`, then one coin.
fn launch(set: impl Fn(&mut riff::ConfigParams)) -> Launch {
    let mut env = setup();
    let mut params = config_params(env.treasury);
    set(&mut params);
    let admin = env.admin.insecure_clone();
    send(
        &mut env.svm,
        initialize_config_ix(&admin.pubkey(), params),
        &[&admin],
    )
    .unwrap();
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), coin_args());
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();
    Launch {
        env,
        creator,
        mint: mint.pubkey(),
    }
}

fn coin(l: &Launch) -> riff::Coin {
    fetch(&l.env.svm, &coin_address(&l.mint))
}

/// Rent-exempt minimum for the coin account, computed from its actual size
/// rather than trusting LiteSVM to enforce it.
fn coin_rent_minimum(l: &Launch) -> u64 {
    let len = l
        .env
        .svm
        .get_account(&coin_address(&l.mint))
        .unwrap()
        .data
        .len();
    l.env.svm.minimum_balance_for_rent_exemption(len)
}

fn coin_lamports(l: &Launch) -> u64 {
    lamports(&l.env.svm, &coin_address(&l.mint))
}

/// Several traders buy and then sell everything, accruing all fee types.
fn trade_round_trips(l: &mut Launch) {
    let traders: Vec<Keypair> = (0..3)
        .map(|_| trader(&mut l.env.svm, &l.mint, 50 * SOL))
        .collect();
    let payer = l.env.admin.insecure_clone();
    for (t, sol) in traders.iter().zip([5 * SOL, SOL / 3, 12 * SOL]) {
        let ix = buy_ix(&t.pubkey(), &l.mint, sol, 0);
        send(&mut l.env.svm, ix, &[&payer, t]).unwrap();
    }
    for t in traders.iter().rev() {
        let all = token_balance(&l.env.svm, &ata_address(&t.pubkey(), &l.mint));
        let ix = sell_ix(&t.pubkey(), &l.mint, all, 0);
        send(&mut l.env.svm, ix, &[&payer, t]).unwrap();
    }
}

fn withdraw_creator(l: &mut Launch) -> litesvm::types::TransactionResult {
    let ix = withdraw_creator_fees_ix(&l.creator.pubkey(), &l.mint);
    let creator = l.creator.insecure_clone();
    send(&mut l.env.svm, ix, &[&creator])
}

fn collect_protocol(l: &mut Launch) -> litesvm::types::TransactionResult {
    let stranger = funded_keypair(&mut l.env.svm);
    let ix = collect_protocol_fees_ix(&l.env.treasury, &l.mint);
    send(&mut l.env.svm, ix, &[&stranger])
}

#[test]
fn draining_every_fee_balance_leaves_coin_rent_exempt_and_usable() {
    // No artist fee, so every fee balance can be drained with today's
    // instructions (the artist withdrawal doesn't exist yet).
    let mut l = launch(|p| {
        p.artist_fee_bps = 0;
        p.creator_fee_bps = 40;
        p.protocol_fee_bps = 60;
    });
    trade_round_trips(&mut l);
    assert!(coin(&l).creator_fees > 0 && coin(&l).protocol_fees > 0);

    withdraw_creator(&mut l).unwrap();
    collect_protocol(&mut l).unwrap();

    let drained = coin(&l);
    assert_eq!(drained.artist_fees, 0);
    assert_eq!(drained.creator_fees, 0);
    assert_eq!(drained.protocol_fees, 0);
    assert_eq!(drained.real_token_reserves, CURVE_TOKEN_SUPPLY);
    // Only the curve's rounding dust and the rent remain.
    assert!(drained.real_sol_reserves < 100);
    let rent_min = coin_rent_minimum(&l);
    assert_eq!(coin_lamports(&l), rent_min + drained.real_sol_reserves);
    assert!(coin_lamports(&l) >= rent_min, "coin must stay rent-exempt");

    // Still fully usable: trade again, accrue again, pay out again.
    let alice = trader(&mut l.env.svm, &l.mint, 10 * SOL);
    let payer = l.env.admin.insecure_clone();
    let ix = buy_ix(&alice.pubkey(), &l.mint, 2 * SOL, 0);
    send(&mut l.env.svm, ix, &[&payer, &alice]).unwrap();
    let half = token_balance(&l.env.svm, &ata_address(&alice.pubkey(), &l.mint)) / 2;
    let ix = sell_ix(&alice.pubkey(), &l.mint, half, 0);
    send(&mut l.env.svm, ix, &[&payer, &alice]).unwrap();
    withdraw_creator(&mut l).unwrap();
    collect_protocol(&mut l).unwrap();
    let c = coin(&l);
    assert_eq!(coin_lamports(&l), rent_min + c.real_sol_reserves);
}

#[test]
fn draining_creator_and_protocol_fees_leaves_artist_fees_backed() {
    let mut l = launch(|_| {});
    trade_round_trips(&mut l);
    let artist_fees = coin(&l).artist_fees;
    assert!(artist_fees > 0);

    withdraw_creator(&mut l).unwrap();
    collect_protocol(&mut l).unwrap();

    let c = coin(&l);
    assert_eq!(c.artist_fees, artist_fees, "artist fees untouched");
    assert_eq!(
        coin_lamports(&l),
        coin_rent_minimum(&l) + c.real_sol_reserves + artist_fees
    );
}

/// Takes `short_by` lamports off the coin account, as if some bug had
/// paid out more than it should have.
fn underfund_coin(l: &mut Launch, short_by: u64) {
    let key = coin_address(&l.mint);
    let mut account = l.env.svm.get_account(&key).unwrap();
    account.lamports -= short_by;
    l.env.svm.set_account(key, account).unwrap();
}

#[test]
fn payouts_refuse_to_underfund_the_coin() {
    let mut l = launch(|_| {});
    trade_round_trips(&mut l);
    underfund_coin(&mut l, 1);

    assert_riff_error(withdraw_creator(&mut l), ErrorCode::CoinUnderfunded);
    assert_riff_error(collect_protocol(&mut l), ErrorCode::CoinUnderfunded);
    assert!(coin(&l).creator_fees > 0 && coin(&l).protocol_fees > 0);
}

#[test]
fn sells_refuse_to_underfund_the_coin() {
    let mut l = launch(|_| {});
    let alice = trader(&mut l.env.svm, &l.mint, 10 * SOL);
    let payer = l.env.admin.insecure_clone();
    let ix = buy_ix(&alice.pubkey(), &l.mint, SOL, 0);
    send(&mut l.env.svm, ix, &[&payer, &alice]).unwrap();
    underfund_coin(&mut l, 1);

    let tokens = token_balance(&l.env.svm, &ata_address(&alice.pubkey(), &l.mint));
    let ix = sell_ix(&alice.pubkey(), &l.mint, tokens, 0);
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&payer, &alice]),
        ErrorCode::CoinUnderfunded,
    );
}

#[test]
fn sol_sent_directly_to_the_coin_does_not_break_payouts() {
    let mut l = launch(|_| {});
    trade_round_trips(&mut l);
    // Anyone can send SOL to any address. Extra lamports must not break the
    // accounting, and nobody can withdraw them as fees.
    l.env.svm.airdrop(&coin_address(&l.mint), 5 * SOL).unwrap();
    let fees = coin(&l).creator_fees;
    let before = lamports(&l.env.svm, &l.creator.pubkey());

    withdraw_creator(&mut l).unwrap();
    // The creator paid this transaction's network fee (5,000 lamports).
    assert_eq!(
        lamports(&l.env.svm, &l.creator.pubkey()),
        before + fees - 5_000
    );
}
