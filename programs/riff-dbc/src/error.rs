use anchor_lang::prelude::*;

#[error_code]
pub enum RiffDbcError {
    #[msg("Artist ID is empty or too long")]
    InvalidArtistId,
    #[msg("Artist name is empty or too long")]
    InvalidArtistName,
    #[msg("The launcher's share must be at most 100%")]
    InvalidShare,
    #[msg("This coin's artist has already claimed it")]
    AlreadyClaimed,
    #[msg("The claim window for this coin has closed")]
    ClaimWindowClosed,
    #[msg("The claim window for this coin is still open")]
    ClaimWindowOpen,
    #[msg("That isn't this coin's pool")]
    WrongPool,
    #[msg("That token account belongs to someone else")]
    WrongReceiver,
    #[msg("Only the program's upgrade authority can initialize it")]
    NotUpgradeAuthority,
}
