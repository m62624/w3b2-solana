use crate::errors::BridgeError;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;

// --- Account Data Structs ---

/// Represents the on-chain profile for a Service Provider (an "Admin").
/// This PDA holds the service's configuration, and serves as a treasury for
/// collected fees. Its address is derived from the admin's wallet key.
#[account]
#[derive(Debug)]
pub struct AdminProfile {
    /// The public key of the admin's wallet, which is the sole `authority`
    /// allowed to manage this profile (e.g., withdraw funds).
    pub authority: Pubkey,
    /// A public key provided by the admin for secure off-chain key exchange,
    /// typically used for hybrid encryption with users.
    pub communication_pubkey: Pubkey,
    /// The public key of the off-chain oracle responsible for signing price data.
    /// The program will only accept price information signed by this authority.
    pub oracle_authority: Pubkey,
    /// The duration in seconds for which an oracle's signature is considered valid.
    /// This helps prevent replay attacks with old price data.
    pub timestamp_validity_seconds: i64,
    /// The internal balance in lamports where fees from paid user commands are collected.
    /// This balance can be withdrawn by the admin via the `admin_withdraw` instruction.
    pub balance: u64,
}

/// Represents a user's on-chain profile for a *specific* Admin service.
/// A single user wallet (`authority`) can have multiple `UserProfile` PDAs, one for each
/// service they interact with. This PDA holds the user's prepaid balance for that service.
#[account]
#[derive(Debug)]
pub struct UserProfile {
    /// The public key of the user's wallet, which is the sole `authority`
    /// allowed to manage this profile (e.g., deposit/withdraw funds, close profile).
    pub authority: Pubkey,
    /// A public key provided by the user for secure off-chain key exchange.
    pub communication_pubkey: Pubkey,
    /// The public key of the `AdminProfile` **PDA** this user profile was created for.
    /// This field permanently links the user's profile to a specific service.
    pub admin_profile_on_creation: Pubkey,
    /// The user's prepaid balance in lamports for this specific service. This balance
    /// is debited by the `user_dispatch_command` instruction.
    pub deposit_balance: u64,
}

// --- Instruction Accounts Structs ---

// --- Admin Instructions ---

/// Defines the accounts required for the `admin_register_profile` instruction.
#[derive(Accounts)]
pub struct AdminRegisterProfile<'info> {
    /// The `Signer` (the admin's wallet) who will become the owner of the new `AdminProfile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The new `AdminProfile` account to be initialized. Its address is a PDA
    /// derived from the `authority`'s key.
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AdminProfile>(), // Anchor will calculate this correctly
        seeds = [b"admin", authority.key().as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The Solana System Program, required by Anchor for account creation (`init`).
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `admin_withdraw` instruction.
#[derive(Accounts)]
pub struct AdminWithdraw<'info> {
    /// The `Signer` (the admin's wallet) who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` from which funds will be withdrawn. Constraints
    /// verify the `authority` and the PDA seeds.
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The account that will receive the withdrawn lamports. It is marked as `mut`
    /// because its lamport balance will be increased.
    /// CHECK: This is safe because it's only used as a destination for a lamport transfer
    /// from a program-controlled PDA, and does not require data deserialization.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
}

/// Defines the accounts for the `admin_set_config` instruction.
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

/// Defines the accounts for the `admin_close_profile` instruction.
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

/// Defines the accounts for the `admin_dispatch_command` instruction.
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

/// Defines the accounts for the `user_create_profile` instruction.
#[derive(Accounts)]
#[instruction(target_admin_pda: Pubkey)]
pub struct UserCreateProfile<'info> {
    /// The `Signer` (the user's wallet) who will become the owner of the new `UserProfile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` PDA that this new user profile will be linked to.
    /// This account is not mutated, but it is read to ensure it exists and is a valid profile.
    #[account(seeds = [b"admin", admin_profile.authority.as_ref()], bump)]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The new `UserProfile` account to be initialized. Its address is a PDA
    /// derived from the user's `authority` key and the `admin_profile`'s PDA key.
    /// The `target_admin_pda` from the instruction must match `admin_profile.key()`.
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

/// Defines the accounts for the `user_deposit` instruction.
#[derive(Accounts)]
pub struct UserDeposit<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`. This is required
    /// to derive and verify the `user_profile` **PDA** address.
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
    /// The System Program, required for the underlying lamport transfer.
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `user_withdraw` instruction.
#[derive(Accounts)]
pub struct UserWithdraw<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`.
    pub admin_profile: Account<'info, AdminProfile>, // Required to derive the user_profile PDA.
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
    /// CHECK: This is safe because it's only a destination for a lamport transfer.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
}

/// Defines the accounts for the `user_update_comm_key` instruction.
#[derive(Accounts)]
pub struct UserUpdateCommKey<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` account to be updated.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// Defines the accounts for the `user_close_profile` instruction.
#[derive(Accounts)]
pub struct UserCloseProfile<'info> {
    /// The `Signer` (the user's wallet) who must be the `authority` of the `user_profile`.
    /// This account will receive the refunded lamports.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` account to be closed. The `close` directive will transfer
    /// all its lamports to the `authority`.
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

/// Defines the accounts for the `user_dispatch_command` instruction.
#[derive(Accounts)]
pub struct UserDispatchCommand<'info> {
    /// The `Signer` of the transaction (the user's wallet).
    pub authority: Signer<'info>,
    /// The user's profile PDA. Constraints ensure the `authority` is the owner
    /// and that this profile is linked to the provided `admin_profile` via its seeds.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The target `AdminProfile` of the service being called. Its seeds are
    /// checked to ensure it's a valid profile created by this program.
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The Instructions sysvar, used to verify the preceding instruction.
    /// CHECK: This is safe because we are only reading from it.
    #[account(address = solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// Defines the accounts for the `log_action` instruction.
#[derive(Accounts)]
pub struct LogAction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The user's profile PDA.
    #[account(
        mut,
        seeds = [b"user", user_profile.authority.as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.admin_profile_on_creation == admin_profile.key() @ BridgeError::AdminMismatch,
        constraint = (user_profile.authority == authority.key() || admin_profile.authority == authority.key()) @ BridgeError::SignerUnauthorized,
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump,
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}
