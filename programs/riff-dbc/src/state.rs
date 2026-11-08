use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    /// riff's verification service: co-signs artist claims.
    pub verifier: Pubkey,
    /// Receives artist fees nobody claimed in time.
    pub charity: Pubkey,
    /// Receives riff's part of the partner share.
    pub treasury: Pubkey,
    /// The Meteora DBC config every riff coin is created with.
    pub dbc_config: Pubkey,
    /// Of the partner share, the part for whoever launched the coin (in bps).
    pub launcher_share_bps: u16,
    /// The most SOL (in lamports) the launcher's launch buy may spend.
    pub max_launch_buy: u64,
    pub claim_window_secs: i64,
    pub bump: u8,
    pub fee_authority_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Coin {
    pub mint: Pubkey,
    /// The coin's Meteora pool.
    pub pool: Pubkey,
    pub launcher: Pubkey,
    #[max_len(64)]
    pub artist_id: String,
    #[max_len(64)]
    pub artist_name: String,
    /// The artist's passport vault, set once they claim. Their share is paid
    /// there.
    pub artist: Option<Pubkey>,
    pub created_at: i64,
    pub claim_deadline: i64,
    /// Artist fees collected so far (held, paid out, or sent to charity).
    pub artist_fees_total: u64,
    pub launcher_fees_total: u64,
    pub bump: u8,
    pub escrow_bump: u8,
    /// Of the partner share, the part for the launcher (in bps), fixed when
    /// the coin was created so a config change can't cut it later. `None`
    /// only for devnet coins from before this field (they read the zero
    /// padding after the strings): they use the config's share.
    pub launcher_share_bps: Option<u16>,
}

impl Coin {
    pub fn launcher_share_bps(&self, config: &Config) -> u16 {
        self.launcher_share_bps.unwrap_or(config.launcher_share_bps)
    }
}

/// A proposed new admin, waiting for that key to accept. A separate account,
/// so the config's layout stays as deployed.
#[account]
#[derive(InitSpace)]
pub struct PendingAdmin {
    pub new_admin: Pubkey,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfigParams {
    pub verifier: Pubkey,
    pub charity: Pubkey,
    pub treasury: Pubkey,
    pub dbc_config: Pubkey,
    pub launcher_share_bps: u16,
    pub max_launch_buy: u64,
    pub claim_window_secs: i64,
}

/// Everything but the Meteora config, which every existing coin depends on,
/// and the admin, which changes with `propose_admin` and `accept_admin`.
/// A new claim window and launcher share apply to coins created after the
/// change.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfigUpdate {
    pub verifier: Pubkey,
    pub charity: Pubkey,
    pub treasury: Pubkey,
    pub launcher_share_bps: u16,
    pub max_launch_buy: u64,
    pub claim_window_secs: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateCoinArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub artist_id: String,
    pub artist_name: String,
}
