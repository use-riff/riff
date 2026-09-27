use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Signer is not the program upgrade authority")]
    NotUpgradeAuthority,
    #[msg("Claim window must be greater than zero")]
    InvalidClaimWindow,
    #[msg("Total trading fee must be at most 1000 bps")]
    InvalidTradeFee,
    #[msg("Creator buy cap must be at most 1000 bps of supply")]
    InvalidCreatorBuyCap,
    #[msg("Invalid bonding curve parameters")]
    InvalidCurveParams,
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
    #[msg("Amount is zero or too small to trade")]
    AmountTooSmall,
    #[msg("Price moved beyond the slippage limit")]
    SlippageExceeded,
    #[msg("Bonding curve is complete; trading is closed until graduation")]
    CurveComplete,
    #[msg("Creator launch buy exceeds the cap")]
    CreatorBuyTooLarge,
    #[msg("Trading opens the slot after launch")]
    TradingNotOpen,
    #[msg("No fees to withdraw")]
    NoFeesToWithdraw,
    #[msg("Collection would leave the treasury below the rent-exempt minimum")]
    TreasuryNotRentExempt,
    #[msg("Payout would leave the coin account unable to cover what it owes")]
    CoinUnderfunded,
    #[msg("Sweep would leave the charity wallet below the rent-exempt minimum")]
    CharityNotRentExempt,
    #[msg("A coin can't pay out to itself")]
    InvalidPayoutRecipient,
    #[msg("Address must not be the default (all zeros) key")]
    InvalidAddress,
    #[msg("Claim must be co-signed by the configured verifier")]
    NotVerifier,
    #[msg("This coin's artist has already claimed it")]
    AlreadyClaimed,
    #[msg("Artist ID doesn't match the coin's artist")]
    ArtistIdMismatch,
    #[msg("Only the coin's claimed artist can do this")]
    NotArtist,
    #[msg("Only the config admin can do this")]
    NotAdmin,
    #[msg("Only the proposed admin can accept")]
    NotPendingAdmin,
    #[msg("The curve hasn't sold out yet")]
    CurveNotComplete,
    #[msg("This coin has already graduated")]
    AlreadyGraduated,
    #[msg("The vault holds less than the graduation reserve")]
    ReserveMissing,
    #[msg("Account isn't the configured Raydium fee tier")]
    InvalidAmmConfig,
    #[msg("Graduation needs prepare_graduation first")]
    GraduationNotPrepared,
    #[msg("Graduation is already prepared")]
    GraduationAlreadyPrepared,
}
