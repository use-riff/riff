use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    token_2022::Token2022,
};
use riff_passport::{Passport, PASSPORT_SEED, VAULT_SEED};

use crate::{
    cp_amm, dynamic_bonding_curve, dynamic_bonding_curve::types::PoolState, error::RiffDbcError,
    events::ArtistClaimed, instructions::shared, *,
};

/// The artist claims their coin with their Artist Passport, co-signed by
/// riff's verifier. The coin's artist becomes the passport's vault: the fees
/// held so far, and every later share, are paid there in SOL.
///
/// A late claim doesn't take back what already belongs to the charity: what
/// was held until now goes to charity, only later fees to the artist. On a
/// graduated coin that includes the creator position's uncollected LP fees,
/// so a late claim must pass the `damm_*` accounts (others can leave them out).
///
/// Anyone can create or hand the escrow a position in the pool, so nothing
/// on chain tells the migration's position from a dust one. riff's verifier
/// co-signs this transaction, and it vouches that `creator_position` is the
/// real one.
#[derive(Accounts)]
pub struct ClaimArtist<'info> {
    /// The passport's wallet. Also pays the rent of `unwrap`, and gets it back.
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(address = config.verifier)]
    pub verifier: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [COIN_SEED, base_mint.key().as_ref()], bump = coin.bump)]
    pub coin: Box<Account<'info, Coin>>,
    /// The passport of this coin's artist: its seeds hold the artist ID.
    #[account(
        seeds = [PASSPORT_SEED, coin.artist_id.as_bytes()],
        bump = passport.bump,
        seeds::program = riff_passport::ID,
        constraint = passport.wallet == wallet.key() @ RiffDbcError::NotPassportWallet,
        constraint = !passport.revoked @ RiffDbcError::PassportRevoked,
    )]
    pub passport: Box<Account<'info, Passport>>,
    /// CHECK: the passport's vault (its address is checked by seeds).
    #[account(
        mut,
        seeds = [VAULT_SEED, passport.key().as_ref()],
        bump = passport.vault_bump,
        seeds::program = riff_passport::ID,
    )]
    pub passport_vault: UncheckedAccount<'info>,
    /// CHECK: PDA, the pool creator.
    #[account(seeds = [ESCROW_SEED, base_mint.key().as_ref()], bump = coin.escrow_bump)]
    pub escrow: UncheckedAccount<'info>,
    #[account(mut, seeds = [ARTIST_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub artist_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = wallet,
        seeds = [UNWRAP_SEED, base_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = escrow,
        token::token_program = token_quote_program,
    )]
    pub unwrap: Box<Account<'info, TokenAccount>>,
    /// CHECK: the escrow's account for the coin (see create_coin).
    #[account(mut, address = shared::escrow_base_address(&escrow.key(), &base_mint.key()))]
    pub escrow_base_account: UncheckedAccount<'info>,
    #[account(mut, token::mint = quote_mint, constraint = charity_quote_account.owner == config.charity @ RiffDbcError::WrongReceiver)]
    pub charity_quote_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: checked by Meteora.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: must be this coin's pool.
    #[account(mut, address = coin.pool @ RiffDbcError::WrongPool)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: this coin's mint.
    pub base_mint: UncheckedAccount<'info>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub quote_mint: Box<Account<'info, Mint>>,
    /// CHECK: checked by Meteora.
    pub dbc_event_authority: UncheckedAccount<'info>,
    pub dbc_program: Program<'info, dynamic_bonding_curve::program::DynamicBondingCurve>,
    pub token_quote_program: Program<'info, Token>,
    pub token_base_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    /// CHECK: checked by Meteora.
    pub damm_pool_authority: Option<UncheckedAccount<'info>>,
    /// CHECK: checked by Meteora: its token A must be this coin's mint.
    #[account(mut)]
    pub damm_pool: Option<UncheckedAccount<'info>>,
    /// CHECK: the escrow's position from the migration; the verifier vouches.
    #[account(mut)]
    pub creator_position: Option<UncheckedAccount<'info>>,
    /// CHECK: checked by Meteora (held by the escrow).
    pub creator_position_nft_account: Option<UncheckedAccount<'info>>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub token_a_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub token_b_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: checked by Meteora.
    pub damm_event_authority: Option<UncheckedAccount<'info>>,
    pub damm_program: Option<Program<'info, cp_amm::program::CpAmm>>,
}

