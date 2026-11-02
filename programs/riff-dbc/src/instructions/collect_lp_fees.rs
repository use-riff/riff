use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    token_2022::Token2022,
};

use crate::{cp_amm, error::RiffDbcError, events::FeesCollected, instructions::shared, *};

/// After graduation: collects the LP fees of a coin's two locked positions
/// in its DAMM v2 pool. Anyone can call it (a crank).
///
/// - riff's side (the partner position, owned by the fee authority) is split
///   between the launcher and riff, like the partner share before.
/// - The artist's side (the creator position, owned by the escrow) goes to
///   the artist vault, like the creator share before.
///
/// Meteora checks that each signer owns its position and that the pool is
/// this coin's: its token A must be the coin's mint.
#[derive(Accounts)]
pub struct CollectLpFees<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(mut, seeds = [COIN_SEED, base_mint.key().as_ref()], bump = coin.bump)]
    pub coin: Box<Account<'info, Coin>>,
    /// CHECK: PDA, owner of the creator position.
    #[account(seeds = [ESCROW_SEED, base_mint.key().as_ref()], bump = coin.escrow_bump)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK: PDA, owner of the partner position.
    #[account(seeds = [FEE_AUTHORITY_SEED], bump = config.fee_authority_bump)]
    pub fee_authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [FEES_VAULT_SEED], bump)]
    pub fees_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [ARTIST_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub artist_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: the escrow's account for the coin. Meteora needs one for
    /// token A fees, which are zero: the pool collects fees in SOL.
    #[account(mut, address = shared::escrow_base_address(&escrow.key(), &base_mint.key()))]
    pub escrow_base_account: UncheckedAccount<'info>,
    #[account(mut, token::mint = quote_mint, constraint = launcher_quote_account.owner == coin.launcher @ RiffDbcError::WrongReceiver)]
    pub launcher_quote_account: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint, constraint = treasury_quote_account.owner == config.treasury @ RiffDbcError::WrongReceiver)]
    pub treasury_quote_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: checked by Meteora.
    pub damm_pool_authority: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub damm_pool: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub creator_position: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora (held by the escrow).
    pub creator_position_nft_account: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub partner_position: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora (held by the fee authority).
    pub partner_position_nft_account: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,
    /// CHECK: this coin's mint.
    pub base_mint: UncheckedAccount<'info>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub quote_mint: Box<Account<'info, Mint>>,
    /// CHECK: checked by Meteora.
    pub damm_event_authority: UncheckedAccount<'info>,
    pub damm_program: Program<'info, cp_amm::program::CpAmm>,
    pub token_quote_program: Program<'info, Token>,
    pub token_base_program: Program<'info, Token2022>,
}

impl<'info> CollectLpFees<'info> {
    fn claim(
        &self,
        position: &UncheckedAccount<'info>,
        nft_account: &UncheckedAccount<'info>,
        to: AccountInfo<'info>,
        owner: &UncheckedAccount<'info>,
        signer: &[&[u8]],
    ) -> Result<()> {
        shared::claim_position_fee(
            &self.damm_program.to_account_info(),
            &self.damm_pool_authority.to_account_info(),
            &self.damm_pool.to_account_info(),
            &position.to_account_info(),
            &nft_account.to_account_info(),
            &self.escrow_base_account.to_account_info(),
            &to,
            &self.token_a_vault.to_account_info(),
            &self.token_b_vault.to_account_info(),
            &self.base_mint.to_account_info(),
            &self.quote_mint.to_account_info(),
            &owner.to_account_info(),
            &self.token_base_program.to_account_info(),
            &self.token_quote_program.to_account_info(),
            &self.damm_event_authority.to_account_info(),
            signer,
        )
    }
}

pub fn handle_collect_lp_fees<'info>(ctx: Context<'info, CollectLpFees<'info>>) -> Result<()> {
    let a = ctx.accounts;
    let fee_authority_bump = [a.config.fee_authority_bump];
    let fee_signer: &[&[u8]] = &[FEE_AUTHORITY_SEED, &fee_authority_bump];

    let before = a.fees_vault.amount;
    a.claim(
        &a.partner_position,
        &a.partner_position_nft_account,
        a.fees_vault.to_account_info(),
        &a.fee_authority,
        fee_signer,
    )?;
    a.fees_vault.reload()?;
    let partner_share = a.fees_vault.amount - before;
    let to_launcher = shared::split_partner_share(
        &a.token_quote_program.to_account_info(),
        &a.fees_vault,
        &a.launcher_quote_account.to_account_info(),
        &a.treasury_quote_account.to_account_info(),
        &a.fee_authority.to_account_info(),
        fee_signer,
        a.config.launcher_share_bps,
        partner_share,
    )?;
    a.coin.launcher_fees_total += to_launcher;

    let mint = a.base_mint.key();
    let escrow_bump = [a.coin.escrow_bump];
    let before = a.artist_vault.amount;
    a.claim(
        &a.creator_position,
        &a.creator_position_nft_account,
        a.artist_vault.to_account_info(),
        &a.escrow,
        &shared::escrow_seeds(&mint, &escrow_bump),
    )?;
    a.artist_vault.reload()?;
    let to_artist_vault = a.artist_vault.amount - before;
    a.coin.artist_fees_total += to_artist_vault;
    emit!(FeesCollected {
        mint,
        lp: true,
        to_launcher,
        to_treasury: partner_share - to_launcher,
        to_artist_vault,
    });
    Ok(())
}
