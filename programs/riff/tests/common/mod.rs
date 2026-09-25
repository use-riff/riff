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

const ATA_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub struct Env {
    pub svm: LiteSVM,
    /// Program upgrade authority.
    pub admin: Keypair,
    pub treasury: Pubkey,
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
    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/riff.so"));
    svm.add_program(riff::ID, bytes).unwrap();

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
    Env {
        svm,
        admin,
        treasury,
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

pub fn config_params(treasury: Pubkey) -> riff::ConfigParams {
    riff::ConfigParams {
        treasury,
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
    let ix = initialize_config_ix(&env.admin.pubkey(), config_params(env.treasury));
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

pub fn create_coin_ix(
    creator: &Pubkey,
    mint: &Pubkey,
    treasury: &Pubkey,
    args: riff::CreateCoinArgs,
) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::CreateCoin { args }.data(),
        riff::accounts::CreateCoin {
            creator: *creator,
            config: config_address(),
            treasury: *treasury,
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
    let ix = create_coin_ix(
        &creator.pubkey(),
        &mint.pubkey(),
        &env.treasury,
        coin_args(),
    );
    send(&mut env.svm, ix, &[&creator, &mint]).unwrap();
    (creator, mint.pubkey())
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
    // Associated Token Account program: CreateIdempotent.
    let ix = Instruction::new_with_bytes(
        ATA_PROGRAM_ID,
        &[1],
        vec![
            AccountMeta::new(kp.pubkey(), true),
            AccountMeta::new(ata_address(&kp.pubkey(), mint), false),
            AccountMeta::new_readonly(kp.pubkey(), false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
        ],
    );
    send(svm, ix, &[&kp]).unwrap();
    kp
}

fn swap_accounts(trader: &Pubkey, mint: &Pubkey, treasury: &Pubkey) -> Vec<AccountMeta> {
    riff::accounts::Swap {
        trader: *trader,
        config: config_address(),
        treasury: *treasury,
        coin: coin_address(mint),
        mint: *mint,
        vault: vault_address(mint),
        trader_token_account: ata_address(trader, mint),
        token_program: TOKEN_2022_ID,
        system_program: system_program::ID,
    }
    .to_account_metas(None)
}

pub fn buy_ix(
    trader: &Pubkey,
    mint: &Pubkey,
    treasury: &Pubkey,
    max_sol_in: u64,
    min_tokens_out: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::Buy {
            max_sol_in,
            min_tokens_out,
        }
        .data(),
        swap_accounts(trader, mint, treasury),
    )
}

pub fn sell_ix(
    trader: &Pubkey,
    mint: &Pubkey,
    treasury: &Pubkey,
    token_amount: u64,
    min_sol_out: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::Sell {
            token_amount,
            min_sol_out,
        }
        .data(),
        swap_accounts(trader, mint, treasury),
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
