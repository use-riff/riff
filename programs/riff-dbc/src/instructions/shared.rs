//! Accounts and calls shared by the instructions that touch a coin's pool.
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, CloseAccount, TokenAccount, Transfer};

use crate::{cp_amm, dynamic_bonding_curve, *};

/// The escrow's account for a coin. Meteora's fee claims need one for the
/// coin's side, whose fees are zero: every fee is charged in SOL.
pub fn escrow_base_address(escrow: &Pubkey, mint: &Pubkey) -> Pubkey {
    anchor_spl::associated_token::get_associated_token_address_with_program_id(
        escrow,
        mint,
        &anchor_spl::token_2022::ID,
    )
}

pub fn escrow_seeds<'a>(mint: &'a Pubkey, bump: &'a [u8; 1]) -> [&'a [u8]; 3] {
    [ESCROW_SEED, mint.as_ref(), bump]
}

/// Moves the creator share Meteora holds for this pool into the coin's
/// artist vault. riff's escrow is the pool creator for good. Returns the
/// amount collected.
#[allow(clippy::too_many_arguments)]
pub fn collect_creator_fees<'info>(
    dbc_program: &AccountInfo<'info>,
    pool_authority: &AccountInfo<'info>,
    pool: &AccountInfo<'info>,
    escrow_base_account: &AccountInfo<'info>,
    artist_vault: &mut Account<'info, TokenAccount>,
    base_vault: &AccountInfo<'info>,
    quote_vault: &AccountInfo<'info>,
    base_mint: &AccountInfo<'info>,
    quote_mint: &AccountInfo<'info>,
    escrow: &AccountInfo<'info>,
    token_base_program: &AccountInfo<'info>,
    token_quote_program: &AccountInfo<'info>,
    event_authority: &AccountInfo<'info>,
    escrow_signer: &[&[u8]],
) -> Result<u64> {
    let before = artist_vault.amount;
    dynamic_bonding_curve::cpi::claim_creator_trading_fee(
        CpiContext::new_with_signer(
            dbc_program.key(),
            dynamic_bonding_curve::cpi::accounts::ClaimCreatorTradingFee {
                pool_authority: pool_authority.clone(),
                pool: pool.clone(),
                token_a_account: escrow_base_account.clone(),
                token_b_account: artist_vault.to_account_info(),
                base_vault: base_vault.clone(),
                quote_vault: quote_vault.clone(),
                base_mint: base_mint.clone(),
                quote_mint: quote_mint.clone(),
                creator: escrow.clone(),
                token_base_program: token_base_program.clone(),
                token_quote_program: token_quote_program.clone(),
                event_authority: event_authority.clone(),
                program: dbc_program.clone(),
            },
            &[escrow_signer],
        ),
        u64::MAX,
        u64::MAX,
    )?;
    artist_vault.reload()?;
    Ok(artist_vault.amount - before)
}

/// Claims the LP fees of one of a graduated coin's locked positions. Meteora
/// checks that `owner` holds the position, that the position is in `pool` and
/// that the pool's token A is `base_mint`.
#[allow(clippy::too_many_arguments)]
pub fn claim_position_fee<'info>(
    damm_program: &AccountInfo<'info>,
    pool_authority: &AccountInfo<'info>,
    pool: &AccountInfo<'info>,
    position: &AccountInfo<'info>,
    nft_account: &AccountInfo<'info>,
    escrow_base_account: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    token_a_vault: &AccountInfo<'info>,
    token_b_vault: &AccountInfo<'info>,
    base_mint: &AccountInfo<'info>,
    quote_mint: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    token_base_program: &AccountInfo<'info>,
    token_quote_program: &AccountInfo<'info>,
    event_authority: &AccountInfo<'info>,
    signer: &[&[u8]],
) -> Result<()> {
    cp_amm::cpi::claim_position_fee(CpiContext::new_with_signer(
        damm_program.key(),
        cp_amm::cpi::accounts::ClaimPositionFee {
            pool_authority: pool_authority.clone(),
            pool: pool.clone(),
            position: position.clone(),
            token_a_account: escrow_base_account.clone(),
            token_b_account: to.clone(),
            token_a_vault: token_a_vault.clone(),
            token_b_vault: token_b_vault.clone(),
            token_a_mint: base_mint.clone(),
            token_b_mint: quote_mint.clone(),
            position_nft_account: nft_account.clone(),
            signer: owner.clone(),
            token_a_program: token_base_program.clone(),
            token_b_program: token_quote_program.clone(),
            event_authority: event_authority.clone(),
            program: damm_program.clone(),
        },
        &[signer],
    ))
}

