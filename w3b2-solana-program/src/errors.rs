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

    /// Used when the `payload` in a dispatch command exceeds the maximum allowed size.
    #[msg("Payload Too Large: The provided payload exceeds the maximum allowed size.")]
    PayloadTooLarge,

    /// Used when the preceding instruction is not the expected Ed25519 signature verification.
    #[msg("Instruction Mismatch: Expected an Ed25519 signature verification instruction.")]
    InstructionMismatch,

    /// Used when the signature in the Ed25519 instruction is invalid.
    #[msg("Signature Verification Failed: The oracle signature could not be verified.")]
    SignatureVerificationFailed,

    /// Used when the signer public key in the Ed25519 instruction does not match the admin's oracle authority.
    #[msg("Invalid Oracle Signer: The signer does not match the registered oracle authority.")]
    InvalidOracleSigner,

    /// Used when the timestamp in the signed message is too far in the past.
    #[msg("Timestamp Too Old: The provided timestamp is outside the acceptable time window.")]
    TimestampTooOld,

    /// Used when a user tries to perform an action while banned.
    #[msg("User Is Banned: This action cannot be performed because the user is banned.")]
    UserIsBanned,

    /// Used when an admin tries to unban a user who is not currently banned.
    #[msg("User Not Banned: This user is not currently banned.")]
    UserNotBanned,

    /// Used when a user who is already banned tries to request an unban again.
    #[msg("Unban Already Requested: An unban has already been requested for this user.")]
    UnbanAlreadyRequested,

    /// Used when an admin tries to ban their own user profile.
    #[msg("Cannot Ban Self: An admin cannot ban their own user profile.")]
    CannotBanSelf,
}
