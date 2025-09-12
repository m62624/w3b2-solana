//! Anchor program for W3B2 bridge.
#![allow(deprecated)]

pub mod command;
pub mod types;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock;
use solana_program::program::invoke_signed;
use solana_program::system_instruction;
use types::*;

declare_id!("W3B2Bridge111111111111111111111111111111111");

#[program]
pub mod w3b2_bridge_program {
    use super::*;

    /// Registers an admin account.
    pub fn register_admin(ctx: Context<RegisterAdmin>) -> Result<()> {
        let admin_profile = &mut ctx.accounts.admin_profile;
        admin_profile.owner = ctx.accounts.authority.key().to_bytes();
        Ok(())
    }

    /// Funds a user wallet from the admin account.
    /// This instruction is called by the admin service.
    pub fn fund_user_wallet(ctx: Context<FundUserWallet>, amount: u64) -> Result<()> {
        let admin_profile = &ctx.accounts.admin_profile;

        // Verify that the signer is the registered admin
        require!(
            admin_profile.owner == ctx.accounts.authority.key().to_bytes(),
            BridgeError::Unauthorized
        );

        // Get the bump seed for the PDA. No unwrap needed.
        let bump = ctx.bumps.admin_profile;

        // Define the seeds for the PDA signer
        let pda_seeds = &[b"admin".as_ref(), admin_profile.owner.as_ref(), &[bump]];

        // Create the transfer instruction
        let transfer_instruction = system_instruction::transfer(
            // Correct way to get the key from the Account struct:
            // Use `.to_account_info()` to get the Pubkey
            ctx.accounts.admin_profile.to_account_info().key,
            ctx.accounts.user_wallet.key,
            amount,
        );

        // Invoke the instruction, signing on behalf of the PDA
        invoke_signed(
            &transfer_instruction,
            &[
                ctx.accounts.admin_profile.to_account_info(),
                ctx.accounts.user_wallet.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[pda_seeds],
        )?;

        // Log the event
        msg!("Transferred {} lamports to user wallet", amount);

        Ok(())
    }

    /// Dispatch a command: validate signer is registered and emit event with payload.
    /// - payload typically contains encrypted Borsh(CommandConfig) for the service.
    pub fn dispatch_command(
        ctx: Context<DispatchCommand>,
        command_id: u64,
        mode: CommandMode,
        payload: Vec<u8>,
    ) -> Result<()> {
        require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

        let pda = &ctx.accounts.user_pda;
        let signer_bytes = ctx.accounts.authority.key().to_bytes();

        // signer must be owner or linked_wallet
        let is_owner = pda.profile.owner == signer_bytes;
        let is_linked = pda.linked_wallet.map_or(false, |lk| lk == signer_bytes);
        require!(is_owner || is_linked, BridgeError::Unauthorized);

        let ts = clock::Clock::get()?.unix_timestamp;

        emit!(CommandEvent {
            sender: signer_bytes,
            command_id,
            mode,
            payload,
            ts,
        });

        Ok(())
    }

    #[error_code]
    pub enum BridgeError {
        #[msg("PDA already registered for this owner")]
        AlreadyRegistered,
        #[msg("Payload too large")]
        PayloadTooLarge,
        #[msg("Unauthorized: signer does not match PDA owner or linked wallet")]
        Unauthorized,
    }
}
