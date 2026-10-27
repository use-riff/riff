use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, AssociatedToken, Create},
    token::{Mint, Token, TokenAccount},
    token_2022::Token2022,
};

use anchor_lang::Discriminator;
use solana_instructions_sysvar::load_instruction_at_checked;

use crate::{
    dynamic_bonding_curve::{self, client::args::Swap as SwapIx},
    error::RiffDbcError,
    *,
};

#[derive(Accounts)]
pub struct CreateCoin<'info> {
    #[account(mut)]
    pub launcher: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(
        init,
        payer = launcher,
        space = 8 + Coin::INIT_SPACE,
        seeds = [COIN_SEED, base_mint.key().as_ref()],
        bump
    )]
    pub coin: Box<Account<'info, Coin>>,
    /// CHECK: PDA that is the pool's creator until the artist claims.
    #[account(seeds = [ESCROW_SEED, base_mint.key().as_ref()], bump)]
    pub escrow: UncheckedAccount<'info>,
    /// Holds the artist's share until they claim.
    #[account(
        init,
        payer = launcher,
        seeds = [ARTIST_VAULT_SEED, base_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = escrow,
        token::token_program = token_quote_program,
    )]
    pub artist_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: the escrow's account for the coin, created after Meteora
    /// creates the mint. Fee claims need one even though fees come in SOL.
    #[account(mut)]
    pub escrow_base_account: UncheckedAccount<'info>,
    /// CHECK: new mint, created and initialized by Meteora.
    #[account(mut)]
    pub base_mint: Signer<'info>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub quote_mint: Box<Account<'info, Mint>>,
    /// CHECK: riff's DBC config.
    #[account(address = config.dbc_config)]
    pub dbc_config: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: created by Meteora, which checks its address.
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: created by Meteora.
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: created by Meteora.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: checked by Meteora.
    pub dbc_event_authority: UncheckedAccount<'info>,
    /// CHECK: read to cap the launch buy (see `check_launch_buy`).
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
    pub dbc_program: Program<'info, dynamic_bonding_curve::program::DynamicBondingCurve>,
    pub token_quote_program: Program<'info, Token>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_coin<'info>(
    ctx: Context<'info, CreateCoin<'info>>,
    args: CreateCoinArgs,
) -> Result<()> {
    let artist_id = args.artist_id.trim().to_string();
    let artist_name = args.artist_name.trim().to_string();
    require!(
        !artist_id.is_empty() && artist_id.len() <= MAX_ARTIST_ID_LEN,
        RiffDbcError::InvalidArtistId
    );
    require!(
        !artist_name.is_empty() && artist_name.len() <= MAX_ARTIST_NAME_LEN,
        RiffDbcError::InvalidArtistName
    );

    let a = &ctx.accounts;
    let mint = a.base_mint.key();
    let escrow_bump = [ctx.bumps.escrow];
    let escrow_signer = shared::escrow_seeds(&mint, &escrow_bump);

    // Meteora creates the mint, pool and vaults, with riff's escrow as creator.
    dynamic_bonding_curve::cpi::initialize_virtual_pool_with_token2022(
        CpiContext::new_with_signer(
            a.dbc_program.key(),
            dynamic_bonding_curve::cpi::accounts::InitializeVirtualPoolWithToken2022 {
                config: a.dbc_config.to_account_info(),
                pool_authority: a.pool_authority.to_account_info(),
                creator: a.escrow.to_account_info(),
                base_mint: a.base_mint.to_account_info(),
                quote_mint: a.quote_mint.to_account_info(),
                pool: a.pool.to_account_info(),
                base_vault: a.base_vault.to_account_info(),
                quote_vault: a.quote_vault.to_account_info(),
                payer: a.launcher.to_account_info(),
                token_quote_program: a.token_quote_program.to_account_info(),
                token_program: a.token_program.to_account_info(),
                system_program: a.system_program.to_account_info(),
                event_authority: a.dbc_event_authority.to_account_info(),
                program: a.dbc_program.to_account_info(),
            },
            &[&escrow_signer],
        ),
        dynamic_bonding_curve::types::InitializePoolParameters {
            name: args.name,
            symbol: args.symbol,
            uri: args.uri,
        },
    )?;

    associated_token::create(CpiContext::new(
        a.associated_token_program.key(),
        Create {
            payer: a.launcher.to_account_info(),
            associated_token: a.escrow_base_account.to_account_info(),
            authority: a.escrow.to_account_info(),
            mint: a.base_mint.to_account_info(),
            system_program: a.system_program.to_account_info(),
            token_program: a.token_program.to_account_info(),
        },
    ))?;

    check_launch_buy(
        &a.instructions_sysvar,
        &a.pool.key(),
        a.config.max_launch_buy,
    )?;

    let now = Clock::get()?.unix_timestamp;
    let claim_window = a.config.claim_window_secs;
    let pool = a.pool.key();
    let launcher = a.launcher.key();
    ctx.accounts.coin.set_inner(Coin {
        mint,
        pool,
        launcher,
        artist_id,
        artist_name,
        artist: None,
        created_at: now,
        claim_deadline: now + claim_window,
        artist_fees_total: 0,
        launcher_fees_total: 0,
        bump: ctx.bumps.coin,
        escrow_bump: ctx.bumps.escrow,
    });
    Ok(())
}

/// The launcher's first buy goes in the transaction that creates the coin,
/// as Meteora `swap` instructions after this one: Meteora charges a buy in
/// the pool's creating transaction the normal fee, not the launch's
/// anti-sniping fee. So that buy is capped, in SOL, here. Buys nested in
/// other programs' instructions don't get the normal fee.
fn check_launch_buy(sysvar: &AccountInfo, pool: &Pubkey, cap: u64) -> Result<()> {
    let mut total: u64 = 0;
    let mut index = 0;
    while let Ok(ix) = load_instruction_at_checked(index, sysvar) {
        index += 1;
        // Every Meteora swap names the pool third.
        if ix.program_id != dynamic_bonding_curve::ID
            || ix.accounts.get(2).map(|a| a.pubkey) != Some(*pool)
        {
            continue;
        }
        // Only exact-in swaps, so the amount is what the launcher pays.
        require!(
            ix.data.len() >= 16 && ix.data[..8] == *SwapIx::DISCRIMINATOR,
            RiffDbcError::LaunchBuyNotSwap
        );
        let amount_in = u64::from_le_bytes(ix.data[8..16].try_into().unwrap());
        total = total
            .checked_add(amount_in)
            .ok_or(error!(RiffDbcError::MathOverflow))?;
    }
    require!(total <= cap, RiffDbcError::LaunchBuyTooLarge);
    Ok(())
}
