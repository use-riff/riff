use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program::invoke_signed,
    },
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::{
        create_idempotent, get_associated_token_address_with_program_id, AssociatedToken, Create,
    },
    token::{self, spl_token::native_mint, Token},
    token_2022::{self, Token2022},
    token_interface::{Mint, TokenAccount},
};

use crate::{
    constants::*,
    error::ErrorCode,
    events::Graduated,
    instructions::trade::release_tokens,
    payout::pay_from_coin,
    state::{Coin, Config},
};

/// Step 2 of 2 of graduation, sent right after `prepare_graduation` in the
/// same transaction. Moves a sold-out coin's liquidity into a Raydium CPMM
/// pool and burns the LP tokens, so the liquidity is locked forever.
///
/// Permissionless. The caller fronts Raydium's pool-creation fee plus a rent
/// allowance; the unspent part is refunded, and the actual cost is
/// reimbursed from the coin's protocol fees as far as they go.
///
/// The pool opens at exactly the curve's final price: it receives exactly
/// the SOL the curve raised and exactly the graduation reserve, both taken
/// from tracked amounts, never from account balances (which anyone can
/// inflate by sending SOL or tokens).
#[derive(Accounts)]
pub struct Graduate<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(
        mut,
        seeds = [COIN_SEED, mint.key().as_ref()],
        bump = coin.bump,
        has_one = mint,
        has_one = vault,
    )]
    pub coin: Box<Account<'info, Coin>>,
    /// Writable only for burning tokens someone planted in the graduation
    /// account (burning lowers the mint's supply).
    #[account(mut)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    /// Signs for riff as the Raydium pool creator. Holds SOL only during
    /// this instruction.
    #[account(mut, seeds = [GRADUATION_SEED, mint.key().as_ref()], bump)]
    pub graduation_authority: SystemAccount<'info>,
    /// CHECK: graduation authority's token account for the coin; created,
    /// emptied and closed here. Address enforced.
    #[account(
        mut,
        address = get_associated_token_address_with_program_id(
            &graduation_authority.key(), &mint.key(), &token_2022_program.key()),
    )]
    pub graduation_token_account: UncheckedAccount<'info>,
    /// CHECK: graduation authority's wrapped-SOL account; created, emptied
    /// and closed here. Address enforced.
    #[account(
        mut,
        address = get_associated_token_address_with_program_id(
            &graduation_authority.key(), &native_mint::ID, &token_program.key()),
    )]
    pub graduation_wsol_account: UncheckedAccount<'info>,
    /// CHECK: graduation authority's LP token account; Raydium creates it,
    /// its LP tokens are burned and it's closed here. Address enforced.
    #[account(
        mut,
        address = get_associated_token_address_with_program_id(
            &graduation_authority.key(), &lp_mint.key(), &token_program.key()),
    )]
    pub graduation_lp_account: UncheckedAccount<'info>,
    /// CHECK: the pool's address, a riff PDA that signs Raydium's pool
    /// creation. Raydium creates and owns the account.
    #[account(mut, seeds = [POOL_SEED, mint.key().as_ref()], bump)]
    pub pool_state: UncheckedAccount<'info>,
    /// CHECK: fixed program ID, so riff never CPIs anywhere else.
    #[account(address = RAYDIUM_CPMM_PROGRAM_ID, executable)]
    pub raydium_program: UncheckedAccount<'info>,
    /// CHECK: the configured Raydium fee tier, owned by Raydium.
    #[account(
        address = config.raydium_amm_config @ ErrorCode::InvalidAmmConfig,
        owner = RAYDIUM_CPMM_PROGRAM_ID @ ErrorCode::InvalidAmmConfig,
    )]
    pub amm_config: UncheckedAccount<'info>,
    /// CHECK: Raydium's vault and LP-mint authority PDA.
    #[account(seeds = [RAYDIUM_AUTH_SEED], bump, seeds::program = RAYDIUM_CPMM_PROGRAM_ID)]
    pub raydium_authority: UncheckedAccount<'info>,
    /// CHECK: Raydium creates it; PDA of Raydium over the pool address.
    #[account(
        mut,
        seeds = [RAYDIUM_POOL_LP_MINT_SEED, pool_state.key().as_ref()],
        bump,
        seeds::program = RAYDIUM_CPMM_PROGRAM_ID,
    )]
    pub lp_mint: UncheckedAccount<'info>,
    /// CHECK: Raydium creates it; the pool's vault for the coin.
    #[account(
        mut,
        seeds = [RAYDIUM_POOL_VAULT_SEED, pool_state.key().as_ref(), mint.key().as_ref()],
        bump,
        seeds::program = RAYDIUM_CPMM_PROGRAM_ID,
    )]
    pub pool_token_vault: UncheckedAccount<'info>,
    /// CHECK: Raydium creates it; the pool's vault for wrapped SOL.
    #[account(
        mut,
        seeds = [RAYDIUM_POOL_VAULT_SEED, pool_state.key().as_ref(), native_mint::ID.as_ref()],
        bump,
        seeds::program = RAYDIUM_CPMM_PROGRAM_ID,
    )]
    pub pool_wsol_vault: UncheckedAccount<'info>,
    /// CHECK: Raydium creates it; the pool's price oracle.
    #[account(
        mut,
        seeds = [RAYDIUM_OBSERVATION_SEED, pool_state.key().as_ref()],
        bump,
        seeds::program = RAYDIUM_CPMM_PROGRAM_ID,
    )]
    pub observation_state: UncheckedAccount<'info>,
    /// CHECK: Raydium's pool-creation fee receiver.
    #[account(mut, address = RAYDIUM_CREATE_POOL_FEE_RECEIVER)]
    pub create_pool_fee_receiver: UncheckedAccount<'info>,
    #[account(address = native_mint::ID)]
    pub wsol_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_graduate(ctx: Context<Graduate>) -> Result<()> {
    let a = &mut *ctx.accounts;
    require!(a.coin.complete, ErrorCode::CurveNotComplete);
    require!(a.coin.pool.is_none(), ErrorCode::AlreadyGraduated);
    // prepare_graduation (earlier in the same transaction) moved the curve's
    // SOL to the graduation authority and recorded the exact amount.
    let sol_amount = a.coin.graduation_sol;
    require!(sol_amount > 0, ErrorCode::GraduationNotPrepared);
    let token_amount = COIN_TOTAL_SUPPLY
        .checked_sub(a.config.curve_token_supply)
        .ok_or(ErrorCode::MathOverflow)?;
    require!(a.vault.amount >= token_amount, ErrorCode::ReserveMissing);

    let mint_key = a.mint.key();
    let grad_seeds: &[&[u8]] = &[
        GRADUATION_SEED,
        mint_key.as_ref(),
        &[ctx.bumps.graduation_authority],
    ];
    let pool_seeds: &[&[u8]] = &[POOL_SEED, mint_key.as_ref(), &[ctx.bumps.pool_state]];
    let grad_info = a.graduation_authority.to_account_info();

    // 1. The caller fronts the cost of creating the pool.
    let budget = raydium_create_pool_fee(&a.amm_config)?
        .checked_add(GRADUATION_RENT_ALLOWANCE)
        .ok_or(ErrorCode::MathOverflow)?;
    // What the authority held besides the curve's SOL (normally nothing;
    // anything someone sent it is refunded to the caller with the rest).
    let grad_start = grad_info
        .lamports()
        .checked_sub(sol_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    transfer(
        CpiContext::new(
            a.system_program.key(),
            Transfer {
                from: a.payer.to_account_info(),
                to: grad_info.clone(),
            },
        ),
        budget,
    )?;

    // 2. Temporary token accounts for the graduation authority.
    for (account, mint, token_program) in [
        (
            a.graduation_token_account.to_account_info(),
            a.mint.to_account_info(),
            a.token_2022_program.to_account_info(),
        ),
        (
            a.graduation_wsol_account.to_account_info(),
            a.wsol_mint.to_account_info(),
            a.token_program.to_account_info(),
        ),
    ] {
        create_idempotent(CpiContext::new_with_signer(
            a.associated_token_program.key(),
            Create {
                payer: grad_info.clone(),
                associated_token: account,
                authority: grad_info.clone(),
                mint,
                system_program: a.system_program.to_account_info(),
                token_program,
            },
            &[grad_seeds],
        ))?;
    }

    // 3. Exactly the curve's SOL, wrapped, and exactly the reserve.
    transfer(
        CpiContext::new_with_signer(
            a.system_program.key(),
            Transfer {
                from: grad_info.clone(),
                to: a.graduation_wsol_account.to_account_info(),
            },
            &[grad_seeds],
        ),
        sol_amount,
    )?;
    token::sync_native(CpiContext::new(
        a.token_program.key(),
        token::SyncNative {
            account: a.graduation_wsol_account.to_account_info(),
        },
    ))?;
    release_tokens(
        &a.token_2022_program,
        &a.vault,
        &a.mint,
        &a.graduation_token_account.to_account_info(),
        &a.coin,
        token_amount,
    )?;

    // 4. Create the Raydium pool. Raydium orders the two mints by address.
    let coin_first = mint_key < native_mint::ID;
    let (amount_0, amount_1) = if coin_first {
        (token_amount, sol_amount)
    } else {
        (sol_amount, token_amount)
    };
    let mut data = RAYDIUM_INITIALIZE_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&amount_0.to_le_bytes());
    data.extend_from_slice(&amount_1.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes()); // open_time: as soon as possible
    let coin_side = (
        a.mint.to_account_info(),
        a.graduation_token_account.to_account_info(),
        a.pool_token_vault.to_account_info(),
        a.token_2022_program.to_account_info(),
    );
    let sol_side = (
        a.wsol_mint.to_account_info(),
        a.graduation_wsol_account.to_account_info(),
        a.pool_wsol_vault.to_account_info(),
        a.token_program.to_account_info(),
    );
    let (side_0, side_1) = if coin_first {
        (coin_side, sol_side)
    } else {
        (sol_side, coin_side)
    };
    let infos = [
        grad_info.clone(),
        a.amm_config.to_account_info(),
        a.raydium_authority.to_account_info(),
        a.pool_state.to_account_info(),
        side_0.0.clone(),
        side_1.0.clone(),
        a.lp_mint.to_account_info(),
        side_0.1.clone(),
        side_1.1.clone(),
        a.graduation_lp_account.to_account_info(),
        side_0.2.clone(),
        side_1.2.clone(),
        a.create_pool_fee_receiver.to_account_info(),
        a.observation_state.to_account_info(),
        a.token_program.to_account_info(),
        side_0.3.clone(),
        side_1.3.clone(),
        a.associated_token_program.to_account_info(),
        a.system_program.to_account_info(),
        a.rent.to_account_info(),
    ];
    // Writability and signers exactly as Raydium's `Initialize` declares.
    let writable = [
        true, false, false, true, false, false, true, true, true, true, true, true, true, true,
        false, false, false, false, false, false,
    ];
    let accounts = infos
        .iter()
        .zip(writable)
        .enumerate()
        .map(|(i, (info, w))| {
            let signer = i == 0 || i == 3; // creator and pool address
            if w {
                AccountMeta::new(info.key(), signer)
            } else {
                AccountMeta::new_readonly(info.key(), signer)
            }
        })
        .collect();
    let mut cpi_infos = infos.to_vec();
    cpi_infos.push(a.raydium_program.to_account_info());
    invoke_signed(
        &Instruction {
            program_id: RAYDIUM_CPMM_PROGRAM_ID,
            accounts,
            data,
        },
        &cpi_infos,
        &[grad_seeds, pool_seeds],
    )?;

    // 5. Burn every LP token: nobody, riff included, can pull the liquidity.
    let lp_burned = token_balance(&a.graduation_lp_account)?;
    token::burn(
        CpiContext::new_with_signer(
            a.token_program.key(),
            token::Burn {
                mint: a.lp_mint.to_account_info(),
                from: a.graduation_lp_account.to_account_info(),
                authority: grad_info.clone(),
            },
            &[grad_seeds],
        ),
        lp_burned,
    )?;

    // 6. Close the temporary accounts, returning their rent. Tokens someone
    // sent to the graduation account beforehand would block closing it, so
    // burn any such leftovers first.
    let stray_tokens = token_balance(&a.graduation_token_account)?;
    if stray_tokens > 0 {
        token_2022::burn(
            CpiContext::new_with_signer(
                a.token_2022_program.key(),
                token_2022::Burn {
                    mint: a.mint.to_account_info(),
                    from: a.graduation_token_account.to_account_info(),
                    authority: grad_info.clone(),
                },
                &[grad_seeds],
            ),
            stray_tokens,
        )?;
    }
    token_2022::close_account(CpiContext::new_with_signer(
        a.token_2022_program.key(),
        token_2022::CloseAccount {
            account: a.graduation_token_account.to_account_info(),
            destination: grad_info.clone(),
            authority: grad_info.clone(),
        },
        &[grad_seeds],
    ))?;
    for account in [
        a.graduation_wsol_account.to_account_info(),
        a.graduation_lp_account.to_account_info(),
    ] {
        token::close_account(CpiContext::new_with_signer(
            a.token_program.key(),
            token::CloseAccount {
                account,
                destination: grad_info.clone(),
                authority: grad_info.clone(),
            },
            &[grad_seeds],
        ))?;
    }

    // 7. Refund what's left to the caller and reimburse the actual cost
    // from the coin's protocol fees, as far as they go.
    let leftover = grad_info.lamports();
    transfer(
        CpiContext::new_with_signer(
            a.system_program.key(),
            Transfer {
                from: grad_info.clone(),
                to: a.payer.to_account_info(),
            },
            &[grad_seeds],
        ),
        leftover,
    )?;
    let cost = grad_start
        .checked_add(budget)
        .and_then(|v| v.checked_sub(leftover))
        .unwrap_or(0);
    let cost_reimbursed = cost.min(a.coin.protocol_fees);
    if cost_reimbursed > 0 {
        a.coin.protocol_fees -= cost_reimbursed;
        pay_from_coin(&a.coin, &a.payer.to_account_info(), cost_reimbursed)?;
    }

    a.coin.graduation_sol = 0;
    a.coin.pool = Some(a.pool_state.key());
    emit!(Graduated {
        coin: a.coin.key(),
        mint: mint_key,
        pool: a.pool_state.key(),
        sol_amount,
        token_amount,
        lp_burned,
        cost_reimbursed,
    });
    Ok(())
}

/// Reads `create_pool_fee` from Raydium's `AmmConfig` account (already
/// checked to be owned by Raydium and to be the configured fee tier).
fn raydium_create_pool_fee(amm_config: &AccountInfo) -> Result<u64> {
    let data = amm_config.try_borrow_data()?;
    let bytes = data
        .get(
            RAYDIUM_AMM_CONFIG_CREATE_POOL_FEE_OFFSET
                ..RAYDIUM_AMM_CONFIG_CREATE_POOL_FEE_OFFSET + 8,
        )
        .ok_or(ErrorCode::InvalidAmmConfig)?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

/// Token amount of an SPL / Token-2022 account (the amount field is at the
/// same place in both).
fn token_balance(account: &AccountInfo) -> Result<u64> {
    let data = account.try_borrow_data()?;
    let bytes = data.get(64..72).ok_or(ErrorCode::MathOverflow)?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}
