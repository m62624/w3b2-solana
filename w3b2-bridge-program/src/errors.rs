use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    #[msg("PDA already registered for this owner")]
    AlreadyRegistered,
    #[msg("Payload too large")]
    PayloadTooLarge,
    #[msg("Unauthorized: signer does not match PDA owner or linked wallet")]
    Unauthorized,
    #[msg("Funding request has already been processed")]
    RequestAlreadyProcessed,
    #[msg("Insufficient funds in admin profile to approve funding request")]
    InsufficientFundsForRent,
}
