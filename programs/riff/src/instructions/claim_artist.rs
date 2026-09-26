use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::ErrorCode,
    events::ArtistClaimed,
    state::{Coin, Config},
};

/// Links a coin to its real artist's wallet.
///
/// riff's verification service checks the artist off-chain (however riff
/// chooses) and co-signs this transaction; the artist signs too, proving
/// they control the wallet being linked.
#[derive(Accounts)]
pub struct ClaimArtist<'info> {
    pub artist: Signer<'info>,
    #[account(address = config.verifier @ ErrorCode::NotVerifier)]
    pub verifier: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [COIN_SEED, coin.mint.as_ref()], bump = coin.bump)]
    pub coin: Account<'info, Coin>,
}

/// `artist_id` must match the coin's, so a co-signature for one artist
/// can't be used to claim another artist's coin.
pub fn handle_claim_artist(ctx: Context<ClaimArtist>, artist_id: String) -> Result<()> {
    let coin = &mut ctx.accounts.coin;
    require!(coin.artist.is_none(), ErrorCode::AlreadyClaimed);
    require!(artist_id == coin.artist_id, ErrorCode::ArtistIdMismatch);

    // A late claim doesn't take back what already belongs to the charity:
    // only fees from now on go to the artist.
    let now = Clock::get()?.unix_timestamp;
    let late = now > coin.claim_deadline;
    let forfeited_to_charity = coin
        .forfeit_unclaimed_artist_fees(now)
        .ok_or(ErrorCode::MathOverflow)?;
    coin.artist = Some(ctx.accounts.artist.key());

    emit!(ArtistClaimed {
        coin: coin.key(),
        artist: ctx.accounts.artist.key(),
        artist_id,
        late,
        forfeited_to_charity,
    });
    Ok(())
}
