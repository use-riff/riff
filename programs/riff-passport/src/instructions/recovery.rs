use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::{RecoveryFinalized, RecoveryRequested, RecoveryVetoed},
    instructions::shared::{fresh_proofs, kinds},
    passkey::{self, PasskeyAction, PasskeyProof},
    state::{proofs_suffice, Passkey, Passport, PassportConfig, ProofRecord, Recovery},
};

#[derive(Accounts)]
pub struct RequestRecovery<'info> {
    pub new_wallet: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(
        mut,
        seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()],
        bump = passport.bump,
        constraint = !passport.revoked @ PassportError::Revoked,
    )]
    pub passport: Account<'info, Passport>,
    /// CHECK: the instructions sysvar, for the new passkey's signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    // remaining_accounts: the new wallet's fresh proof records.
}

/// A lost wallet or passkey: prove it again with fresh proofs from a new
/// wallet and a new passkey. The change waits out a public time-lock.
pub fn handle_request_recovery<'info>(
    ctx: Context<'info, RequestRecovery<'info>>,
    new_passkey: Passkey,
    passkey_proof: PasskeyProof,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let config = &ctx.accounts.config;
    let new_wallet = ctx.accounts.new_wallet.key();
    let passport = &mut ctx.accounts.passport;
    require!(passport.recovery.is_none(), PassportError::RecoveryPending);
    let records = fresh_proofs(
        ctx.remaining_accounts,
        &passport.artist_id.clone(),
        &new_wallet,
        now,
        config.proof_max_age,
    )?;
    require!(
        proofs_suffice(&kinds(&records)),
        PassportError::NotEnoughProofs
    );

    let expected = passkey::challenge(
        &passport.key(),
        passport.nonce,
        &PasskeyAction::Register {
            passkey: new_passkey,
        },
    )?;
    passkey::verify(
        &ctx.accounts.instructions,
        &new_passkey,
        &config.rp_id_hash,
        &expected,
        &passkey_proof,
    )?;
    passport.nonce = passport
        .nonce
        .checked_add(1)
        .ok_or(error!(PassportError::MathOverflow))?;

    let effective_at = now
        .checked_add(config.recovery_delay)
        .ok_or(error!(PassportError::MathOverflow))?;
    passport.recovery = Some(Recovery {
        new_wallet,
        new_passkey,
        requested_at: now,
        effective_at,
    });
    emit!(RecoveryRequested {
        passport: passport.key(),
        new_wallet,
        effective_at
    });
    Ok(())
}

#[derive(Accounts)]
pub struct VetoRecovery<'info> {
    pub vetoer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(mut, seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()], bump = passport.bump)]
    pub passport: Account<'info, Passport>,
    /// CHECK: the instructions sysvar, for the passkey signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    // remaining_accounts[0], without a passkey: a guardian proof record.
}

/// Cancels a pending recovery. Either the current passkey approves, or a
/// guardian does: a proof of a kind already on the passport, freshly made
/// (after the recovery was requested) by the wallet signing this.
/// A stolen wallet alone can't veto.
pub fn handle_veto_recovery<'info>(
    ctx: Context<'info, VetoRecovery<'info>>,
    passkey_proof: Option<PasskeyProof>,
) -> Result<()> {
    let passport = &mut ctx.accounts.passport;
    let recovery = passport.recovery.ok_or(error!(PassportError::NoRecovery))?;
    let mut by_proof = None;
    match passkey_proof {
        Some(proof) => {
            let expected = passkey::challenge(
                &passport.key(),
                passport.nonce,
                &PasskeyAction::Veto {
                    new_wallet: recovery.new_wallet,
                },
            )?;
            passkey::verify(
                &ctx.accounts.instructions,
                &passport.passkey,
                &ctx.accounts.config.rp_id_hash,
                &expected,
                &proof,
            )?;
        }
        None => {
            let info = ctx
                .remaining_accounts
                .first()
                .ok_or(error!(PassportError::NotAGuardian))?;
            let record = Account::<ProofRecord>::try_from(info)?;
            require!(
                record.artist_id == passport.artist_id
                    && record.wallet == ctx.accounts.vetoer.key()
                    && record.verified_at >= recovery.requested_at
                    && passport.has_kind(record.kind),
                PassportError::NotAGuardian
            );
            by_proof = Some(record.kind);
        }
    }
    passport.nonce = passport
        .nonce
        .checked_add(1)
        .ok_or(error!(PassportError::MathOverflow))?;
    passport.recovery = None;
    emit!(RecoveryVetoed {
        passport: passport.key(),
        by_passkey: by_proof.is_none(),
        by_proof
    });
    Ok(())
}

#[derive(Accounts)]
pub struct FinalizeRecovery<'info> {
    #[account(
        mut,
        seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()],
        bump = passport.bump,
        constraint = !passport.revoked @ PassportError::Revoked,
    )]
    pub passport: Account<'info, Passport>,
}

/// Anyone may finish a recovery once its time-lock has passed without a veto.
pub fn handle_finalize_recovery(ctx: Context<FinalizeRecovery>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let passport = &mut ctx.accounts.passport;
    let recovery = passport.recovery.ok_or(error!(PassportError::NoRecovery))?;
    require!(now >= recovery.effective_at, PassportError::RecoveryLocked);
    passport.wallet = recovery.new_wallet;
    passport.passkey = recovery.new_passkey;
    passport.recovery = None;
    passport.withdraw_window_start = now;
    passport.withdrawn_in_window = 0;
    passport.nonce = passport
        .nonce
        .checked_add(1)
        .ok_or(error!(PassportError::MathOverflow))?;
    emit!(RecoveryFinalized {
        passport: passport.key(),
        new_wallet: recovery.new_wallet
    });
    Ok(())
}
