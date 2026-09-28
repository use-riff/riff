//! Accounts and calls shared by the instructions that touch a coin's pool.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{dynamic_bonding_curve, *};

pub fn escrow_seeds<'a>(mint: &'a Pubkey, bump: &'a [u8; 1]) -> [&'a [u8]; 3] {
    [ESCROW_SEED, mint.as_ref(), bump]
}

/// Moves the creator share Meteora holds for this pool into the coin's
/// artist vault, while riff's escrow is still the pool creator. Returns the
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

/// Pays out the artist vault's whole balance.
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
