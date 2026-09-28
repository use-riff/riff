//! riff on Meteora's Dynamic Bonding Curve, against Meteora's real program
//! (`make dbc-fixtures` downloads it from mainnet).
//!
//! Proves the four things riff needs from Meteora:
//! 1. riff's program can create a coin's pool with a riff escrow as creator;
//! 2. the partner share can be collected per coin and split with the launcher;
//! 3. the creator share (the artist's) is held until the artist verifies;
//! 4. claiming hands the pool to the artist, and Meteora then pays them
//!    directly; unclaimed fees go to charity after the window.

use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::{bpf_loader_upgradeable, instruction::Instruction, system_program},
        AnchorDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{spl_associated_token_account, ID as ATA_PROGRAM_ID},
        token::spl_token,
        token_2022::ID as TOKEN_2022_ID,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    riff_dbc::dynamic_bonding_curve::{self as dbc, accounts::VirtualPool},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const SOL: u64 = 1_000_000_000;
const START_TIME: i64 = 1_790_000_000;
const CLAIM_WINDOW_SECS: i64 = 90 * 24 * 60 * 60;
/// Of the partner share: 0.3% of the 0.8%, in bps.
const LAUNCHER_SHARE_BPS: u16 = 3_750;
const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
const WSOL: Pubkey = spl_token::native_mint::ID;
const DBC_POOL_AUTHORITY: Pubkey =
    Pubkey::from_str_const("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");

struct Env {
    svm: LiteSVM,
    admin: Keypair,
    treasury: Keypair,
    charity: Keypair,
    verifier: Keypair,
    dbc_config: Pubkey,
}

struct CoinKeys {
    launcher: Keypair,
    mint: Pubkey,
    pool: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
}

// ------------------------------------------------------------ addresses

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &riff_dbc::ID).0
}
fn dbc_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &dbc::ID).0
}
fn dbc_event_authority() -> Pubkey {
    dbc_pda(&[b"__event_authority"])
}
fn ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    anchor_spl::associated_token::get_associated_token_address_with_program_id(
        owner,
        mint,
        token_program,
    )
}
fn escrow(mint: &Pubkey) -> Pubkey {
    pda(&[riff_dbc::ESCROW_SEED, mint.as_ref()])
}

// ------------------------------------------------------------ plumbing

#[allow(clippy::result_large_err)]
fn send(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) -> TransactionResult {
    let msg = Message::new_with_blockhash(ixs, Some(&signers[0].pubkey()), &svm.latest_blockhash());
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    let res = svm.send_transaction(tx);
    svm.expire_blockhash();
    res
}

fn ok(res: TransactionResult) {
    if let Err(e) = res {
        panic!(
            "transaction failed: {:?}\n{}",
            e.err,
            e.meta.logs.join("\n")
        );
    }
}

fn fails_with(res: TransactionResult, needle: &str) {
    let e = res.expect_err("expected the transaction to fail");
    let logs = e.meta.logs.join("\n");
    assert!(
        logs.contains(needle),
        "expected {needle:?} in logs:\n{logs}"
    );
}

fn funded(svm: &mut LiteSVM) -> Keypair {
    let kp = Keypair::new();
    svm.airdrop(&kp.pubkey(), 100 * SOL).unwrap();
    kp
}

fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm.get_account(account).unwrap().data;
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

fn compute_budget(units: u32) -> Instruction {
    let mut data = vec![2u8];
    data.extend_from_slice(&units.to_le_bytes());
    Instruction::new_with_bytes(
        Pubkey::from_str_const("ComputeBudget111111111111111111111111111111"),
        &data,
        vec![],
    )
}

/// A WSOL account for `owner`, holding `lamports` of wrapped SOL.
fn wsol_account(svm: &mut LiteSVM, owner: &Keypair, lamports: u64) -> Pubkey {
    let account = ata(&owner.pubkey(), &WSOL, &TOKEN_PROGRAM_ID);
    let mut ixs = vec![
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &owner.pubkey(),
            &owner.pubkey(),
            &WSOL,
            &TOKEN_PROGRAM_ID,
        ),
    ];
    if lamports > 0 {
        ixs.push(anchor_lang::solana_program::system_instruction::transfer(
            &owner.pubkey(),
            &account,
            lamports,
        ));
        ixs.push(spl_token::instruction::sync_native(&TOKEN_PROGRAM_ID, &account).unwrap());
    }
    ok(send(svm, &ixs, &[owner]));
    account
}

