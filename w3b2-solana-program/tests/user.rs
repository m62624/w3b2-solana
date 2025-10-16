//! This module contains all integration tests for User-related instructions.
//!
//! The tests follow a standard Arrange-Act-Assert pattern:
//! 1.  **Arrange:** Set up the initial on-chain state (create admins, users, fund wallets).
//! 2.  **Act:** Execute the single instruction being tested.
//! 3.  **Assert:** Fetch the resulting on-chain state and verify that it matches the expected outcome.

mod instructions;

use anchor_lang::prelude::Clock;
use anchor_lang::AccountDeserialize;
use instructions::*;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::sysvar::rent::Rent;
use solana_sdk::signature::Signer;
use w3b2_solana_program::state::{AdminProfile, UserProfile};

/// Tests the successful creation of a `UserProfile` PDA.
/// Verifies that a user can create a profile linked to a specific admin.
/// Checks that the profile is initialized with correct default values and rent-exempt lamports.
#[test]
fn test_user_create_profile_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // We need an admin to exist first, which our user profile will link to.
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    // Now, create the user that will interact with this admin.
    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_comm_key = create_keypair();

    // === 2. Act ===
    println!("Attempting to create user profile...");

    // Call the user helper to create a profile, targeting the admin we just made.
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        user_comm_key.pubkey(),
        admin_pda, // <-- Link to the specific admin
    );

    println!("User profile created successfully at: {user_pda}");

    // === 3. Assert ===
    let user_account_data = svm.get_account(&user_pda).unwrap();
    let user_profile =
        UserProfile::try_deserialize(&mut user_account_data.data.as_slice()).unwrap();

    assert_eq!(user_profile.authority, user_authority.pubkey());
    assert_eq!(user_profile.communication_pubkey, user_comm_key.pubkey());
    assert_eq!(
        user_profile.deposit_balance, 0,
        "Deposit balance should be 0 on initialization"
    );

    let rent = Rent::default();
    let space = 8 + std::mem::size_of::<UserProfile>();
    let rent_exempt_minimum = rent.minimum_balance(space);
    assert_eq!(user_account_data.lamports, rent_exempt_minimum);

    println!("✅ Create User Profile Test Passed!");
    println!("   -> User Authority: {}", user_profile.authority);
    let lamports = user_account_data.lamports;
    println!("   -> PDA Lamports: {lamports} (matches rent-exempt minimum)");
}

/// Tests the successful update of a `UserProfile`'s communication key.
/// Verifies that the `communication_pubkey` field is updated correctly while other
/// fields in the `UserProfile` remain unchanged.
#[test]
fn test_user_update_comm_key_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let initial_comm_key = create_keypair();
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        initial_comm_key.pubkey(),
        admin_pda,
    );

    let new_comm_key = create_keypair();

    // === 2. Act ===
    println!("Updating user communication key...");
    user::update_comm_key(&mut svm, &user_authority, admin_pda, new_comm_key.pubkey());

    // === 3. Assert ===
    let user_account_data = svm.get_account(&user_pda).unwrap();
    let user_profile =
        UserProfile::try_deserialize(&mut user_account_data.data.as_slice()).unwrap();

    assert_eq!(user_profile.communication_pubkey, new_comm_key.pubkey());
    assert_ne!(user_profile.communication_pubkey, initial_comm_key.pubkey());
    assert_eq!(user_profile.authority, user_authority.pubkey());
    assert_eq!(user_profile.deposit_balance, 0);

    println!("✅ Update User Comm Key Test Passed!");
    println!("   -> Old Key: {}", initial_comm_key.pubkey());
    println!("   -> New Key: {}", user_profile.communication_pubkey);
}

/// Tests the successful closure of a `UserProfile` account.
/// Verifies that the PDA account is deleted and its rent lamports are refunded
/// to the user's wallet (`authority`).
#[test]
fn test_user_close_profile_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let pda_balance = svm.get_balance(&user_pda).unwrap();
    let authority_balance_before = svm.get_balance(&user_authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Closing user profile...");
    user::close_profile(&mut svm, &user_authority, admin_pda);
    println!("Profile closed.");

    // === 3. Assert ===
    let closed_account = svm.get_account(&user_pda);
    assert!(closed_account.is_none(), "Account was not closed!");

    let authority_balance_after = svm.get_balance(&user_authority.pubkey()).unwrap();
    let expected_balance = authority_balance_before + pda_balance - 5000;
    assert_eq!(authority_balance_after, expected_balance);

    println!("✅ Close User Profile Test Passed!");
    println!(
        "   -> User authority balance correctly refunded: {authority_balance_before} -> {authority_balance_after}"
    );
}

