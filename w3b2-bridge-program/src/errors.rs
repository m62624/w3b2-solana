use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    #[msg("Admin is not authorized to approve this request")]
    Unauthorized,
    #[msg("PDA already registered for this owner")]
    AlreadyRegistered,
    #[msg("Payload too large")]
    PayloadTooLarge,
    #[msg("Funding request has already been processed")]
    RequestAlreadyProcessed,
    #[msg("Insufficient funds in admin profile to approve funding request")]
    InsufficientFundsForFunding,
    #[msg("Insufficient funds to create admin profile PDA")]
    InsufficientFundsForAdmin,
}