fn load_account_json(svm: &mut LiteSVM, json: &str) {
    use base64::Engine;
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    let a = &v["account"];
    let account = solana_account::Account {
        lamports: a["lamports"].as_u64().unwrap(),
        data: base64::engine::general_purpose::STANDARD
            .decode(a["data"][0].as_str().unwrap())
            .unwrap(),
        owner: a["owner"].as_str().unwrap().parse().unwrap(),
        executable: a["executable"].as_bool().unwrap(),
        rent_epoch: 0,
    };
    svm.set_account(v["pubkey"].as_str().unwrap().parse().unwrap(), account)
        .unwrap();
}

fn pool_creator(svm: &LiteSVM, pool: &Pubkey) -> Pubkey {
    let data = svm.get_account(pool).unwrap().data;
    let size = std::mem::size_of::<VirtualPool>();
    bytemuck::pod_read_unaligned::<VirtualPool>(&data[8..8 + size])
        .pool_state
        .creator
}

// ------------------------------------------------------------ setup

fn setup() -> Env {
    let mut svm = LiteSVM::new();
    svm.add_program(
        riff_dbc::ID,
        include_bytes!(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/../deploy/riff_dbc.so"
        )),
    )
    .unwrap();
    svm.add_program(dbc::ID, include_bytes!("fixtures/dynamic_bonding_curve.so"))
        .unwrap();
    // The token programs mainnet runs, shared with riff's own tests.
    svm.add_program(
        TOKEN_PROGRAM_ID,
        include_bytes!("../../riff/tests/fixtures/spl_token.so"),
    )
    .unwrap();
    svm.add_program(
        TOKEN_2022_ID,
        include_bytes!("../../riff/tests/fixtures/spl_token_2022.so"),
    )
    .unwrap();
    load_account_json(
        &mut svm,
        include_str!("../../riff/tests/fixtures/wsol_mint.json"),
    );

    // LiteSVM deploys with no upgrade authority; set one (see riff's tests).
    let admin = funded(&mut svm);
    let program_data =
        Pubkey::find_program_address(&[riff_dbc::ID.as_ref()], &bpf_loader_upgradeable::ID).0;
    let mut account = svm.get_account(&program_data).unwrap();
    account.data[12] = 1;
    account.data[13..45].copy_from_slice(admin.pubkey().as_ref());
    svm.set_account(program_data, account).unwrap();

    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = START_TIME;
    svm.set_sysvar(&clock);

    let (treasury, charity, verifier) = (funded(&mut svm), funded(&mut svm), Keypair::new());
    let fee_authority = pda(&[riff_dbc::FEE_AUTHORITY_SEED]);

    // riff's Meteora config, with riff's program as fee claimer. The
    // parameters come from config/gen-config.mjs (Meteora's SDK).
    let dbc_config = Keypair::new();
    let params = dbc::types::ConfigParameters::deserialize(
        &mut &include_bytes!("fixtures/config_parameters.bin")[..],
    )
    .unwrap();
    let create_config = Instruction {
        program_id: dbc::ID,
        accounts: dbc::client::accounts::CreateConfig {
            config: dbc_config.pubkey(),
            fee_claimer: fee_authority,
            leftover_receiver: treasury.pubkey(),
            quote_mint: WSOL,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: dbc_event_authority(),
            program: dbc::ID,
        }
        .to_account_metas(None),
        data: dbc::client::args::CreateConfig {
            config_parameters: params,
        }
        .data(),
    };
    ok(send(&mut svm, &[create_config], &[&admin, &dbc_config]));

    let initialize = Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::Initialize {
            admin: admin.pubkey(),
            config: pda(&[riff_dbc::CONFIG_SEED]),
            fee_authority,
            fees_vault: pda(&[riff_dbc::FEES_VAULT_SEED]),
            quote_mint: WSOL,
            program: riff_dbc::ID,
            program_data,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::Initialize {
            params: riff_dbc::ConfigParams {
                verifier: verifier.pubkey(),
                charity: charity.pubkey(),
                treasury: treasury.pubkey(),
                dbc_config: dbc_config.pubkey(),
                launcher_share_bps: LAUNCHER_SHARE_BPS,
                claim_window_secs: CLAIM_WINDOW_SECS,
            },
        }
        .data(),
    };
    ok(send(&mut svm, &[initialize], &[&admin]));
    Env {
        svm,
        admin,
        treasury,
        charity,
        verifier,
        dbc_config: dbc_config.pubkey(),
    }
}

