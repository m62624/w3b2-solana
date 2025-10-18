//! # Instruction Logic
//!
//! This module contains the business logic for each on-chain instruction.
//!
//! Each function in this module corresponds to an instruction defined in `lib.rs` and
//! is responsible for:
//! 1.  Validating inputs and state.
//! 2.  Mutating on-chain accounts (`AdminProfile`, `UserProfile`).
//! 3.  Performing Cross-Program Invocations (CPIs), such as lamport transfers.
//! 4.  Emitting events to be consumed by off-chain clients.
//!
//! The logic is intentionally kept separate from the `lib.rs` program module to
//! improve code organization and readability.

use super::*;
use crate::instructions::solana_program::program::invoke;
use anchor_lang::solana_program;
use solana_program::{
    system_instruction,
    sysvar::instructions::{load_current_index_checked, load_instruction_at_checked},
};
use solana_sdk_ids::ed25519_program;

/// The maximum size in bytes for the `payload` in dispatch instructions.
pub const MAX_PAYLOAD_SIZE: usize = 1000;
/// The default maximum age of a signed timestamp in seconds before it is considered expired.
pub const MAX_TIMESTAMP_AGE_SECONDS: i64 = 60;

// --- Admin Instructions ---

/// Initializes a new `AdminProfile` for a service provider.
///
/// This creates the on-chain representation of a service, setting its owner (`authority`),
/// its off-chain communication key, and initializing its internal balance. The oracle
/// authority is set to the admin's own key by default but can be changed later.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminRegisterProfile`] accounts.
/// * `communication_pubkey` - The public key the admin will use for off-chain communication.
///
/// # Events
///
/// * [`AdminProfileRegistered`] - On successful creation of the profile.
pub fn admin_register_profile(
    ctx: Context<AdminRegisterProfile>,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    admin_profile.authority = ctx.accounts.authority.key();
    admin_profile.communication_pubkey = communication_pubkey;
    // By default, the admin is their own oracle. They can delegate this later.
    admin_profile.oracle_authority = ctx.accounts.authority.key();
    admin_profile.timestamp_validity_seconds = MAX_TIMESTAMP_AGE_SECONDS; // Set default value
    admin_profile.balance = 0;
    admin_profile.unban_fee = 0; // Default unban fee is 0

    emit!(AdminProfileRegistered {
        authority: admin_profile.authority,
        admin_pda: admin_profile.key(),
        communication_pubkey: admin_profile.communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Bans a user, preventing them from using the service.
///
/// Sets the `banned` flag on the specified `UserProfile` to `true`.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminBanUser`] accounts.
///
/// # Errors
///
/// * `CannotBanSelf` - If the admin attempts to ban their own user profile.
///
/// # Events
///
/// * [`UserBanned`] - On successful ban.
pub fn admin_ban_user(ctx: Context<AdminBanUser>) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;

    // An admin cannot ban a user profile associated with their own authority key.
    require_keys_neq!(
        user_profile.authority,
        ctx.accounts.authority.key(),
        BridgeError::CannotBanSelf
    );

    user_profile.banned = true;

    emit!(UserBanned {
        admin_authority: ctx.accounts.authority.key(),
        admin_pda: ctx.accounts.admin_profile.key(),
        user_profile_pda: user_profile.key(),
        ts: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Unbans a user, restoring their access to the service.
///
/// Sets the `banned` flag to `false` and resets the `unban_requested` flag. This action
/// is at the admin's discretion, regardless of whether the user has paid an `unban_fee`.
///
/// # Design Philosophy: The "Request for Review" Model
///
/// The unban process is intentionally asynchronous and requires admin intervention.
/// This might seem unusual compared to a fully atomic "pay-to-unban" model, but it's a
/// deliberate design choice that reflects real-world business needs and protects the service provider.
///
/// 1.  **Admin Sovereignty**: A ban is a disciplinary measure. If a user could automatically
///     unban themselves simply by paying a fee, the ban would lose its meaning as a deterrent
///     for malicious behavior (e.g., spam). It would become a mere "tax on bad behavior."
///     The service administrator must have the final say.
///
/// 2.  **Flexibility**: The current model allows the admin to handle various scenarios:
///     - Unban a user for free if the ban was issued in error.
///     - Refuse an unban request even if the fee was paid, if the user's behavior warrants it.
///
/// 3.  **"Fee for Review" not "Payment for Unban"**: The `unban_fee` is not a purchase of an
///     unban. It is a payment for the admin's time to review the appeal. This is analogous
///     to a court filing fee, which does not guarantee a favorable verdict.
///
/// The smart contract's role is to act as an incorruptible source of truth and financial arbiter.
/// It verifiably records the facts: "Yes, the user paid the fee. Yes, they have requested a review."
/// The business decision remains off-chain, with the admin's backend infrastructure (listeners,
/// databases, dashboards) responsible for reliably processing this queue of requests.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminUnbanUser`] accounts.
///
/// # Errors
///
/// * `UserNotBanned` - If the user is not currently banned.
///
/// # Events
///
/// * [`UserUnbanned`] - On successful unban.
pub fn admin_unban_user(ctx: Context<AdminUnbanUser>) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;

    require!(user_profile.banned, BridgeError::UserNotBanned);

    user_profile.banned = false;
    user_profile.unban_requested = false; // Reset the request flag

    emit!(UserUnbanned {
        admin_authority: ctx.accounts.authority.key(),
        admin_pda: ctx.accounts.admin_profile.key(),
        user_profile_pda: user_profile.key(),
        ts: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Sets the configuration for an `AdminProfile`.
///
/// Allows the admin to update the `oracle_authority`, `timestamp_validity_seconds`,
/// `communication_pubkey`, and `unban_fee`. Any field passed as `None` will be ignored.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminSetConfig`] accounts.
/// * `new_oracle_authority` - An optional new `Pubkey` for the oracle.
/// * `new_timestamp_validity` - An optional new duration in seconds for signature validity.
/// * `new_communication_pubkey` - An optional new `Pubkey` for off-chain communication.
/// * `new_unban_fee` - An optional new fee in lamports for unban requests.
///
/// # Events
///
/// * [`AdminConfigUpdated`] - Always emitted on successful execution.
/// * [`AdminUnbanFeeUpdated`] - Emitted only if the `unban_fee` was changed.
pub fn admin_set_config(
    ctx: Context<AdminSetConfig>,
    new_oracle_authority: Option<Pubkey>,
    new_timestamp_validity: Option<i64>,
    new_communication_pubkey: Option<Pubkey>,
    new_unban_fee: Option<u64>,
) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    let mut fee_updated = false;

    if let Some(new_oracle) = new_oracle_authority {
        admin_profile.oracle_authority = new_oracle;
    }
    if let Some(new_validity) = new_timestamp_validity {
        admin_profile.timestamp_validity_seconds = new_validity;
    }
    if let Some(new_comm_key) = new_communication_pubkey {
        admin_profile.communication_pubkey = new_comm_key;
    }
    if let Some(new_fee) = new_unban_fee {
        admin_profile.unban_fee = new_fee;
        fee_updated = true;
    }

    emit!(AdminConfigUpdated {
        authority: admin_profile.authority,
        admin_pda: admin_profile.key(),
        new_oracle_authority: admin_profile.oracle_authority,
        new_timestamp_validity: admin_profile.timestamp_validity_seconds,
        new_communication_pubkey: admin_profile.communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });

    if fee_updated {
        emit!(AdminUnbanFeeUpdated {
            authority: admin_profile.authority,
            admin_pda: admin_profile.key(),
            new_unban_fee: admin_profile.unban_fee,
            ts: Clock::get()?.unix_timestamp,
        });
    }

    Ok(())
}

/// Closes an `AdminProfile` account and refunds its rent lamports to the owner.
///
/// **Note:** This instruction only returns the lamports required for rent. Any funds
/// in the internal `balance` must be withdrawn via `admin_withdraw` *before* closing.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminCloseProfile`] accounts.
///
/// # Events
///
/// * [`AdminProfileClosed`] - On successful closure.
pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
    emit!(AdminProfileClosed {
        authority: ctx.accounts.authority.key(),
        admin_pda: ctx.accounts.admin_profile.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Withdraws earned funds from an `AdminProfile`'s internal balance.
///
/// Performs a direct lamport transfer from the `AdminProfile` PDA to a specified
/// `destination` account.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminWithdraw`] accounts.
/// * `amount` - The number of lamports to withdraw.
///
/// # Errors
///
/// * `InsufficientAdminBalance` - If the `amount` exceeds the profile's internal `balance`.
/// * `RentExemptViolation` - If the withdrawal would leave the PDA's lamport balance below the rent-exempt minimum.
///
/// # Events
///
/// * [`AdminFundsWithdrawn`] - On successful withdrawal.
pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    let destination = &ctx.accounts.destination;

    // Check if the internal balance is sufficient.
    require!(
        admin_profile.balance >= amount,
        BridgeError::InsufficientAdminBalance
    );

    // Check if the on-chain lamport balance will remain above the rent-exempt minimum.
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(admin_profile.to_account_info().data_len());
    require!(
        admin_profile.to_account_info().lamports() - amount >= rent_exempt_minimum,
        BridgeError::RentExemptViolation
    );

    // Perform the lamport transfer by directly debiting and crediting the accounts.
    **admin_profile.to_account_info().try_borrow_mut_lamports()? -= amount;
    **destination.to_account_info().try_borrow_mut_lamports()? += amount;

    // Update the internal balance state.
    admin_profile.balance -= amount;

    emit!(AdminFundsWithdrawn {
        authority: admin_profile.authority,
        admin_pda: admin_profile.key(),
        amount,
        destination: destination.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Dispatches a command or notification from an admin to a user.
///
/// This is a non-financial transaction. Its primary purpose is to emit an
/// [`AdminCommandDispatched`] event that an off-chain user `connector` can listen to.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`AdminDispatchCommand`] accounts.
/// * `command_id` - A `u64` identifier for the admin's command.
/// * `payload` - An opaque `Vec<u8>` for application-specific data.
///
/// # Errors
///
/// * `PayloadTooLarge` - If the `payload` exceeds `MAX_PAYLOAD_SIZE`.
///
/// # Events
///
/// * [`AdminCommandDispatched`] - On successful dispatch.
pub fn admin_dispatch_command(
    ctx: Context<AdminDispatchCommand>,
    command_id: u64,
    payload: Vec<u8>,
) -> Result<()> {
    require!(
        payload.len() <= MAX_PAYLOAD_SIZE,
        BridgeError::PayloadTooLarge
    );

    emit!(AdminCommandDispatched {
        sender: ctx.accounts.admin_authority.key(),
        sender_admin_pda: ctx.accounts.admin_profile.key(),
        target_user_pda: ctx.accounts.user_profile.key(),
        command_id,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// --- User Instructions ---

/// Creates a `UserProfile` PDA, linking a user's wallet to a specific admin service.
///
/// The `admin_profile_on_creation` field is set to the `AdminProfile` PDA's key, creating
/// a permanent, verifiable link between the user and the service.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserCreateProfile`] accounts.
/// * `target_admin_pda` - The public key of the `AdminProfile` PDA this user is registering with.
/// * `communication_pubkey` - The user's public key for off-chain communication.
///
/// # Events
///
/// * [`UserProfileCreated`] - On successful creation.
pub fn user_create_profile(
    ctx: Context<UserCreateProfile>,
    target_admin_pda: Pubkey,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    user_profile.authority = ctx.accounts.authority.key();
    user_profile.deposit_balance = 0;
    user_profile.communication_pubkey = communication_pubkey;
    user_profile.admin_profile_on_creation = target_admin_pda;
    user_profile.banned = false;
    user_profile.unban_requested = false;

    emit!(UserProfileCreated {
        authority: user_profile.authority,
        user_pda: user_profile.key(),
        target_admin_pda,
        communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the `communication_pubkey` for an existing `UserProfile`.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserUpdateCommKey`] accounts.
/// * `new_key` - The new `Pubkey` to set as the communication key.
///
/// # Events
///
/// * [`UserCommKeyUpdated`] - On successful update.
pub fn user_update_comm_key(ctx: Context<UserUpdateCommKey>, new_key: Pubkey) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    user_profile.communication_pubkey = new_key;
    emit!(UserCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        user_profile_pda: user_profile.key(),
        new_comm_pubkey: new_key,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Closes a `UserProfile` account and refunds its lamports to the owner.
///
/// The `close` directive in the `UserCloseProfile` account context ensures all lamports
/// held by the `user_profile` PDA (both for rent and from any remaining `deposit_balance`)
/// are safely returned to the user's `authority` wallet.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserCloseProfile`] accounts.
///
/// # Events
///
/// * [`UserProfileClosed`] - On successful closure.
pub fn user_close_profile(ctx: Context<UserCloseProfile>) -> Result<()> {
    emit!(UserProfileClosed {
        authority: ctx.accounts.authority.key(),
        user_pda: ctx.accounts.user_profile.key(),
        admin_pda: ctx.accounts.admin_profile.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows a banned user to pay a fee to request an unban from the admin.
///
/// If the `unban_fee` is greater than zero, this instruction transfers the fee amount
/// from the user's `deposit_balance` to the admin's `balance`. It then sets the
/// `unban_requested` flag to `true`.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserRequestUnban`] accounts.
///
/// # Errors
///
/// * `UserNotBanned` - If the user is not currently banned.
/// * `UnbanAlreadyRequested` - If an unban has already been requested and is pending review.
/// * `InsufficientDepositBalance` - If the user's balance is less than the `unban_fee`.
/// * `RentExemptViolation` - If the fee transfer would leave the user's PDA below the rent-exempt minimum.
///
/// # Events
///
/// * [`UserUnbanRequested`] - On successful request.
pub fn user_request_unban(ctx: Context<UserRequestUnban>) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    let admin_profile = &mut ctx.accounts.admin_profile;
    let fee = admin_profile.unban_fee;

    // The user must be banned to request an unban.
    require!(user_profile.banned, BridgeError::UserNotBanned);
    // The user cannot request an unban if one is already pending.
    require!(
        !user_profile.unban_requested,
        BridgeError::UnbanAlreadyRequested
    );

    // Process the unban fee payment.
    if fee > 0 {
        require!(
            user_profile.deposit_balance >= fee,
            BridgeError::InsufficientDepositBalance
        );

        // Check if the on-chain lamport balance will remain above the rent-exempt minimum.
        let rent = Rent::get()?;
        let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
        require!(
            user_profile.to_account_info().lamports() - fee >= rent_exempt_minimum,
            BridgeError::RentExemptViolation
        );

        // Transfer lamports from user PDA to admin PDA
        **user_profile.to_account_info().try_borrow_mut_lamports()? -= fee;
        **admin_profile.to_account_info().try_borrow_mut_lamports()? += fee;

        // Update internal balances
        user_profile.deposit_balance -= fee;
        admin_profile.balance += fee;
    }

    // Set the flag indicating an unban has been requested.
    user_profile.unban_requested = true;

    emit!(UserUnbanRequested {
        user_authority: user_profile.authority,
        user_profile_pda: user_profile.key(),
        admin_pda: admin_profile.key(),
        fee_paid: fee,
        ts: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Deposits lamports into a `UserProfile` PDA.
///
/// This pre-funds a user's account to pay for future service calls to the linked admin.
/// It performs a CPI to the System Program to transfer lamports.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserDeposit`] accounts.
/// * `amount` - The number of lamports to deposit.
///
/// # Events
///
/// * [`UserFundsDeposited`] - On successful deposit.
pub fn user_deposit(ctx: Context<UserDeposit>, amount: u64) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;

    // Perform a Cross-Program Invocation (CPI) to the System Program to transfer lamports
    // from the user's `authority` wallet to the `user_profile` PDA.
    invoke(
        &system_instruction::transfer(
            &ctx.accounts.authority.key(),
            &user_profile.to_account_info().key(),
            amount,
        ),
        &[
            ctx.accounts.authority.to_account_info(),
            user_profile.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Update the internal deposit balance state.
    user_profile.deposit_balance += amount;

    emit!(UserFundsDeposited {
        authority: user_profile.authority,
        user_profile_pda: user_profile.key(),
        amount,
        new_deposit_balance: user_profile.deposit_balance,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Withdraws unspent funds from a `UserProfile`'s deposit balance.
///
/// This instruction performs a direct lamport transfer from the `UserProfile` PDA to a
/// specified `destination` account.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserWithdraw`] accounts.
/// * `amount` - The number of lamports to withdraw.
///
/// # Errors
///
/// * `InsufficientDepositBalance` - If the `amount` exceeds the profile's `deposit_balance`.
/// * `RentExemptViolation` - If the withdrawal would leave the PDA's lamport balance below the rent-exempt minimum.
///
/// # Events
///
/// * [`UserFundsWithdrawn`] - On successful withdrawal.
pub fn user_withdraw(ctx: Context<UserWithdraw>, amount: u64) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    let destination = &ctx.accounts.destination;

    // Check if the internal deposit balance is sufficient.
    require!(
        user_profile.deposit_balance >= amount,
        BridgeError::InsufficientDepositBalance
    );

    // Check if the on-chain lamport balance will remain above the rent-exempt minimum.
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
    require!(
        user_profile.to_account_info().lamports() - amount >= rent_exempt_minimum,
        BridgeError::RentExemptViolation
    );

    // Perform the lamport transfer.
    **user_profile.to_account_info().try_borrow_mut_lamports()? -= amount;
    **destination.to_account_info().try_borrow_mut_lamports()? += amount;

    // Update the internal deposit balance state.
    user_profile.deposit_balance -= amount;

    emit!(UserFundsWithdrawn {
        authority: user_profile.authority,
        user_profile_pda: user_profile.key(),
        amount,
        destination: destination.key(),
        new_deposit_balance: user_profile.deposit_balance,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

// --- Operational Instructions ---

/// Dispatches a command from a user to a service, potentially with payment.
///
/// This is the primary instruction for user-service interaction. It verifies a price
/// signature from the admin's designated oracle and, if valid and the price is non-zero,
/// transfers payment from the user's profile to the admin's profile.
///
/// # Pre-requisites
///
/// This instruction **must** be preceded by an `ed25519_dalek` signature verification
/// instruction in the same transaction. The program inspects this preceding instruction
/// to authenticate the `price`, `command_id`, and `timestamp`.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`UserDispatchCommand`] accounts.
/// * `command_id` - The `u16` identifier of the service's command.
/// * `price` - The price in lamports, as signed by the oracle.
/// * `timestamp` - The Unix timestamp from the signed message, to prevent replay attacks.
/// * `payload` - An opaque `Vec<u8>` for application-specific data.
///
/// # Errors
///
/// * `UserIsBanned` - If the user's profile is marked as banned.
/// * `PayloadTooLarge` - If the `payload` exceeds `MAX_PAYLOAD_SIZE`.
/// * `InstructionMismatch` - If the preceding instruction is not a valid Ed25519 signature verification.
/// * `InvalidOracleSigner` - If the signature was not from the admin's designated `oracle_authority`.
/// * `TimestampTooOld` - If the signed timestamp has expired.
/// * `SignatureVerificationFailed` - If the signed message content does not match the provided arguments.
/// * `InsufficientDepositBalance` - If the user's balance is less than the `price`.
/// * `RentExemptViolation` - If the payment would leave the user's PDA below the rent-exempt minimum.
///
/// # Events
///
/// * [`UserCommandDispatched`] - On successful dispatch and payment.
pub fn user_dispatch_command(
    ctx: Context<UserDispatchCommand>,
    command_id: u16,
    price: u64,
    timestamp: i64,
    payload: Vec<u8>,
) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;

    require!(!user_profile.banned, BridgeError::UserIsBanned);
    require!(
        payload.len() <= MAX_PAYLOAD_SIZE,
        BridgeError::PayloadTooLarge
    );

    let user_profile = &mut ctx.accounts.user_profile;
    let admin_profile = &mut ctx.accounts.admin_profile;
    let ixs = &ctx.accounts.instructions;

    // --- Oracle Signature Verification ---

    // The transaction must include an ed25519 signature verification instruction
    // immediately before this one. We will inspect it to authenticate the price.
    let current_ix_index = load_current_index_checked(&ixs.to_account_info())?;

    // The verification instruction must be the one immediately preceding this one.
    require_gt!(current_ix_index, 0, BridgeError::InstructionMismatch);
    let verify_ix_index = (current_ix_index - 1) as usize;
    let verify_ix = load_instruction_at_checked(verify_ix_index, &ixs.to_account_info())?;

    // Check that it's an ed25519 program instruction.
    require_keys_eq!(
        verify_ix.program_id,
        ed25519_program::ID,
        BridgeError::InstructionMismatch
    );

    // The signature is considered valid if the instruction did not fail.
    // We now need to check *what* was signed.
    let ix_data = &verify_ix.data;

    // Extract pubkey, signature, and message from the instruction data.
    // See https://docs.solana.com/developing/runtime-facilities/programs#ed25519-program
    // let signer_pubkey_bytes = &ix_data[16..48];
    // let message_data = &ix_data[48..];
    let signer_pubkey_bytes = &ix_data[16..48];
    let message_data = &ix_data[112..];

    // Verify the signer is the admin's designated oracle.
    let signer_pubkey = Pubkey::new_from_array(
        signer_pubkey_bytes
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    require_keys_eq!(
        signer_pubkey,
        admin_profile.oracle_authority,
        BridgeError::InvalidOracleSigner
    );

    // Verify the timestamp isn't too old to prevent replay attacks.
    let now = Clock::get()?.unix_timestamp;
    require!(
        now.saturating_sub(timestamp) <= admin_profile.timestamp_validity_seconds,
        BridgeError::TimestampTooOld
    );

    // Reconstruct the signed message and verify it matches.
    // The message format is: command_id (2 bytes) | price (8 bytes) | timestamp (8 bytes)
    let expected_message = [
        command_id.to_le_bytes().as_ref(),
        price.to_le_bytes().as_ref(),
        timestamp.to_le_bytes().as_ref(),
    ]
    .concat();

    require!(
        message_data == expected_message,
        BridgeError::SignatureVerificationFailed
    );

    // --- Payment Processing ---

    // If the command is not free, process the payment.
    if price > 0 {
        require!(
            user_profile.deposit_balance >= price,
            BridgeError::InsufficientDepositBalance
        );

        let rent = Rent::get()?;
        let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
        require!(
            user_profile.to_account_info().lamports() - price >= rent_exempt_minimum,
            BridgeError::RentExemptViolation
        );

        // Transfer lamports from the user's PDA to the admin's PDA.
        **user_profile.to_account_info().try_borrow_mut_lamports()? -= price;
        **admin_profile.to_account_info().try_borrow_mut_lamports()? += price;

        // Update the internal balances of both profiles.
        user_profile.deposit_balance -= price;
        admin_profile.balance += price;
    }

    emit!(UserCommandDispatched {
        sender: ctx.accounts.authority.key(),
        sender_user_pda: user_profile.key(),
        target_admin_pda: admin_profile.key(),
        command_id,
        price_paid: price,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Logs a significant off-chain action to the blockchain.
///
/// This creates an immutable, auditable record of events that happen outside the
/// on-chain protocol, such as a successful HTTP request in a Web2 service. The signer
/// can be either the user or the admin associated with the profiles.
///
/// # Arguments
///
/// * `ctx` - The context, containing the [`LogAction`] accounts.
/// * `session_id` - A `u64` identifier to correlate this action with a session.
/// * `action_code` - A `u16` code representing the specific off-chain action.
///
/// # Events
///
/// * [`OffChainActionLogged`] - On successful logging.
pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
    let actor = ctx.accounts.authority.key();

    emit!(OffChainActionLogged {
        actor,
        user_profile_pda: ctx.accounts.user_profile.key(),
        admin_profile_pda: ctx.accounts.admin_profile.key(),
        session_id,
        action_code,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
