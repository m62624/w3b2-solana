//! Anchor program for W3B2 bridge.
#![allow(deprecated)]
#![allow(unexpected_cfgs)]

pub mod command;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod sm_accounts;
pub mod types;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock;

use errors::*;
use events::*;
use sm_accounts::*;
use types::*;

declare_id!("W3B2Bridge111111111111111111111111111111111");

#[program]
pub mod w3b2_bridge_program {
    use super::*;

    /// Registers an admin account.
    pub fn register_admin(ctx: Context<RegisterAdmin>, funding_amount: u64) -> Result<()> {
        instructions::register_admin(ctx, funding_amount)
    }

    /// User requests funding.
    /// This instruction is called by the user's wallet.
    pub fn request_funding(
        ctx: Context<RequestFunding>,
        amount: u64,
        target_admin: Pubkey,
    ) -> Result<()> {
        instructions::request_funding(ctx, amount, target_admin)
    }

    /// Admin approves and funds a user's request.
    /// This is called by the service's admin wallet.
    pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
        instructions::approve_funding(ctx)
    }

    /// Dispatch a command: validate signer is registered and emit event with payload.
    /// - payload typically contains encrypted Borsh(CommandConfig) for the service.
    pub fn dispatch_command(
        ctx: Context<DispatchCommand>,
        command_id: u64,
        mode: CommandMode,
        payload: Vec<u8>,
        target_admin: Pubkey,
    ) -> Result<()> {
        instructions::dispatch_command(ctx, command_id, mode, payload, target_admin)
    }
}
