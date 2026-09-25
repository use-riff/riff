use anchor_lang::prelude::*;

use crate::{
    constants::*, error::ErrorCode, events::CreatorFeesWithdrawn, payout::pay_from_coin,
    state::Coin,
};

#[derive(Accounts)]
pub struct WithdrawCreatorFees<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        seeds = [COIN_SEED, coin.mint.as_ref()],
        bump = coin.bump,
        has_one = creator,
    )]
    pub coin: Account<'info, Coin>,
}

/// Pays the coin's creator every creator fee accrued so far.
pub fn handle_withdraw_creator_fees(ctx: Context<WithdrawCreatorFees>) -> Result<()> {
    let coin = &mut ctx.accounts.coin;
    let amount = coin.creator_fees;
    require!(amount > 0, ErrorCode::NoFeesToWithdraw);
    coin.creator_fees = 0;

    pay_from_coin(coin, &ctx.accounts.creator.to_account_info(), amount)?;

    emit!(CreatorFeesWithdrawn {
        coin: coin.key(),
        creator: ctx.accounts.creator.key(),
        amount,
    });
    Ok(())
}
