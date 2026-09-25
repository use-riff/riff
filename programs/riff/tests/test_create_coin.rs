mod common;

use {
    anchor_lang::prelude::Pubkey,
    anchor_spl::{
        token_2022::{
            spl_token_2022::{
                extension::{
                    metadata_pointer::MetadataPointer, BaseStateWithExtensions, StateWithExtensions,
                },
                state::{Account as TokenAccount, Mint},
            },
            ID as TOKEN_2022_ID,
        },
        token_interface::spl_token_metadata_interface::state::TokenMetadata,
    },
    common::*,
    riff::{error::ErrorCode, COIN_DECIMALS, COIN_TOTAL_SUPPLY},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

/// (label, field setter, value, expected error)
type InvalidArgCase = (
    &'static str,
    fn(&mut riff::CreateCoinArgs, String),
    String,
    ErrorCode,
);

fn setup_with_config() -> (Env, Keypair) {
    let mut env = setup();
    initialize_config(&mut env);
    let creator = funded_keypair(&mut env.svm);
    (env, creator)
}

#[test]
fn creates_coin() {
    let (mut env, creator) = setup_with_config();
    let mint = Keypair::new();
    let args = coin_args();
    let ix = create_coin_ix(
        &creator.pubkey(),
        &mint.pubkey(),
        &env.treasury,
        args.clone(),
    );
    let meta = send(&mut env.svm, ix, &[&creator, &mint]).unwrap();

    let coin_key = coin_address(&mint.pubkey());
    let vault_key = vault_address(&mint.pubkey());

    let created = events::<riff::events::CoinCreated>(&meta.logs);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].coin, coin_key);
    assert_eq!(created[0].artist_id, args.artist_id);
    assert_eq!(created[0].creator_buy_tokens, 0);
    assert_eq!(created[0].creator_buy_sol, 0);
    assert!(events::<riff::events::Trade>(&meta.logs).is_empty());

    // Mint: Token-2022, fixed 1B supply, no mint or freeze authority.
    let mint_account = env.svm.get_account(&mint.pubkey()).unwrap();
    assert_eq!(mint_account.owner, TOKEN_2022_ID);
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account.data).unwrap();
    assert_eq!(mint_state.base.decimals, COIN_DECIMALS);
    assert_eq!(mint_state.base.supply, 1_000_000_000_000_000);
    assert_eq!(mint_state.base.supply, COIN_TOTAL_SUPPLY);
    assert!(mint_state.base.mint_authority.is_none());
    assert!(mint_state.base.freeze_authority.is_none());

    // Metadata lives on the mint itself.
    let pointer = mint_state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(Option::from(pointer.metadata_address), Some(mint.pubkey()));
    assert_eq!(Option::from(pointer.authority), Some(coin_key));
    let metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(metadata.mint, mint.pubkey());
    assert_eq!(metadata.name, args.name);
    assert_eq!(metadata.symbol, args.symbol);
    assert_eq!(metadata.uri, args.uri);
    assert_eq!(Option::from(metadata.update_authority), Some(coin_key));
    assert!(metadata.additional_metadata.is_empty());

    // Rent-exempt after the metadata realloc.
    let rent_min = env
        .svm
        .minimum_balance_for_rent_exemption(mint_account.data.len());
    assert!(mint_account.lamports >= rent_min);

    // Entire supply sits in the program-owned vault.
    let vault_account = env.svm.get_account(&vault_key).unwrap();
    assert_eq!(vault_account.owner, TOKEN_2022_ID);
    let vault = StateWithExtensions::<TokenAccount>::unpack(&vault_account.data).unwrap();
    assert_eq!(vault.base.mint, mint.pubkey());
    assert_eq!(vault.base.owner, coin_key);
    assert_eq!(vault.base.amount, COIN_TOTAL_SUPPLY);

    // Coin records the artist as text only, unclaimed.
    let coin: riff::Coin = fetch(&env.svm, &coin_key);
    assert_eq!(coin.mint, mint.pubkey());
    assert_eq!(coin.vault, vault_key);
    assert_eq!(coin.creator, creator.pubkey());
    assert_eq!(coin.artist_id, args.artist_id);
    assert_eq!(coin.artist_name, args.artist_name);
    assert_eq!(coin.artist, None);
    assert_eq!(coin.created_at, START_TIME);
    assert_eq!(coin.claim_deadline, START_TIME + CLAIM_WINDOW_SECS);

    // Curve starts from the config's parameters.
    assert_eq!(coin.virtual_sol_reserves, INITIAL_VIRTUAL_SOL);
    assert_eq!(coin.virtual_token_reserves, INITIAL_VIRTUAL_TOKEN);
    assert_eq!(coin.real_sol_reserves, 0);
    assert_eq!(coin.real_token_reserves, CURVE_TOKEN_SUPPLY);
    assert_eq!(coin.artist_fees, 0);
    assert_eq!(coin.creator_fees, 0);
    assert!(!coin.complete);

    // No launch buy: no creator token account, and the event says so.
    assert!(env
        .svm
        .get_account(&ata_address(&creator.pubkey(), &mint.pubkey()))
        .is_none());
}

