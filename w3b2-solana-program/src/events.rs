use anchor_lang::prelude::*;

use crate::state::PriceEntry;

// --- Admin Events ---

/// Emitted when a new AdminProfile PDA is created.
/// This signifies that a new service has been registered on the protocol.
#[event]
#[derive(Debug, Clone)]
pub struct AdminProfileRegistered {
    /// The public key of the admin's wallet (`authority`), which serves as the unique owner
    /// and signer for the `AdminProfile` PDA.
    pub authority: Pubkey,
    /// The public key of the `AdminProfile` PDA that was registered.
    pub admin_pda: Pubkey,
    /// The public key provided by the admin for secure off-chain communication,
    /// typically used for hybrid encryption.
    pub communication_pubkey: Pubkey,
    /// The Unix timestamp (in seconds) when the registration occurred.
    pub ts: i64,
}

/// Emitted when an admin updates their off-chain communication public key.
#[event]
#[derive(Debug, Clone)]
pub struct AdminCommKeyUpdated {
    /// The public key of the admin's wallet (`authority`) that authorized this update.
    pub authority: Pubkey,
    /// The public key of the `AdminProfile` PDA that was updated.
    pub admin_pda: Pubkey,
    /// The new communication public key that has been set for the `AdminProfile`.
    pub new_comm_pubkey: Pubkey,
    /// The Unix timestamp of the update.
    pub ts: i64,
}

/// Emitted when an admin updates their service prices.
#[event]
#[derive(Debug, Clone)]
pub struct AdminPricesUpdated {
    /// The public key of the `AdminProfile`'s owner (the admin's `authority` wallet).
    pub authority: Pubkey,
    /// The public key of the `AdminProfile` PDA that was updated.
    pub admin_pda: Pubkey,
    /// The new price list for the service, as a vector of `PriceEntry` structs.
    pub new_prices: Vec<PriceEntry>,
    /// The Unix timestamp of the price update.
    pub ts: i64,
}

/// Emitted when an admin withdraws earned funds from their profile's internal balance.
#[event]
#[derive(Debug, Clone)]
pub struct AdminFundsWithdrawn {
    /// The public key of the admin's wallet (`authority`) who initiated the withdrawal.
    pub authority: Pubkey,
    /// The public key of the `AdminProfile` PDA from which funds were withdrawn.
    pub admin_pda: Pubkey,
    /// The amount of lamports withdrawn from the `AdminProfile`'s internal balance.
    pub amount: u64,
    /// The public key of the wallet that received the withdrawn funds.
    pub destination: Pubkey,
    /// The Unix timestamp of the withdrawal.
    pub ts: i64,
}

/// Emitted when an `AdminProfile` PDA is closed, effectively unregistering the service.
#[event]
#[derive(Debug, Clone)]
pub struct AdminProfileClosed {
    /// The public key of the admin's wallet (`authority`) whose profile was closed.
    pub authority: Pubkey,
    /// The public key of the `AdminProfile` **PDA** that was closed.
    pub admin_pda: Pubkey,
    /// The Unix timestamp of the account closure.
    pub ts: i64,
}

/// Emitted when an admin sends a command (notification) to a user.
#[event]
#[derive(Debug, Clone)]
pub struct AdminCommandDispatched {
    /// The public key of the admin's wallet (`authority`), who is the initiator of this command.
    pub sender: Pubkey,
    /// The public key of the sender's `AdminProfile` PDA.
    pub sender_admin_pda: Pubkey,
    /// The public key of the target `UserProfile` **PDA**.
    pub target_user_pda: Pubkey,
    /// A `u64` identifier for the specific command or notification being sent.
    pub command_id: u64,
    /// An opaque byte array containing application-specific data for the command.
    pub payload: Vec<u8>,
    /// The Unix timestamp when the command was dispatched.
    pub ts: i64,
}

// --- User Lifecycle & Financial Events ---

/// Emitted when a new `UserProfile` PDA is created, linking a user to a specific admin.
#[event]
#[derive(Debug, Clone)]
pub struct UserProfileCreated {
    /// The public key of the user's wallet (`authority`), which is the sole owner of this `UserProfile` PDA.
    pub authority: Pubkey,
    /// The public key of the `UserProfile` PDA that was created.
    pub user_pda: Pubkey,
    /// The public key of the `AdminProfile` **PDA** that this `UserProfile` is associated with.
    pub target_admin_pda: Pubkey,
    /// The public key provided by the user for secure off-chain communication.
    pub communication_pubkey: Pubkey,
    /// The Unix timestamp of the profile creation.
    pub ts: i64,
}