/// Tests the successful deposit of funds into a `UserProfile`.
/// Verifies that the internal `deposit_balance` is correctly incremented and that
/// the on-chain lamport balance of both the user's wallet and the PDA are updated
/// correctly after the transfer.
#[test]
fn test_user_deposit_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let authority_balance_before = svm.get_balance(&user_authority.pubkey()).unwrap();
    let pda_lamports_before = svm.get_balance(&user_pda).unwrap();

    let deposit_amount = 2 * LAMPORTS_PER_SOL;

    // === 2. Act ===
    println!("User depositing {deposit_amount} lamports...");
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);
    println!("Deposit successful.");

    // === 3. Assert ===
    let user_account_data_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_data_after.data.as_slice()).unwrap();
    let authority_balance_after = svm.get_balance(&user_authority.pubkey()).unwrap();

    assert_eq!(user_profile_after.deposit_balance, deposit_amount);

    assert_eq!(
        user_account_data_after.lamports,
        pda_lamports_before + deposit_amount
    );

    let expected_authority_balance = authority_balance_before - deposit_amount - 5000;
    assert_eq!(authority_balance_after, expected_authority_balance);

    println!("✅ User Deposit Test Passed!");
    println!(
        "   -> PDA internal balance is now: {}",
        user_profile_after.deposit_balance
    );
    println!(
        "   -> PDA lamport balance increased from {pda_lamports_before} to {}",
        user_account_data_after.lamports
    );
}

/// Tests the successful withdrawal of funds from a `UserProfile`.
/// Verifies that the internal `deposit_balance` is correctly decremented, the PDA's
/// lamport balance decreases, and the destination wallet's balance increases by the
/// withdrawn amount.
#[test]
fn test_user_withdraw_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    let destination_wallet = create_keypair();

    let pda_lamports_before = svm.get_balance(&user_pda).unwrap();
    let destination_balance_before = 0;
    let withdraw_amount = LAMPORTS_PER_SOL;

    // === 2. Act ===
    println!("User withdrawing {withdraw_amount} lamports...");
    user::withdraw(
        &mut svm,
        &user_authority,
        admin_pda,
        destination_wallet.pubkey(),
        withdraw_amount,
    );
    println!("Withdrawal successful.");

    // === 3. Assert ===
    let user_account_data_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_data_after.data.as_slice()).unwrap();
    let destination_balance_after = svm.get_balance(&destination_wallet.pubkey()).unwrap();

    let expected_deposit_balance = deposit_amount - withdraw_amount;
    assert_eq!(user_profile_after.deposit_balance, expected_deposit_balance);

    assert_eq!(
        user_account_data_after.lamports,
        pda_lamports_before - withdraw_amount
    );

    assert_eq!(
        destination_balance_after,
        destination_balance_before + withdraw_amount
    );

    println!("✅ User Withdraw Test Passed!");
    println!(
        "   -> PDA internal balance is now: {}",
        user_profile_after.deposit_balance
    );
    println!("   -> Destination wallet received: {destination_balance_after} lamports");
}

