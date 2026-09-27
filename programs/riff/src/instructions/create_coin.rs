use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::{
        create as create_associated_token_account, get_associated_token_address_with_program_id,
        AssociatedToken, Create,
    },
    token_2022::{
        set_authority, spl_token_2022::instruction::AuthorityType, SetAuthority, Token2022,
    },
    token_interface::{
        mint_to_checked, spl_pod::optional_keys::OptionalNonZeroPubkey,
        spl_token_metadata_interface::state::TokenMetadata, token_metadata_initialize,
        token_metadata_update_authority, Mint, MintToChecked, TokenAccount,
        TokenMetadataInitialize, TokenMetadataUpdateAuthority,
    },
};

use crate::{
    constants::*,
    error::ErrorCode,
    events::CoinCreated,
    instructions::trade::{emit_trade, pay_for_buy, release_tokens, settle_buy},
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
    /// Most SOL (fee included) the creator spends buying at launch, from the
    /// same curve as everyone else. 0 for no buy. The tokens bought may not
    /// exceed the config's cap.
    pub creator_buy_sol: u64,
    /// Fewest tokens the launch buy may return. The curve is fresh, but the
    /// config's fees and curve settings could change between signing and
    /// landing.
    pub creator_buy_min_tokens: u64,
}

#[derive(Accounts)]
pub struct CreateCoin<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(
        init,
        payer = creator,
        space = 8 + Coin::INIT_SPACE,
        seeds = [COIN_SEED, mint.key().as_ref()],
        bump
    )]
    pub coin: Box<Account<'info, Coin>>,
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
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init,
        payer = creator,
        seeds = [VAULT_SEED, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = coin,
        token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: the creator's associated token account for the new mint.
    /// Created only if the creator buys at launch; its address is enforced.
    #[account(
        mut,
        address = get_associated_token_address_with_program_id(
            &creator.key(),
            &mint.key(),
            &token_program.key(),
        ),
    )]
    pub creator_token_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
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

    // Final metadata: nobody, riff included, can rename the coin or point it
    // at other metadata once people have bought it.
    token_metadata_update_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TokenMetadataUpdateAuthority {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: mint_info.clone(),
                current_authority: ctx.accounts.coin.to_account_info(),
                new_authority: ctx.accounts.coin.to_account_info(),
            },
            signer_seeds,
        ),
        OptionalNonZeroPubkey::default(),
    )?;
    set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            SetAuthority {
                current_authority: ctx.accounts.coin.to_account_info(),
                account_or_mint: mint_info.clone(),
            },
            signer_seeds,
        ),
        AuthorityType::MetadataPointer,
        None,
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

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    let claim_deadline = now
        .checked_add(ctx.accounts.config.claim_window_secs)
        .ok_or(ErrorCode::MathOverflow)?;

    let config = &ctx.accounts.config;
    ctx.accounts.coin.set_inner(Coin {
        mint: mint_key,
        vault: ctx.accounts.vault.key(),
        creator: ctx.accounts.creator.key(),
        artist_id: args.artist_id.clone(),
        artist_name: args.artist_name.clone(),
        artist: None,
        created_at: now,
        created_slot: clock.slot,
        claim_deadline,
        virtual_sol_reserves: config.initial_virtual_sol_reserves,
        virtual_token_reserves: config.initial_virtual_token_reserves,
        real_sol_reserves: 0,
        real_token_reserves: config.curve_token_supply,
        artist_fees: 0,
        charity_fees: 0,
        creator_fees: 0,
        protocol_fees: 0,
        complete: false,
        pool: None,
        graduation_sol: 0,
        artist_fees_total: 0,
        peak_virtual_sol: config.initial_virtual_sol_reserves,
        volume_hourly: [0; VOLUME_HOURS],
        volume_hour: now.div_euclid(3600),
        bump,
        vault_bump: ctx.bumps.vault,
    });

    // Optional launch buy, priced on the fresh curve exactly like any other
    // buy.
    let a = ctx.accounts;
    let creator_buy = if args.creator_buy_sol > 0 {
        let (quote, fees) = settle_buy(&a.config, &mut a.coin, args.creator_buy_sol)?;
        require!(
            quote.tokens_out <= a.config.max_creator_buy_tokens(),
            ErrorCode::CreatorBuyTooLarge
        );
        require!(
            quote.tokens_out >= args.creator_buy_min_tokens,
            ErrorCode::SlippageExceeded
        );
        Some((quote, fees))
    } else {
        None
    };

    emit!(CoinCreated {
        coin: coin_key,
        mint: mint_key,
        creator: a.creator.key(),
        artist_id: args.artist_id,
        artist_name: args.artist_name,
        claim_deadline,
        creator_buy_tokens: creator_buy.map_or(0, |(q, _)| q.tokens_out),
        creator_buy_sol: creator_buy.map_or(0, |(q, _)| q.total_cost),
    });

    if let Some((quote, fees)) = creator_buy {
        create_associated_token_account(CpiContext::new(
            a.associated_token_program.key(),
            Create {
                payer: a.creator.to_account_info(),
                associated_token: a.creator_token_account.to_account_info(),
                authority: a.creator.to_account_info(),
                mint: a.mint.to_account_info(),
                system_program: a.system_program.to_account_info(),
                token_program: a.token_program.to_account_info(),
            },
        ))?;
        pay_for_buy(
            &a.system_program,
            &a.creator.to_account_info(),
            &a.coin.to_account_info(),
            &quote,
        )?;
        release_tokens(
            &a.token_program,
            &a.vault,
            &a.mint,
            &a.creator_token_account.to_account_info(),
            &a.coin,
            quote.tokens_out,
        )?;
        emit_trade(
            &a.coin,
            a.creator.key(),
            true,
            quote.sol_to_curve,
            quote.tokens_out,
            &fees,
        )?;
    }
    Ok(())
}
