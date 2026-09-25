#![allow(dead_code)]

use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::{bpf_loader_upgradeable, instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::token_2022::ID as TOKEN_2022_ID,
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

pub const START_TIME: i64 = 1_790_000_000;
pub const ARTIST_FEE_SHARE_BPS: u16 = 2_500;
pub const CLAIM_WINDOW_SECS: i64 = 90 * 24 * 60 * 60;

pub struct Env {
    pub svm: LiteSVM,
    /// Program upgrade authority.
    pub admin: Keypair,
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
    Env { svm, admin }
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

pub fn initialize_config_ix(
    admin: &Pubkey,
    artist_fee_share_bps: u16,
    claim_window_secs: i64,
) -> Instruction {
    Instruction::new_with_bytes(
        riff::ID,
        &riff::instruction::InitializeConfig {
            artist_fee_share_bps,
            claim_window_secs,
        }
        .data(),
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
    let ix = initialize_config_ix(&env.admin.pubkey(), ARTIST_FEE_SHARE_BPS, CLAIM_WINDOW_SECS);
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
            token_program: TOKEN_2022_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn fetch<T: AccountDeserialize>(svm: &LiteSVM, address: &Pubkey) -> T {
    let account = svm.get_account(address).expect("account missing");
    T::try_deserialize(&mut account.data.as_slice()).unwrap()
}
