use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::VaultWithdrawn,
    instructions::passport::require_passkey,
    passkey::{PasskeyAction, PasskeyProof},
    state::{Passport, PassportConfig, Vault},
};

#[derive(Accounts)]
pub struct WithdrawVault<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(
        mut,
        seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()],
        bump = passport.bump,
        constraint = passport.wallet == wallet.key() @ PassportError::NotPassportWallet,
        constraint = !passport.revoked @ PassportError::Revoked,
    )]
    pub passport: Account<'info, Passport>,
    #[account(mut, seeds = [VAULT_SEED, passport.key().as_ref()], bump = passport.vault_bump)]
    pub vault: Account<'info, Vault>,
    /// CHECK: the instructions sysvar, for the passkey signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// Pays the passport's wallet from the vault. A small daily amount needs
/// only the wallet; anything more needs the passkey too.
pub fn handle_withdraw(
    mut ctx: Context<WithdrawVault>,
    amount: u64,
    passkey_proof: Option<PasskeyProof>,
) -> Result<()> {
    require!(amount > 0, PassportError::ZeroAmount);
    let now = Clock::get()?.unix_timestamp;
    let accounts = &mut ctx.accounts;
    accounts.passport.require_settled(&accounts.config, now)?;
    let vault_info = accounts.vault.to_account_info();
    let keep = Rent::get()?.minimum_balance(vault_info.data_len());
    let available = vault_info.lamports().saturating_sub(keep);
    require!(amount <= available, PassportError::InsufficientVault);

    let passport = &mut accounts.passport;
    if now - passport.withdraw_window_start >= WITHDRAW_WINDOW_SECS {
        passport.withdraw_window_start = now;
        passport.withdrawn_in_window = 0;
    }
    let spent = passport
        .withdrawn_in_window
        .checked_add(amount)
        .ok_or(error!(PassportError::MathOverflow))?;
    // While a recovery is pending, the wallet may be the reason for it (lost
    // or stolen), so even the daily amount needs the passkey.
    let with_passkey = spent > accounts.config.free_withdraw_per_day || passport.recovery.is_some();
    if with_passkey {
        let proof = passkey_proof.ok_or(error!(PassportError::PasskeyRequired))?;
        let action = PasskeyAction::Withdraw {
            to: accounts.wallet.key(),
            amount,
        };
        require_passkey(
            passport,
            &accounts.config,
            &accounts.instructions,
            &action,
            &proof,
        )?;
    }
    passport.withdrawn_in_window = spent;

    **vault_info.try_borrow_mut_lamports()? -= amount;
    **accounts
        .wallet
        .to_account_info()
        .try_borrow_mut_lamports()? += amount;
    emit!(VaultWithdrawn {
        passport: passport.key(),
        to: accounts.wallet.key(),
        amount,
        with_passkey
    });
    Ok(())
}
