//! The only way SOL leaves a coin account.
//!
//! A coin account holds its own rent, the curve's SOL, and three fee
//! balances. Every payout comes out of exactly one of those tracked balances,
//! and afterwards the account must still hold all of them. If a bug ever let
//! a payout take more than its balance, the transaction fails instead of
//! eating into another party's SOL or the account's rent.

use anchor_lang::prelude::*;

use crate::{error::ErrorCode, state::Coin};

impl Coin {
    /// Lamports the coin account must hold: rent plus everything it owes.
    /// Extra lamports (e.g. someone sent SOL directly) are allowed.
    pub fn required_lamports(&self, data_len: usize) -> Result<u64> {
        Rent::get()?
            .minimum_balance(data_len)
            .checked_add(self.real_sol_reserves)
            .and_then(|v| v.checked_add(self.artist_fees))
            .and_then(|v| v.checked_add(self.charity_fees))
            .and_then(|v| v.checked_add(self.creator_fees))
            .and_then(|v| v.checked_add(self.protocol_fees))
            .ok_or_else(|| ErrorCode::MathOverflow.into())
    }
}

/// Pays `amount` lamports from the coin account to `to`.
///
/// The caller must already have deducted `amount` from the tracked balance
/// it's paid from (a fee balance, or the curve's SOL on a sell).
pub(crate) fn pay_from_coin(coin: &Account<Coin>, to: &AccountInfo, amount: u64) -> Result<()> {
    // Paying the coin to itself would write its balance twice.
    require_keys_neq!(coin.key(), to.key(), ErrorCode::InvalidPayoutRecipient);
    let from = coin.to_account_info();
    let remaining = from
        .lamports()
        .checked_sub(amount)
        .ok_or(ErrorCode::CoinUnderfunded)?;
    let received = to
        .lamports()
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    **from.try_borrow_mut_lamports()? = remaining;
    **to.try_borrow_mut_lamports()? = received;

    require!(
        remaining >= coin.required_lamports(from.data_len())?,
        ErrorCode::CoinUnderfunded
    );
    Ok(())
}

/// Solana rejects leaving an account with a nonzero balance below its
/// rent-exempt minimum. Check it up front for a clear error, and so tests
/// don't depend on the runtime enforcing it (LiteSVM doesn't for accounts
/// without data).
pub(crate) fn require_rent_exempt_after(
    recipient: &AccountInfo,
    amount: u64,
    error: ErrorCode,
) -> Result<()> {
    let balance_after = recipient
        .lamports()
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    if !Rent::get()?.is_exempt(balance_after, recipient.data_len()) {
        return Err(error.into());
    }
    Ok(())
}