fn create_coin(env: &mut Env) -> CoinKeys {
    let launcher = funded(&mut env.svm);
    let mint = Keypair::new();
    let m = mint.pubkey();
    let (hi, lo) = if m > WSOL { (m, WSOL) } else { (WSOL, m) };
    let pool = dbc_pda(&[b"pool", env.dbc_config.as_ref(), hi.as_ref(), lo.as_ref()]);
    let base_vault = dbc_pda(&[b"token_vault", m.as_ref(), pool.as_ref()]);
    let quote_vault = dbc_pda(&[b"token_vault", WSOL.as_ref(), pool.as_ref()]);
    let ix = Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::CreateCoin {
            launcher: launcher.pubkey(),
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: pda(&[riff_dbc::COIN_SEED, m.as_ref()]),
            escrow: escrow(&m),
            artist_vault: pda(&[riff_dbc::ARTIST_VAULT_SEED, m.as_ref()]),
            escrow_base_account: ata(&escrow(&m), &m, &TOKEN_2022_ID),
            base_mint: m,
            quote_mint: WSOL,
            dbc_config: env.dbc_config,
            pool_authority: DBC_POOL_AUTHORITY,
            pool,
            base_vault,
            quote_vault,
            dbc_event_authority: dbc_event_authority(),
            dbc_program: dbc::ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            token_program: TOKEN_2022_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::CreateCoin {
            args: riff_dbc::CreateCoinArgs {
                name: "Moss Garden".into(),
                symbol: "MOSS".into(),
                uri: "https://riffpad.fun/meta/moss.json".into(),
                artist_id: "spotify:0LilaMossDemoArtist000".into(),
                artist_name: "Lila Moss".into(),
            },
        }
        .data(),
    };
    ok(send(
        &mut env.svm,
        &[compute_budget(400_000), ix],
        &[&launcher, &mint],
    ));
    CoinKeys {
        launcher,
        mint: m,
        pool,
        base_vault,
        quote_vault,
    }
}

/// A trade straight on Meteora, the way Jupiter or Axiom would route it:
/// riff's program isn't involved.
fn buy_on_meteora(env: &mut Env, coin: &CoinKeys, lamports: u64) {
    let trader = funded(&mut env.svm);
    let wsol = wsol_account(&mut env.svm, &trader, lamports);
    let base = ata(&trader.pubkey(), &coin.mint, &TOKEN_2022_ID);
    let create_base =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &trader.pubkey(),
            &trader.pubkey(),
            &coin.mint,
            &TOKEN_2022_ID,
        );
    let swap = Instruction {
        program_id: dbc::ID,
        accounts: dbc::client::accounts::Swap {
            pool_authority: DBC_POOL_AUTHORITY,
            config: env.dbc_config,
            pool: coin.pool,
            input_token_account: wsol,
            output_token_account: base,
            base_vault: coin.base_vault,
            quote_vault: coin.quote_vault,
            base_mint: coin.mint,
            quote_mint: WSOL,
            payer: trader.pubkey(),
            token_base_program: TOKEN_2022_ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            referral_token_account: None,
            event_authority: dbc_event_authority(),
            program: dbc::ID,
        }
        .to_account_metas(None),
        data: dbc::client::args::Swap {
            params: dbc::types::SwapParameters {
                amount_in: lamports,
                minimum_amount_out: 0,
            },
        }
        .data(),
    };
    ok(send(
        &mut env.svm,
        &[compute_budget(400_000), create_base, swap],
        &[&trader],
    ));
}