/// Tests the successful execution of a paid command from a user to an admin.
/// This is the primary integration test for the payment flow.
/// Verifies that when a user calls a priced command, their `deposit_balance` decreases
/// and the admin's internal `balance` increases by the command's price. Also checks
/// that the on-chain lamport balances of both PDAs are updated accordingly.
#[test]
fn test_user_dispatch_command_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // Admin acts as its own oracle by default
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    let user_pda_lamports_before = svm.get_balance(&user_pda).unwrap();
    let admin_pda_lamports_before = svm.get_balance(&admin_pda).unwrap();
    let admin_account_data_before = svm.get_account(&admin_pda).unwrap();
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut admin_account_data_before.data.as_slice()).unwrap();

    let command_id_to_call = 1;
    let command_price = LAMPORTS_PER_SOL;
    let timestamp = svm.get_sysvar::<Clock>().unix_timestamp;

    // === 2. Act ===
    println!("User dispatching paid command with oracle signature...");
    user::dispatch_command(
        &mut svm,
        &user_authority,
        admin_pda,
        &admin_authority, // Oracle signer
        user::DispatchCommandArgs {
            command_id: command_id_to_call,
            price: command_price,
            timestamp,
            payload: vec![1, 2, 3], // Arbitrary payload
        },
    );
    println!("Command dispatched successfully.");

    // === 3. Assert ===
    let user_account_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_after.data.as_slice()).unwrap();

    let admin_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut admin_account_after.data.as_slice()).unwrap();

    // Assert user balances
    assert_eq!(
        user_profile_after.deposit_balance,
        deposit_amount - command_price
    );
    assert_eq!(
        user_account_after.lamports,
        user_pda_lamports_before - command_price
    );

    // Assert admin balances
    assert_eq!(
        admin_profile_after.balance,
        admin_profile_before.balance + command_price
    );
    assert_eq!(
        admin_account_after.lamports,
        admin_pda_lamports_before + command_price
    );

    println!("✅ User Dispatch Command Test Passed!");
    println!(
        "   -> User balance changed: {deposit_amount} -> {}",
        user_profile_after.deposit_balance
    );
    println!(
        "   -> Admin balance changed: {} -> {}",
        admin_profile_before.balance, admin_profile_after.balance
    );
}

/// Tests the user's ability to request an unban, both with and without a fee.
#[test]
fn test_user_request_unban_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let (admin_authority, admin_pda, user_authority, user_pda) = setup_profiles(&mut svm);

    // Ban the user first
    admin::ban_user(&mut svm, &admin_authority, user_pda);

    // --- Scenario 1: Unban request with no fee ---
    println!("Testing unban request with zero fee...");

    // === 2. Act ===
    user::request_unban(&mut svm, &user_authority, admin_pda);

    // === 3. Assert ===
    let user_profile_after_free_req = {
        let account_data = svm.get_account(&user_pda).unwrap();
        UserProfile::try_deserialize(&mut account_data.data.as_slice()).unwrap()
    };
    assert!(
        user_profile_after_free_req.unban_requested,
        "unban_requested flag should be true after a free request"
    );
    println!("✅ Free unban request successful.");

    // Reset state for the next scenario
    admin::unban_user(&mut svm, &admin_authority, user_pda);
    svm.expire_blockhash();
    admin::ban_user(&mut svm, &admin_authority, user_pda);

    // --- Scenario 2: Unban request with a fee ---
    println!("Testing unban request with a fee...");

    // === 4. Arrange (continued) ===
    let unban_fee = 100_000;
    let deposit_amount = unban_fee + 50_000; // Deposit more than the fee

    // Admin sets the unban fee
    admin::set_config(
        &mut svm,
        &admin_authority,
        None,
        None,
        None,
        Some(unban_fee),
    );

    // User deposits funds to pay the fee
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    let user_balance_before = deposit_amount;
    let admin_balance_before = {
        let account_data = svm.get_account(&admin_pda).unwrap();
        AdminProfile::try_deserialize(&mut account_data.data.as_slice())
            .unwrap()
            .balance
    };

    // === 5. Act ===
    user::request_unban(&mut svm, &user_authority, admin_pda);

    // === 6. Assert ===
    let user_profile_after_paid_req = {
        let account_data = svm.get_account(&user_pda).unwrap();
        UserProfile::try_deserialize(&mut account_data.data.as_slice()).unwrap()
    };
    let admin_profile_after_paid_req = {
        let account_data = svm.get_account(&admin_pda).unwrap();
        AdminProfile::try_deserialize(&mut account_data.data.as_slice()).unwrap()
    };

    assert!(
        user_profile_after_paid_req.unban_requested,
        "unban_requested flag should be true after a paid request"
    );
    assert_eq!(
        user_profile_after_paid_req.deposit_balance,
        user_balance_before - unban_fee,
        "User balance should be debited by the fee"
    );
    assert_eq!(
        admin_profile_after_paid_req.balance,
        admin_balance_before + unban_fee,
        "Admin balance should be credited with the fee"
    );
    println!("✅ Paid unban request successful, balances updated correctly.");

    println!("✅ User Request Unban Test Passed!");
}
