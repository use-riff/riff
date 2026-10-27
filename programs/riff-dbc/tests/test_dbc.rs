//! riff on Meteora's Dynamic Bonding Curve, against Meteora's real programs
//! (`make dbc-fixtures` downloads them from mainnet).
//!
//! Proves what riff needs from Meteora:
//! 1. riff's program creates a coin's pool with a riff escrow as creator,
//!    optionally with the launcher's capped first buy;
//! 2. the partner share is collected per coin and split with the launcher;
//! 3. the creator share (the artist's) is held until the artist claims with
//!    their passport, then paid to the passport's vault in SOL;
//! 4. unclaimed fees go to charity after the window;
//! 5. after graduation to DAMM v2, both locked positions' LP fees are
//!    routed the same way. (The graduating buy fills the curve exactly, so
//!    Meteora's surplus is zero: there's nothing to collect there.)

use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::{bpf_loader_upgradeable, instruction::Instruction, system_program},
        AccountSerialize, AnchorDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{spl_associated_token_account, ID as ATA_PROGRAM_ID},
        token::spl_token,
        token_2022::ID as TOKEN_2022_ID,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    riff_dbc::{
        cp_amm as damm,
        dynamic_bonding_curve::{self as dbc, accounts::VirtualPool},
    },
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
/// The most the launcher's launch buy may spend.
const MAX_LAUNCH_BUY: u64 = SOL;
const ARTIST_ID: &str = "spotify:0LilaMossDemoArtist000";
const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
const WSOL: Pubkey = spl_token::native_mint::ID;
const DBC_POOL_AUTHORITY: Pubkey =
    Pubkey::from_str_const("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
const DAMM_POOL_AUTHORITY: Pubkey =
    Pubkey::from_str_const("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
/// Meteora's DAMM v2 config for DBC pools with a customizable migrated fee.
const DAMM_CONFIG: Pubkey = Pubkey::from_str_const("A8gMrEPJkacWkcb3DGwtJwTe16HktSEfvwtuDh2MCtck");
const INSTRUCTIONS_SYSVAR: Pubkey =
    Pubkey::from_str_const("Sysvar1nstructions1111111111111111111111111");

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

struct ArtistPassport {
    wallet: Keypair,
    passport: Pubkey,
    vault: Pubkey,
}

// ------------------------------------------------------------ addresses

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &riff_dbc::ID).0
}
fn dbc_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &dbc::ID).0
}
fn damm_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &damm::ID).0
}
fn passport_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &riff_passport::ID).0
}
fn dbc_event_authority() -> Pubkey {
    dbc_pda(&[b"__event_authority"])
}
fn damm_event_authority() -> Pubkey {
    damm_pda(&[b"__event_authority"])
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
fn artist_vault(mint: &Pubkey) -> Pubkey {
    pda(&[riff_dbc::ARTIST_VAULT_SEED, mint.as_ref()])
}
fn unwrap_account(mint: &Pubkey) -> Pubkey {
    pda(&[riff_dbc::UNWRAP_SEED, mint.as_ref()])
}
fn coin_pda(mint: &Pubkey) -> Pubkey {
    pda(&[riff_dbc::COIN_SEED, mint.as_ref()])
}
/// Meteora sorts the two mints into a pool's seeds, larger first.
fn sorted(a: Pubkey, b: Pubkey) -> (Pubkey, Pubkey) {
    if a > b {
        (a, b)
    } else {
        (b, a)
    }
}
fn damm_pool(mint: &Pubkey) -> Pubkey {
    let (hi, lo) = sorted(*mint, WSOL);
    damm_pda(&[b"pool", DAMM_CONFIG.as_ref(), hi.as_ref(), lo.as_ref()])
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
    svm.airdrop(&kp.pubkey(), 200 * SOL).unwrap();
    kp
}

fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm.get_account(account).unwrap().data;
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

fn token_owner(svm: &LiteSVM, account: &Pubkey) -> Pubkey {
    let data = svm.get_account(account).unwrap().data;
    Pubkey::try_from(&data[32..64]).unwrap()
}

fn lamports(svm: &LiteSVM, account: &Pubkey) -> u64 {
    svm.get_account(account).map_or(0, |a| a.lamports)
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

fn advance(svm: &mut LiteSVM, secs: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp += secs;
    svm.set_sysvar(&clock);
}

/// Instructions that give `owner` a WSOL account holding `lamports`.
fn wrap_ixs(owner: &Pubkey, lamports: u64) -> Vec<Instruction> {
    let account = ata(owner, &WSOL, &TOKEN_PROGRAM_ID);
    let mut ixs = vec![
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            owner,
            owner,
            &WSOL,
            &TOKEN_PROGRAM_ID,
        ),
    ];
    if lamports > 0 {
        ixs.push(anchor_lang::solana_program::system_instruction::transfer(
            owner, &account, lamports,
        ));
        ixs.push(spl_token::instruction::sync_native(&TOKEN_PROGRAM_ID, &account).unwrap());
    }
    ixs
}

fn wsol_account(svm: &mut LiteSVM, owner: &Keypair, lamports: u64) -> Pubkey {
    ok(send(svm, &wrap_ixs(&owner.pubkey(), lamports), &[owner]));
    ata(&owner.pubkey(), &WSOL, &TOKEN_PROGRAM_ID)
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

fn coin_record(svm: &LiteSVM, mint: &Pubkey) -> riff_dbc::Coin {
    let data = svm.get_account(&coin_pda(mint)).unwrap().data;
    anchor_lang::AccountDeserialize::try_deserialize(&mut &data[..]).unwrap()
}

/// Within 0.01%: Meteora rounds each fee part down.
fn assert_close(actual: u64, expected: u64, what: &str) {
    assert!(
        actual.abs_diff(expected) <= expected / 10_000 + 3,
        "{what}: got {actual}, expected about {expected}"
    );
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
    svm.add_program(damm::ID, include_bytes!("fixtures/cp_amm.so"))
        .unwrap();
    load_account_json(&mut svm, include_str!("fixtures/damm_v2_config.json"));
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
    // Meteora's pool authority pays the graduated pool's rent; on mainnet
    // and devnet it holds SOL for that.
    svm.airdrop(&DBC_POOL_AUTHORITY, 100 * SOL).unwrap();

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
                max_launch_buy: MAX_LAUNCH_BUY,
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

/// An Artist Passport as riff-passport writes it (that program's own tests
/// cover issuing one), with its vault.
fn passport(env: &mut Env, artist_id: &str, revoked: bool) -> ArtistPassport {
    let wallet = funded(&mut env.svm);
    let (passport, bump) = Pubkey::find_program_address(
        &[riff_passport::PASSPORT_SEED, artist_id.as_bytes()],
        &riff_passport::ID,
    );
    let (vault, vault_bump) = Pubkey::find_program_address(
        &[riff_passport::VAULT_SEED, passport.as_ref()],
        &riff_passport::ID,
    );
    let mut data = Vec::new();
    riff_passport::Passport {
        artist_id: artist_id.into(),
        wallet: wallet.pubkey(),
        passkey: [2; 33],
        proofs: vec![],
        nonce: 0,
        recovery: None,
        revoked,
        issued_at: START_TIME,
        wallet_since: START_TIME,
        withdraw_window_start: 0,
        withdrawn_in_window: 0,
        bump,
        vault_bump,
    }
    .try_serialize(&mut data)
    .unwrap();
    let account = |data: Vec<u8>| solana_account::Account {
        lamports: 10_000_000,
        data,
        owner: riff_passport::ID,
        executable: false,
        rent_epoch: 0,
    };
    env.svm.set_account(passport, account(data)).unwrap();
    let mut data = Vec::new();
    riff_passport::Vault { passport }
        .try_serialize(&mut data)
        .unwrap();
    env.svm.set_account(vault, account(data)).unwrap();
    assert_eq!(
        passport,
        passport_pda(&[riff_passport::PASSPORT_SEED, artist_id.as_bytes()])
    );
    ArtistPassport {
        wallet,
        passport,
        vault,
    }
}

fn create_coin_ix(env: &Env, launcher: &Pubkey, mint: &Pubkey) -> Instruction {
    let m = *mint;
    let (hi, lo) = sorted(m, WSOL);
    let pool = dbc_pda(&[b"pool", env.dbc_config.as_ref(), hi.as_ref(), lo.as_ref()]);
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::CreateCoin {
            launcher: *launcher,
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: coin_pda(&m),
            escrow: escrow(&m),
            artist_vault: artist_vault(&m),
            escrow_base_account: ata(&escrow(&m), &m, &TOKEN_2022_ID),
            base_mint: m,
            quote_mint: WSOL,
            dbc_config: env.dbc_config,
            pool_authority: DBC_POOL_AUTHORITY,
            pool,
            base_vault: dbc_pda(&[b"token_vault", m.as_ref(), pool.as_ref()]),
            quote_vault: dbc_pda(&[b"token_vault", WSOL.as_ref(), pool.as_ref()]),
            dbc_event_authority: dbc_event_authority(),
            instructions_sysvar: INSTRUCTIONS_SYSVAR,
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
                artist_id: ARTIST_ID.into(),
                artist_name: "Lila Moss".into(),
            },
        }
        .data(),
    }
}

/// Creates a coin, with the launcher's launch buy if `launch_buy` > 0.
#[allow(clippy::result_large_err)]
fn try_create_coin(env: &mut Env, launch_buy: u64) -> (CoinKeys, TransactionResult) {
    let launcher = funded(&mut env.svm);
    let mint = Keypair::new();
    let m = mint.pubkey();
    let (hi, lo) = sorted(m, WSOL);
    let pool = dbc_pda(&[b"pool", env.dbc_config.as_ref(), hi.as_ref(), lo.as_ref()]);
    let keys = CoinKeys {
        launcher,
        mint: m,
        pool,
        base_vault: dbc_pda(&[b"token_vault", m.as_ref(), pool.as_ref()]),
        quote_vault: dbc_pda(&[b"token_vault", WSOL.as_ref(), pool.as_ref()]),
    };
    let launcher = keys.launcher.pubkey();
    let mut ixs = vec![compute_budget(600_000)];
    ixs.push(create_coin_ix(env, &launcher, &m));
    if launch_buy > 0 {
        // The launch buy: plain Meteora swaps right after, in the same transaction.
        ixs.extend(wrap_ixs(&launcher, launch_buy));
        ixs.extend(buy_ixs(env, &keys, &launcher, launch_buy, false));
    }
    let res = send(&mut env.svm, &ixs, &[&keys.launcher, &mint]);
    (keys, res)
}

fn create_coin(env: &mut Env) -> CoinKeys {
    let (coin, res) = try_create_coin(env, 0);
    ok(res);
    coin
}

/// A trade straight on Meteora, the way Jupiter or Axiom would route it:
/// riff's program isn't involved.
fn buy_on_meteora(env: &mut Env, coin: &CoinKeys, lamports: u64) {
    let trader = funded(&mut env.svm);
    wsol_account(&mut env.svm, &trader, lamports);
    let mut ixs = vec![compute_budget(400_000)];
    ixs.extend(buy_ixs(env, coin, &trader.pubkey(), lamports, false));
    ok(send(&mut env.svm, &ixs, &[&trader]));
}

/// A Meteora buy from `trader`'s WSOL account. With `partial`, a buy larger
/// than what's left on the curve fills up to graduation.
fn buy_ixs(
    env: &Env,
    coin: &CoinKeys,
    trader: &Pubkey,
    lamports: u64,
    partial: bool,
) -> Vec<Instruction> {
    let create_base =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            trader,
            trader,
            &coin.mint,
            &TOKEN_2022_ID,
        );
    let accounts = dbc::client::accounts::Swap {
        pool_authority: DBC_POOL_AUTHORITY,
        config: env.dbc_config,
        pool: coin.pool,
        input_token_account: ata(trader, &WSOL, &TOKEN_PROGRAM_ID),
        output_token_account: ata(trader, &coin.mint, &TOKEN_2022_ID),
        base_vault: coin.base_vault,
        quote_vault: coin.quote_vault,
        base_mint: coin.mint,
        quote_mint: WSOL,
        payer: *trader,
        token_base_program: TOKEN_2022_ID,
        token_quote_program: TOKEN_PROGRAM_ID,
        referral_token_account: None,
        event_authority: dbc_event_authority(),
        program: dbc::ID,
    }
    .to_account_metas(None);
    let data = if partial {
        dbc::client::args::Swap2 {
            params: dbc::types::SwapParameters2 {
                amount_0: lamports,
                amount_1: 0,
                swap_mode: 1,
            },
        }
        .data()
    } else {
        dbc::client::args::Swap {
            params: dbc::types::SwapParameters {
                amount_in: lamports,
                minimum_amount_out: 0,
            },
        }
        .data()
    };
    vec![
        create_base,
        Instruction {
            program_id: dbc::ID,
            accounts,
            data,
        },
    ]
}