/// Accounts shared by collect_fees, claim_artist and sweep_charity.
struct PoolAccounts {
    escrow: Pubkey,
    artist_vault: Pubkey,
    escrow_base_account: Pubkey,
}

fn pool_accounts(coin: &CoinKeys) -> PoolAccounts {
    PoolAccounts {
        escrow: escrow(&coin.mint),
        artist_vault: pda(&[riff_dbc::ARTIST_VAULT_SEED, coin.mint.as_ref()]),
        escrow_base_account: ata(&escrow(&coin.mint), &coin.mint, &TOKEN_2022_ID),
    }
}

fn collect_fees_ix(
    dbc_config: Pubkey,
    coin: &CoinKeys,
    launcher_account: Pubkey,
    treasury_account: Pubkey,
) -> Instruction {
    let p = pool_accounts(coin);
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::CollectFees {
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: pda(&[riff_dbc::COIN_SEED, coin.mint.as_ref()]),
            escrow: p.escrow,
            fee_authority: pda(&[riff_dbc::FEE_AUTHORITY_SEED]),
            fees_vault: pda(&[riff_dbc::FEES_VAULT_SEED]),
            artist_vault: p.artist_vault,
            escrow_base_account: p.escrow_base_account,
            launcher_quote_account: launcher_account,
            treasury_quote_account: treasury_account,
            dbc_config,
            pool_authority: DBC_POOL_AUTHORITY,
            pool: coin.pool,
            base_vault: coin.base_vault,
            quote_vault: coin.quote_vault,
            base_mint: coin.mint,
            quote_mint: WSOL,
            dbc_event_authority: dbc_event_authority(),
            dbc_program: dbc::ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            token_base_program: TOKEN_2022_ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::CollectFees {}.data(),
    }
}

fn claim_artist_ix(
    dbc_config: Pubkey,
    coin: &CoinKeys,
    artist: &Pubkey,
    artist_account: Pubkey,
    verifier: &Pubkey,
) -> Instruction {
    let p = pool_accounts(coin);
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::ClaimArtist {
            artist: *artist,
            verifier: *verifier,
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: pda(&[riff_dbc::COIN_SEED, coin.mint.as_ref()]),
            escrow: p.escrow,
            artist_vault: p.artist_vault,
            escrow_base_account: p.escrow_base_account,
            artist_quote_account: artist_account,
            dbc_config,
            pool_authority: DBC_POOL_AUTHORITY,
            pool: coin.pool,
            base_vault: coin.base_vault,
            quote_vault: coin.quote_vault,
            base_mint: coin.mint,
            quote_mint: WSOL,
            dbc_event_authority: dbc_event_authority(),
            dbc_program: dbc::ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            token_base_program: TOKEN_2022_ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::ClaimArtist {}.data(),
    }
}

fn sweep_charity_ix(coin: &CoinKeys, charity_account: Pubkey) -> Instruction {
    let p = pool_accounts(coin);
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::SweepCharity {
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: pda(&[riff_dbc::COIN_SEED, coin.mint.as_ref()]),
            escrow: p.escrow,
            artist_vault: p.artist_vault,
            escrow_base_account: p.escrow_base_account,
            charity_quote_account: charity_account,
            pool_authority: DBC_POOL_AUTHORITY,
            pool: coin.pool,
            base_vault: coin.base_vault,
            quote_vault: coin.quote_vault,
            base_mint: coin.mint,
            quote_mint: WSOL,
            dbc_event_authority: dbc_event_authority(),
            dbc_program: dbc::ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            token_base_program: TOKEN_2022_ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::SweepCharity {}.data(),
    }
}

/// Within a few lamports: Meteora rounds each fee part down.
fn assert_close(actual: u64, expected: u64, what: &str) {
    assert!(
        actual.abs_diff(expected) <= 3,
        "{what}: got {actual}, expected about {expected}"
    );
}

// ------------------------------------------------------------ tests

