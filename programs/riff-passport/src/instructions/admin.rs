use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::{PassportReissued, PassportRevoked},
    instructions::shared::{fresh_proofs, kinds},
    passkey::{self, PasskeyAction, PasskeyProof},
    program::RiffPassport,
    state::{proofs_suffice, Passkey, Passport, PassportConfig, ProofSummary},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PassportConfigParams {
    pub verifier: Pubkey,
    pub rp_id_hash: [u8; 32],
    pub recovery_delay: i64,
    pub free_withdraw_per_day: u64,
    pub proof_max_age: i64,
}

impl PassportConfigParams {
    fn apply(self, config: &mut PassportConfig) -> Result<()> {
        require!(
            self.recovery_delay > 0 && self.proof_max_age > 0,
            PassportError::InvalidConfig
        );
        config.verifier = self.verifier;
        config.rp_id_hash = self.rp_id_hash;
        config.recovery_delay = self.recovery_delay;
        config.free_withdraw_per_day = self.free_withdraw_per_day;
        config.proof_max_age = self.proof_max_age;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Must be the program's upgrade authority, so nobody can front-run the config.
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + PassportConfig::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, RiffPassport>,
    #[account(
        constraint = program_data.upgrade_authority_address == Some(admin.key())
            @ PassportError::NotUpgradeAuthority
    )]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_config(
    ctx: Context<InitializeConfig>,
    params: PassportConfigParams,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.bump = ctx.bumps.config;
    params.apply(config)
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ PassportError::NotAdmin)]
    pub config: Account<'info, PassportConfig>,
}

pub fn handle_update_config(
    ctx: Context<UpdateConfig>,
    params: PassportConfigParams,
) -> Result<()> {
    params.apply(&mut ctx.accounts.config)
}

#[derive(Accounts)]
pub struct RevokePassport<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ PassportError::NotAdmin)]
    pub config: Account<'info, PassportConfig>,
    #[account(mut, seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()], bump = passport.bump)]
    pub passport: Account<'info, Passport>,
}

/// For a compromised or wrongly issued passport. Public: the reason is logged.
pub fn handle_revoke_passport(ctx: Context<RevokePassport>, reason: String) -> Result<()> {
    require!(reason.len() <= 200, PassportError::InvalidConfig);
    ctx.accounts.passport.revoked = true;
    emit!(PassportRevoked {
        passport: ctx.accounts.passport.key(),
        reason
    });
    Ok(())
}

#[derive(Accounts)]
pub struct ReissuePassport<'info> {
    pub admin: Signer<'info>,
    /// The artist's new wallet, with fresh proofs of its own.
    pub wallet: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ PassportError::NotAdmin)]
    pub config: Account<'info, PassportConfig>,
    #[account(
        mut,
        seeds = [PASSPORT_SEED, passport.artist_id.as_bytes()],
        bump = passport.bump,
        constraint = passport.revoked @ PassportError::NotRevoked,
    )]
    pub passport: Account<'info, Passport>,
    /// CHECK: the instructions sysvar, for the passkey signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    // remaining_accounts: the new wallet's proof records for this artist.
}

/// Gives a revoked passport to the real artist, for when someone else got it
/// first (for example from a hacked inbox). Needs riff's admin (a multisig
/// with a time-lock on mainnet) and the artist: enough fresh proofs from
/// their wallet, and a new passkey. The vault stays, so what was claimed
/// into it is the artist's; the passport starts a new probation.
pub fn handle_reissue_passport<'info>(
    ctx: Context<'info, ReissuePassport<'info>>,
    passkey: Passkey,
    passkey_proof: PasskeyProof,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let config = &ctx.accounts.config;
    let wallet = ctx.accounts.wallet.key();
    let passport = &mut ctx.accounts.passport;
    let records = fresh_proofs(
        ctx.remaining_accounts,
        &passport.artist_id.clone(),
        &wallet,
        now,
        config.proof_max_age,
    )?;
    require!(
        proofs_suffice(&kinds(&records)),
        PassportError::NotEnoughProofs
    );

    // The nonce has moved on since the first registration, so an old
    // registration signature can't be replayed here.
    let expected = passkey::challenge(
        &passport.key(),
        passport.nonce,
        &PasskeyAction::Register { passkey },
    )?;
    passkey::verify(
        &ctx.accounts.instructions,
        &passkey,
        &config.rp_id_hash,
        &expected,
        &passkey_proof,
    )?;

    passport.wallet = wallet;
    passport.passkey = passkey;
    passport.proofs = records.iter().map(|(_, r)| ProofSummary::from(r)).collect();
    passport.nonce = passport
        .nonce
        .checked_add(1)
        .ok_or(error!(PassportError::MathOverflow))?;
    passport.recovery = None;
    passport.revoked = false;
    passport.issued_at = now;
    passport.withdraw_window_start = now;
    passport.withdrawn_in_window = 0;

    emit!(PassportReissued {
        passport: passport.key(),
        wallet,
        proofs: kinds(&records)
    });
    Ok(())
}
