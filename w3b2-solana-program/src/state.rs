//! # State & Account Structs
//!
//! This module defines the core data structures used by the on-chain program.
//! It includes:
//!
//! 1.  **Account Data Structs (`AdminProfile`, `UserProfile`):** These define the shape of the
//!     on-chain data stored in Program-Derived Address (PDA) accounts. They represent the
//!     state of service providers and their users.
//!
//! 2.  **Instruction Accounts Structs (e.g., `AdminRegisterProfile`):** These are `#[derive(Accounts)]`
//!     structs that define the set of accounts required by each on-chain instruction. They
//!     enforce security constraints, such as ensuring signers are authorized and that PDAs
//!     are derived correctly.

use crate::errors::BridgeError;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;

// --- Account Data Structs ---

/// # Admin Profile
///
/// Represents the on-chain profile for a Service Provider (an "Admin").
///
/// This PDA holds the service's configuration and serves as a treasury for collected fees.
/// Its address is deterministically derived from the admin's wallet key, ensuring that
/// each admin can only have one profile.
///
/// - **PDA Seeds:** `[b"admin", authority.key().as_ref()]`
#[account]
#[derive(Debug)]
pub struct AdminProfile {
    /// The public key of the admin's wallet. This key is the sole `authority`
    /// allowed to manage this profile (e.g., withdraw funds, change configuration).
    pub authority: Pubkey,
    /// A public key provided by the admin for secure off-chain key exchange,
    /// typically used for hybrid encryption with users.
    pub communication_pubkey: Pubkey,
    /// The public key of the off-chain oracle responsible for signing price data.
    /// The program will only accept price information signed by this authority.
    /// By default, this is set to the `authority` key.
    pub oracle_authority: Pubkey,
    /// The duration in seconds for which an oracle's signature is considered valid.
    /// This helps prevent replay attacks with old price data.
    pub timestamp_validity_seconds: i64,
    /// The internal balance in lamports where fees from paid user commands are collected.
    /// This balance can be withdrawn by the admin via the `admin_withdraw` instruction.
    pub balance: u64,
    /// The fee in lamports that a banned user must pay to request an unban.
    /// This can be configured by the admin.
    pub unban_fee: u64,
}

/// # User Profile
///
/// Represents a user's on-chain profile for a *specific* Admin service.
///
/// A single user wallet (`authority`) can have multiple `UserProfile` PDAs—one for each
/// service they interact with. This PDA holds the user's prepaid balance for that service
/// and tracks their status (e.g., `banned`).
///
/// - **PDA Seeds:** `[b"user", authority.key().as_ref(), admin_profile.key().as_ref()]`
#[account]
#[derive(Debug)]
pub struct UserProfile {
    /// The public key of the user's wallet. This key is the sole `authority`
    /// allowed to manage this profile (e.g., deposit/withdraw funds, close profile).
    pub authority: Pubkey,
    /// A public key provided by the user for secure off-chain key exchange.
    pub communication_pubkey: Pubkey,
    /// The public key of the `AdminProfile` **PDA** this user profile was created for.
    /// This field permanently and verifiably links the user's profile to a specific service.
    pub admin_profile_on_creation: Pubkey,
    /// The user's prepaid balance in lamports for this specific service. This balance
    /// is debited by the `user_dispatch_command` instruction to pay for services.
    pub deposit_balance: u64,
    /// A flag indicating whether the user is banned from using the service.
    /// If `true`, most user-initiated actions will be blocked by the program.
    pub banned: bool,
    /// A flag indicating that the user has paid the `unban_fee` and requested to be unbanned.
    /// This does not automatically lift the ban; it only signals the request to the admin.
    pub unban_requested: bool,
}

// --- Instruction Accounts Structs ---

// --- Admin Instructions ---

/// # Accounts for `admin_register_profile`
///
/// Defines the accounts required to initialize a new `AdminProfile` for a service provider.
#[derive(Accounts)]
pub struct AdminRegisterProfile<'info> {
    /// The `Signer` (the admin's wallet) who will become the owner of the new `AdminProfile`.
    /// This account pays for the creation of the `admin_profile` PDA.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The new `AdminProfile` account to be initialized. Its address is a PDA
    /// derived from the `authority`'s key, ensuring one profile per admin.
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AdminProfile>(),
        seeds = [b"admin", authority.key().as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The Solana System Program, required by Anchor for account creation (`init`).
    pub system_program: Program<'info, System>,
}