/// The receiving WSOL accounts of a coin's launcher and riff's treasury.
fn partner_accounts(env: &mut Env, coin: &CoinKeys) -> (Pubkey, Pubkey) {
    let launcher = coin.launcher.insecure_clone();
    let treasury = env.treasury.insecure_clone();
    (
        wsol_account(&mut env.svm, &launcher, 0),
        wsol_account(&mut env.svm, &treasury, 0),
    )
}

fn collect_fees_ix(
    dbc_config: Pubkey,
    coin: &CoinKeys,
    launcher_account: Pubkey,
    treasury_account: Pubkey,
) -> Instruction {
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::CollectFees {
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: coin_pda(&coin.mint),
            escrow: escrow(&coin.mint),
            fee_authority: pda(&[riff_dbc::FEE_AUTHORITY_SEED]),
            fees_vault: pda(&[riff_dbc::FEES_VAULT_SEED]),
            artist_vault: artist_vault(&coin.mint),
            escrow_base_account: ata(&escrow(&coin.mint), &coin.mint, &TOKEN_2022_ID),
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

fn collect(env: &mut Env, coin: &CoinKeys) -> (Pubkey, Pubkey) {
    let (launcher_account, treasury_account) = partner_accounts(env, coin);
    let admin = env.admin.insecure_clone();
    ok(send(
        &mut env.svm,
        &[collect_fees_ix(
            env.dbc_config,
            coin,
            launcher_account,
            treasury_account,
        )],
        &[&admin],
    ));
    (launcher_account, treasury_account)
}

fn claim_artist_ix(
    coin: &CoinKeys,
    wallet: &Pubkey,
    passport: &Pubkey,
    vault: &Pubkey,
    verifier: &Pubkey,
    charity_account: Pubkey,
) -> Instruction {
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::ClaimArtist {
            wallet: *wallet,
            verifier: *verifier,
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: coin_pda(&coin.mint),
            passport: *passport,
            passport_vault: *vault,
            escrow: escrow(&coin.mint),
            artist_vault: artist_vault(&coin.mint),
            unwrap: unwrap_account(&coin.mint),
            escrow_base_account: ata(&escrow(&coin.mint), &coin.mint, &TOKEN_2022_ID),
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
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::ClaimArtist {}.data(),
    }
}

#[allow(clippy::result_large_err)]
fn claim(env: &mut Env, coin: &CoinKeys, artist: &ArtistPassport) -> TransactionResult {
    let charity = env.charity.insecure_clone();
    let charity_account = wsol_account(&mut env.svm, &charity, 0);
    let verifier = env.verifier.insecure_clone();
    let ix = claim_artist_ix(
        coin,
        &artist.wallet.pubkey(),
        &artist.passport,
        &artist.vault,
        &verifier.pubkey(),
        charity_account,
    );
    send(&mut env.svm, &[ix], &[&artist.wallet, &verifier])
}

fn pay_artist_ix(coin: &CoinKeys, payer: &Pubkey, vault: &Pubkey) -> Instruction {
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::PayArtist {
            payer: *payer,
            coin: coin_pda(&coin.mint),
            passport_vault: *vault,
            escrow: escrow(&coin.mint),
            artist_vault: artist_vault(&coin.mint),
            unwrap: unwrap_account(&coin.mint),
            base_mint: coin.mint,
            quote_mint: WSOL,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::PayArtist {}.data(),
    }
}

fn sweep_charity_ix(coin: &CoinKeys, charity_account: Pubkey) -> Instruction {
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::SweepCharity {
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: coin_pda(&coin.mint),
            escrow: escrow(&coin.mint),
            artist_vault: artist_vault(&coin.mint),
            escrow_base_account: ata(&escrow(&coin.mint), &coin.mint, &TOKEN_2022_ID),
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

// ------------------------------------------------------------ graduation

struct Graduated {
    pool: Pubkey,
    creator_position: Pubkey,
    creator_nft_account: Pubkey,
    partner_position: Pubkey,
    partner_nft_account: Pubkey,
}

/// Buys the curve out, then migrates the pool to DAMM v2 the way Meteora's
/// migration keeper does (anyone may).
fn graduate(env: &mut Env, coin: &CoinKeys) -> Graduated {
    let whale = funded(&mut env.svm);
    wsol_account(&mut env.svm, &whale, 90 * SOL);
    let mut ixs = vec![compute_budget(400_000)];
    ixs.extend(buy_ixs(env, coin, &whale.pubkey(), 90 * SOL, true));
    ok(send(&mut env.svm, &ixs, &[&whale]));
    let payer = funded(&mut env.svm);
    let (first, second) = (Keypair::new(), Keypair::new());
    let pool = damm_pool(&coin.mint);
    let position = |nft: &Keypair| damm_pda(&[b"position", nft.pubkey().as_ref()]);
    let nft_account = |nft: &Keypair| damm_pda(&[b"position_nft_account", nft.pubkey().as_ref()]);
    let mut accounts = dbc::client::accounts::MigrationDammV2 {
        virtual_pool: coin.pool,
        migration_metadata: dbc_pda(&[b"damm_v2", coin.pool.as_ref()]),
        config: env.dbc_config,
        pool_authority: DBC_POOL_AUTHORITY,
        pool,
        first_position_nft_mint: first.pubkey(),
        first_position_nft_account: nft_account(&first),
        first_position: position(&first),
        second_position_nft_mint: Some(second.pubkey()),
        second_position_nft_account: Some(nft_account(&second)),
        second_position: Some(position(&second)),
        damm_pool_authority: DAMM_POOL_AUTHORITY,
        amm_program: damm::ID,
        base_mint: coin.mint,
        quote_mint: WSOL,
        token_a_vault: damm_pda(&[b"token_vault", coin.mint.as_ref(), pool.as_ref()]),
        token_b_vault: damm_pda(&[b"token_vault", WSOL.as_ref(), pool.as_ref()]),
        base_vault: coin.base_vault,
        quote_vault: coin.quote_vault,
        payer: payer.pubkey(),
        token_base_program: TOKEN_2022_ID,
        token_quote_program: TOKEN_PROGRAM_ID,
        token_2022_program: TOKEN_2022_ID,
        damm_event_authority: damm_event_authority(),
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    accounts.push(anchor_lang::prelude::AccountMeta::new_readonly(
        DAMM_CONFIG,
        false,
    ));
    let migrate = Instruction {
        program_id: dbc::ID,
        accounts,
        data: dbc::client::args::MigrationDammV2 {}.data(),
    };
    ok(send(
        &mut env.svm,
        &[compute_budget(1_400_000), migrate],
        &[&payer, &first, &second],
    ));

    // Which position went to whom: the escrow (creator) and riff's fee
    // authority (partner) each hold one NFT.
    let escrow = escrow(&coin.mint);
    let (creator, partner) = if token_owner(&env.svm, &nft_account(&first)) == escrow {
        (&first, &second)
    } else {
        (&second, &first)
    };
    assert_eq!(token_owner(&env.svm, &nft_account(creator)), escrow);
    assert_eq!(
        token_owner(&env.svm, &nft_account(partner)),
        pda(&[riff_dbc::FEE_AUTHORITY_SEED])
    );
    Graduated {
        pool,
        creator_position: position(creator),
        creator_nft_account: nft_account(creator),
        partner_position: position(partner),
        partner_nft_account: nft_account(partner),
    }
}

fn buy_on_damm(env: &mut Env, coin: &CoinKeys, g: &Graduated, lamports: u64) {
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
        program_id: damm::ID,
        accounts: damm::client::accounts::Swap {
            pool_authority: DAMM_POOL_AUTHORITY,
            pool: g.pool,
            input_token_account: wsol,
            output_token_account: base,
            token_a_vault: damm_pda(&[b"token_vault", coin.mint.as_ref(), g.pool.as_ref()]),
            token_b_vault: damm_pda(&[b"token_vault", WSOL.as_ref(), g.pool.as_ref()]),
            token_a_mint: coin.mint,
            token_b_mint: WSOL,
            payer: trader.pubkey(),
            token_a_program: TOKEN_2022_ID,
            token_b_program: TOKEN_PROGRAM_ID,
            referral_token_account: None,
            event_authority: damm_event_authority(),
            program: damm::ID,
        }
        .to_account_metas(None),
        data: damm::client::args::Swap {
            _params: damm::types::SwapParameters {
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

fn collect_lp_fees_ix(
    coin: &CoinKeys,
    g: &Graduated,
    launcher_account: Pubkey,
    treasury_account: Pubkey,
) -> Instruction {
    Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::CollectLpFees {
            config: pda(&[riff_dbc::CONFIG_SEED]),
            coin: coin_pda(&coin.mint),
            escrow: escrow(&coin.mint),
            fee_authority: pda(&[riff_dbc::FEE_AUTHORITY_SEED]),
            fees_vault: pda(&[riff_dbc::FEES_VAULT_SEED]),
            artist_vault: artist_vault(&coin.mint),
            escrow_base_account: ata(&escrow(&coin.mint), &coin.mint, &TOKEN_2022_ID),
            launcher_quote_account: launcher_account,
            treasury_quote_account: treasury_account,
            damm_pool_authority: DAMM_POOL_AUTHORITY,
            damm_pool: g.pool,
            creator_position: g.creator_position,
            creator_position_nft_account: g.creator_nft_account,
            partner_position: g.partner_position,
            partner_position_nft_account: g.partner_nft_account,
            token_a_vault: damm_pda(&[b"token_vault", coin.mint.as_ref(), g.pool.as_ref()]),
            token_b_vault: damm_pda(&[b"token_vault", WSOL.as_ref(), g.pool.as_ref()]),
            base_mint: coin.mint,
            quote_mint: WSOL,
            damm_event_authority: damm_event_authority(),
            damm_program: damm::ID,
            token_quote_program: TOKEN_PROGRAM_ID,
            token_base_program: TOKEN_2022_ID,
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::CollectLpFees {}.data(),
    }
}

// ------------------------------------------------------------ tests

#[test]
fn a_coin_is_a_meteora_pool_held_by_riffs_escrow() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    assert_eq!(pool_creator(&env.svm, &coin.pool), escrow(&coin.mint));
    let record = coin_record(&env.svm, &coin.mint);
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

    // 10 SOL traded on this coin, 5 SOL on another: fees stay per coin.
    buy_on_meteora(&mut env, &coin, 10 * SOL);
    buy_on_meteora(&mut env, &other, 5 * SOL);
    let (launcher_account, treasury_account) = collect(&mut env, &coin);

    // 2% fee: 0.4% Meteora, 0.8% artist (held), 0.3% launcher, 0.5% riff.
    let held = artist_vault(&coin.mint);
    assert_close(
        token_balance(&env.svm, &held),
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
    let before = token_balance(&env.svm, &held);
    collect(&mut env, &coin);
    assert_eq!(token_balance(&env.svm, &held), before);
}

#[test]
fn the_launch_buy_is_capped() {
    let mut env = setup();
    let (coin, res) = try_create_coin(&mut env, SOL / 2);
    ok(res);
    let bought = token_balance(
        &env.svm,
        &ata(&coin.launcher.pubkey(), &coin.mint, &TOKEN_2022_ID),
    );
    assert!(bought > 0);
    collect(&mut env, &coin);
    assert_close(
        token_balance(&env.svm, &artist_vault(&coin.mint)),
        SOL / 2 * 80 / 10_000,
        "artist share of the launch buy",
    );

    // Over the 1 SOL cap: refused.
    let (_, res) = try_create_coin(&mut env, MAX_LAUNCH_BUY + 1);
    fails_with(res, "LaunchBuyTooLarge");
}

#[test]
fn fee_shares_only_go_to_the_right_wallets() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let stranger = funded(&mut env.svm);
    let stranger_account = wsol_account(&mut env.svm, &stranger, 0);
    let (launcher_account, treasury_account) = partner_accounts(&mut env, &coin);
    buy_on_meteora(&mut env, &coin, SOL);
    let admin = env.admin.insecure_clone();
    for (launcher, treasury) in [
        (stranger_account, treasury_account),
        (launcher_account, stranger_account),
    ] {
        fails_with(
            send(
                &mut env.svm,
                &[collect_fees_ix(env.dbc_config, &coin, launcher, treasury)],
                &[&admin],
            ),
            "WrongReceiver",
        );
    }
}

#[test]
fn the_artist_claims_with_their_passport_and_is_paid_in_sol() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    buy_on_meteora(&mut env, &coin, 10 * SOL);
    let artist = passport(&mut env, ARTIST_ID, false);

    // Another wallet can't claim with the artist's passport, and another
    // artist's passport can't claim this coin.
    let thief = ArtistPassport {
        wallet: funded(&mut env.svm),
        passport: artist.passport,
        vault: artist.vault,
    };
    fails_with(claim(&mut env, &coin, &thief), "NotPassportWallet");
    let other = passport(&mut env, "spotify:1OtherArtistDemo000000", false);
    fails_with(claim(&mut env, &coin, &other), "ConstraintSeeds");

    // Without riff's verifier, no claim.
    let impostor = Keypair::new();
    let charity = env.charity.insecure_clone();
    let charity_account = wsol_account(&mut env.svm, &charity, 0);
    let ix = claim_artist_ix(
        &coin,
        &artist.wallet.pubkey(),
        &artist.passport,
        &artist.vault,
        &impostor.pubkey(),
        charity_account,
    );
    fails_with(
        send(&mut env.svm, &[ix], &[&artist.wallet, &impostor]),
        "ConstraintAddress",
    );

    // Claimed: the held 0.8% (collected in the same step) arrives in the
    // passport's vault as SOL. The wallet only fronts the unwrap rent.
    let vault_before = lamports(&env.svm, &artist.vault);
    let wallet_before = lamports(&env.svm, &artist.wallet.pubkey());
    ok(claim(&mut env, &coin, &artist));
    assert_close(
        lamports(&env.svm, &artist.vault) - vault_before,
        10 * SOL * 80 / 10_000,
        "held fees paid to the vault",
    );
    assert!(wallet_before - lamports(&env.svm, &artist.wallet.pubkey()) < 20_000);
    assert_eq!(lamports(&env.svm, &unwrap_account(&coin.mint)), 0);
    assert_eq!(coin_record(&env.svm, &coin.mint).artist, Some(artist.vault));
    // The escrow stays the pool's creator, so a lost wallet loses nothing.
    assert_eq!(pool_creator(&env.svm, &coin.pool), escrow(&coin.mint));
    fails_with(claim(&mut env, &coin, &artist), "AlreadyClaimed");

    // Later trades: collected by the crank, paid to the vault by anyone.
    buy_on_meteora(&mut env, &coin, 5 * SOL);
    collect(&mut env, &coin);
    let cranker = funded(&mut env.svm);
    let stranger = funded(&mut env.svm);
    fails_with(
        send(
            &mut env.svm,
            &[pay_artist_ix(&coin, &cranker.pubkey(), &stranger.pubkey())],
            &[&cranker],
        ),
        "WrongArtistVault",
    );
    let before = lamports(&env.svm, &artist.vault);
    let cranker_before = lamports(&env.svm, &cranker.pubkey());
    ok(send(
        &mut env.svm,
        &[pay_artist_ix(&coin, &cranker.pubkey(), &artist.vault)],
        &[&cranker],
    ));
    assert_close(
        lamports(&env.svm, &artist.vault) - before,
        5 * SOL * 80 / 10_000,
        "later fees paid to the vault",
    );
    assert_eq!(
        cranker_before - lamports(&env.svm, &cranker.pubkey()),
        5_000
    );
    assert_eq!(token_balance(&env.svm, &artist_vault(&coin.mint)), 0);
}

#[test]
fn a_revoked_passport_cant_claim() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let artist = passport(&mut env, ARTIST_ID, true);
    fails_with(claim(&mut env, &coin, &artist), "PassportRevoked");
}

#[test]
fn unpaid_coins_cant_be_paid_out() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let cranker = funded(&mut env.svm);
    let artist = passport(&mut env, ARTIST_ID, false);
    fails_with(
        send(
            &mut env.svm,
            &[pay_artist_ix(&coin, &cranker.pubkey(), &artist.vault)],
            &[&cranker],
        ),
        "WrongArtistVault",
    );
}

#[test]
fn unclaimed_artist_fees_go_to_charity_and_a_late_claim_gets_later_fees() {
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

    advance(&mut env.svm, CLAIM_WINDOW_SECS + 1);
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

    // More trades before the artist shows up: those belong to charity too.
    buy_on_meteora(&mut env, &coin, 5 * SOL);
    let artist = passport(&mut env, ARTIST_ID, false);
    let vault_before = lamports(&env.svm, &artist.vault);
    ok(claim(&mut env, &coin, &artist));
    assert_eq!(lamports(&env.svm, &artist.vault), vault_before);
    assert_close(
        token_balance(&env.svm, &charity_account),
        15 * SOL * 80 / 10_000,
        "late claim: what was held went to charity",
    );

    // From the claim on, the artist is paid.
    buy_on_meteora(&mut env, &coin, 2 * SOL);
    collect(&mut env, &coin);
    let cranker = funded(&mut env.svm);
    ok(send(
        &mut env.svm,
        &[pay_artist_ix(&coin, &cranker.pubkey(), &artist.vault)],
        &[&cranker],
    ));
    assert_close(
        lamports(&env.svm, &artist.vault) - vault_before,
        2 * SOL * 80 / 10_000,
        "fees after a late claim",
    );
    fails_with(
        send(
            &mut env.svm,
            &[sweep_charity_ix(&coin, charity_account)],
            &[&admin],
        ),
        "AlreadyClaimed",
    );
}

#[test]
fn after_graduation_lp_fees_follow_the_same_routes() {
    let mut env = setup();
    let coin = create_coin(&mut env);
    let g = graduate(&mut env, &coin);
    let (launcher_account, treasury_account) = collect(&mut env, &coin);
    let held = artist_vault(&coin.mint);
    let admin = env.admin.insecure_clone();

    // Trades on the graduated pool: 1% LP fee, shared by the two locked
    // positions; riff's side split with the launcher, the artist's held.
    buy_on_damm(&mut env, &coin, &g, 10 * SOL);
    let (held_before, launcher_before, treasury_before) = (
        token_balance(&env.svm, &held),
        token_balance(&env.svm, &launcher_account),
        token_balance(&env.svm, &treasury_account),
    );
    ok(send(
        &mut env.svm,
        &[
            compute_budget(400_000),
            collect_lp_fees_ix(&coin, &g, launcher_account, treasury_account),
        ],
        &[&admin],
    ));
    let artist_lp = token_balance(&env.svm, &held) - held_before;
    let launcher_lp = token_balance(&env.svm, &launcher_account) - launcher_before;
    let riff_lp = token_balance(&env.svm, &treasury_account) - treasury_before;
    let partner_lp = launcher_lp + riff_lp;
    assert!(artist_lp > 10 * SOL / 1_000, "artist LP fees: {artist_lp}");
    assert_close(artist_lp, partner_lp, "both positions earn the same");
    assert_close(
        launcher_lp,
        partner_lp * LAUNCHER_SHARE_BPS as u64 / 10_000,
        "launcher's part of LP fees",
    );

    // Nobody claimed: after the window, it all goes to charity.
    advance(&mut env.svm, CLAIM_WINDOW_SECS + 1);
    let charity = env.charity.insecure_clone();
    let charity_account = wsol_account(&mut env.svm, &charity, 0);
    let held_total = token_balance(&env.svm, &held);
    ok(send(
        &mut env.svm,
        &[sweep_charity_ix(&coin, charity_account)],
        &[&admin],
    ));
    assert_eq!(token_balance(&env.svm, &charity_account), held_total);
}

#[test]
fn only_the_admin_updates_the_config() {
    let mut env = setup();
    let update = |admin: &Pubkey, launcher_share_bps: u16| Instruction {
        program_id: riff_dbc::ID,
        accounts: riff_dbc::accounts::UpdateConfig {
            admin: *admin,
            config: pda(&[riff_dbc::CONFIG_SEED]),
        }
        .to_account_metas(None),
        data: riff_dbc::instruction::UpdateConfig {
            params: riff_dbc::ConfigUpdate {
                admin: *admin,
                verifier: Pubkey::new_unique(),
                charity: Pubkey::new_unique(),
                treasury: Pubkey::new_unique(),
                launcher_share_bps,
                max_launch_buy: MAX_LAUNCH_BUY,
                claim_window_secs: CLAIM_WINDOW_SECS,
            },
        }
        .data(),
    };
    let stranger = funded(&mut env.svm);
    fails_with(
        send(&mut env.svm, &[update(&stranger.pubkey(), 0)], &[&stranger]),
        "ConstraintAddress",
    );
    let admin = env.admin.insecure_clone();
    fails_with(
        send(&mut env.svm, &[update(&admin.pubkey(), 10_001)], &[&admin]),
        "InvalidShare",
    );
    ok(send(
        &mut env.svm,
        &[update(&admin.pubkey(), 5_000)],
        &[&admin],
    ));
}
