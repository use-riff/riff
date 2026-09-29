#![allow(dead_code)]

use {
    anchor_lang::solana_program::instruction::AccountMeta,
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::{bpf_loader_upgradeable, instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::token_2022::{
        spl_token_2022::{extension::StateWithExtensions, state::Account as TokenAccount},
        ID as TOKEN_2022_ID,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

pub const SOL: u64 = 1_000_000_000;
pub const TOKEN: u64 = 1_000_000;
pub const START_TIME: i64 = 1_790_000_000;
/// 1% per trade: 0.5% artist, 0.2% creator, 0.3% protocol.
pub const ARTIST_FEE_BPS: u16 = 50;
pub const CREATOR_FEE_BPS: u16 = 20;
pub const PROTOCOL_FEE_BPS: u16 = 30;
pub const TOTAL_FEE_BPS: u16 = ARTIST_FEE_BPS + CREATOR_FEE_BPS + PROTOCOL_FEE_BPS;
pub const CLAIM_WINDOW_SECS: i64 = 90 * 24 * 60 * 60;
pub const INITIAL_VIRTUAL_SOL: u64 = 30 * SOL;
pub const CURVE_TOKEN_SUPPLY: u64 = 793_100_000 * TOKEN;
/// What the program derives from the supply split (793.1M / 206.9M).
pub const INITIAL_VIRTUAL_TOKEN: u64 = 1_073_025_605_595_359;
/// 3% of supply.
pub const MAX_CREATOR_BUY_BPS: u16 = 300;
/// Starting treasury balance: enough to stay rent-exempt.
pub const TREASURY_START: u64 = 1_000_000;
/// Starting charity balance: enough to stay rent-exempt.
pub const CHARITY_START: u64 = 1_000_000;

/// Raydium CPMM fee tier 0 (0.25% trade fee) on mainnet; see fixtures/.
pub const RAYDIUM_AMM_CONFIG: Pubkey =
    Pubkey::from_str_const("D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2");
/// Raydium CPMM fee tier 1 (1% trade fee): a real tier riff isn't configured for.
pub const RAYDIUM_AMM_CONFIG_1PCT: Pubkey =
    Pubkey::from_str_const("G95xxie3XbkCqtE39GgQ9Ggc7xBC8Uceve7HFDEFApkc");

const ATA_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub struct Env {
    pub svm: LiteSVM,
    /// Program upgrade authority.
    pub admin: Keypair,
    pub treasury: Pubkey,
    /// Receives unclaimable artist fees.
    pub charity: Pubkey,
    /// riff's verification service: co-signs artist claims.
    pub verifier: Keypair,
}

pub fn program_data_address() -> Pubkey {
    Pubkey::find_program_address(&[riff::ID.as_ref()], &bpf_loader_upgradeable::ID).0
}

pub fn config_address() -> Pubkey {
    Pubkey::find_program_address(&[riff::CONFIG_SEED], &riff::ID).0
}

pub fn coin_address(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[riff::COIN_SEED, mint.as_ref()], &riff::ID).0
}

pub fn vault_address(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[riff::VAULT_SEED, mint.as_ref()], &riff::ID).0
}

/// Fresh VM with riff deployed and `admin` set as its upgrade authority.
pub fn setup() -> Env {
    let mut svm = LiteSVM::new();
    let bytes = &std::fs::read(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/riff.so"))
        .expect("riff.so is missing: run `make build` first");
    svm.add_program(riff::ID, bytes).unwrap();
    load_live_token_programs(&mut svm);
    load_raydium(&mut svm);

    // LiteSVM deploys with no upgrade authority. Rewrite the ProgramData
    // header (bincode: u32 tag, u64 slot, Option<Pubkey>) to set one.
    let admin = Keypair::new();
    let program_data = program_data_address();
    let mut account = svm.get_account(&program_data).unwrap();
    account.data[12] = 1;
    account.data[13..45].copy_from_slice(admin.pubkey().as_ref());
    svm.set_account(program_data, account).unwrap();

    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = START_TIME;
    svm.set_sysvar(&clock);

    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    let treasury = Pubkey::new_unique();
    svm.airdrop(&treasury, TREASURY_START).unwrap();
    let charity = Pubkey::new_unique();
    svm.airdrop(&charity, CHARITY_START).unwrap();
    Env {
        svm,
        admin,
        treasury,
        charity,
        verifier: Keypair::new(),
    }
}

pub fn funded_keypair(svm: &mut LiteSVM) -> Keypair {
    let kp = Keypair::new();
    svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    kp
}

// TransactionResult is LiteSVM's type; its large Err is not ours to box.
#[allow(clippy::result_large_err)]
pub fn send(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) -> TransactionResult {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&signers[0].pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    let res = svm.send_transaction(tx);
    svm.expire_blockhash();
    res
}

/// Sends several instructions as one atomic transaction; the first signer
/// pays.
#[allow(clippy::result_large_err)]
pub fn send_many(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> TransactionResult {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&signers[0].pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    let res = svm.send_transaction(tx);
    svm.expire_blockhash();
    res
}

/// Associated Token Account program CreateIdempotent for `owner`, paid by
/// `payer`.
pub fn create_ata_ix(payer: &Pubkey, owner: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        ATA_PROGRAM_ID,
        &[1],
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(ata_address(owner, mint), false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
        ],
    )
}

/// Asserts the transaction failed with the given riff error.
pub fn assert_riff_error(res: TransactionResult, expected: riff::error::ErrorCode) {
    let err = res.expect_err("transaction should have failed");
    let code = u32::from(expected);
    let logs = err.meta.logs.join("\n");
    assert!(
        format!("{:?}", err.err).contains(&format!("Custom({code})")),
        "expected {expected:?} ({code}), got {:?}\n{logs}",
        err.err
    );
}

pub fn config_params(env: &Env) -> riff::ConfigParams {
    riff::ConfigParams {
        treasury: env.treasury,
        charity: env.charity,
        verifier: env.verifier.pubkey(),
        raydium_amm_config: RAYDIUM_AMM_CONFIG,
        artist_fee_bps: ARTIST_FEE_BPS,
        creator_fee_bps: CREATOR_FEE_BPS,
        protocol_fee_bps: PROTOCOL_FEE_BPS,
        claim_window_secs: CLAIM_WINDOW_SECS,
        initial_virtual_sol_reserves: INITIAL_VIRTUAL_SOL,
        curve_token_supply: CURVE_TOKEN_SUPPLY,
        max_creator_buy_bps: MAX_CREATOR_BUY_BPS,
    }
}

pub fn initialize_config_ix(admin: &Pubkey, params: riff::ConfigParams) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::InitializeConfig { params }.data(),
        riff::accounts::InitializeConfig {
            admin: *admin,
            config: config_address(),
            program: riff::ID,
            program_data: program_data_address(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn initialize_config(env: &mut Env) {
    let ix = initialize_config_ix(&env.admin.pubkey(), config_params(env));
    let admin = env.admin.insecure_clone();
    send(&mut env.svm, ix, &[&admin]).unwrap();
}

pub fn coin_args() -> riff::CreateCoinArgs {
    riff::CreateCoinArgs {
        name: "Midnight Static".into(),
        symbol: "STATIC".into(),
        uri: "https://example.com/static.json".into(),
        artist_id: "4Z8W4fKeB5YxbusRsdQVPb".into(),
        artist_name: "Midnight Static".into(),
        creator_buy_sol: 0,
        creator_buy_min_tokens: 0,
    }
}

pub fn create_coin_ix(creator: &Pubkey, mint: &Pubkey, args: riff::CreateCoinArgs) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::CreateCoin { args }.data(),
        riff::accounts::CreateCoin {
            creator: *creator,
            config: config_address(),
            coin: coin_address(mint),
            mint: *mint,
            vault: vault_address(mint),
            creator_token_account: ata_address(creator, mint),
            token_program: TOKEN_2022_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn fetch<T: AccountDeserialize>(svm: &LiteSVM, address: &Pubkey) -> T {
    let account = svm.get_account(address).expect("account missing");
    T::try_deserialize(&mut account.data.as_slice()).unwrap()
}

/// Config initialized and one coin created. Returns the coin's mint.
pub fn setup_coin(env: &mut Env) -> Pubkey {
    setup_coin_with_creator(env).1
}

/// Like [`setup_coin`], also returning the creator.
pub fn setup_coin_with_creator(env: &mut Env) -> (Keypair, Pubkey) {
    initialize_config(env);
    let creator = funded_keypair(&mut env.svm);
    let mint = Keypair::new();
    let ix = create_coin_ix(&creator.pubkey(), &mint.pubkey(), coin_args());
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();
    next_slot(&mut env.svm);
    (creator, mint.pubkey())
}

/// Advances one slot. `buy` is closed in a coin's launch slot, so tests move
/// on before trading, as the chain would.
pub fn next_slot(svm: &mut LiteSVM) {
    let slot = svm.get_sysvar::<Clock>().slot;
    svm.warp_to_slot(slot + 1);
}

pub fn withdraw_creator_fees_ix(creator: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::WithdrawCreatorFees {}.data(),
        riff::accounts::WithdrawCreatorFees {
            creator: *creator,
            coin: coin_address(mint),
        }
        .to_account_metas(None),
    )
}

/// Decodes every Anchor event of type `E` from a transaction's logs, the way
/// an indexer would.
pub fn events<E: anchor_lang::Event + anchor_lang::AnchorDeserialize>(logs: &[String]) -> Vec<E> {
    use base64::Engine;
    logs.iter()
        .filter_map(|l| l.strip_prefix("Program data: "))
        .filter_map(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
        .filter(|data| data.starts_with(E::DISCRIMINATOR))
        .map(|data| E::try_from_slice(&data[E::DISCRIMINATOR.len()..]).unwrap())
        .collect()
}

pub fn ata_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), TOKEN_2022_ID.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

/// Wallet with SOL and an empty token account for `mint`.
pub fn trader(svm: &mut LiteSVM, mint: &Pubkey, lamports: u64) -> Keypair {
    let kp = Keypair::new();
    svm.airdrop(&kp.pubkey(), lamports).unwrap();
    let ix = create_ata_ix(&kp.pubkey(), &kp.pubkey(), mint);
    send(svm, ix, &[&kp]).unwrap();
    kp
}

fn swap_accounts(trader: &Pubkey, mint: &Pubkey) -> Vec<AccountMeta> {
    riff::accounts::Swap {
        trader: *trader,
        config: config_address(),
        coin: coin_address(mint),
        mint: *mint,
        vault: vault_address(mint),
        trader_token_account: ata_address(trader, mint),
        token_program: TOKEN_2022_ID,
        system_program: system_program::ID,
    }
    .to_account_metas(None)
}

pub fn buy_ix(trader: &Pubkey, mint: &Pubkey, max_sol_in: u64, min_tokens_out: u64) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::Buy {
            max_sol_in,
            min_tokens_out,
        }
        .data(),
        swap_accounts(trader, mint),
    )
}

pub fn sell_ix(trader: &Pubkey, mint: &Pubkey, token_amount: u64, min_sol_out: u64) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::Sell {
            token_amount,
            min_sol_out,
        }
        .data(),
        swap_accounts(trader, mint),
    )
}

