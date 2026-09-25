mod common;

use {
    anchor_lang::prelude::Pubkey,
    common::*,
    riff::{
        curve::{quote_buy, quote_sell, split_fee},
        error::ErrorCode,
        COIN_TOTAL_SUPPLY,
    },
    solana_keypair::Keypair,
    solana_signer::Signer,
};

/// The coin account must always hold exactly its rent, the curve's SOL, and
/// the unpaid artist fees — no more, no less.
fn assert_coin_solvent(env: &Env, mint: &Pubkey) {
    let coin_key = coin_address(mint);
    let coin: riff::Coin = fetch(&env.svm, &coin_key);
    let data_len = env.svm.get_account(&coin_key).unwrap().data.len();
    let rent = env.svm.minimum_balance_for_rent_exemption(data_len);
    assert_eq!(
        lamports(&env.svm, &coin_key),
        rent + coin.real_sol_reserves + coin.artist_fees + coin.creator_fees
    );
    // Vault holds the unsold curve tokens plus the graduation reserve.
    assert_eq!(
        token_balance(&env.svm, &vault_address(mint)),
        coin.real_token_reserves + (COIN_TOTAL_SUPPLY - CURVE_TOKEN_SUPPLY)
    );
}

/// Sends a trade with the admin paying the network fee, so the trader's
/// balance changes by exactly the trade amount.
// TransactionResult is LiteSVM's type; its large Err is not ours to box.
#[allow(clippy::result_large_err)]
fn trade(
    env: &mut Env,
    trader: &Keypair,
    ix: anchor_lang::solana_program::instruction::Instruction,
) -> litesvm::types::TransactionResult {
    let payer = env.admin.insecure_clone();
    send(&mut env.svm, ix, &[&payer, trader])
}

#[test]
fn buy_delivers_tokens_and_splits_fee() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let coin_key = coin_address(&mint);

    let before: riff::Coin = fetch(&env.svm, &coin_key);
    let quote = quote_buy(&before.reserves(), TOTAL_FEE_BPS, SOL).unwrap();
    let fees = split_fee(quote.fee, ARTIST_FEE_BPS, CREATOR_FEE_BPS, PROTOCOL_FEE_BPS);
    let coin_lamports = lamports(&env.svm, &coin_key);
    let alice_lamports = lamports(&env.svm, &alice.pubkey());

    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, SOL, quote.tokens_out);
    trade(&mut env, &alice, ix).unwrap();

    // 1% fee: 0.5% artist, 0.2% creator, 0.3% treasury.
    assert_eq!(quote.fee, 10_000_000);
    assert_eq!(fees.artist, 5_000_000);
    assert_eq!(fees.creator, 2_000_000);
    assert_eq!(fees.protocol, 3_000_000);

    assert_eq!(
        token_balance(&env.svm, &ata_address(&alice.pubkey(), &mint)),
        quote.tokens_out
    );
    assert_eq!(lamports(&env.svm, &alice.pubkey()), alice_lamports - SOL);
    assert_eq!(
        lamports(&env.svm, &env.treasury),
        TREASURY_START + fees.protocol
    );
    assert_eq!(
        lamports(&env.svm, &coin_key),
        coin_lamports + quote.sol_to_curve + fees.artist + fees.creator
    );

    let after: riff::Coin = fetch(&env.svm, &coin_key);
    assert_eq!(
        after.reserves(),
        before.reserves().apply_buy(&quote).unwrap()
    );
    assert_eq!(after.artist_fees, fees.artist);
    assert_eq!(after.creator_fees, fees.creator);
    assert!(!after.complete);
    assert_coin_solvent(&env, &mint);
}