/// # Accounts for `admin_withdraw`
///
/// Defines the accounts required for an admin to withdraw collected fees
/// from their `AdminProfile`'s internal balance.
#[derive(Accounts)]
pub struct AdminWithdraw<'info> {
    /// The `Signer` (the admin's wallet) who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` from which funds will be withdrawn. Constraints
    /// verify that the `authority` is the legitimate owner and the PDA seeds are correct.
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The account that will receive the withdrawn lamports. It is marked as `mut`
    /// because its lamport balance will be increased.
    ///
    /// **Security:** `CHECK:` is used here because this account is only a destination for a
    /// lamport transfer from a program-controlled PDA. No data is read from or written to it,
    /// so deserialization into a typed account is unnecessary and inefficient.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
}

/// # Accounts for `admin_set_config`
///
/// Defines the accounts required for an admin to update their `AdminProfile`'s configuration,
/// such as the oracle key or timestamp validity period.
#[derive(Accounts)]
pub struct AdminSetConfig<'info> {
    /// The `Signer` (the admin's wallet) who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` account to be updated. Constraints verify the `authority`
    /// and the account's PDA seeds.
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}

/// # Accounts for `admin_ban_user`
///
/// Defines the accounts required for an admin to ban a user, preventing them
/// from interacting with the service.
#[derive(Accounts)]
pub struct AdminBanUser<'info> {
    /// The `Signer` (the admin's wallet) who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` of the admin initiating the ban. Constraints verify
    /// the `authority` and the PDA seeds.
    #[account(
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` to be banned. This account will be mutated to set `banned = true`.
    /// A constraint ensures this user profile is associated with this specific `admin_profile`.
    #[account(
        mut,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// # Accounts for `admin_unban_user`
///
/// Defines the accounts required for an admin to unban a user, restoring their
/// access to the service.
#[derive(Accounts)]
pub struct AdminUnbanUser<'info> {
    /// The `Signer` (the admin's wallet) who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` of the admin initiating the unban. Constraints verify
    /// the `authority` and the PDA seeds.
    #[account(
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` to be unbanned. This account will be mutated to set `banned = false`.
    /// A constraint ensures this user profile is associated with this specific `admin_profile`.
    #[account(
        mut,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// # Accounts for `user_request_unban`
///
/// Defines the accounts required for a banned user to pay a fee and request an unban.
#[derive(Accounts)]
pub struct UserRequestUnban<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`. Its internal balance
    /// will be credited with the `unban_fee`.
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` requesting the unban. Its `deposit_balance` will be debited
    /// for the fee, and `unban_requested` flag will be set to `true`.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// # Accounts for `admin_close_profile`
///
/// Defines the accounts required to close an `AdminProfile` and reclaim its rent lamports.
#[derive(Accounts)]
pub struct AdminCloseProfile<'info> {
    /// The `Signer` (the admin's wallet) who must be the `authority` of the `admin_profile`.
    /// This account will receive the rent lamports back from the closed account.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` account to be closed. Constraints verify the `authority`
    /// and PDA seeds. The `close` directive tells Anchor to return all lamports
    /// from this account to the `authority`.
    #[account(
        mut,
        close = authority,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}