pub fn token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account = svm
        .get_account(token_account)
        .expect("token account missing");
    StateWithExtensions::<TokenAccount>::unpack(&account.data)
        .unwrap()
        .base
        .amount
}

pub fn lamports(svm: &LiteSVM, address: &Pubkey) -> u64 {
    svm.get_account(address).map_or(0, |a| a.lamports)
}

pub fn collect_protocol_fees_ix(treasury: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::CollectProtocolFees {}.data(),
        riff::accounts::CollectProtocolFees {
            config: config_address(),
            treasury: *treasury,
            coin: coin_address(mint),
        }
        .to_account_metas(None),
    )
}

pub fn claim_artist_ix(
    artist: &Pubkey,
    verifier: &Pubkey,
    mint: &Pubkey,
    artist_id: &str,
) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::ClaimArtist {
            artist_id: artist_id.to_string(),
        }
        .data(),
        riff::accounts::ClaimArtist {
            artist: *artist,
            verifier: *verifier,
            config: config_address(),
            coin: coin_address(mint),
        }
        .to_account_metas(None),
    )
}

/// The coin's artist claims it, co-signed by the configured verifier.
#[allow(clippy::result_large_err)]
pub fn claim_artist(env: &mut Env, artist: &Keypair, mint: &Pubkey) -> TransactionResult {
    let artist_id = fetch::<riff::Coin>(&env.svm, &coin_address(mint)).artist_id;
    let verifier = env.verifier.insecure_clone();
    let ix = claim_artist_ix(&artist.pubkey(), &verifier.pubkey(), mint, &artist_id);
    send(&mut env.svm, ix, &[artist, &verifier])
}

