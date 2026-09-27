//! H-01: graduating a sold-out coin into a Raydium CPMM pool.
//!
//! Runs against Raydium's real mainnet program (tests/fixtures/).

mod common;

use {
    anchor_lang::prelude::{Clock, Pubkey},
    anchor_spl::token_2022::ID as TOKEN_2022_ID,
    common::*,
    riff::{error::ErrorCode, COIN_TOTAL_SUPPLY},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

const RESERVE: u64 = COIN_TOTAL_SUPPLY - CURVE_TOKEN_SUPPLY;

struct Launch {
    env: Env,
    mint: Pubkey,
    holder: Keypair,
}

fn coin(l: &Launch) -> riff::Coin {
    fetch(&l.env.svm, &coin_address(&l.mint))
}

/// A coin with one ordinary holder, bought out by a whale so the curve is
/// complete and ready to graduate.
fn sold_out() -> Launch {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let payer = env.admin.insecure_clone();
    let holder = trader(&mut env.svm, &mint, 20 * SOL);
    send(
        &mut env.svm,
        buy_ix(&holder.pubkey(), &mint, 5 * SOL, 0),
        &[&payer, &holder],
    )
    .unwrap();
    let whale = trader(&mut env.svm, &mint, 1_000 * SOL);
    send(
        &mut env.svm,
        buy_ix(&whale.pubkey(), &mint, 500 * SOL, 0),
        &[&payer, &whale],
    )
    .unwrap();
    let l = Launch { env, mint, holder };
    assert!(coin(&l).complete);
    l
}

fn keeper(l: &mut Launch) -> Keypair {
    funded_keypair(&mut l.env.svm)
}

#[test]
fn graduation_moves_exact_liquidity_into_a_locked_raydium_pool() {
    let mut l = sold_out();
    let before = coin(&l);
    let raised = before.real_sol_reserves;
    let g = graduation_accounts(&l.mint);
    let k = keeper(&mut l);
    let keeper_start = lamports(&l.env.svm, &k.pubkey());

    let meta = graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    println!("graduate: {} CU", meta.compute_units_consumed);

    // Pool holds exactly what the curve raised and exactly the reserve.
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_wsol_vault), raised);
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_token_vault), RESERVE);
    assert_eq!(token_balance(&l.env.svm, &vault_address(&l.mint)), 0);

    // Every LP token riff received was burned: nobody can pull liquidity.
    let event = &events::<riff::events::Graduated>(&meta.logs)[0];
    assert!(event.lp_burned > 0);
    assert_eq!(event.pool, g.pool);
    assert_eq!(event.sol_amount, raised);
    assert_eq!(event.token_amount, RESERVE);
    assert_eq!(mint_supply(&l.env.svm, &g.lp_mint), 0, "no LP tokens exist");

    // Temporary accounts are gone and the graduation authority holds nothing.
    for temp in [g.token_account, g.wsol_account, g.lp_account] {
        assert!(l.env.svm.get_account(&temp).is_none_or(|a| a.lamports == 0));
    }
    assert_eq!(lamports(&l.env.svm, &g.authority), 0);

    // The coin records the pool and still backs every fee balance.
    let after = coin(&l);
    assert_eq!(after.pool, Some(g.pool));
    assert_eq!(after.real_sol_reserves, 0);
    assert_eq!(
        after.protocol_fees,
        before.protocol_fees - event.cost_reimbursed
    );
    let key = coin_address(&l.mint);
    let len = l.env.svm.get_account(&key).unwrap().data.len();
    assert_eq!(
        lamports(&l.env.svm, &key),
        l.env.svm.minimum_balance_for_rent_exemption(len)
            + after.artist_fees
            + after.charity_fees
            + after.creator_fees
            + after.protocol_fees
    );

    // The keeper was reimbursed in full from protocol fees: only the
    // network fee remains as their cost.
    assert!(
        event.cost_reimbursed > 150_000_000,
        "includes Raydium's 0.15 SOL fee"
    );
    assert_eq!(lamports(&l.env.svm, &k.pubkey()), keeper_start - 5_000);
}

