//! This module contains all integration tests for Admin-related instructions.
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

/// Tests the successful creation of an `AdminProfile` PDA.
/// Verifies that the profile is initialized with correct default values and rent-exempt lamports.
#[test]
fn test_admin_create_profile_success() {
    // === 1. Arrange (Setup) ===
    let mut svm = setup_svm();

    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

    // === 2. Act (Execution) ===
    println!("Attempting to create admin profile...");
    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());
    println!("Admin profile created successfully at: {}", admin_pda);

    // === 3. Assert (Verification) ===
    let admin_account_data = svm.get_account(&admin_pda).unwrap();
    let admin_profile =
        AdminProfile::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

    assert_eq!(admin_profile.authority, authority.pubkey());
    assert_eq!(admin_profile.communication_pubkey, comm_key.pubkey());
    assert_eq!(
        admin_profile.balance, 0,
        "Balance should be 0 on initialization"
    );

    let rent = Rent::default();
    let space = 8 + std::mem::size_of::<AdminProfile>();
    let rent_exempt_minimum = rent.minimum_balance(space);
    assert_eq!(admin_account_data.lamports, rent_exempt_minimum);

    println!("✅ Assertions passed!");
    println!("   -> Authority: {}", admin_profile.authority);
    println!(
        "   -> PDA Lamports: {} (matches rent-exempt minimum)",
        admin_account_data.lamports
    );
}

/// Tests the successful update of an `AdminProfile`'s communication key.
/// Verifies that the `communication_pubkey` field is updated while other fields remain unchanged.
#[test]
fn test_admin_update_comm_key_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let initial_comm_key = create_keypair();
    let admin_pda = admin::create_profile(&mut svm, &authority, initial_comm_key.pubkey());

    let new_comm_key = create_keypair();

    // === 2. Act ===
    println!("Updating communication key...");
    admin::update_comm_key(&mut svm, &authority, new_comm_key.pubkey());

    // === 3. Assert ===
    let admin_account_data = svm.get_account(&admin_pda).unwrap();
    let admin_profile =
        AdminProfile::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

    assert_eq!(admin_profile.communication_pubkey, new_comm_key.pubkey());
    assert_ne!(
        admin_profile.communication_pubkey,
        initial_comm_key.pubkey()
    );
    assert_eq!(admin_profile.authority, authority.pubkey());

    println!("✅ Update Comm Key Test Passed!");
    println!("   -> Old Key: {}", initial_comm_key.pubkey());
    println!("   -> New Key: {}", admin_profile.communication_pubkey);
}

/// Tests the successful closure of an `AdminProfile` account.
/// Verifies that the PDA account is deleted and its rent lamports are refunded
/// to the admin's wallet (`authority`).
#[test]
fn test_admin_close_profile_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());

    let pda_balance = svm.get_balance(&admin_pda).unwrap();
    let authority_balance_before = svm.get_balance(&authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Closing admin profile...");
    admin::close_profile(&mut svm, &authority);
    println!("Profile closed.");

    // === 3. Assert ===
    let closed_account = svm.get_account(&admin_pda);
    assert!(closed_account.is_none(), "Account was not closed!");

    let authority_balance_after = svm.get_balance(&authority.pubkey()).unwrap();
    let expected_balance = authority_balance_before + pda_balance - 5000;
    assert_eq!(authority_balance_after, expected_balance);

    println!("✅ Close Profile Test Passed!");
    println!(
        "   -> Authority balance correctly refunded: {} -> {}",
        authority_balance_before, authority_balance_after
    );
}

