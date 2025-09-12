//! Anchor program for W3B2 bridge.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock;

use w3b2_common::{AccountType, CommandMode, UserAccount};

declare_id!("W3B2Bridge111111111111111111111111111111111");

#[program]
pub mod w3b2_bridge_program {
    use super::*;

    /// Register a user PDA.
    /// - Creates PDA seeded by ["user", authority_pubkey].
    /// - Stores UserAccount (owner + account_type).
    /// - Optionally stores linked_wallet if provided.
    pub fn register_user(
        ctx: Context<RegisterUser>,
        account_type: AccountType,
        linked_wallet: Option<[u8; 32]>,
    ) -> Result<()> {
        let user_pda = &mut ctx.accounts.user_pda;

        // prevent double registration
        require!(
            user_pda.profile.owner == [0u8; 32],
            BridgeError::AlreadyRegistered
        );

        user_pda.profile = UserAccount {
            owner: ctx.accounts.authority.key().to_bytes(),
            account_type,
        };
        user_pda.linked_wallet = linked_wallet;
        user_pda.created_at = clock::Clock::get()?.unix_timestamp as u64;

        emit!(UserRegistered {
            owner: ctx.accounts.authority.key(),
            account_type,
            linked_wallet,
            ts: user_pda.created_at as i64,
        });

        Ok(())
    }

    /// Dispatch a command: validate signer is registered and emit event with payload.
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
            sender: ctx.accounts.authority.key(),
            command_id,
            mode,
            payload,
            ts,
        });

        Ok(())
    }
}

/* ---------- Accounts ---------- */

#[account]
pub struct UserPda {
    /// Common user profile (owner + account_type).
    pub profile: UserAccount,
    /// optional linked wallet pubkey bytes
    pub linked_wallet: Option<[u8; 32]>,
    /// unix timestamp when created
    pub created_at: u64,
}

// space calculation:
// discriminator: 8
// UserAccount: owner [u8;32] + account_type (u8) = 33
// linked_wallet: Option<[u8;32]> = 1 + 32 = 33
// created_at: 8
// total = 8 + 33 + 33 + 8 = 82
// add margin for alignment = 96
#[derive(Accounts)]
pub struct RegisterUser<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = 96,
        seeds = [b"user", authority.key().as_ref()],
        bump
    )]
    pub user_pda: Account<'info, UserPda>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// The wallet that signs and will be registered (controller)
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DispatchCommand<'info> {
    #[account(mut, seeds = [b"user", authority.key().as_ref()], bump)]
    pub user_pda: Account<'info, UserPda>,

    /// signer issuing the command
    #[account(mut)]
    pub authority: Signer<'info>,
}

/* ---------- Events ---------- */

#[event]
pub struct UserRegistered {
    pub owner: Pubkey,
    pub account_type: AccountType,
    pub linked_wallet: Option<[u8; 32]>,
    pub ts: i64,
}

#[event]
pub struct CommandEvent {
    pub sender: Pubkey,
    pub command_id: u64,
    pub mode: CommandMode,
    pub payload: Vec<u8>,
    pub ts: i64,
}

/* ---------- Errors ---------- */

#[error_code]
pub enum BridgeError {
    #[msg("PDA already registered for this owner")]
    AlreadyRegistered,
    #[msg("Invalid account type")]
    InvalidAccountType,
    #[msg("Payload too large")]
    PayloadTooLarge,
    #[msg("Unauthorized: signer does not match PDA owner or linked wallet")]
    Unauthorized,
}
