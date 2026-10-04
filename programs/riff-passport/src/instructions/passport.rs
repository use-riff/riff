use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::PassportError,
    events::{EndorsementChanged, PasskeyChanged, PassportIssued, ProofsAdded, WalletChanged},
    instructions::shared::{fresh_proofs, kinds, spotify_id},
    passkey::{self, PasskeyAction, PasskeyProof},
    state::{
        proofs_suffice, Endorsement, EndorsementStatus, Passkey, Passport, PassportConfig,
        ProofSummary, Vault,
    },
};

#[derive(Accounts)]
#[instruction(artist_id: String)]
pub struct IssuePassport<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, PassportConfig>,
    #[account(
        init,
        payer = wallet,
        space = 8 + Passport::INIT_SPACE,
        seeds = [PASSPORT_SEED, artist_id.as_bytes()],
        bump
    )]
    pub passport: Account<'info, Passport>,
    #[account(
        init,
        payer = wallet,
        space = 8 + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, passport.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: the instructions sysvar, for the passkey signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    // remaining_accounts: the wallet's proof records for this artist.
}

/// Issues the passport once the wallet has enough fresh proofs, and binds
/// the passkey it just proved it holds.
pub fn handle_issue_passport<'info>(
    ctx: Context<'info, IssuePassport<'info>>,
    artist_id: String,
    passkey: Passkey,
    passkey_proof: PasskeyProof,
) -> Result<()> {
    spotify_id(&artist_id)?;
    let now = Clock::get()?.unix_timestamp;
    let config = &ctx.accounts.config;
    let wallet = ctx.accounts.wallet.key();
    let records = fresh_proofs(
        ctx.remaining_accounts,
        &artist_id,
        &wallet,
        now,
        config.proof_max_age,
    )?;
    require!(
        proofs_suffice(&kinds(&records)),
        PassportError::NotEnoughProofs
    );

    let passport_key = ctx.accounts.passport.key();
    let expected = passkey::challenge(&passport_key, 0, &PasskeyAction::Register { passkey })?;
    passkey::verify(
        &ctx.accounts.instructions,
        &passkey,
        &config.rp_id_hash,
        &expected,
        &passkey_proof,
    )?;

    let passport = &mut ctx.accounts.passport;
    passport.artist_id = artist_id.clone();
    passport.wallet = wallet;
    passport.passkey = passkey;
    passport.proofs = records.iter().map(|(_, r)| ProofSummary::from(r)).collect();
    passport.nonce = 1;
    passport.recovery = None;
    passport.revoked = false;
    passport.issued_at = now;
    passport.withdraw_window_start = now;
    passport.withdrawn_in_window = 0;
    passport.bump = ctx.bumps.passport;
    passport.vault_bump = ctx.bumps.vault;
    ctx.accounts.vault.passport = passport_key;

    emit!(PassportIssued {
        passport: passport_key,
        artist_id,
        wallet,
        proofs: kinds(&records)
    });
    Ok(())
}

/// Accounts for an action by the passport's wallet with its passkey.
#[derive(Accounts)]
pub struct PasskeyAct<'info> {
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
    /// CHECK: the instructions sysvar, for the passkey signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// Checks the passport's passkey approved `action`, then uses up the nonce.
pub fn require_passkey(
    passport: &mut Account<Passport>,
    config: &PassportConfig,
    instructions: &AccountInfo,
    action: &PasskeyAction,
    proof: &PasskeyProof,
) -> Result<()> {
    let expected = passkey::challenge(&passport.key(), passport.nonce, action)?;
    passkey::verify(
        instructions,
        &passport.passkey,
        &config.rp_id_hash,
        &expected,
        proof,
    )?;
    passport.nonce = passport
        .nonce
        .checked_add(1)
        .ok_or(error!(PassportError::MathOverflow))?;
    Ok(())
}