/// Pays out the artist vault's whole balance, as WSOL.
pub fn empty_artist_vault<'info>(
    token_program: &AccountInfo<'info>,
    artist_vault: &Account<'info, TokenAccount>,
    to: &AccountInfo<'info>,
    escrow: &AccountInfo<'info>,
    escrow_signer: &[&[u8]],
) -> Result<u64> {
    let amount = artist_vault.amount;
    if amount > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                token_program.key(),
                Transfer {
                    from: artist_vault.to_account_info(),
                    to: to.clone(),
                    authority: escrow.clone(),
                },
                &[escrow_signer],
            ),
            amount,
        )?;
    }
    Ok(amount)
}

/// Pays out the artist vault's whole balance as SOL to the artist's passport
/// vault. WSOL only unwraps by closing an account, so the balance goes
/// through `unwrap`, a WSOL account the payer created for this instruction:
/// closing it returns its rent and the balance to the payer, who passes the
/// balance on.
#[allow(clippy::too_many_arguments)]
pub fn pay_artist_sol<'info>(
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    artist_vault: &Account<'info, TokenAccount>,
    unwrap: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    passport_vault: &AccountInfo<'info>,
    escrow: &AccountInfo<'info>,
    escrow_signer: &[&[u8]],
) -> Result<u64> {
    let amount = empty_artist_vault(token_program, artist_vault, unwrap, escrow, escrow_signer)?;
    token::close_account(CpiContext::new_with_signer(
        token_program.key(),
        CloseAccount {
            account: unwrap.clone(),
            destination: payer.clone(),
            authority: escrow.clone(),
        },
        &[escrow_signer],
    ))?;
    if amount > 0 {
        system_program::transfer(
            CpiContext::new(
                system_program.key(),
                system_program::Transfer {
                    from: payer.clone(),
                    to: passport_vault.clone(),
                },
            ),
            amount,
        )?;
    }
    Ok(amount)
}

/// The launcher's and treasury's accounts for a partner split: both, or
/// neither (the partner share then waits with Meteora for a later call).
pub fn partner_receivers<'info>(
    launcher: Option<&Account<'info, TokenAccount>>,
    treasury: Option<&Account<'info, TokenAccount>>,
) -> Result<Option<(AccountInfo<'info>, AccountInfo<'info>)>> {
    match (launcher, treasury) {
        (Some(l), Some(t)) => Ok(Some((l.to_account_info(), t.to_account_info()))),
        (None, None) => Ok(None),
        _ => err!(crate::error::RiffDbcError::PartnerAccountsIncomplete),
    }
}

/// Splits a partner share that just arrived in the fees vault between the
/// coin's launcher and riff's treasury. Returns the launcher's part.
#[allow(clippy::too_many_arguments)]
pub fn split_partner_share<'info>(
    token_program: &AccountInfo<'info>,
    fees_vault: &Account<'info, TokenAccount>,
    launcher_account: &AccountInfo<'info>,
    treasury_account: &AccountInfo<'info>,
    fee_authority: &AccountInfo<'info>,
    fee_signer: &[&[u8]],
    launcher_share_bps: u16,
    amount: u64,
) -> Result<u64> {
    let to_launcher = (amount as u128 * launcher_share_bps as u128 / BPS as u128) as u64;
    for (to, part) in [
        (launcher_account, to_launcher),
        (treasury_account, amount - to_launcher),
    ] {
        if part > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    token_program.key(),
                    Transfer {
                        from: fees_vault.to_account_info(),
                        to: to.clone(),
                        authority: fee_authority.clone(),
                    },
                    &[fee_signer],
                ),
                part,
            )?;
        }
    }
    Ok(to_launcher)
}