#[test]
fn a_coin_is_a_meteora_pool_held_by_riffs_escrow() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    assert_eq!(pool_creator(&env.svm, &coin.pool), escrow(&coin.mint));
    let record: riff_dbc::Coin = {
        let data = env
            .svm
            .get_account(&pda(&[riff_dbc::COIN_SEED, coin.mint.as_ref()]))
            .unwrap()
            .data;
        anchor_lang::AccountDeserialize::try_deserialize(&mut &data[..]).unwrap()
    };
    assert_eq!(record.pool, coin.pool);
    assert_eq!(record.launcher, coin.launcher.pubkey());
    assert_eq!(record.artist, None);
    assert_eq!(record.claim_deadline, START_TIME + CLAIM_WINDOW_SECS);
}

#[test]
fn fees_from_trades_on_meteora_are_split_per_coin() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let other = create_coin(&mut env);
    let launcher_account = wsol_account(&mut env.svm, &coin.launcher, 0);
    let treasury = env.treasury.insecure_clone();
    let treasury_account = wsol_account(&mut env.svm, &treasury, 0);

    // 10 SOL traded on this coin, 5 SOL on another: fees stay per coin.
    buy_on_meteora(&mut env, &coin, 10 * SOL);
    buy_on_meteora(&mut env, &other, 5 * SOL);
    ok(send(
        &mut env.svm,
        &[collect_fees_ix(
            env.dbc_config,
            &coin,
            launcher_account,
            treasury_account,
        )],
        &[&env.admin],
    ));

    // 2% fee: 0.4% Meteora, 0.8% artist (held), 0.3% launcher, 0.5% riff.
    let artist_vault = pool_accounts(&coin).artist_vault;
    assert_close(
        token_balance(&env.svm, &artist_vault),
        10 * SOL * 80 / 10_000,
        "artist share held",
    );
    assert_close(
        token_balance(&env.svm, &launcher_account),
        10 * SOL * 30 / 10_000,
        "launcher share",
    );
    assert_close(
        token_balance(&env.svm, &treasury_account),
        10 * SOL * 50 / 10_000,
        "riff share",
    );

    // Collecting again right away finds nothing new.
    let before = token_balance(&env.svm, &artist_vault);
    ok(send(
        &mut env.svm,
        &[collect_fees_ix(
            env.dbc_config,
            &coin,
            launcher_account,
            treasury_account,
        )],
        &[&env.admin],
    ));
    assert_eq!(token_balance(&env.svm, &artist_vault), before);
}

#[test]
fn fee_shares_only_go_to_the_right_wallets() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let stranger = funded(&mut env.svm);
    let stranger_account = wsol_account(&mut env.svm, &stranger, 0);
    let launcher_account = wsol_account(&mut env.svm, &coin.launcher, 0);
    let treasury = env.treasury.insecure_clone();
    let treasury_account = wsol_account(&mut env.svm, &treasury, 0);
    buy_on_meteora(&mut env, &coin, SOL);
    let admin = env.admin.insecure_clone();
    fails_with(
        send(
            &mut env.svm,
            &[collect_fees_ix(
                env.dbc_config,
                &coin,
                stranger_account,
                treasury_account,
            )],
            &[&admin],
        ),
        "WrongReceiver",
    );
    fails_with(
        send(
            &mut env.svm,
            &[collect_fees_ix(
                env.dbc_config,
                &coin,
                launcher_account,
                stranger_account,
            )],
            &[&admin],
        ),
        "WrongReceiver",
    );
}

