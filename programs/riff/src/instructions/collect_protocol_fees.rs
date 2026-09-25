use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    events::ProtocolFeesCollected,
    payout::pay_from_coin,
    state::{Coin, Config},
};

/// Permissionless: anyone may pay the network fee to sweep a coin's protocol
/// fees, but they can only ever go to the configured treasury.
#[derive(Accounts)]
pub struct CollectProtocolFees<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, address = config.treasury)]
    pub treasury: SystemAccount<'info>,
    #[account(mut, seeds = [COIN_SEED, coin.mint.as_ref()], bump = coin.bump)]
    pub coin: Account<'info, Coin>,
}

pub fn handle_collect_protocol_fees(ctx: Context<CollectProtocolFees>) -> Result<()> {
    let coin = &mut ctx.accounts.coin;
    let treasury = ctx.accounts.treasury.to_account_info();
    let amount = coin.protocol_fees;
    require!(amount > 0, ErrorCode::NoFeesToWithdraw);

    // Solana rejects leaving an account with a nonzero balance below the
    // rent-exempt minimum. Check it here too, for a clear error instead of a
    // runtime failure, and so tests don't depend on the runtime enforcing it.
    let balance_after = treasury
        .lamports()
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    require!(
        Rent::get()?.is_exempt(balance_after, treasury.data_len()),
        ErrorCode::TreasuryNotRentExempt
    );

    coin.protocol_fees = 0;
    pay_from_coin(coin, &treasury, amount)?;

    emit!(ProtocolFeesCollected {
        coin: coin.key(),
        treasury: treasury.key(),
        amount,
    });
    Ok(())
}
