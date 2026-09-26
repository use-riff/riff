use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    events::CharityFeesSwept,
    payout::{pay_from_coin, require_rent_exempt_after},
    state::{Coin, Config},
};

/// Permissionless: anyone may pay the network fee to sweep a coin's charity
/// fees, but they can only ever go to the configured charity wallet.
#[derive(Accounts)]
pub struct SweepCharityFees<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, address = config.charity)]
    pub charity: SystemAccount<'info>,
    #[account(mut, seeds = [COIN_SEED, coin.mint.as_ref()], bump = coin.bump)]
    pub coin: Account<'info, Coin>,
}

pub fn handle_sweep_charity_fees(ctx: Context<SweepCharityFees>) -> Result<()> {
    let coin = &mut ctx.accounts.coin;
    let charity = ctx.accounts.charity.to_account_info();

    // If the claim window closed unclaimed, fees held for the artist are the
    // charity's now too.
    coin.forfeit_unclaimed_artist_fees(Clock::get()?.unix_timestamp)
        .ok_or(ErrorCode::MathOverflow)?;
    let amount = coin.charity_fees;
    require!(amount > 0, ErrorCode::NoFeesToWithdraw);
    require_rent_exempt_after(&charity, amount, ErrorCode::CharityNotRentExempt)?;

    coin.charity_fees = 0;
    pay_from_coin(coin, &charity, amount)?;

    emit!(CharityFeesSwept {
        coin: coin.key(),
        charity: charity.key(),
        amount,
    });
    Ok(())
}
