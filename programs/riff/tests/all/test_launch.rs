//! M-02: the creator-buy cap must hold for the whole launch, not just the
//! `creator_buy_sol` field. `buy` is closed in the coin's creation slot.

use {crate::common::*, riff::error::ErrorCode, solana_keypair::Keypair, solana_signer::Signer};

const CAP: u64 = 30_000_000 * TOKEN;

fn launch_args() -> riff::CreateCoinArgs {
    let mut args = coin_args();
    args.creator_buy_sol = SOL / 2;
    args
}

#[test]
fn creator_cannot_bundle_a_buy_into_the_launch_transaction() {
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ixs = [
        create_coin_ix(&creator.pubkey(), &mint.pubkey(), launch_args()),
        buy_ix(&creator.pubkey(), &mint.pubkey(), 5 * SOL, 0),
    ];

    assert_riff_error(
        send_many(&mut env.svm, &ixs, &[&creator, &mint]),
        ErrorCode::TradingNotOpen,
    );
    // Atomic: the launch itself didn't happen either.
    assert!(env.svm.get_account(&coin_address(&mint.pubkey())).is_none());
}

#[test]
fn another_wallet_cannot_buy_in_the_launch_transaction() {
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let sidekick = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ixs = [
        create_coin_ix(&creator.pubkey(), &mint.pubkey(), launch_args()),
        create_ata_ix(&creator.pubkey(), &sidekick.pubkey(), &mint.pubkey()),
        buy_ix(&sidekick.pubkey(), &mint.pubkey(), 5 * SOL, 0),
    ];

    assert_riff_error(
        send_many(&mut env.svm, &ixs, &[&creator, &mint, &sidekick]),
        ErrorCode::TradingNotOpen,
    );
}

#[test]
fn nobody_can_buy_in_the_launch_slot() {
    // Separate transactions in the same slot, e.g. a Jito bundle landing
    // right behind the launch.
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), launch_args());
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();

    for buyer in [creator.insecure_clone(), funded_keypair(&mut env.svm)] {
        if buyer.pubkey() != creator.pubkey() {
            let ix = create_ata_ix(&buyer.pubkey(), &buyer.pubkey(), &mint.pubkey());
            send(&mut env.svm, ix, &[&buyer]).unwrap();
        }
        let ix = buy_ix(&buyer.pubkey(), &mint.pubkey(), 5 * SOL, 0);
        assert_riff_error(send(&mut env.svm, ix, &[&buyer]), ErrorCode::TradingNotOpen);
    }

    // The creator launched holding only their capped launch buy.
    let held = token_balance(&env.svm, &ata_address(&creator.pubkey(), &mint.pubkey()));
    assert!(held > 0 && held <= CAP, "creator holds {held}");
}

#[test]
fn trading_opens_the_slot_after_launch() {
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), launch_args());
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();
    let alice = trader(&mut env.svm, &mint.pubkey(), 10 * SOL);

    next_slot(&mut env.svm);
    send(
        &mut env.svm,
        buy_ix(&alice.pubkey(), &mint.pubkey(), SOL, 0),
        &[&alice],
    )
    .unwrap();
    assert!(token_balance(&env.svm, &ata_address(&alice.pubkey(), &mint.pubkey())) > 0);
}

#[test]
fn creator_can_still_sell_in_the_launch_slot() {
    // Only buys are closed, so nobody's tokens are ever locked. Selling at
    // launch just returns the launch buy minus fees.
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), launch_args());
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();

    let held = token_balance(&env.svm, &ata_address(&creator.pubkey(), &mint.pubkey()));
    let ix = sell_ix(&creator.pubkey(), &mint.pubkey(), held, 0);
    send(&mut env.svm, ix, &[&creator]).unwrap();
    assert_eq!(
        token_balance(&env.svm, &ata_address(&creator.pubkey(), &mint.pubkey())),
        0
    );
}

#[test]
fn launch_records_its_slot() {
    let mut env = setup();
    env.svm.warp_to_slot(1_234);
    let mint = setup_coin(&mut env);
    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint));
    assert_eq!(coin.created_slot, 1_234);
}