#[test]
fn sell_returns_sol_minus_fee() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let alice_ata = ata_address(&alice.pubkey(), &mint);
    let coin_key = coin_address(&mint);

    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, 2 * SOL, 0);
    trade(&mut env, &alice, ix).unwrap();
    let tokens = token_balance(&env.svm, &alice_ata);
    let before: riff::Coin = fetch(&env.svm, &coin_key);
    let alice_before = lamports(&env.svm, &alice.pubkey());
    let treasury_before = lamports(&env.svm, &env.treasury);

    let quote = quote_sell(&before.reserves(), TOTAL_FEE_BPS, tokens).unwrap();
    let fees = split_fee(quote.fee, ARTIST_FEE_BPS, CREATOR_FEE_BPS, PROTOCOL_FEE_BPS);
    let ix = sell_ix(&alice.pubkey(), &mint, &env.treasury, tokens, quote.sol_out);
    trade(&mut env, &alice, ix).unwrap();

    assert_eq!(token_balance(&env.svm, &alice_ata), 0);
    assert_eq!(
        lamports(&env.svm, &alice.pubkey()),
        alice_before + quote.sol_out
    );
    assert_eq!(
        lamports(&env.svm, &env.treasury),
        treasury_before + fees.protocol
    );

    let after: riff::Coin = fetch(&env.svm, &coin_key);
    assert_eq!(after.artist_fees, before.artist_fees + fees.artist);
    assert_eq!(after.creator_fees, before.creator_fees + fees.creator);
    assert_eq!(after.real_token_reserves, CURVE_TOKEN_SUPPLY);
    // A full round trip costs roughly two 1% fees, never profits.
    assert!(lamports(&env.svm, &alice.pubkey()) < 10 * SOL);
    assert!(lamports(&env.svm, &alice.pubkey()) > 10 * SOL * 97 / 100);
    assert_coin_solvent(&env, &mint);
}

#[test]
fn buy_rejects_slippage() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);

    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint));
    let quote = quote_buy(&coin.reserves(), TOTAL_FEE_BPS, SOL).unwrap();
    let ix = buy_ix(
        &alice.pubkey(),
        &mint,
        &env.treasury,
        SOL,
        quote.tokens_out + 1,
    );
    assert_riff_error(trade(&mut env, &alice, ix), ErrorCode::SlippageExceeded);
}

#[test]
fn sell_rejects_slippage() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, SOL, 0);
    trade(&mut env, &alice, ix).unwrap();
    let tokens = token_balance(&env.svm, &ata_address(&alice.pubkey(), &mint));

    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint));
    let quote = quote_sell(&coin.reserves(), TOTAL_FEE_BPS, tokens).unwrap();
    let ix = sell_ix(
        &alice.pubkey(),
        &mint,
        &env.treasury,
        tokens,
        quote.sol_out + 1,
    );
    assert_riff_error(trade(&mut env, &alice, ix), ErrorCode::SlippageExceeded);
}

#[test]
fn rejects_zero_amounts() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);

    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, 0, 0);
    assert_riff_error(trade(&mut env, &alice, ix), ErrorCode::AmountTooSmall);
    let ix = sell_ix(&alice.pubkey(), &mint, &env.treasury, 0, 0);
    assert_riff_error(trade(&mut env, &alice, ix), ErrorCode::AmountTooSmall);
}

#[test]
fn cannot_sell_tokens_you_do_not_have() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let bob = trader(&mut env.svm, &mint, 10 * SOL);
    // Alice's buy puts SOL in the curve, so the sell fails on Bob's balance.
    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, 5 * SOL, 0);
    trade(&mut env, &alice, ix).unwrap();

    let ix = sell_ix(&bob.pubkey(), &mint, &env.treasury, 1_000 * TOKEN, 0);
    assert!(trade(&mut env, &bob, ix).is_err());
    assert_coin_solvent(&env, &mint);
}

#[test]
fn cannot_use_someone_elses_token_account() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let bob = trader(&mut env.svm, &mint, 10 * SOL);
    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, SOL, 0);
    trade(&mut env, &alice, ix).unwrap();

    // Bob signs, but points at Alice's token account.
    let mut ix = sell_ix(&bob.pubkey(), &mint, &env.treasury, TOKEN, 0);
    let alice_ata = ata_address(&alice.pubkey(), &mint);
    let bob_ata = ata_address(&bob.pubkey(), &mint);
    for meta in &mut ix.accounts {
        if meta.pubkey == bob_ata {
            meta.pubkey = alice_ata;
        }
    }
    assert!(trade(&mut env, &bob, ix).is_err());
}

