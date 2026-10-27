use anchor_lang::prelude::*;

#[error_code]
pub enum RiffDbcError {
    #[msg("Artist ID is empty or too long")]
    InvalidArtistId,
    #[msg("Artist name is empty or too long")]
    InvalidArtistName,
    #[msg("A share must be at most 100%")]
    InvalidShare,
    #[msg("The claim window must be positive")]
    InvalidClaimWindow,
    #[msg("This coin's artist has already claimed it")]
    AlreadyClaimed,
    #[msg("This coin's artist hasn't claimed it")]
    NotClaimed,
    #[msg("The claim window for this coin is still open")]
    ClaimWindowOpen,
    #[msg("That isn't this coin's pool")]
    WrongPool,
    #[msg("That token account belongs to someone else")]
    WrongReceiver,
    #[msg("That isn't this coin's artist vault")]
    WrongArtistVault,
    #[msg("That wallet isn't the passport's")]
    NotPassportWallet,
    #[msg("The passport is revoked")]
    PassportRevoked,
    #[msg("Only exact-in Meteora swaps can go with a coin's creation")]
    LaunchBuyNotSwap,
    #[msg("The launch buy is over the cap")]
    LaunchBuyTooLarge,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Only the program's upgrade authority can initialize it")]
    NotUpgradeAuthority,
}