#[test]
fn accepts_max_length_fields() {
    let (mut env, creator) = setup_with_config();
    let mint = Keypair::new();
    let args = riff::CreateCoinArgs {
        name: "n".repeat(riff::MAX_NAME_LEN),
        symbol: "S".repeat(riff::MAX_SYMBOL_LEN),
        uri: "u".repeat(riff::MAX_URI_LEN),
        artist_id: "a".repeat(riff::MAX_ARTIST_ID_LEN),
        // Multi-byte UTF-8: limit is in bytes.
        artist_name: "é".repeat(riff::MAX_ARTIST_NAME_LEN / 2),
        creator_buy_sol: 0,
        creator_buy_min_tokens: 0,
    };
    let ix = create_coin_ix(
        &creator.pubkey(),
        &mint.pubkey(),
        &env.treasury,
        args.clone(),
    );
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();

    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint.pubkey()));
    assert_eq!(coin.artist_id, args.artist_id);
    assert_eq!(coin.artist_name, args.artist_name);
}

#[test]
fn rejects_invalid_args() {
    let (mut env, creator) = setup_with_config();
    let too_long = |n: usize| "x".repeat(n + 1);

    let cases: Vec<InvalidArgCase> = vec![
        (
            "empty name",
            |a, v| a.name = v,
            String::new(),
            ErrorCode::InvalidName,
        ),
        (
            "long name",
            |a, v| a.name = v,
            too_long(riff::MAX_NAME_LEN),
            ErrorCode::InvalidName,
        ),
        (
            "empty symbol",
            |a, v| a.symbol = v,
            String::new(),
            ErrorCode::InvalidSymbol,
        ),
        (
            "long symbol",
            |a, v| a.symbol = v,
            too_long(riff::MAX_SYMBOL_LEN),
            ErrorCode::InvalidSymbol,
        ),
        (
            "empty uri",
            |a, v| a.uri = v,
            String::new(),
            ErrorCode::InvalidUri,
        ),
        (
            "long uri",
            |a, v| a.uri = v,
            too_long(riff::MAX_URI_LEN),
            ErrorCode::InvalidUri,
        ),
        (
            "empty artist id",
            |a, v| a.artist_id = v,
            String::new(),
            ErrorCode::InvalidArtistId,
        ),
        (
            "long artist id",
            |a, v| a.artist_id = v,
            too_long(riff::MAX_ARTIST_ID_LEN),
            ErrorCode::InvalidArtistId,
        ),
        (
            "artist id with space",
            |a, v| a.artist_id = v,
            "abc def".into(),
            ErrorCode::InvalidArtistId,
        ),
        (
            "non-ascii artist id",
            |a, v| a.artist_id = v,
            "аbc".into(),
            ErrorCode::InvalidArtistId,
        ),
        (
            "empty artist name",
            |a, v| a.artist_name = v,
            String::new(),
            ErrorCode::InvalidArtistName,
        ),
        (
            "long artist name",
            |a, v| a.artist_name = v,
            too_long(riff::MAX_ARTIST_NAME_LEN),
            ErrorCode::InvalidArtistName,
        ),
    ];

    for (label, set, value, expected) in cases {
        let mint = Keypair::new();
        let mut args = coin_args();
        set(&mut args, value);
        let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), &env.treasury, args);
        let res = send(&mut env.svm, ix, &[&creator, &mint]);
        assert!(res.is_err(), "{label}: should fail");
        assert_riff_error(res, expected);
        assert!(
            env.svm.get_account(&mint.pubkey()).is_none(),
            "{label}: mint left behind"
        );
    }
}

#[test]
fn requires_config() {
    let mut env = setup();
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(
        &creator.pubkey(),
        &mint.pubkey(),
        &env.treasury,
        coin_args(),
    );
    assert!(send(&mut env.svm, ix, &[&creator, &mint]).is_err());
}

#[test]
fn requires_mint_signature() {
    let (mut env, creator) = setup_with_config();
    let mint = Keypair::new();
    let mut ix = create_coin_ix(
        &creator.pubkey(),
        &mint.pubkey(),
        &env.treasury,
        coin_args(),
    );
    for meta in &mut ix.accounts {
        if meta.pubkey == mint.pubkey() {
            meta.is_signer = false;
        }
    }
    assert!(send(&mut env.svm, ix, &[&creator]).is_err());
}

#[test]
fn claim_deadline_uses_config_window_at_creation() {
    let (mut env, creator) = setup_with_config();
    let later = START_TIME + 12_345;
    let mut clock = env.svm.get_sysvar::<anchor_lang::prelude::Clock>();
    clock.unix_timestamp = later;
    env.svm.set_sysvar(&clock);

    let mint = Keypair::new();
    let ix = create_coin_ix(
        &creator.pubkey(),
        &mint.pubkey(),
        &env.treasury,
        coin_args(),
    );
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();

    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint.pubkey()));
    assert_eq!(coin.created_at, later);
    assert_eq!(coin.claim_deadline, later + CLAIM_WINDOW_SECS);
}