/// # Accounts for `admin_dispatch_command`
///
/// Defines the accounts for an admin to send a non-financial command to a user.
/// This is used for notifications or other non-billable interactions.
#[derive(Accounts)]
pub struct AdminDispatchCommand<'info> {
    /// The `Signer` of the transaction (the admin's wallet).
    pub admin_authority: Signer<'info>,
    /// The admin's own profile PDA. Constraints ensure that the `admin_authority`
    /// is the legitimate owner of this profile.
    #[account(
        seeds = [b"admin", admin_authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == admin_authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The target `UserProfile` to which the command is being sent. A constraint
    /// ensures this profile is associated with this specific `admin_profile`.
    #[account(
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

// --- User Instructions ---

/// # Accounts for `user_create_profile`
///
/// Defines the accounts to create a `UserProfile`, linking a user's wallet to a specific admin service.
#[derive(Accounts)]
#[instruction(target_admin_pda: Pubkey)]
pub struct UserCreateProfile<'info> {
    /// The `Signer` (the user's wallet) who will become the owner of the new `UserProfile`.
    /// This account pays for the creation of the `user_profile` PDA.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` PDA that this new user profile will be linked to.
    /// This account is read-only but its existence and PDA derivation are verified.
    #[account(seeds = [b"admin", admin_profile.authority.as_ref()], bump)]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The new `UserProfile` account to be initialized. Its address is a PDA
    /// derived from the user's `authority` key and the `admin_profile`'s PDA key,
    /// ensuring a unique profile for each user-service pair.
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<UserProfile>(),
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = target_admin_pda == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The Solana System Program, required by Anchor for account creation (`init`).
    pub system_program: Program<'info, System>,
}

/// # Accounts for `user_deposit`
///
/// Defines the accounts for a user to deposit lamports into their `UserProfile`'s
/// internal `deposit_balance`.
#[derive(Accounts)]
pub struct UserDeposit<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    /// This account is the source of the deposited funds.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`. This is required
    /// to correctly derive and verify the `user_profile` PDA address.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` to receive the deposit. Constraints verify the PDA seeds
    /// (linking it to the `authority` and `admin_profile`) and ownership.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The System Program, required to perform the lamport transfer from the user's
    /// wallet to their profile PDA.
    pub system_program: Program<'info, System>,
}

/// # Accounts for `user_withdraw`
///
/// Defines the accounts for a user to withdraw unspent funds from their
/// `UserProfile`'s `deposit_balance`.
#[derive(Accounts)]
pub struct UserWithdraw<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`, required to derive the user PDA.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` from which funds will be withdrawn.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The account that will receive the withdrawn lamports.
    ///
    /// **Security:** `CHECK:` is used here because this account is only a destination for a
    /// lamport transfer. No data is read from or written to it.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
}

/// # Accounts for `user_update_comm_key`
///
/// Defines the accounts for a user to update their `communication_pubkey`.
#[derive(Accounts)]
pub struct UserUpdateCommKey<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`, required to derive the user PDA.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` account whose `communication_pubkey` will be updated.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// # Accounts for `user_close_profile`
///
/// Defines the accounts to close a `UserProfile`, reclaiming its rent and any remaining deposit balance.
#[derive(Accounts)]
pub struct UserCloseProfile<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    /// This account will receive all lamports from the closed account.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`, required to derive the user PDA.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` account to be closed. The `close` directive will transfer
    /// all its lamports (rent and deposit balance) to the `authority`.
    #[account(
        mut,
        close = authority,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// # Accounts for `user_dispatch_command`
///
/// Defines the accounts for a user to call a paid service command. This is the primary
/// operational instruction for user-service interaction.
#[derive(Accounts)]
pub struct UserDispatchCommand<'info> {
    /// The `Signer` of the transaction (the user's wallet).
    pub authority: Signer<'info>,
    /// The user's profile PDA. It is debited for the command `price`. Constraints ensure
    /// the `authority` is the owner and the profile is linked to the correct `admin_profile`.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The target `AdminProfile` of the service being called. It is credited with the
    /// command `price`. Its seeds are checked to ensure it's a valid profile.
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The Instructions sysvar (`ixs`), used to verify the preceding `Ed25519Program`
    /// instruction for oracle signature authentication.
    ///
    /// **Security:** `CHECK:` is used as we are only reading instruction data from this
    /// sysvar account, not deserializing its data.
    #[account(address = solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// # Accounts for `log_action`
///
/// Defines the accounts for logging a significant off-chain action to the blockchain.
/// This creates an immutable, auditable record.
#[derive(Accounts)]
pub struct LogAction<'info> {
    /// The `Signer` of the transaction, who can be either the user or the admin associated
    /// with the profiles.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `UserProfile` associated with the action being logged.
    #[account(
        mut,
        seeds = [b"user", user_profile.authority.as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch,
        constraint = (user_profile.authority == authority.key() || admin_profile.authority == authority.key()) @ BridgeError::SignerUnauthorized,
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The `AdminProfile` associated with the action being logged.
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump,
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}
