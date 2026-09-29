use {
    crate::common::*, anchor_lang::prelude::Pubkey, riff::error::ErrorCode,
    solana_keypair::Keypair, solana_signer::Signer,
};

/// A treasury like a fresh Squads vault: an off-curve address (so nobody
/// holds its key) that has never received any SOL.
fn unfunded_treasury() -> Pubkey {
    Pubkey::find_program_address(&[b"multisig", b"vault"], &Pubkey::new_unique()).0
}

fn setup_with_treasury(treasury: Pubkey) -> (Env, Pubkey) {
    let mut env = setup();
    env.treasury = treasury;
    let mint = setup_coin(&mut env);
    (env, mint)
}

/// Rent-exempt minimum for an account with no data, such as a wallet or a
/// Squads vault. Computed here rather than trusting LiteSVM to enforce it:
/// LiteSVM skips the rent check for accounts with no data, mainnet doesn't.
fn wallet_rent_minimum(env: &Env) -> u64 {
    env.svm.minimum_balance_for_rent_exemption(0)
}

fn buy(env: &mut Env, trader: &Keypair, mint: &Pubkey, sol: u64) {
    let payer = env.admin.insecure_clone();
    let ix = buy_ix(&trader.pubkey(), mint, sol, 0);
    send(&mut env.svm, ix, &[&payer, trader]).unwrap();
}

fn protocol_fees(env: &Env, mint: &Pubkey) -> u64 {
    fetch::<riff::Coin>(&env.svm, &coin_address(mint)).protocol_fees
}

/// Anyone can collect; a stranger pays the network fee here.
#[allow(clippy::result_large_err)]
fn collect(env: &mut Env, treasury: &Pubkey, mint: &Pubkey) -> litesvm::types::TransactionResult {
    let stranger = funded_keypair(&mut env.svm);
    send(
        &mut env.svm,
        collect_protocol_fees_ix(treasury, mint),
        &[&stranger],
    )
}

#[test]
fn trades_work_with_unfunded_treasury() {
    let treasury = unfunded_treasury();
    let (mut env, mint) = setup_with_treasury(treasury);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);

    // A tiny buy: its 0.3% protocol fee is far below the rent minimum.
    buy(&mut env, &alice, &mint, SOL / 100);
    let tokens = token_balance(&env.svm, &ata_address(&alice.pubkey(), &mint));
    let payer = env.admin.insecure_clone();
    let ix = sell_ix(&alice.pubkey(), &mint, tokens, 0);
    send(&mut env.svm, ix, &[&payer, &alice]).unwrap();

    assert!(protocol_fees(&env, &mint) > 0);
    assert!(protocol_fees(&env, &mint) < wallet_rent_minimum(&env));
    assert!(
        env.svm.get_account(&treasury).is_none(),
        "treasury untouched"
    );
}

#[test]
fn collect_sends_accrued_fees_to_treasury() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    buy(&mut env, &alice, &mint, 2 * SOL);
    let accrued = protocol_fees(&env, &mint);
    let coin_before: riff::Coin = fetch(&env.svm, &coin_address(&mint));

    let treasury = env.treasury;
    let meta = collect(&mut env, &treasury, &mint).unwrap();

    assert_eq!(lamports(&env.svm, &treasury), TREASURY_START + accrued);
    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint));
    assert_eq!(coin.protocol_fees, 0);
    assert_eq!(coin.artist_fees, coin_before.artist_fees);
    assert_eq!(coin.creator_fees, coin_before.creator_fees);
    assert_eq!(coin.real_sol_reserves, coin_before.real_sol_reserves);
    let collected = events::<riff::events::ProtocolFeesCollected>(&meta.logs);
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].amount, accrued);
    assert_eq!(collected[0].treasury, treasury);

    // Nothing left: a second collection fails cleanly.
    assert_riff_error(
        collect(&mut env, &treasury, &mint),
        ErrorCode::NoFeesToWithdraw,
    );
}

#[test]
fn collect_refuses_to_leave_unfunded_treasury_below_rent_minimum() {
    let treasury = unfunded_treasury();
    let (mut env, mint) = setup_with_treasury(treasury);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    let rent_min = wallet_rent_minimum(&env);

    // 0.1 SOL buy: 300,000 lamports of protocol fee, under the minimum.
    buy(&mut env, &alice, &mint, SOL / 10);
    let accrued = protocol_fees(&env, &mint);
    assert!(accrued < rent_min);

    assert_riff_error(
        collect(&mut env, &treasury, &mint),
        ErrorCode::TreasuryNotRentExempt,
    );
    assert!(env.svm.get_account(&treasury).is_none());
    assert_eq!(protocol_fees(&env, &mint), accrued, "fees kept for later");

    // Once enough has accrued, collection succeeds and funds the treasury.
    buy(&mut env, &alice, &mint, SOL / 2);
    let accrued = protocol_fees(&env, &mint);
    assert!(accrued >= rent_min);
    collect(&mut env, &treasury, &mint).unwrap();

    let balance = lamports(&env.svm, &treasury);
    assert_eq!(balance, accrued);
    assert!(balance >= rent_min, "treasury must end rent-exempt");
}

#[test]
fn collect_rent_rule_is_exact_at_the_boundary() {
    let treasury = unfunded_treasury();
    let (mut env, mint) = setup_with_treasury(treasury);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    buy(&mut env, &alice, &mint, SOL / 10);
    let accrued = protocol_fees(&env, &mint);
    let rent_min = wallet_rent_minimum(&env);

    // Pre-fund so collecting would land 1 lamport short of the minimum.
    // (Mainnet wouldn't allow this balance at all; LiteSVM's airdrop does.)
    env.svm.airdrop(&treasury, rent_min - accrued - 1).unwrap();
    assert_riff_error(
        collect(&mut env, &treasury, &mint),
        ErrorCode::TreasuryNotRentExempt,
    );

    // One more lamport and it lands exactly on the minimum: allowed.
    env.svm.airdrop(&treasury, 1).unwrap();
    collect(&mut env, &treasury, &mint).unwrap();
    assert_eq!(lamports(&env.svm, &treasury), rent_min);
}

#[test]
fn collect_only_pays_configured_treasury() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let alice = trader(&mut env.svm, &mint, 10 * SOL);
    buy(&mut env, &alice, &mint, SOL);
    let accrued = protocol_fees(&env, &mint);

    let thief = Pubkey::new_unique();
    env.svm.airdrop(&thief, TREASURY_START).unwrap();
    assert!(collect(&mut env, &thief, &mint).is_err());
    assert_eq!(lamports(&env.svm, &thief), TREASURY_START);
    assert_eq!(protocol_fees(&env, &mint), accrued);
}

#[test]
fn collect_from_several_coins() {
    let mut env = setup();
    let mint_a = setup_coin(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let mint_b = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint_b.pubkey(), coin_args());
    send(&mut env.svm, ix, &[&creator, &mint_b]).unwrap();
    next_slot(&mut env.svm);
    let mint_b = mint_b.pubkey();

    let alice = trader(&mut env.svm, &mint_a, 10 * SOL);
    let bob = trader(&mut env.svm, &mint_b, 10 * SOL);
    buy(&mut env, &alice, &mint_a, SOL);
    buy(&mut env, &bob, &mint_b, 3 * SOL);
    let total = protocol_fees(&env, &mint_a) + protocol_fees(&env, &mint_b);

    let treasury = env.treasury;
    collect(&mut env, &treasury, &mint_a).unwrap();
    collect(&mut env, &treasury, &mint_b).unwrap();
    assert_eq!(lamports(&env.svm, &treasury), TREASURY_START + total);
}