#[test]
fn fees_must_go_to_configured_treasury() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let thief = Pubkey::new_unique();
    env.svm.airdrop(&thief, TREASURY_START).unwrap();

    let ix = buy_ix(&alice.pubkey(), &mint, &thief, SOL, 0);
    assert!(trade(&mut env, &alice, ix).is_err());
    assert_eq!(lamports(&env.svm, &thief), TREASURY_START);
}

#[test]
fn final_buy_completes_curve_and_stops_trading() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let whale = trader(&mut env.svm, &mint, 1_000 * SOL);
    let whale_ata = ata_address(&whale.pubkey(), &mint);

    // Far more than the curve needs: only the remainder is bought and paid for.
    let ix = buy_ix(&whale.pubkey(), &mint, &env.treasury, 500 * SOL, 0);
    trade(&mut env, &whale, ix).unwrap();

    assert_eq!(token_balance(&env.svm, &whale_ata), CURVE_TOKEN_SUPPLY);
    let spent = 1_000 * SOL - lamports(&env.svm, &whale.pubkey());
    assert!((85 * SOL..95 * SOL).contains(&spent), "spent {spent}");

    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint));
    assert!(coin.complete);
    assert_eq!(coin.real_token_reserves, 0);
    assert_coin_solvent(&env, &mint);

    let ix = buy_ix(&whale.pubkey(), &mint, &env.treasury, SOL, 0);
    assert_riff_error(trade(&mut env, &whale, ix), ErrorCode::CurveComplete);
    let ix = sell_ix(&whale.pubkey(), &mint, &env.treasury, TOKEN, 0);
    assert_riff_error(trade(&mut env, &whale, ix), ErrorCode::CurveComplete);
}

#[test]
fn many_traders_round_trip_stays_solvent() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let traders: Vec<Keypair> = (0..4)
        .map(|_| trader(&mut env.svm, &mint, 100 * SOL))
        .collect();
    let balances = |env: &Env| -> u64 {
        traders
            .iter()
            .map(|t| lamports(&env.svm, &t.pubkey()))
            .sum()
    };
    let traders_start = balances(&env);

    // Interleaved buys and partial sells.
    let amounts = [3 * SOL, SOL / 7, 11 * SOL, 123_456_789];
    for (t, amount) in traders.iter().zip(amounts) {
        let ix = buy_ix(&t.pubkey(), &mint, &env.treasury, amount, 0);
        trade(&mut env, t, ix).unwrap();
        assert_coin_solvent(&env, &mint);
    }
    for t in &traders {
        let half = token_balance(&env.svm, &ata_address(&t.pubkey(), &mint)) / 2;
        let ix = sell_ix(&t.pubkey(), &mint, &env.treasury, half, 0);
        trade(&mut env, t, ix).unwrap();
        assert_coin_solvent(&env, &mint);
    }
    // Everyone exits, in reverse order.
    for t in traders.iter().rev() {
        let rest = token_balance(&env.svm, &ata_address(&t.pubkey(), &mint));
        let ix = sell_ix(&t.pubkey(), &mint, &env.treasury, rest, 0);
        trade(&mut env, t, ix).unwrap();
        assert_coin_solvent(&env, &mint);
    }

    // All tokens are back on the curve and only rounding dust remains as SOL.
    // Early buyers may profit from later ones, but as a group traders lose
    // exactly what went to fees and dust — every lamport is accounted for.
    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint));
    assert_eq!(coin.real_token_reserves, CURVE_TOKEN_SUPPLY);
    assert!(
        coin.real_sol_reserves < 100,
        "dust {}",
        coin.real_sol_reserves
    );
    assert!(coin.artist_fees > 0);
    let protocol_fees = lamports(&env.svm, &env.treasury) - TREASURY_START;
    assert_eq!(
        traders_start - balances(&env),
        protocol_fees + coin.artist_fees + coin.creator_fees + coin.real_sol_reserves
    );
}