#[test]
fn pool_opens_at_the_curves_final_price() {
    let mut l = sold_out();
    let c = coin(&l);
    let g = graduation_accounts(&l.mint);
    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();

    // Curve's final price: virtual SOL / virtual tokens.
    // Pool's opening price: SOL in pool / tokens in pool.
    let pool_sol = raw_token_balance(&l.env.svm, &g.pool_wsol_vault) as u128;
    let pool_tokens = raw_token_balance(&l.env.svm, &g.pool_token_vault) as u128;
    let final_x = c.virtual_sol_reserves as u128 * pool_tokens;
    let pool_x = pool_sol * c.virtual_token_reserves as u128;
    let gap = (pool_x as f64 - final_x as f64).abs() / final_x as f64;
    assert!(gap < 1e-9, "gap {gap:e}");
}

#[test]
fn holders_can_sell_on_raydium_after_graduation() {
    let mut l = sold_out();
    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    // Raydium opens the pool one second after creation.
    let mut clock = l.env.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += 2;
    l.env.svm.set_sysvar(&clock);

    let holder = l.holder.insecure_clone();
    let tokens = token_balance(&l.env.svm, &ata_address(&holder.pubkey(), &l.mint));
    assert!(tokens > 0);
    let ixs = [
        create_ata_for_ix(
            &holder.pubkey(),
            &holder.pubkey(),
            &WSOL_MINT,
            &TOKEN_PROGRAM_ID,
        ),
        raydium_sell_ix(&holder.pubkey(), &l.mint, tokens, 1),
    ];
    send_many(&mut l.env.svm, &ixs, &[&holder]).unwrap();

    let wsol = raw_token_balance(
        &l.env.svm,
        &ata_for(&holder.pubkey(), &WSOL_MINT, &TOKEN_PROGRAM_ID),
    );
    assert_eq!(
        token_balance(&l.env.svm, &ata_address(&holder.pubkey(), &l.mint)),
        0
    );
    // Bought for 5 SOL early on the curve; the price rose, so they get more.
    assert!(wsol > 5 * SOL, "received {wsol} lamports of wrapped SOL");
}

#[test]
fn cannot_graduate_before_the_curve_sells_out() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let payer = env.admin.insecure_clone();
    let alice = trader(&mut env.svm, &mint, 50 * SOL);
    send(
        &mut env.svm,
        buy_ix(&alice.pubkey(), &mint, 10 * SOL, 0),
        &[&payer, &alice],
    )
    .unwrap();
    let k = funded_keypair(&mut env.svm);
    assert_riff_error(
        graduate(&mut env.svm, &k, &mint),
        ErrorCode::CurveNotComplete,
    );
}

#[test]
fn cannot_graduate_twice() {
    let mut l = sold_out();
    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    assert_riff_error(
        graduate(&mut l.env.svm, &k, &l.mint),
        ErrorCode::AlreadyGraduated,
    );
}

#[test]
fn keeper_covers_the_cost_only_when_protocol_fees_fall_short() {
    let mut l = sold_out();
    // Someone collected the protocol fees just before graduation.
    let stranger = keeper(&mut l);
    let ix = collect_protocol_fees_ix(&l.env.treasury, &l.mint);
    send(&mut l.env.svm, ix, &[&stranger]).unwrap();
    assert_eq!(coin(&l).protocol_fees, 0);

    let k = keeper(&mut l);
    let start = lamports(&l.env.svm, &k.pubkey());
    let meta = graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    let event = &events::<riff::events::Graduated>(&meta.logs)[0];
    assert_eq!(event.cost_reimbursed, 0);
    let spent = start - lamports(&l.env.svm, &k.pubkey());
    // Raydium's 0.15 SOL fee plus rent for the pool's accounts.
    assert!(
        (150_000_000..250_000_000).contains(&spent),
        "keeper spent {spent}"
    );
    // Graduation still moved the exact liquidity.
    let g = graduation_accounts(&l.mint);
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_token_vault), RESERVE);
}

// ------------------------------------------------------------------ attacks

