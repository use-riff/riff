use anchor_lang::prelude::*;

use crate::{
    constants::*, error::ErrorCode, events::ArtistFeesWithdrawn, payout::pay_from_coin, state::Coin,
};

#[derive(Accounts)]
pub struct WithdrawArtistFees<'info> {
    #[account(mut)]
    pub artist: Signer<'info>,
    #[account(
        mut,
        seeds = [COIN_SEED, coin.mint.as_ref()],
        bump = coin.bump,
        constraint = coin.artist == Some(artist.key()) @ ErrorCode::NotArtist,
    )]
    pub coin: Account<'info, Coin>,
}

/// Pays the claimed artist every artist fee held for them.
pub fn handle_withdraw_artist_fees(ctx: Context<WithdrawArtistFees>) -> Result<()> {
    let coin = &mut ctx.accounts.coin;
    let amount = coin.artist_fees;
    require!(amount > 0, ErrorCode::NoFeesToWithdraw);
    coin.artist_fees = 0;

    pay_from_coin(coin, &ctx.accounts.artist.to_account_info(), amount)?;

    emit!(ArtistFeesWithdrawn {
        coin: coin.key(),
        artist: ctx.accounts.artist.key(),
        amount,
    });
    Ok(())
}
