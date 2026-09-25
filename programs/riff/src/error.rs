use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Signer is not the program upgrade authority")]
    NotUpgradeAuthority,
    #[msg("Artist fee share must be at most 10000 bps")]
    InvalidArtistFeeShare,
    #[msg("Claim window must be greater than zero")]
    InvalidClaimWindow,
    #[msg("Token name must be 1-32 bytes")]
    InvalidName,
    #[msg("Token symbol must be 1-10 bytes")]
    InvalidSymbol,
    #[msg("Metadata URI must be 1-200 bytes")]
    InvalidUri,
    #[msg("Artist ID must be 1-64 printable ASCII characters with no spaces")]
    InvalidArtistId,
    #[msg("Artist name must be 1-64 bytes")]
    InvalidArtistName,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