/// Raydium `initialize` for `mint`/wrapped SOL at `pool`, funded by
/// `creator` (who must already hold the tokens and wrapped SOL).
fn raydium_initialize_ix(
    creator: &Pubkey,
    pool: &Pubkey,
    pool_signs: bool,
    mint: &Pubkey,
    tokens: u64,
    sol: u64,
) -> anchor_lang::solana_program::instruction::Instruction {
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    let raydium = riff::RAYDIUM_CPMM_PROGRAM_ID;
    let pda = |seeds: &[&[u8]]| Pubkey::find_program_address(seeds, &raydium).0;
    let lp_mint = pda(&[b"pool_lp_mint", pool.as_ref()]);
    let coin_first = *mint < WSOL_MINT;
    let coin_side = (
        *mint,
        ata_for(creator, mint, &TOKEN_2022_ID),
        pda(&[b"pool_vault", pool.as_ref(), mint.as_ref()]),
        TOKEN_2022_ID,
        tokens,
    );
    let sol_side = (
        WSOL_MINT,
        ata_for(creator, &WSOL_MINT, &TOKEN_PROGRAM_ID),
        pda(&[b"pool_vault", pool.as_ref(), WSOL_MINT.as_ref()]),
        TOKEN_PROGRAM_ID,
        sol,
    );
    let (s0, s1) = if coin_first {
        (coin_side, sol_side)
    } else {
        (sol_side, coin_side)
    };
    let mut data = riff::RAYDIUM_INITIALIZE_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&s0.4.to_le_bytes());
    data.extend_from_slice(&s1.4.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes());
    Instruction::new_with_bytes(
        raydium,
        &data,
        vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new_readonly(RAYDIUM_AMM_CONFIG, false),
            AccountMeta::new_readonly(pda(&[b"vault_and_lp_mint_auth_seed"]), false),
            AccountMeta::new(*pool, pool_signs),
            AccountMeta::new_readonly(s0.0, false),
            AccountMeta::new_readonly(s1.0, false),
            AccountMeta::new(lp_mint, false),
            AccountMeta::new(s0.1, false),
            AccountMeta::new(s1.1, false),
            AccountMeta::new(ata_for(creator, &lp_mint, &TOKEN_PROGRAM_ID), false),
            AccountMeta::new(s0.2, false),
            AccountMeta::new(s1.2, false),
            AccountMeta::new(riff::RAYDIUM_CREATE_POOL_FEE_RECEIVER, false),
            AccountMeta::new(pda(&[b"observation", pool.as_ref()]), false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(s0.3, false),
            AccountMeta::new_readonly(s1.3, false),
            AccountMeta::new_readonly(
                Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
                false,
            ),
            AccountMeta::new_readonly(
                Pubkey::from_str_const("11111111111111111111111111111111"),
                false,
            ),
            AccountMeta::new_readonly(
                Pubkey::from_str_const("SysvarRent111111111111111111111111111111111"),
                false,
            ),
        ],
    )
}

/// Wraps `sol` of `owner`'s SOL into their wrapped-SOL account.
fn wrap_sol_ixs(
    owner: &Pubkey,
    sol: u64,
) -> Vec<anchor_lang::solana_program::instruction::Instruction> {
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    let wsol = ata_for(owner, &WSOL_MINT, &TOKEN_PROGRAM_ID);
    let mut transfer = vec![2, 0, 0, 0];
    transfer.extend_from_slice(&sol.to_le_bytes());
    vec![
        create_ata_for_ix(owner, owner, &WSOL_MINT, &TOKEN_PROGRAM_ID),
        Instruction::new_with_bytes(
            Pubkey::from_str_const("11111111111111111111111111111111"),
            &transfer,
            vec![
                AccountMeta::new(*owner, true),
                AccountMeta::new(wsol, false),
            ],
        ),
        Instruction::new_with_bytes(TOKEN_PROGRAM_ID, &[17], vec![AccountMeta::new(wsol, false)]),
    ]
}

#[test]
fn a_rival_pool_created_first_does_not_block_graduation() {
    let mut l = sold_out();
    let holder = l.holder.insecure_clone();
    let riff_pool = graduation_accounts(&l.mint).pool;
    let wrap = wrap_sol_ixs(&holder.pubkey(), SOL / 10);
    send_many(&mut l.env.svm, &wrap, &[&holder]).unwrap();

    // 1. Nobody else can create a pool at riff's pool address: Raydium
    //    requires a non-standard pool address to sign, and only riff can
    //    sign for its PDA.
    let ix = raydium_initialize_ix(
        &holder.pubkey(),
        &riff_pool,
        false,
        &l.mint,
        1_000 * TOKEN,
        SOL / 20,
    );
    assert!(send_many(
        &mut l.env.svm,
        &[compute_budget_ix(600_000), ix],
        &[&holder]
    )
    .is_err());

    // 2. But anyone holding the coin can open Raydium's standard pool for
    //    it, at a price of their choosing.
    let (lo, hi) = if l.mint < WSOL_MINT {
        (l.mint, WSOL_MINT)
    } else {
        (WSOL_MINT, l.mint)
    };
    let rival = Pubkey::find_program_address(
        &[
            b"pool",
            RAYDIUM_AMM_CONFIG.as_ref(),
            lo.as_ref(),
            hi.as_ref(),
        ],
        &riff::RAYDIUM_CPMM_PROGRAM_ID,
    )
    .0;
    let ix = raydium_initialize_ix(
        &holder.pubkey(),
        &rival,
        false,
        &l.mint,
        1_000 * TOKEN,
        SOL / 20,
    );
    send_many(
        &mut l.env.svm,
        &[compute_budget_ix(600_000), ix],
        &[&holder],
    )
    .unwrap();
    assert!(l.env.svm.get_account(&rival).is_some(), "rival pool exists");

    // riff's graduation is unaffected: its own pool, its exact liquidity.
    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    let g = graduation_accounts(&l.mint);
    assert_ne!(g.pool, rival);
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_token_vault), RESERVE);
}

