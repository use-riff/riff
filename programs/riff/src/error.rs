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
}