#[test]
fn same_artist_can_have_multiple_coins() {
    let (mut env, creator) = setup_with_config();
    for _ in 0..2 {
        let mint = Keypair::new();
        let ix = create_coin_ix(
            &creator.pubkey(),
            &mint.pubkey(),
            &env.treasury,
            coin_args(),
        );
        send(&mut env.svm, ix, &[&creator, &mint]).unwrap();
    }
}

/// Tokens the creator would get for `sol` on a fresh curve.
fn fresh_quote(env: &Env, sol: u64) -> Option<riff::curve::BuyQuote> {
    let config: riff::Config = fetch(&env.svm, &config_address());
    let fresh = riff::curve::Reserves {
        virtual_sol: config.initial_virtual_sol_reserves,
        virtual_token: config.initial_virtual_token_reserves,
        real_token: config.curve_token_supply,
        real_sol: 0,
    };
    riff::curve::quote_buy(&fresh, config.total_fee_bps(), sol)
}

#[test]
fn creator_buys_at_launch() {
    let (mut env, creator) = setup_with_config();
    let mint = Keypair::new();
    let mut args = coin_args();
    args.creator_buy_sol = SOL / 2;
    let quote = fresh_quote(&env, SOL / 2).unwrap();
    args.creator_buy_min_tokens = quote.tokens_out;
    let fees = riff::curve::split_fee(quote.fee, ARTIST_FEE_BPS, CREATOR_FEE_BPS, PROTOCOL_FEE_BPS);

    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), &env.treasury, args);
    let meta = send(&mut env.svm, ix, &[&creator, &mint]).unwrap();

    // Same price and fees as any buyer on a fresh curve.
    let creator_ata = ata_address(&creator.pubkey(), &mint.pubkey());
    assert_eq!(token_balance(&env.svm, &creator_ata), quote.tokens_out);
    assert_eq!(
        lamports(&env.svm, &env.treasury),
        TREASURY_START + fees.protocol
    );
    let coin: riff::Coin = fetch(&env.svm, &coin_address(&mint.pubkey()));
    assert_eq!(coin.real_sol_reserves, quote.sol_to_curve);
    assert_eq!(
        coin.real_token_reserves,
        CURVE_TOKEN_SUPPLY - quote.tokens_out
    );
    assert_eq!(coin.artist_fees, fees.artist);
    assert_eq!(coin.creator_fees, fees.creator);

    // Visible to indexers: in CoinCreated, and as a regular Trade.
    let created = events::<riff::events::CoinCreated>(&meta.logs);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].creator_buy_tokens, quote.tokens_out);
    assert_eq!(created[0].creator_buy_sol, SOL / 2);
    let trades = events::<riff::events::Trade>(&meta.logs);
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].trader, creator.pubkey());
    assert!(trades[0].is_buy);
    assert_eq!(trades[0].token_amount, quote.tokens_out);
}

#[test]
fn creator_buy_is_capped_at_3_percent_of_supply() {
    let (mut env, creator) = setup_with_config();
    let cap = 30_000_000 * TOKEN;

    // Largest SOL amount whose tokens stay within the cap.
    let (mut lo, mut hi) = (0, 10 * SOL);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if fresh_quote(&env, mid).is_some_and(|q| q.tokens_out <= cap) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let max_sol = lo;
    assert!(fresh_quote(&env, max_sol + 1).unwrap().tokens_out > cap);

    // One lamport over the cap fails and leaves nothing behind.
    let mint = Keypair::new();
    let mut args = coin_args();
    args.creator_buy_sol = max_sol + 1;
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), &env.treasury, args);
    assert_riff_error(
        send(&mut env.svm, ix, &[&creator, &mint]),
        ErrorCode::CreatorBuyTooLarge,
    );
    assert!(env.svm.get_account(&mint.pubkey()).is_none());

    // Exactly at the cap succeeds.
    let mut args = coin_args();
    args.creator_buy_sol = max_sol;
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), &env.treasury, args);
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();
    let bought = token_balance(&env.svm, &ata_address(&creator.pubkey(), &mint.pubkey()));
    assert!(bought <= cap && bought > cap - TOKEN, "bought {bought}");
}

#[test]
fn creator_buy_fees_go_to_configured_treasury() {
    let (mut env, creator) = setup_with_config();
    let mint = Keypair::new();
    let thief = Pubkey::new_unique();
    let mut args = coin_args();
    args.creator_buy_sol = SOL / 10;
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), &thief, args);
    assert!(send(&mut env.svm, ix, &[&creator, &mint]).is_err());
}

#[test]
fn creator_buy_respects_min_tokens() {
    let (mut env, creator) = setup_with_config();
    let mint = Keypair::new();
    let quote = fresh_quote(&env, SOL / 2).unwrap();
    let mut args = coin_args();
    args.creator_buy_sol = SOL / 2;
    args.creator_buy_min_tokens = quote.tokens_out + 1;
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), &env.treasury, args);
    assert_riff_error(
        send(&mut env.svm, ix, &[&creator, &mint]),
        ErrorCode::SlippageExceeded,
    );
    assert!(env.svm.get_account(&mint.pubkey()).is_none());
}