#[test]
fn tokens_planted_in_the_graduation_account_do_not_block_graduation() {
    let mut l = sold_out();
    let g = graduation_accounts(&l.mint);
    let holder = l.holder.insecure_clone();
    // An attacker creates the graduation authority's token account and
    // plants one token base unit in it.
    let transfer = anchor_spl::token_2022::spl_token_2022::instruction::transfer_checked(
        &TOKEN_2022_ID,
        &ata_address(&holder.pubkey(), &l.mint),
        &l.mint,
        &g.token_account,
        &holder.pubkey(),
        &[],
        1,
        riff::COIN_DECIMALS,
    )
    .unwrap();
    let ixs = [
        create_ata_for_ix(&holder.pubkey(), &g.authority, &l.mint, &TOKEN_2022_ID),
        transfer,
    ];
    send_many(&mut l.env.svm, &ixs, &[&holder]).unwrap();
    assert_eq!(raw_token_balance(&l.env.svm, &g.token_account), 1);

    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_token_vault), RESERVE);
    assert!(l
        .env
        .svm
        .get_account(&g.token_account)
        .is_none_or(|a| a.lamports == 0));
}

#[test]
fn donations_do_not_change_what_the_pool_receives() {
    let mut l = sold_out();
    let raised = coin(&l).real_sol_reserves;
    let g = graduation_accounts(&l.mint);
    // Extra SOL sent to the coin and the graduation authority, and extra
    // tokens sent to the vault, must not skew the opening price.
    l.env.svm.airdrop(&coin_address(&l.mint), 3 * SOL).unwrap();
    l.env.svm.airdrop(&g.authority, SOL).unwrap();
    let holder = l.holder.insecure_clone();
    let ix = anchor_spl::token_2022::spl_token_2022::instruction::transfer_checked(
        &TOKEN_2022_ID,
        &ata_address(&holder.pubkey(), &l.mint),
        &l.mint,
        &vault_address(&l.mint),
        &holder.pubkey(),
        &[],
        1_000 * TOKEN,
        riff::COIN_DECIMALS,
    )
    .unwrap();
    send(&mut l.env.svm, ix, &[&holder]).unwrap();

    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_wsol_vault), raised);
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_token_vault), RESERVE);
}

#[test]
fn graduation_only_uses_the_configured_fee_tier() {
    let mut l = sold_out();
    let k = keeper(&mut l);
    // A different real Raydium tier (1% instead of the configured 0.25%),
    // and an account Raydium doesn't own at all.
    for wrong in [
        RAYDIUM_AMM_CONFIG_1PCT,
        riff::RAYDIUM_CREATE_POOL_FEE_RECEIVER,
    ] {
        let mut ix = graduate_ix(&k.pubkey(), &l.mint);
        for m in &mut ix.accounts {
            if m.pubkey == RAYDIUM_AMM_CONFIG {
                m.pubkey = wrong;
            }
        }
        let res = send_many(
            &mut l.env.svm,
            &[
                compute_budget_ix(600_000),
                prepare_graduation_ix(&l.mint),
                ix,
            ],
            &[&k],
        );
        assert_riff_error(res, ErrorCode::InvalidAmmConfig);
    }
    assert!(coin(&l).pool.is_none());
}

#[test]
fn trading_on_the_curve_stays_closed_after_graduation() {
    let mut l = sold_out();
    let k = keeper(&mut l);
    graduate(&mut l.env.svm, &k, &l.mint).unwrap();
    let holder = l.holder.insecure_clone();
    let ix = sell_ix(&holder.pubkey(), &l.mint, TOKEN, 0);
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&holder]),
        ErrorCode::CurveComplete,
    );
    let ix = buy_ix(&holder.pubkey(), &l.mint, SOL, 0);
    assert_riff_error(
        send(&mut l.env.svm, ix, &[&holder]),
        ErrorCode::CurveComplete,
    );
}

