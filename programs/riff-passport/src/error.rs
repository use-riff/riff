use anchor_lang::prelude::*;

#[error_code]
pub enum PassportError {
    #[msg("Signer is not the program upgrade authority")]
    NotUpgradeAuthority,
    #[msg("Signer is not the passport config admin")]
    NotAdmin,
    #[msg("Signer is not riff's verifier")]
    NotVerifier,
    #[msg("Invalid config value")]
    InvalidConfig,
    #[msg("Artist ID must be spotify: plus a 22-character Spotify ID")]
    InvalidArtistId,
    #[msg("A proof doesn't belong to this artist and wallet")]
    ProofMismatch,
    #[msg("A proof is too old; prove it again")]
    ProofExpired,
    #[msg("Needs at least 2 proofs from different sources, including a strong one")]
    NotEnoughProofs,
    #[msg("Too many proofs")]
    TooManyProofs,
    #[msg("The passport has been revoked")]
    Revoked,
    #[msg("Signer is not the passport's wallet")]
    NotPassportWallet,
    #[msg("No matching passkey signature in this transaction")]
    PasskeyMissing,
    #[msg("The passkey signature is for a different action")]
    PasskeyWrongChallenge,
    #[msg("The passkey signature is for a different site")]
    PasskeyWrongSite,
    #[msg("The passkey wasn't used with the user present and verified")]
    PasskeyNotVerified,
    #[msg("Malformed passkey data")]
    PasskeyMalformed,
    #[msg("A recovery is already pending")]
    RecoveryPending,
    #[msg("No recovery is pending")]
    NoRecovery,
    #[msg("The recovery time-lock hasn't passed yet")]
    RecoveryLocked,
    #[msg("This proof can't veto the recovery")]
    NotAGuardian,
    #[msg("Withdrawal needs the passkey")]
    PasskeyRequired,
    #[msg("The vault doesn't hold that much")]
    InsufficientVault,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("The passport is new: endorsing and withdrawing unlock after its waiting period")]
    OnProbation,
    #[msg("Only a revoked passport can be reissued")]
    NotRevoked,
}