pub fn withdraw_artist_fees_ix(artist: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::WithdrawArtistFees {}.data(),
        riff::accounts::WithdrawArtistFees {
            artist: *artist,
            coin: coin_address(mint),
        }
        .to_account_metas(None),
    )
}

pub fn sweep_charity_fees_ix(charity: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::SweepCharityFees {}.data(),
        riff::accounts::SweepCharityFees {
            config: config_address(),
            charity: *charity,
            coin: coin_address(mint),
        }
        .to_account_metas(None),
    )
}

pub fn update_config_ix(admin: &Pubkey, params: riff::UpdateConfigParams) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::UpdateConfig { params }.data(),
        riff::accounts::UpdateConfig {
            admin: *admin,
            config: config_address(),
        }
        .to_account_metas(None),
    )
}

/// Moves the clock `secs` forward.
pub fn advance_time(svm: &mut LiteSVM, secs: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp += secs;
    svm.set_sysvar(&clock);
}

/// Replaces LiteSVM's bundled token programs with the builds mainnet and
/// devnet actually run (tests/fixtures/). The bundled SPL Token is older and
/// more lenient: it accepted a call the live program rejects.
fn load_live_token_programs(svm: &mut LiteSVM) {
    svm.add_program(TOKEN_PROGRAM_ID, include_bytes!("../fixtures/spl_token.so"))
        .unwrap();
    svm.add_program(
        TOKEN_2022_ID,
        include_bytes!("../fixtures/spl_token_2022.so"),
    )
    .unwrap();
}