// ------------------------------------------------------------ two-step flow

fn graduate_after_prepare(l: &mut Launch, k: &Keypair) {
    let ix = graduate_ix(&k.pubkey(), &l.mint);
    send_many(&mut l.env.svm, &[compute_budget_ix(600_000), ix], &[k]).unwrap();
}

#[test]
fn prepare_moves_exactly_the_curve_sol_to_the_graduation_authority() {
    let mut l = sold_out();
    let raised = coin(&l).real_sol_reserves;
    let g = graduation_accounts(&l.mint);
    let k = keeper(&mut l);
    let meta = send(&mut l.env.svm, prepare_graduation_ix(&l.mint), &[&k]).unwrap();

    let c = coin(&l);
    assert_eq!(c.real_sol_reserves, 0);
    assert_eq!(c.graduation_sol, raised);
    assert_eq!(lamports(&l.env.svm, &g.authority), raised);
    assert_eq!(
        events::<riff::events::GraduationPrepared>(&meta.logs)[0].sol_amount,
        raised
    );
    // The coin still backs every fee balance, so payouts keep working.
    let key = coin_address(&l.mint);
    let len = l.env.svm.get_account(&key).unwrap().data.len();
    assert_eq!(
        lamports(&l.env.svm, &key),
        l.env.svm.minimum_balance_for_rent_exemption(len)
            + c.artist_fees
            + c.charity_fees
            + c.creator_fees
            + c.protocol_fees
    );

    // Graduation can finish in a later transaction.
    graduate_after_prepare(&mut l, &k);
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_wsol_vault), raised);
    assert_eq!(coin(&l).graduation_sol, 0);
    assert_eq!(lamports(&l.env.svm, &g.authority), 0);
}

#[test]
fn graduate_requires_prepare() {
    let mut l = sold_out();
    let k = keeper(&mut l);
    let ix = graduate_ix(&k.pubkey(), &l.mint);
    assert_riff_error(
        send_many(&mut l.env.svm, &[compute_budget_ix(600_000), ix], &[&k]),
        ErrorCode::GraduationNotPrepared,
    );
}

#[test]
fn prepare_only_once_and_only_when_sold_out() {
    let mut env = setup();
    let mint = setup_coin(&mut env);
    let payer = env.admin.insecure_clone();
    let alice = trader(&mut env.svm, &mint, 20 * SOL);
    send(
        &mut env.svm,
        buy_ix(&alice.pubkey(), &mint, 5 * SOL, 0),
        &[&payer, &alice],
    )
    .unwrap();
    assert_riff_error(
        send(&mut env.svm, prepare_graduation_ix(&mint), &[&payer]),
        ErrorCode::CurveNotComplete,
    );

    let mut l = sold_out();
    let k = keeper(&mut l);
    send(&mut l.env.svm, prepare_graduation_ix(&l.mint), &[&k]).unwrap();
    assert_riff_error(
        send(&mut l.env.svm, prepare_graduation_ix(&l.mint), &[&k]),
        ErrorCode::GraduationAlreadyPrepared,
    );
    graduate_after_prepare(&mut l, &k);
    assert_riff_error(
        send(&mut l.env.svm, prepare_graduation_ix(&l.mint), &[&k]),
        ErrorCode::AlreadyGraduated,
    );
}

#[test]
fn sol_sent_to_the_authority_between_steps_goes_to_the_caller_not_the_pool() {
    let mut l = sold_out();
    let raised = coin(&l).real_sol_reserves;
    let g = graduation_accounts(&l.mint);
    let k = keeper(&mut l);
    send(&mut l.env.svm, prepare_graduation_ix(&l.mint), &[&k]).unwrap();
    l.env.svm.airdrop(&g.authority, 2 * SOL).unwrap();
    let before = lamports(&l.env.svm, &k.pubkey());

    graduate_after_prepare(&mut l, &k);
    assert_eq!(raw_token_balance(&l.env.svm, &g.pool_wsol_vault), raised);
    assert_eq!(lamports(&l.env.svm, &g.authority), 0);
    // The stray 2 SOL came back to the caller along with the unspent budget.
    assert!(lamports(&l.env.svm, &k.pubkey()) > before + SOL);
}