/// Emitted when a user updates their off-chain communication public key.
#[event]
#[derive(Debug, Clone)]
pub struct UserCommKeyUpdated {
    /// The public key of the user's wallet (`authority`) who authorized this update.
    pub authority: Pubkey,
    /// The PDA of the user profile that was updated.
    pub user_profile_pda: Pubkey,
    /// The new communication public key for the `UserProfile`.
    pub new_comm_pubkey: Pubkey,
    /// The Unix timestamp of the update.
    pub ts: i64,
}

/// Emitted when a user deposits funds into their `UserProfile` to pay for services.
#[event]
#[derive(Debug, Clone)]
pub struct UserFundsDeposited {
    /// The public key of the user's wallet (`authority`) who made the deposit.
    pub authority: Pubkey,
    /// The PDA of the user profile that received the deposit.
    pub user_profile_pda: Pubkey,
    /// The amount of lamports deposited into the `UserProfile`.
    pub amount: u64, // This is correct, it's the amount for this specific deposit.
    /// The user's new total `deposit_balance` after this transaction.
    pub new_deposit_balance: u64,
    /// The Unix timestamp of the deposit.
    pub ts: i64,
}

/// Emitted when a user withdraws unspent funds from their `UserProfile`.
#[event]
#[derive(Debug, Clone)]
pub struct UserFundsWithdrawn {
    /// The public key of the user's wallet (`authority`) who made the withdrawal.
    pub authority: Pubkey,
    /// The PDA of the user profile from which funds were withdrawn.
    pub user_profile_pda: Pubkey,
    /// The amount of lamports withdrawn from the `UserProfile`.
    pub amount: u64,
    /// The public key of the wallet that received the funds.
    pub destination: Pubkey,
    /// The user's new total `deposit_balance` after this transaction.
    pub new_deposit_balance: u64,
    /// The Unix timestamp of the withdrawal.
    pub ts: i64,
}

/// Emitted when a `UserProfile` PDA is closed.
#[event]
#[derive(Debug, Clone)]
pub struct UserProfileClosed {
    /// The public key of the user's wallet (`authority`) whose profile was closed.
    pub authority: Pubkey,
    /// The public key of the `UserProfile` PDA that was closed.
    pub user_pda: Pubkey,
    /// The public key of the `AdminProfile` **PDA** this profile was linked to.
    pub admin_pda: Pubkey,
    /// The Unix timestamp of the account closure.
    pub ts: i64,
}

// --- Operational Events ---

/// Emitted when a user calls a service's command, potentially a paid one.
#[event]
#[derive(Debug, Clone)]
pub struct UserCommandDispatched {
    /// The public key of the user's wallet (`authority`), who is the initiator of the command.
    pub sender: Pubkey,
    /// The public key of the sender's `UserProfile` PDA.
    pub sender_user_pda: Pubkey,
    /// The public key of the target `AdminProfile` **PDA**.
    pub target_admin_pda: Pubkey,
    /// A `u16` identifier for the specific command being executed.
    pub command_id: u16,
    /// The amount in lamports deducted from the user's deposit balance for this command (0 if free).
    pub price_paid: u64,
    /// An opaque byte array containing application-specific data for the command.
    pub payload: Vec<u8>,
    /// The Unix timestamp when the command was dispatched.
    pub ts: i64,
}

/// A generic event for logging significant off-chain actions for auditing purposes.
#[event]
#[derive(Debug, Clone)]
pub struct OffChainActionLogged {
    /// The public key of the wallet (`authority` of either a user or admin) that performed the off-chain action.
    pub actor: Pubkey,
    /// The public key of the `UserProfile` PDA involved in this action.
    pub user_profile_pda: Pubkey,
    /// The public key of the `AdminProfile` PDA involved in this action.
    pub admin_profile_pda: Pubkey,
    /// A `u64` identifier used to correlate multiple off-chain actions to a single on-chain session.
    pub session_id: u64,
    /// A `u16` code representing the specific type of off-chain action taken (e.g., 200 for HTTP OK).
    pub action_code: u16,
    /// The Unix timestamp of the logged action.
    pub ts: i64,
}