/// Loads Raydium CPMM and the mainnet accounts it needs, from snapshots in
/// tests/fixtures/ (taken with `solana program dump` / `solana account`).
fn load_raydium(svm: &mut LiteSVM) {
    let so = include_bytes!("../fixtures/raydium_cpmm.so");
    svm.add_program(riff::RAYDIUM_CPMM_PROGRAM_ID, so).unwrap();
    for json in [
        include_str!("../fixtures/amm_config_0.json"),
        include_str!("../fixtures/amm_config_1.json"),
        include_str!("../fixtures/create_pool_fee_receiver.json"),
        include_str!("../fixtures/wsol_mint.json"),
    ] {
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
        let key: Pubkey = v["pubkey"].as_str().unwrap().parse().unwrap();
        svm.set_account(key, account).unwrap();
    }
}

// ---------------------------------------------------------------- graduation

pub const TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const WSOL_MINT: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
const COMPUTE_BUDGET_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ComputeBudget111111111111111111111111111111");

pub fn ata_for(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

fn raydium_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &riff::RAYDIUM_CPMM_PROGRAM_ID).0
}

/// Every address involved in graduating `mint`.
pub struct GraduationAccounts {
    pub authority: Pubkey,
    pub token_account: Pubkey,
    pub wsol_account: Pubkey,
    pub lp_account: Pubkey,
    pub pool: Pubkey,
    pub raydium_authority: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_token_vault: Pubkey,
    pub pool_wsol_vault: Pubkey,
    pub observation: Pubkey,
}

pub fn graduation_accounts(mint: &Pubkey) -> GraduationAccounts {
    let authority =
        Pubkey::find_program_address(&[riff::GRADUATION_SEED, mint.as_ref()], &riff::ID).0;
    let pool = Pubkey::find_program_address(&[riff::POOL_SEED, mint.as_ref()], &riff::ID).0;
    let lp_mint = raydium_pda(&[riff::RAYDIUM_POOL_LP_MINT_SEED, pool.as_ref()]);
    GraduationAccounts {
        authority,
        token_account: ata_for(&authority, mint, &TOKEN_2022_ID),
        wsol_account: ata_for(&authority, &WSOL_MINT, &TOKEN_PROGRAM_ID),
        lp_account: ata_for(&authority, &lp_mint, &TOKEN_PROGRAM_ID),
        pool,
        raydium_authority: raydium_pda(&[riff::RAYDIUM_AUTH_SEED]),
        lp_mint,
        pool_token_vault: raydium_pda(&[
            riff::RAYDIUM_POOL_VAULT_SEED,
            pool.as_ref(),
            mint.as_ref(),
        ]),
        pool_wsol_vault: raydium_pda(&[
            riff::RAYDIUM_POOL_VAULT_SEED,
            pool.as_ref(),
            WSOL_MINT.as_ref(),
        ]),
        observation: raydium_pda(&[riff::RAYDIUM_OBSERVATION_SEED, pool.as_ref()]),
    }
}

pub fn compute_budget_ix(units: u32) -> Instruction {
    let mut data = vec![2];
    data.extend_from_slice(&units.to_le_bytes());
    Instruction::new_with_bytes(COMPUTE_BUDGET_PROGRAM_ID, &data, vec![])
}