/// Tests the successful dispatch of a command *from* an admin *to* a user.
/// Verifies that a non-financial command can be sent without altering any internal
/// or on-chain lamport balances of the profiles.
#[test]
fn test_admin_dispatch_command_success() {
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

    let admin_account_before = svm.get_account(&admin_pda).unwrap();
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut admin_account_before.data.as_slice()).unwrap();

    let user_account_before = svm.get_account(&user_pda).unwrap();
    let user_profile_before =
        UserProfile::try_deserialize(&mut user_account_before.data.as_slice()).unwrap();

    assert_eq!(user_profile_before.admin_profile_on_creation, admin_pda);

    let admin_authority_lamports_before = svm.get_balance(&admin_authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Admin dispatching command to user...");
    admin::dispatch_command(
        &mut svm,
        &admin_authority,
        user_pda,
        101, // Notification command ID
        vec![4, 5, 6],
    );
    println!("Command dispatched successfully.");

    // === 3. Assert ===
    let admin_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut admin_account_after.data.as_slice()).unwrap();

    let user_account_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_after.data.as_slice()).unwrap();

    let admin_authority_lamports_after = svm.get_balance(&admin_authority.pubkey()).unwrap();

    // Assert that internal balances are unchanged
    assert_eq!(admin_profile_after.balance, admin_profile_before.balance);
    assert_eq!(
        user_profile_after.deposit_balance,
        user_profile_before.deposit_balance
    );

    // Assert that PDA lamport balances are unchanged
    assert_eq!(admin_account_after.lamports, admin_account_before.lamports);
    assert_eq!(user_account_after.lamports, user_account_before.lamports);

    // Assert admin's signer balance only changed by the transaction fee
    let expected_admin_authority_balance = admin_authority_lamports_before - 5000;
    assert_eq!(
        admin_authority_lamports_after,
        expected_admin_authority_balance
    );

    println!("✅ Admin Dispatch Command Test Passed!");
    println!(
        "   -> Balances remained unchanged (Admin: {}, User: {})",
        admin_profile_after.balance, user_profile_after.deposit_balance
    );
}

/// Tests the successful withdrawal of *earned* funds by an admin.
/// This is an integration test: a user pays an admin, then the admin withdraws the earnings.
/// Verifies that the admin's internal `balance` and the PDA's lamport balance decrease
/// correctly, and the destination wallet's balance increases.
#[test]
fn test_admin_withdraw_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // Create Admin. By default, the admin is their own oracle.
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    // Create a User who will pay the Admin
    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let _ = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    // User deposits funds into their profile
    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    // User "buys" the service, transferring funds to the Admin
    let command_price = LAMPORTS_PER_SOL;
    // Get the clock from SVM, don't use Clock::get() in tests
    let timestamp = svm.get_sysvar::<Clock>().unix_timestamp;
    println!("User pays admin {} lamports...", command_price);
    user::dispatch_command(
        &mut svm,
        &user_authority,
        admin_pda,
        &admin_authority, // Oracle signer is the admin
        1,                // command_id
        command_price,
        timestamp,
        vec![1, 2, 3], // payload
    );

    // Prepare for the withdrawal
    let destination_wallet = create_keypair();
    let withdraw_amount = command_price / 2; // Withdraw half of the earnings

    // Get state *before* the withdrawal
    let pda_account_before = svm.get_account(&admin_pda).unwrap();
    let pda_lamports_before = pda_account_before.lamports;
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut pda_account_before.data.as_slice()).unwrap();
    let destination_balance_before = 0;

    assert_eq!(admin_profile_before.balance, command_price);

    // === 2. Act ===
    println!("Admin withdrawing {} lamports...", withdraw_amount);
    admin::withdraw(
        &mut svm,
        &admin_authority,
        destination_wallet.pubkey(),
        withdraw_amount,
    );
    println!("Withdrawal successful.");

    // === 3. Assert ===
    let pda_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut pda_account_after.data.as_slice()).unwrap();
    let destination_balance_after = svm.get_balance(&destination_wallet.pubkey()).unwrap();

    assert_eq!(
        admin_profile_after.balance,
        admin_profile_before.balance - withdraw_amount
    );
    assert_eq!(
        pda_account_after.lamports,
        pda_lamports_before - withdraw_amount
    );
    assert_eq!(
        destination_balance_after,
        destination_balance_before + withdraw_amount
    );

    println!("✅ Admin Withdraw Test Passed!");
    println!(
        "   -> PDA internal balance is now: {}",
        admin_profile_after.balance
    );
    println!(
        "   -> Destination wallet received: {} lamports",
        destination_balance_after
    );
}
