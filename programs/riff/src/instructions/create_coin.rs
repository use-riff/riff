use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    token_2022::{
        set_authority, spl_token_2022::instruction::AuthorityType, SetAuthority, Token2022,
    },
    token_interface::{
        mint_to_checked, spl_pod::optional_keys::OptionalNonZeroPubkey,
        spl_token_metadata_interface::state::TokenMetadata, token_metadata_initialize, Mint,
        MintToChecked, TokenAccount, TokenMetadataInitialize,
    },
};

use crate::{
    constants::*,
    error::ErrorCode,
    events::CoinCreated,
    state::{Coin, Config},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateCoinArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    /// External artist identifier (e.g. a streaming-platform artist ID).
    pub artist_id: String,
    pub artist_name: String,
}

#[derive(Accounts)]
pub struct CreateCoin<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = creator,
        space = 8 + Coin::INIT_SPACE,
        seeds = [COIN_SEED, mint.key().as_ref()],
        bump
    )]
    pub coin: Account<'info, Coin>,
    /// Fresh keypair. The coin PDA is mint authority only for the length of
    /// this instruction; no freeze authority is ever set.
    #[account(
        init,
        payer = creator,
        mint::decimals = COIN_DECIMALS,
        mint::authority = coin,
        mint::token_program = token_program,
        extensions::metadata_pointer::authority = coin,
        extensions::metadata_pointer::metadata_address = mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = creator,
        seeds = [VAULT_SEED, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = coin,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl CreateCoinArgs {
    fn validate(&self) -> Result<()> {
        require!(
            (1..=MAX_NAME_LEN).contains(&self.name.len()),
            ErrorCode::InvalidName
        );
        require!(
            (1..=MAX_SYMBOL_LEN).contains(&self.symbol.len()),
            ErrorCode::InvalidSymbol
        );
        require!(
            (1..=MAX_URI_LEN).contains(&self.uri.len()),
            ErrorCode::InvalidUri
        );
        require!(
            (1..=MAX_ARTIST_ID_LEN).contains(&self.artist_id.len())
                && self.artist_id.bytes().all(|b| b.is_ascii_graphic()),
            ErrorCode::InvalidArtistId
        );
        require!(
            (1..=MAX_ARTIST_NAME_LEN).contains(&self.artist_name.len()),
            ErrorCode::InvalidArtistName
        );
        Ok(())
    }
}

pub fn handle_create_coin(ctx: Context<CreateCoin>, args: CreateCoinArgs) -> Result<()> {
    args.validate()?;

    let coin_key = ctx.accounts.coin.key();
    let mint_key = ctx.accounts.mint.key();
    let bump = ctx.bumps.coin;
    let signer_seeds: &[&[&[u8]]] = &[&[COIN_SEED, mint_key.as_ref(), &[bump]]];

    // Token-2022 reallocs the mint for the metadata TLV but doesn't fund it,
    // so top up rent before initializing.
    let metadata = TokenMetadata {
        update_authority: OptionalNonZeroPubkey(coin_key),
        mint: mint_key,
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        uri: args.uri.clone(),
        additional_metadata: vec![],
    };
    let mint_info = ctx.accounts.mint.to_account_info();
    let new_len = mint_info
        .data_len()
        .checked_add(metadata.tlv_size_of()?)
        .ok_or(ErrorCode::MathOverflow)?;
    let shortfall = Rent::get()?
        .minimum_balance(new_len)
        .saturating_sub(mint_info.lamports());
    if shortfall > 0 {
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                Transfer {
                    from: ctx.accounts.creator.to_account_info(),
                    to: mint_info.clone(),
                },
            ),
            shortfall,
        )?;
    }

    token_metadata_initialize(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TokenMetadataInitialize {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: mint_info.clone(),
                update_authority: ctx.accounts.coin.to_account_info(),
                mint_authority: ctx.accounts.coin.to_account_info(),
                mint: mint_info.clone(),
            },
            signer_seeds,
        ),
        args.name,
        args.symbol,
        args.uri,
    )?;

    mint_to_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintToChecked {
                mint: mint_info.clone(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.coin.to_account_info(),
            },
            signer_seeds,
        ),
        COIN_TOTAL_SUPPLY,
        COIN_DECIMALS,
    )?;

    // Fixed supply: nobody can ever mint again.
    set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            SetAuthority {
                current_authority: ctx.accounts.coin.to_account_info(),
                account_or_mint: mint_info,
            },
            signer_seeds,
        ),
        AuthorityType::MintTokens,
        None,
    )?;

    let now = Clock::get()?.unix_timestamp;
    let claim_deadline = now
        .checked_add(ctx.accounts.config.claim_window_secs)
        .ok_or(ErrorCode::MathOverflow)?;

    ctx.accounts.coin.set_inner(Coin {
        mint: mint_key,
        vault: ctx.accounts.vault.key(),
        creator: ctx.accounts.creator.key(),
        artist_id: args.artist_id.clone(),
        artist_name: args.artist_name.clone(),
        artist: None,
        created_at: now,
        claim_deadline,
        bump,
        vault_bump: ctx.bumps.vault,
    });

    emit!(CoinCreated {
        coin: coin_key,
        mint: mint_key,
        creator: ctx.accounts.creator.key(),
        artist_id: args.artist_id,
        artist_name: args.artist_name,
        claim_deadline,
    });
    Ok(())
}