#[test]
fn the_verified_artist_gets_held_fees_and_then_the_pool() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let launcher_account = wsol_account(&mut env.svm, &coin.launcher, 0);
    let treasury = env.treasury.insecure_clone();
    let treasury_account = wsol_account(&mut env.svm, &treasury, 0);
    let artist = funded(&mut env.svm);
    let artist_account = wsol_account(&mut env.svm, &artist, 0);
    buy_on_meteora(&mut env, &coin, 10 * SOL);

    // Without riff's verifier, no claim.
    let impostor_verifier = Keypair::new();
    fails_with(
        send(
            &mut env.svm,
            &[claim_artist_ix(
                env.dbc_config,
                &coin,
                &artist.pubkey(),
                artist_account,
                &impostor_verifier.pubkey(),
            )],
            &[&artist, &impostor_verifier],
        ),
        "ConstraintAddress",
    );

    // Verified: the held 0.8% arrives (collected in the same step), and the
    // pool's creator is now the artist.
    let verifier = env.verifier.insecure_clone();
    ok(send(
        &mut env.svm,
        &[claim_artist_ix(
            env.dbc_config,
            &coin,
            &artist.pubkey(),
            artist_account,
            &verifier.pubkey(),
        )],
        &[&artist, &verifier],
    ));
    assert_close(
        token_balance(&env.svm, &artist_account),
        10 * SOL * 80 / 10_000,
        "held fees paid out",
    );
    assert_eq!(pool_creator(&env.svm, &coin.pool), artist.pubkey());
    fails_with(
        send(
            &mut env.svm,
            &[claim_artist_ix(
                env.dbc_config,
                &coin,
                &artist.pubkey(),
                artist_account,
                &verifier.pubkey(),
            )],
            &[&artist, &verifier],
        ),
        "AlreadyClaimed",
    );

    // Later trades: riff's crank no longer touches the artist's share, and
    // the artist collects it from Meteora themselves.
    buy_on_meteora(&mut env, &coin, 5 * SOL);
    let admin = env.admin.insecure_clone();
    ok(send(
        &mut env.svm,
        &[collect_fees_ix(
            env.dbc_config,
            &coin,
            launcher_account,
            treasury_account,
        )],
        &[&admin],
    ));
    assert_eq!(
        token_balance(&env.svm, &pool_accounts(&coin).artist_vault),
        0
    );
    let before = token_balance(&env.svm, &artist_account);
    let artist_base = ata(&artist.pubkey(), &coin.mint, &TOKEN_2022_ID);
    let create_base =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &artist.pubkey(),
            &artist.pubkey(),
            &coin.mint,
            &TOKEN_2022_ID,
        );
    let claim_direct = Instruction {
        program_id: dbc::ID,
        accounts: dbc::client::accounts::ClaimCreatorTradingFee {
            pool_authority: DBC_POOL_AUTHORITY,
            pool: coin.pool,
            token_a_account: artist_base,
            token_b_account: artist_account,
            base_vault: coin.base_vault,
            quote_vault: coin.quote_vault,
            base_mint: coin.mint,
            quote_mint: WSOL,
            creator: artist.pubkey(),
            token_base_program: TOKEN_2022_ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            event_authority: dbc_event_authority(),
            program: dbc::ID,
        }
        .to_account_metas(None),
        data: dbc::client::args::ClaimCreatorTradingFee {
            max_base_amount: u64::MAX,
            max_quote_amount: u64::MAX,
        }
        .data(),
    };
    ok(send(&mut env.svm, &[create_base, claim_direct], &[&artist]));
    assert_close(
        token_balance(&env.svm, &artist_account) - before,
        5 * SOL * 80 / 10_000,
        "paid by Meteora directly",
    );
}

#[test]
fn unclaimed_artist_fees_go_to_charity_after_the_window() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let charity = env.charity.insecure_clone();
    let charity_account = wsol_account(&mut env.svm, &charity, 0);
    buy_on_meteora(&mut env, &coin, 10 * SOL);

    let admin = env.admin.insecure_clone();
    fails_with(
        send(
            &mut env.svm,
            &[sweep_charity_ix(&coin, charity_account)],
            &[&admin],
        ),
        "ClaimWindowOpen",
    );

    let mut clock = env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += CLAIM_WINDOW_SECS + 1;
    env.svm.set_sysvar(&clock);

    ok(send(
        &mut env.svm,
        &[sweep_charity_ix(&coin, charity_account)],
        &[&admin],
    ));
    assert_close(
        token_balance(&env.svm, &charity_account),
        10 * SOL * 80 / 10_000,
        "sent to charity",
    );

    // Too late to claim now.
    let artist = funded(&mut env.svm);
    let artist_account = wsol_account(&mut env.svm, &artist, 0);
    let verifier = env.verifier.insecure_clone();
    fails_with(
        send(
            &mut env.svm,
            &[claim_artist_ix(
                env.dbc_config,
                &coin,
                &artist.pubkey(),
                artist_account,
                &verifier.pubkey(),
            )],
            &[&artist, &verifier],
        ),
        "ClaimWindowClosed",
    );
}