pub fn graduate_ix(payer: &Pubkey, mint: &Pubkey) -> Instruction {
    let g = graduation_accounts(mint);
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::Graduate {}.data(),
        riff::accounts::Graduate {
            payer: *payer,
            config: config_address(),
            coin: coin_address(mint),
            mint: *mint,
            vault: vault_address(mint),
            graduation_authority: g.authority,
            graduation_token_account: g.token_account,
            graduation_wsol_account: g.wsol_account,
            graduation_lp_account: g.lp_account,
            pool_state: g.pool,
            raydium_program: riff::RAYDIUM_CPMM_PROGRAM_ID,
            amm_config: RAYDIUM_AMM_CONFIG,
            raydium_authority: g.raydium_authority,
            lp_mint: g.lp_mint,
            pool_token_vault: g.pool_token_vault,
            pool_wsol_vault: g.pool_wsol_vault,
            observation_state: g.observation,
            create_pool_fee_receiver: riff::RAYDIUM_CREATE_POOL_FEE_RECEIVER,
            wsol_mint: WSOL_MINT,
            token_program: TOKEN_PROGRAM_ID,
            token_2022_program: TOKEN_2022_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
            rent: Pubkey::from_str_const("SysvarRent111111111111111111111111111111111"),
        }
        .to_account_metas(None),
    )
}

pub fn prepare_graduation_ix(mint: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::PrepareGraduation {}.data(),
        riff::accounts::PrepareGraduation {
            coin: coin_address(mint),
            mint: *mint,
            graduation_authority: graduation_accounts(mint).authority,
        }
        .to_account_metas(None),
    )
}

/// Graduates `mint` the way clients do: prepare + graduate in one
/// transaction, with a generous compute budget (Raydium's pool creation is
/// heavy); `payer` pays and signs.
#[allow(clippy::result_large_err)]
pub fn graduate(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey) -> TransactionResult {
    send_many(
        svm,
        &[
            compute_budget_ix(600_000),
            prepare_graduation_ix(mint),
            graduate_ix(&payer.pubkey(), mint),
        ],
        &[payer],
    )
}

/// Raydium `swap_base_input` selling `amount_in` of the coin for wrapped SOL.
pub fn raydium_sell_ix(
    trader: &Pubkey,
    mint: &Pubkey,
    amount_in: u64,
    min_out: u64,
) -> Instruction {
    let g = graduation_accounts(mint);
    let mut data = vec![143, 190, 90, 218, 196, 30, 51, 222];
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_out.to_le_bytes());
    Instruction::new_with_bytes(
        riff::RAYDIUM_CPMM_PROGRAM_ID,
        &data,
        vec![
            AccountMeta::new_readonly(*trader, true),
            AccountMeta::new_readonly(g.raydium_authority, false),
            AccountMeta::new_readonly(RAYDIUM_AMM_CONFIG, false),
            AccountMeta::new(g.pool, false),
            AccountMeta::new(ata_for(trader, mint, &TOKEN_2022_ID), false),
            AccountMeta::new(ata_for(trader, &WSOL_MINT, &TOKEN_PROGRAM_ID), false),
            AccountMeta::new(g.pool_token_vault, false),
            AccountMeta::new(g.pool_wsol_vault, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(WSOL_MINT, false),
            AccountMeta::new(g.observation, false),
        ],
    )
}

/// ATA program CreateIdempotent for any token program.
pub fn create_ata_for_ix(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        ATA_PROGRAM_ID,
        &[1],
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(ata_for(owner, mint, token_program), false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
    )
}

/// Token amount of any SPL / Token-2022 account (same offset in both).
pub fn raw_token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm
        .get_account(account)
        .expect("token account missing")
        .data;
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

/// Supply of any SPL / Token-2022 mint.
pub fn mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let data = svm.get_account(mint).expect("mint missing").data;
    u64::from_le_bytes(data[36..44].try_into().unwrap())
}

pub fn transfer_admin_ix(admin: &Pubkey, new_admin: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::TransferAdmin {
            new_admin: *new_admin,
        }
        .data(),
        riff::accounts::TransferAdmin {
            admin: *admin,
            config: config_address(),
        }
        .to_account_metas(None),
    )
}

pub fn accept_admin_ix(pending_admin: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::AcceptAdmin {}.data(),
        riff::accounts::AcceptAdmin {
            pending_admin: *pending_admin,
            config: config_address(),
        }
        .to_account_metas(None),
    )
}
