use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    constants::*, error::ErrorCode, events::GraduationPrepared, payout::pay_from_coin, state::Coin,
};

/// Step 1 of 2 of graduation, sent in the same transaction as `graduate`.
///
/// Moves the curve's SOL from the coin account to the graduation authority.
/// The coin account holds data, so its lamports can only be moved directly,
/// and Solana only accepts a direct move if no cross-program call follows it
/// in the same instruction without both accounts. Doing it here, as this
/// instruction's last action, lets `graduate` use ordinary calls throughout.
///
/// Permissionless. Only riff can sign for the graduation authority, and
/// `graduate` uses the tracked amount, never the authority's balance.
#[derive(Accounts)]
pub struct PrepareGraduation<'info> {
    #[account(
        mut,
        seeds = [COIN_SEED, mint.key().as_ref()],
        bump = coin.bump,
        has_one = mint,
    )]
    pub coin: Account<'info, Coin>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut, seeds = [GRADUATION_SEED, mint.key().as_ref()], bump)]
    pub graduation_authority: SystemAccount<'info>,
}

pub fn handle_prepare_graduation(ctx: Context<PrepareGraduation>) -> Result<()> {
    let coin = &mut ctx.accounts.coin;
    require!(coin.complete, ErrorCode::CurveNotComplete);
    require!(coin.pool.is_none(), ErrorCode::AlreadyGraduated);
    require!(
        coin.graduation_sol == 0,
        ErrorCode::GraduationAlreadyPrepared
    );

    let sol_amount = coin.real_sol_reserves;
    coin.real_sol_reserves = 0;
    coin.graduation_sol = sol_amount;
    pay_from_coin(
        coin,
        &ctx.accounts.graduation_authority.to_account_info(),
        sol_amount,
    )?;

    emit!(GraduationPrepared {
        coin: coin.key(),
        mint: coin.mint,
        sol_amount,
    });
    Ok(())
}
