use anchor_lang::prelude::*;
use riff::program::Riff;

use crate::{
    constants::*,
    error::PassportError,
    events::{CoinClaimed, VaultWithdrawn},
    instructions::passport::require_passkey,
    passkey::{PasskeyAction, PasskeyProof},
    state::{Passport, PassportConfig, Vault},
};

#[derive(Accounts)]
pub struct ClaimCoin<'info> {
    pub wallet: Signer<'info>,
    /// riff's verifier, co-signing as riff's `claim_artist` requires.
    pub verifier: Signer<'info>,
    #[account(
        seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()],
        bump = passport.bump,
        constraint = passport.wallet == wallet.key() @ PassportError::NotPassportWallet,
        constraint = !passport.revoked @ PassportError::Revoked,
    )]
    pub passport: Account<'info, Passport>,
    #[account(seeds = [VAULT_SEED, passport.key().as_ref()], bump = passport.vault_bump)]
    pub vault: Account<'info, Vault>,
    /// CHECK: riff's config, checked by riff.
    pub riff_config: UncheckedAccount<'info>,
    /// CHECK: the riff coin, checked by riff (including its artist ID).
    #[account(mut)]
    pub coin: UncheckedAccount<'info>,
    pub riff_program: Program<'info, Riff>,
}

/// Claims a riff coin for the passport: the vault becomes the coin's artist,
/// so its fees belong to the passport, not to any one wallet.
pub fn handle_claim_coin(ctx: Context<ClaimCoin>) -> Result<()> {
    let passport_key = ctx.accounts.passport.key();
    let seeds: &[&[u8]] = &[
        VAULT_SEED,
        passport_key.as_ref(),
        &[ctx.accounts.passport.vault_bump],
    ];
    riff::cpi::claim_artist(
        CpiContext::new_with_signer(
            ctx.accounts.riff_program.key(),
            riff::cpi::accounts::ClaimArtist {
                artist: ctx.accounts.vault.to_account_info(),
                verifier: ctx.accounts.verifier.to_account_info(),
                config: ctx.accounts.riff_config.to_account_info(),
                coin: ctx.accounts.coin.to_account_info(),
            },
            &[seeds],
        ),
        ctx.accounts.passport.artist_id.clone(),
    )?;
    emit!(CoinClaimed {
        passport: passport_key,
        mint: coin_mint(&ctx.accounts.coin)?
    });
    Ok(())
}

/// The mint of a riff coin account (its first field after the discriminator).
fn coin_mint(coin: &AccountInfo) -> Result<Pubkey> {
    let data = coin.try_borrow_data()?;
    let bytes: [u8; 32] = data
        .get(8..40)
        .and_then(|b| b.try_into().ok())
        .ok_or(error!(PassportError::MathOverflow))?;
    Ok(Pubkey::new_from_array(bytes))
}

#[derive(Accounts)]
pub struct CollectFees<'info> {
    #[account(seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()], bump = passport.bump)]
    pub passport: Account<'info, Passport>,
    #[account(mut, seeds = [VAULT_SEED, passport.key().as_ref()], bump = passport.vault_bump)]
    pub vault: Account<'info, Vault>,
    /// CHECK: the riff coin, checked by riff (its artist must be this vault).
    #[account(mut)]
    pub coin: UncheckedAccount<'info>,
    pub riff_program: Program<'info, Riff>,
}

/// Moves a claimed coin's artist fees into the vault. Anyone may call it:
/// the money can only go to the vault.
pub fn handle_collect_fees(ctx: Context<CollectFees>) -> Result<()> {
    let passport_key = ctx.accounts.passport.key();
    let seeds: &[&[u8]] = &[
        VAULT_SEED,
        passport_key.as_ref(),
        &[ctx.accounts.passport.vault_bump],
    ];
    riff::cpi::withdraw_artist_fees(CpiContext::new_with_signer(
        ctx.accounts.riff_program.key(),
        riff::cpi::accounts::WithdrawArtistFees {
            artist: ctx.accounts.vault.to_account_info(),
            coin: ctx.accounts.coin.to_account_info(),
        },
        &[seeds],
    ))
}

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
    let with_passkey = spent > accounts.config.free_withdraw_per_day;
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
