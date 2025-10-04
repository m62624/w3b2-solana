use super::*;
use crate::instructions::solana_program::program::invoke;
use crate::instructions::solana_program::system_instruction;
use anchor_lang::solana_program;

/// The maximum size in bytes for the `payload` in dispatch instructions.
pub const MAX_PAYLOAD_SIZE: usize = 1000;

// --- Admin Instructions ---

/// Initializes a new `AdminProfile` PDA for a service provider (an "Admin").
/// This instruction creates the on-chain representation of a service, setting its
/// owner (`authority`), its off-chain communication key, and initializing its
/// internal balance and price list.
pub fn admin_register_profile(
    ctx: Context<AdminRegisterProfile>,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    admin_profile.authority = ctx.accounts.authority.key();
    admin_profile.communication_pubkey = communication_pubkey;
    admin_profile.prices = Vec::new();
    admin_profile.balance = 0;

    emit!(AdminProfileRegistered {
        authority: admin_profile.authority,
        communication_pubkey: admin_profile.communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the `communication_pubkey` for an existing `AdminProfile`.
/// This allows a service provider to rotate their off-chain encryption keys.
pub fn admin_update_comm_key(ctx: Context<AdminUpdateCommKey>, new_key: Pubkey) -> Result<()> {
    ctx.accounts.admin_profile.communication_pubkey = new_key;
    emit!(AdminCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        new_comm_pubkey: new_key,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Closes an `AdminProfile` account and refunds its rent lamports to the owner.
/// This effectively unregisters a service from the protocol.
///
/// **Note:** This instruction only returns the lamports required for rent. Any funds
/// in the internal `balance` must be withdrawn via `admin_withdraw` *before* closing.
pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
    emit!(AdminProfileClosed {
        authority: ctx.accounts.authority.key(),
        admin_pda: ctx.accounts.admin_profile.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the price list for an admin's services.
/// The instruction sorts and de-duplicates the provided list by `command_id` to ensure
/// a canonical representation. The associated `AdminProfile` account is automatically
/// resized (`realloc`) by Anchor to accommodate the new list size.
pub fn admin_update_prices(
    ctx: Context<AdminUpdatePrices>,
    mut new_prices: Vec<PriceEntry>,
) -> Result<()> {
    new_prices.sort_unstable_by_key(|k| k.command_id);
    new_prices.dedup_by_key(|k| k.command_id);
    ctx.accounts.admin_profile.prices = new_prices.clone();
    emit!(AdminPricesUpdated {
        authority: ctx.accounts.authority.key(),
        new_prices,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows an admin to withdraw earned funds from their `AdminProfile`'s internal balance.
///
/// This instruction performs a direct lamport transfer from the `AdminProfile` PDA to a
/// specified `destination` account. It performs critical safety checks to ensure the
/// withdrawal is authorized, the internal balance is sufficient, and the PDA remains rent-exempt.
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
        amount,
        destination: destination.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows an admin to send a command or notification to a user.
///
/// This is a non-financial transaction. Its primary purpose is to emit an
/// `AdminCommandDispatched` event that an off-chain user `connector` can listen and react to.
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
        target_user_pda: ctx.accounts.user_profile.key(),
        command_id,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// --- User Instructions ---

/// Creates a `UserProfile` PDA, linking a user's wallet (`authority`) to a specific admin service.
///
/// The `admin_profile_on_creation` field is set to the `AdminProfile` PDA's key, creating
/// a permanent, verifiable link between the user and the service.
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

    emit!(UserProfileCreated {
        authority: user_profile.authority,
        target_admin_pda,
        communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the `communication_pubkey` for an existing `UserProfile`.
pub fn user_update_comm_key(ctx: Context<UserUpdateCommKey>, new_key: Pubkey) -> Result<()> {
    ctx.accounts.user_profile.communication_pubkey = new_key;
    emit!(UserCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        user_profile_pda: ctx.accounts.user_profile.key(),
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
pub fn user_close_profile(ctx: Context<UserCloseProfile>) -> Result<()> {
    emit!(UserProfileClosed {
        authority: ctx.accounts.authority.key(),
        admin_pda: ctx.accounts.admin_profile.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows a user to deposit lamports into their `UserProfile` PDA.
/// This pre-funds their account to pay for future service calls to the linked admin.
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

/// Allows a user to withdraw unspent funds from their `UserProfile` deposit balance.
///
/// This instruction performs a direct lamport transfer from the `UserProfile` PDA to a
/// specified `destination` account. It performs critical safety checks to ensure the
/// withdrawal is authorized, the internal `deposit_balance` is sufficient, and the PDA
/// remains rent-exempt.
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

/// The primary instruction for a user to call a service's API.
///
/// If the `command_id` is found in the admin's price list, this instruction handles
/// the payment by transferring lamports from the `UserProfile` PDA to the `AdminProfile` PDA.
pub fn user_dispatch_command(
    ctx: Context<UserDispatchCommand>,
    command_id: u16,
    payload: Vec<u8>,
) -> Result<()> {
    require!(
        payload.len() <= MAX_PAYLOAD_SIZE,
        BridgeError::PayloadTooLarge
    );

    let user_profile = &mut ctx.accounts.user_profile;
    let admin_profile = &mut ctx.accounts.admin_profile;

    let command_price = match admin_profile
        .prices
        .binary_search_by_key(&command_id, |id| id.command_id)
    {
        Ok(index) => admin_profile.prices[index].price,
        Err(_) => 0,
    };

    // If the command is not free, process the payment.
    if command_price > 0 {
        require!(
            user_profile.deposit_balance >= command_price,
            BridgeError::InsufficientDepositBalance
        );

        let rent = Rent::get()?;
        let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
        require!(
            user_profile.to_account_info().lamports() - command_price >= rent_exempt_minimum,
            BridgeError::RentExemptViolation
        );

        // Transfer lamports from the user's PDA to the admin's PDA.
        **user_profile.to_account_info().try_borrow_mut_lamports()? -= command_price;
        **admin_profile.to_account_info().try_borrow_mut_lamports()? += command_price;

        // Update the internal balances of both profiles.
        user_profile.deposit_balance -= command_price;
        admin_profile.balance += command_price;
    }

    emit!(UserCommandDispatched {
        sender: ctx.accounts.authority.key(),
        target_admin_pda: admin_profile.key(),
        command_id,
        price_paid: command_price,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// A generic instruction to log a significant off-chain action to the blockchain.
/// This creates an immutable, auditable record of events that happen outside the
/// on-chain protocol, such as a successful HTTP request in a Web2 service.
pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
    let actor = ctx.accounts.authority.key();
    let target = if actor == ctx.accounts.user_profile.authority {
        // If the user is the signer, the target is the admin.
        ctx.accounts.admin_profile.key()
    } else {
        // If the admin is the signer, the target is the user.
        ctx.accounts.user_profile.key()
    };

    emit!(OffChainActionLogged {
        actor,
        target,
        session_id,
        action_code,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
