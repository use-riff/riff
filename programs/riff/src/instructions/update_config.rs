use anchor_lang::prelude::*;

use crate::{constants::*, error::ErrorCode, events::ConfigUpdated, state::Config};

/// Addresses to rotate; `None` leaves one unchanged. Fees and curve settings
/// can't be changed here, and nothing here can touch funds held by coins.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateConfigParams {
    pub treasury: Option<Pubkey>,
    pub charity: Option<Pubkey>,
    pub verifier: Option<Pubkey>,
    pub raydium_amm_config: Option<Pubkey>,
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

/// Lets the admin replace a lost or leaked key: most importantly the
/// verifier, which can link any unclaimed coin to a wallet.
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

    emit!(ConfigUpdated {
        treasury: config.treasury,
        charity: config.charity,
        verifier: config.verifier,
        raydium_amm_config: config.raydium_amm_config,
    });
    Ok(())
}