#[test]
fn creator_withdraws_accrued_fees() {
    let mut env = setup();
    let (creator, mint) = setup_coin_with_creator(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, 3 * SOL, 0);
    trade(&mut env, &alice, ix).unwrap();
    let tokens = token_balance(&env.svm, &ata_address(&alice.pubkey(), &mint));
    let ix = sell_ix(&alice.pubkey(), &mint, &env.treasury, tokens / 2, 0);
    trade(&mut env, &alice, ix).unwrap();

    let coin_key = coin_address(&mint);
    let accrued = fetch::<riff::Coin>(&env.svm, &coin_key).creator_fees;
    assert!(accrued > 0);
    let artist_fees = fetch::<riff::Coin>(&env.svm, &coin_key).artist_fees;
    let creator_before = lamports(&env.svm, &creator.pubkey());

    let ix = withdraw_creator_fees_ix(&creator.pubkey(), &mint);
    let meta = trade(&mut env, &creator, ix).unwrap();

    assert_eq!(
        lamports(&env.svm, &creator.pubkey()),
        creator_before + accrued
    );
    let coin: riff::Coin = fetch(&env.svm, &coin_key);
    assert_eq!(coin.creator_fees, 0);
    assert_eq!(coin.artist_fees, artist_fees, "artist fees untouched");
    assert_coin_solvent(&env, &mint);
    let withdrawn = events::<riff::events::CreatorFeesWithdrawn>(&meta.logs);
    assert_eq!(withdrawn.len(), 1);
    assert_eq!(withdrawn[0].amount, accrued);

    // Nothing left to withdraw.
    let ix = withdraw_creator_fees_ix(&creator.pubkey(), &mint);
    assert_riff_error(trade(&mut env, &creator, ix), ErrorCode::NoFeesToWithdraw);
}

#[test]
fn only_creator_can_withdraw_creator_fees() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let ix = buy_ix(&alice.pubkey(), &mint, &env.treasury, SOL, 0);
    trade(&mut env, &alice, ix).unwrap();

    let ix = withdraw_creator_fees_ix(&alice.pubkey(), &mint);
    assert!(trade(&mut env, &alice, ix).is_err());
    assert!(fetch::<riff::Coin>(&env.svm, &coin_address(&mint)).creator_fees > 0);
}

/// Relative gap between the curve's final price and the price a DEX pool
/// would open at, pairing all raised SOL with the tokens left in the vault.
fn graduation_price_gap(env: &Env, mint: &Pubkey) -> f64 {
    let coin: riff::Coin = fetch(&env.svm, &coin_address(mint));
    assert!(coin.complete);
    let reserve = token_balance(&env.svm, &vault_address(mint)) as u128;
    assert_eq!(reserve, (COIN_TOTAL_SUPPLY - CURVE_TOKEN_SUPPLY) as u128);
    // final = virtual_sol / virtual_token; pool = real_sol / reserve.
    let final_x = coin.virtual_sol_reserves as u128 * reserve;
    let pool_x = coin.real_sol_reserves as u128 * coin.virtual_token_reserves as u128;
    (pool_x as f64 - final_x as f64).abs() / final_x as f64
}

#[test]
fn graduation_pool_opens_at_curve_final_price() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let whale = trader(&mut env.svm, &mint, 1_000 * SOL);
    let ix = buy_ix(&whale.pubkey(), &mint, &env.treasury, 500 * SOL, 0);
    trade(&mut env, &whale, ix).unwrap();

    let gap = graduation_price_gap(&env, &mint);
    assert!(gap < 1e-9, "gap {gap:e}");
}

#[test]
fn graduation_price_holds_after_mixed_trading() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let traders: Vec<Keypair> = (0..3)
        .map(|_| trader(&mut env.svm, &mint, 1_000 * SOL))
        .collect();
    for (t, amount) in traders.iter().zip([7 * SOL, 13 * SOL + 7, SOL / 3]) {
        let ix = buy_ix(&t.pubkey(), &mint, &env.treasury, amount, 0);
        trade(&mut env, t, ix).unwrap();
        let some = token_balance(&env.svm, &ata_address(&t.pubkey(), &mint)) / 3;
        let ix = sell_ix(&t.pubkey(), &mint, &env.treasury, some, 0);
        trade(&mut env, t, ix).unwrap();
    }
    let ix = buy_ix(&traders[0].pubkey(), &mint, &env.treasury, 500 * SOL, 0);
    trade(&mut env, &traders[0], ix).unwrap();

    let gap = graduation_price_gap(&env, &mint);
    assert!(gap < 1e-9, "gap {gap:e}");
}
