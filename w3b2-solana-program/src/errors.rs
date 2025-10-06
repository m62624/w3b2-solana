use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    /// Used when the transaction signer does not match the `authority` field of a profile.
    #[msg("Signer is not the authorized authority for this profile.")]
    SignerUnauthorized,

    /// Used when a UserProfile is passed with an incorrect AdminProfile.
    #[msg("Admin Mismatch: The provided UserProfile is not associated with the provided AdminProfile.")]
    AdminMismatch,

    /// Used when a user's `deposit_balance` is insufficient for a paid command.
    #[msg(
        "Insufficient Deposit Balance: The user's deposit is not enough to pay for this command."
    )]
    InsufficientDepositBalance,

    /// Used when an admin's internal `balance` is not enough to cover a withdrawal.
    #[msg("Insufficient Admin Balance: The admin's internal balance is not enough to cover the withdrawal amount.")]
    InsufficientAdminBalance,

    /// Used when a transaction would leave a PDA with lamports below the rent-exempt minimum.
    #[msg("Rent-Exempt Violation: This transaction would leave the PDA with a balance below the rent-exempt minimum.")]
    RentExemptViolation,

    /// Used when a `command_id` is not found in the admin's price list.
    #[msg("Command Not Found: The requested command_id does not exist in the admin's price list.")]
    CommandNotFound,

    /// Used when the `payload` in a dispatch command exceeds the maximum allowed size.
    #[msg("Payload Too Large: The provided payload exceeds the maximum allowed size.")]
    PayloadTooLarge,
}
