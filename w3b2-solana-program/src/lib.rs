//! # W3B2 Solana Program
//!
//! The core on-chain smart contract for the W3B2 toolset.
//!
//! This Anchor program provides a secure and verifiable framework for Web2 services to
//! interact with the Solana blockchain. It enables services to manage user profiles,
//! handle payments, and dispatch commands in a non-custodial manner, where users
//! always retain control of their funds and sign all transactions with their own wallets.
//!
//! ## Key Concepts
//!
//! - **Admin & User Profiles:** The program establishes two primary PDA account types:
//!   - [`AdminProfile`]: Represents the service provider (the "Admin"). It holds configuration
//!     like the oracle key and serves as a treasury for collected fees.
//!   - [`UserProfile`]: Represents an end-user's relationship with a specific service. It
//!     holds the user's pre-paid deposit balance for that service.
//!
//! - **Non-Custodial Payments:** Users deposit funds into their own `UserProfile` PDA, which
//!   is controlled by the program. Payments for services are transferred from the user's
//!   profile to the admin's profile only upon the user's explicit, signed approval via
//!   the `user_dispatch_command` instruction.
//!
//! - **Off-Chain Oracle:** For dynamic pricing, the program uses an off-chain oracle pattern.
//!   The service's backend signs payment details (price, command, timestamp), and the on-chain
//!   program verifies this signature before processing a payment. This keeps business logic
//!   flexible and off-chain, while keeping value transfer secure and on-chain.
//!
//! - **Event-Driven Architecture:** The program emits detailed events for every significant
//!   action (e.g., [`UserProfileCreated`], [`UserCommandDispatched`]). Off-chain clients, like the
//!   `w3b2-solana-connector`, can listen for these events to synchronize state and trigger
//!   backend processes.
//!
//! ## Modules
//!
//! - [`instructions`]: Contains the business logic for each on-chain instruction.
//! - [`state`]: Defines the data structures for all on-chain accounts (PDAs).
//! - [`events`]: Declares all on-chain events emitted by the program.
//! - [`errors`]: Defines custom errors for clear and specific failure modes.

#![allow(deprecated)]
#![allow(unexpected_cfgs)]
#![allow(elided_lifetimes_in_paths)]

pub mod errors;
pub mod events;
pub mod instructions;
pub mod protocols;
pub mod state;

use anchor_lang::prelude::*;
use errors::*;
use events::*;
use state::*;

declare_id!("HykRMCadVCe49q4GVrXKTwLG3fqCEgd5W5qQqN3AFAEY");

/// # W3B2 Program Instruction Interface
///
/// Each public function in this module corresponds to a callable on-chain instruction.
/// The detailed logic for each instruction is implemented in the [`instructions`] module.
#[program]
pub mod w3b2_solana_program {
    use super::*;

    // --- Admin Instructions ---

    /// Initializes a new `AdminProfile` PDA for a service provider.
    /// See [`instructions::admin_register_profile`] for details.
    pub fn admin_register_profile(
        ctx: Context<AdminRegisterProfile>,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::admin_register_profile(ctx, communication_pubkey)
    }

    /// Closes an `AdminProfile` account and refunds its rent lamports to the owner.
    /// See [`instructions::admin_close_profile`] for details.
    pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
        instructions::admin_close_profile(ctx)
    }

    /// Sets or updates the configuration for an existing `AdminProfile`.
    /// See [`instructions::admin_set_config`] for details.
    pub fn admin_set_config(
        ctx: Context<AdminSetConfig>,
        new_oracle_authority: Option<Pubkey>,
        new_timestamp_validity: Option<i64>,
        new_communication_pubkey: Option<Pubkey>,
        new_unban_fee: Option<u64>,
    ) -> Result<()> {
        instructions::admin_set_config(
            ctx,
            new_oracle_authority,
            new_timestamp_validity,
            new_communication_pubkey,
            new_unban_fee,
        )
    }

    /// Withdraws earned funds from an `AdminProfile`'s internal balance.
    /// See [`instructions::admin_withdraw`] for details.
    pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
        instructions::admin_withdraw(ctx, amount)
    }

    /// Dispatches a non-financial command from an admin to a user.
    /// See [`instructions::admin_dispatch_command`] for details.
    pub fn admin_dispatch_command(
        ctx: Context<AdminDispatchCommand>,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::admin_dispatch_command(ctx, command_id, payload)
    }

    /// Bans a user, preventing them from interacting with the service.
    /// See [`instructions::admin_ban_user`] for details.
    pub fn admin_ban_user(ctx: Context<AdminBanUser>) -> Result<()> {
        instructions::admin_ban_user(ctx)
    }

    /// Unbans a user, restoring their access to the service.
    /// See [`instructions::admin_unban_user`] for details.
    pub fn admin_unban_user(ctx: Context<AdminUnbanUser>) -> Result<()> {
        instructions::admin_unban_user(ctx)
    }

    // --- User Instructions ---

    /// Creates a `UserProfile` PDA, linking a user's wallet to a specific admin service.
    /// See [`instructions::user_create_profile`] for details.
    pub fn user_create_profile(
        ctx: Context<UserCreateProfile>,
        target_admin_pda: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::user_create_profile(ctx, target_admin_pda, communication_pubkey)
    }

    /// Updates the `communication_pubkey` for an existing `UserProfile`.
    /// See [`instructions::user_update_comm_key`] for details.
    pub fn user_update_comm_key(ctx: Context<UserUpdateCommKey>, new_key: Pubkey) -> Result<()> {
        instructions::user_update_comm_key(ctx, new_key)
    }

    /// Closes a `UserProfile` account and refunds all lamports to the user.
    /// See [`instructions::user_close_profile`] for details.
    pub fn user_close_profile(ctx: Context<UserCloseProfile>) -> Result<()> {
        instructions::user_close_profile(ctx)
    }

    /// Deposits lamports into a `UserProfile` PDA to pre-fund future payments.
    /// See [`instructions::user_deposit`] for details.
    pub fn user_deposit(ctx: Context<UserDeposit>, amount: u64) -> Result<()> {
        instructions::user_deposit(ctx, amount)
    }

    /// Withdraws unspent funds from a `UserProfile`'s deposit balance.
    /// See [`instructions::user_withdraw`] for details.
    pub fn user_withdraw(ctx: Context<UserWithdraw>, amount: u64) -> Result<()> {
        instructions::user_withdraw(ctx, amount)
    }

    /// Allows a banned user to pay a fee to request an unban from the admin.
    /// See [`instructions::user_request_unban`] for details.
    pub fn user_request_unban(ctx: Context<UserRequestUnban>) -> Result<()> {
        instructions::user_request_unban(ctx)
    }

    // --- Operational Instructions ---

    /// Dispatches a command from a user to a service, verifying a signed price from an oracle.
    /// See [`instructions::user_dispatch_command`] for details.
    pub fn user_dispatch_command(
        ctx: Context<UserDispatchCommand>,
        command_id: u16,
        price: u64,
        timestamp: i64,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::user_dispatch_command(ctx, command_id, price, timestamp, payload)
    }

    /// Logs a significant off-chain action to the blockchain for an audit trail.
    /// See [`instructions::log_action`] for details.
    pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
        instructions::log_action(ctx, session_id, action_code)
    }
}