/// Adds fresh proofs to the passport. Needs the passkey: every proof becomes
/// a guardian that can veto a recovery.
pub fn handle_add_proofs<'info>(
    ctx: Context<'info, PasskeyAct<'info>>,
    passkey_proof: PasskeyProof,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let passport = &mut ctx.accounts.passport;
    let records = fresh_proofs(
        ctx.remaining_accounts,
        &passport.artist_id.clone(),
        &passport.wallet.clone(),
        now,
        ctx.accounts.config.proof_max_age,
    )?;
    require!(!records.is_empty(), PassportError::NotEnoughProofs);
    let action = PasskeyAction::AddProofs {
        records: records.iter().map(|(k, _)| *k).collect(),
    };
    require_passkey(
        passport,
        &ctx.accounts.config,
        &ctx.accounts.instructions,
        &action,
        &passkey_proof,
    )?;
    for (_, record) in &records {
        passport.upsert_proof(ProofSummary::from(record))?;
    }
    emit!(ProofsAdded {
        passport: passport.key(),
        proofs: kinds(&records)
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SetWallet<'info> {
    pub act: PasskeyAct<'info>,
    /// Signs too, so a typo can't send the passport to a wallet nobody holds.
    pub new_wallet: Signer<'info>,
}

pub fn handle_set_wallet(ctx: Context<SetWallet>, passkey_proof: PasskeyProof) -> Result<()> {
    let new_wallet = ctx.accounts.new_wallet.key();
    let act = &mut ctx.accounts.act;
    let action = PasskeyAction::SetWallet { new_wallet };
    require_passkey(
        &mut act.passport,
        &act.config,
        &act.instructions,
        &action,
        &passkey_proof,
    )?;
    let old_wallet = act.passport.wallet;
    act.passport.wallet = new_wallet;
    emit!(WalletChanged {
        passport: act.passport.key(),
        old_wallet,
        new_wallet
    });
    Ok(())
}

/// Replaces the passkey: the old one approves, the new one proves it's held.
pub fn handle_set_passkey(
    ctx: Context<PasskeyAct>,
    new_passkey: Passkey,
    old_proof: PasskeyProof,
    new_proof: PasskeyProof,
) -> Result<()> {
    let passport = &mut ctx.accounts.passport;
    let config = &ctx.accounts.config;
    let nonce = passport.nonce;
    let register = passkey::challenge(
        &passport.key(),
        nonce,
        &PasskeyAction::Register {
            passkey: new_passkey,
        },
    )?;
    passkey::verify(
        &ctx.accounts.instructions,
        &new_passkey,
        &config.rp_id_hash,
        &register,
        &new_proof,
    )?;
    let action = PasskeyAction::SetPasskey { new_passkey };
    require_passkey(
        passport,
        config,
        &ctx.accounts.instructions,
        &action,
        &old_proof,
    )?;
    passport.passkey = new_passkey;
    emit!(PasskeyChanged {
        passport: passport.key()
    });
    Ok(())
}

#[derive(Accounts)]
#[instruction(mint: Pubkey)]
pub struct SetEndorsement<'info> {
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
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + Endorsement::INIT_SPACE,
        seeds = [ENDORSEMENT_SEED, passport.key().as_ref(), mint.as_ref()],
        bump
    )]
    pub endorsement: Account<'info, Endorsement>,
    /// CHECK: the instructions sysvar, for the passkey signature.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

fn write_endorsement(
    ctx: &mut Context<SetEndorsement>,
    mint: Pubkey,
    status: EndorsementStatus,
) -> Result<()> {
    let passport = ctx.accounts.passport.key();
    let endorsement = &mut ctx.accounts.endorsement;
    endorsement.passport = passport;
    endorsement.mint = mint;
    endorsement.status = status;
    endorsement.updated_at = Clock::get()?.unix_timestamp;
    endorsement.bump = ctx.bumps.endorsement;
    emit!(EndorsementChanged {
        passport,
        mint,
        status
    });
    Ok(())
}

/// The artist vouches for a coin, on any launchpad. Needs the passkey.
pub fn handle_endorse(
    mut ctx: Context<SetEndorsement>,
    mint: Pubkey,
    passkey_proof: PasskeyProof,
) -> Result<()> {
    let accounts = &mut ctx.accounts;
    accounts
        .passport
        .require_settled(&accounts.config, Clock::get()?.unix_timestamp)?;
    require_passkey(
        &mut accounts.passport,
        &accounts.config,
        &accounts.instructions,
        &PasskeyAction::Endorse { mint },
        &passkey_proof,
    )?;
    write_endorsement(&mut ctx, mint, EndorsementStatus::Endorsed)
}

/// The artist says a coin isn't theirs. The wallet alone may do this:
/// disavowing is always safe, and should be instant.
pub fn handle_disavow(mut ctx: Context<SetEndorsement>, mint: Pubkey) -> Result<()> {
    write_endorsement(&mut ctx, mint, EndorsementStatus::Disavowed)
}