/// Whether Meteora has migrated this DBC pool to DAMM v2.
fn is_migrated(pool: &AccountInfo) -> Result<bool> {
    let at = 8 + core::mem::offset_of!(PoolState, is_migrated);
    let data = pool.try_borrow_data()?;
    Ok(*data.get(at).ok_or(error!(RiffDbcError::WrongPool))? != 0)
}

pub fn handle_claim_artist<'info>(ctx: Context<'info, ClaimArtist<'info>>) -> Result<()> {
    let a = ctx.accounts;
    require!(a.coin.artist.is_none(), RiffDbcError::AlreadyClaimed);

    let mint = a.base_mint.key();
    let escrow_bump = [a.coin.escrow_bump];
    let escrow_signer = shared::escrow_seeds(&mint, &escrow_bump);

    let collected = shared::collect_creator_fees(
        &a.dbc_program.to_account_info(),
        &a.pool_authority.to_account_info(),
        &a.pool.to_account_info(),
        &a.escrow_base_account.to_account_info(),
        &mut a.artist_vault,
        &a.base_vault.to_account_info(),
        &a.quote_vault.to_account_info(),
        &a.base_mint.to_account_info(),
        &a.quote_mint.to_account_info(),
        &a.escrow.to_account_info(),
        &a.token_base_program.to_account_info(),
        &a.token_quote_program.to_account_info(),
        &a.dbc_event_authority.to_account_info(),
        &escrow_signer,
    )?;
    a.coin.artist_fees_total += collected;

    let late = Clock::get()?.unix_timestamp > a.coin.claim_deadline;
    let mut sent_to_charity = 0;
    if late {
        if is_migrated(&a.pool.to_account_info())? {
            let (
                Some(pool_authority),
                Some(damm_pool),
                Some(position),
                Some(nft_account),
                Some(token_a_vault),
                Some(token_b_vault),
                Some(event_authority),
                Some(damm_program),
            ) = (
                &a.damm_pool_authority,
                &a.damm_pool,
                &a.creator_position,
                &a.creator_position_nft_account,
                &a.token_a_vault,
                &a.token_b_vault,
                &a.damm_event_authority,
                &a.damm_program,
            )
            else {
                return err!(RiffDbcError::LateClaimNeedsLpAccounts);
            };
            let before = a.artist_vault.amount;
            shared::claim_position_fee(
                &damm_program.to_account_info(),
                &pool_authority.to_account_info(),
                &damm_pool.to_account_info(),
                &position.to_account_info(),
                &nft_account.to_account_info(),
                &a.escrow_base_account.to_account_info(),
                &a.artist_vault.to_account_info(),
                &token_a_vault.to_account_info(),
                &token_b_vault.to_account_info(),
                &a.base_mint.to_account_info(),
                &a.quote_mint.to_account_info(),
                &a.escrow.to_account_info(),
                &a.token_base_program.to_account_info(),
                &a.token_quote_program.to_account_info(),
                &event_authority.to_account_info(),
                &escrow_signer,
            )?;
            a.artist_vault.reload()?;
            a.coin.artist_fees_total += a.artist_vault.amount - before;
        }
        sent_to_charity = shared::empty_artist_vault(
            &a.token_quote_program.to_account_info(),
            &a.artist_vault,
            &a.charity_quote_account.to_account_info(),
            &a.escrow.to_account_info(),
            &escrow_signer,
        )?;
        a.artist_vault.reload()?;
    }
    let paid_to_artist = shared::pay_artist_sol(
        &a.token_quote_program.to_account_info(),
        &a.system_program.to_account_info(),
        &a.artist_vault,
        &a.unwrap.to_account_info(),
        &a.wallet.to_account_info(),
        &a.passport_vault.to_account_info(),
        &a.escrow.to_account_info(),
        &escrow_signer,
    )?;
    a.coin.artist = Some(a.passport_vault.key());
    emit!(ArtistClaimed {
        mint,
        passport: a.passport.key(),
        passport_vault: a.passport_vault.key(),
        late,
        paid_to_artist,
        sent_to_charity,
    });
    Ok(())
}
