use anchor_lang::prelude::*;

use crate::{constants::*, error::ErrorCode, events::ConfigUpdated, state::Config};

/// Settings to change; `None` leaves one unchanged. Fees and curve settings
/// can't be changed here, and nothing here can touch funds held by coins.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateConfigParams {
    pub treasury: Option<Pubkey>,
    pub charity: Option<Pubkey>,
    pub verifier: Option<Pubkey>,
    pub raydium_amm_config: Option<Pubkey>,
    /// Claim window for coins launched from now on. Existing coins keep the
    /// deadline they got at launch.
    pub claim_window_secs: Option<i64>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ ErrorCode::NotAdmin,
    )]
    pub config: Account<'info, Config>,
}

/// Lets the admin replace a lost or leaked key (most importantly the
/// verifier, which can link any unclaimed coin to a wallet) and tune the
/// claim window for future coins.
pub fn handle_update_config(ctx: Context<UpdateConfig>, params: UpdateConfigParams) -> Result<()> {
    let config: &mut Config = &mut ctx.accounts.config;
    for (slot, new) in [
        (&mut config.treasury, params.treasury),
        (&mut config.charity, params.charity),
        (&mut config.verifier, params.verifier),
        (&mut config.raydium_amm_config, params.raydium_amm_config),
    ] {
        if let Some(key) = new {
            require_keys_neq!(key, Pubkey::default(), ErrorCode::InvalidAddress);
            *slot = key;
        }
    }
    if let Some(secs) = params.claim_window_secs {
        require!(secs > 0, ErrorCode::InvalidClaimWindow);
        config.claim_window_secs = secs;
    }

    emit!(ConfigUpdated {
        treasury: config.treasury,
        charity: config.charity,
        verifier: config.verifier,
        raydium_amm_config: config.raydium_amm_config,
        claim_window_secs: config.claim_window_secs,
    });
    Ok(())
}
