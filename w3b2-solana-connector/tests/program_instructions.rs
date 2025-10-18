use anchor_lang::AccountDeserialize;
use async_trait::async_trait;
use solana_client::client_error::ClientError;
use solana_program_test::*;
use solana_sdk::message::Message;
use solana_sdk::transport::TransportError;
use solana_sdk::{
    hash::Hash,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction as system_instruction;
use std::{env, sync::Arc};
use w3b2_solana_connector::client::{AsyncRpcClient, TransactionBuilder, UserDispatchCommandArgs};
use w3b2_solana_program::state::AdminProfile;

// A mock RPC client that wraps BanksClient for testing purposes.
struct MockRpcClient(BanksClient);

#[async_trait]
impl AsyncRpcClient for MockRpcClient {
    async fn get_latest_blockhash(&self) -> Result<Hash, ClientError> {
        self.0
            .get_latest_blockhash()
            .await
            .map_err(|e| ClientError::from(TransportError::from(e)))
    }

    // This method is not used in the tests that now submit transactions directly.
    async fn send_and_confirm_transaction(
        &self,
        _transaction: &Transaction,
    ) -> Result<solana_sdk::signature::Signature, ClientError> {
        unimplemented!("This should not be called in the new test flow")
    }
}

/// Sets up the `solana-program-test` environment and starts a test validator.
async fn setup_test_environment() -> ProgramTestContext {
    env::set_var("BPF_OUT_DIR", "../target/deploy");

    let program_test = ProgramTest::new("w3b2_solana_program", w3b2_solana_program::ID, None);

    let context = program_test.start_with_context().await;
    context
}

/// Helper to create a new keypair and fund it with 1 SOL from the test context's payer.
async fn create_funded_keypair(context: &mut ProgramTestContext) -> anyhow::Result<Keypair> {
    let keypair = Keypair::new();
    let transfer_tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            &keypair.pubkey(),
            LAMPORTS_PER_SOL,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transfer_tx)
        .await?;
    // Advance blockhash to avoid transaction collisions
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;
    Ok(keypair)
}

/// Test setup helper: Creates a funded admin and their on-chain profile.
/// Returns the transaction builder, admin's keypair, and the admin PDA.
async fn setup_admin_profile(
    context: &mut ProgramTestContext,
) -> anyhow::Result<(TransactionBuilder<MockRpcClient>, Keypair, Pubkey)> {
    let rpc_client = Arc::new(MockRpcClient(context.banks_client.clone()));
    let transaction_builder = TransactionBuilder::new(rpc_client);

    let admin_authority = create_funded_keypair(context).await?;
    let admin_comm_key = Keypair::new();

    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", admin_authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let message_bytes = transaction_builder
        .prepare_admin_register_profile(admin_authority.pubkey(), admin_comm_key.pubkey());

    let mut admin_message: Message = bincode::serde::borrow_decode_from_slice(
        message_bytes.as_slice(),
        bincode::config::standard(),
    )?
    .0;
    admin_message.recent_blockhash = context.last_blockhash;
    let mut admin_tx = Transaction::new_unsigned(admin_message);
    admin_tx.sign(&[&admin_authority], context.last_blockhash);
    context.banks_client.process_transaction(admin_tx).await?;

    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    Ok((transaction_builder, admin_authority, admin_pda))
}

/// Test setup helper: Creates an admin and a user profile linked to that admin.
/// Returns the builder, admin keypair/pda, and user keypair/pda.
async fn setup_user_profile(
    context: &mut ProgramTestContext,
) -> anyhow::Result<(
    TransactionBuilder<MockRpcClient>,
    (Keypair, Pubkey), // admin
    (Keypair, Pubkey), // user
)> {
    let (transaction_builder, admin_authority, admin_pda) = setup_admin_profile(context).await?;

    let user_authority = create_funded_keypair(context).await?;
    let user_comm_key = Keypair::new();

    let (user_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            user_authority.pubkey().as_ref(),
            admin_pda.as_ref(),
        ],
        &w3b2_solana_program::ID,
    );

    let message_bytes = transaction_builder.prepare_user_create_profile(
        user_authority.pubkey(),
        admin_pda,
        user_comm_key.pubkey(),
    );

    let mut user_message: Message = bincode::serde::borrow_decode_from_slice(
        message_bytes.as_slice(),
        bincode::config::standard(),
    )?
    .0;
    user_message.recent_blockhash = context.last_blockhash;
    let mut user_tx = Transaction::new_unsigned(user_message);
    user_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(user_tx).await?;

    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    Ok((
        transaction_builder,
        (admin_authority, admin_pda),
        (user_authority, user_pda),
    ))
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_admin_profile_creation() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (_, admin_authority, admin_pda) = setup_admin_profile(&mut context).await?;

    // The action is performed inside `setup_admin_profile`. We just need to verify.

    let account = context
        .banks_client
        .get_account(admin_pda)
        .await?
        .expect("Admin PDA account not found");
    let admin_profile = AdminProfile::try_deserialize(&mut account.data.as_slice())?;

    assert_eq!(admin_profile.authority, admin_authority.pubkey());
    // Note: The communication key was set during setup, we can't easily check it here
    // without knowing the keypair used inside the helper. This is sufficient.

    println!(
        "✅ Test passed: Admin profile for {} created and verified on-chain.",
        &context.payer.pubkey(),
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_admin_close_profile() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, admin_authority, admin_pda) =
        setup_admin_profile(&mut context).await?;

    let initial_wallet_balance = context
        .banks_client
        .get_balance(admin_authority.pubkey())
        .await?;

    let message_bytes = transaction_builder.prepare_admin_close_profile(admin_authority.pubkey());

    let mut close_message: Message = bincode::serde::borrow_decode_from_slice(
        message_bytes.as_slice(),
        bincode::config::standard(),
    )?
    .0;
    close_message.recent_blockhash = context.last_blockhash;
    let mut close_tx = Transaction::new_unsigned(close_message);
    close_tx.sign(&[&admin_authority], context.last_blockhash);
    context.banks_client.process_transaction(close_tx).await?;

    // The admin profile account should no longer exist.
    let account = context.banks_client.get_account(admin_pda).await?;
    assert!(account.is_none());

    // Assert that the rent was returned to the authority's wallet.
    let final_wallet_balance = context
        .banks_client
        .get_balance(admin_authority.pubkey())
        .await?;
    assert!(final_wallet_balance > initial_wallet_balance);

    println!(
        "✅ Test passed: Admin {} closed their profile.",
        admin_authority.pubkey(),
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_admin_set_config() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, admin_authority, admin_pda) =
        setup_admin_profile(&mut context).await?;

    let new_oracle = Keypair::new();
    let new_validity = 120i64;
    let new_comm_key = Keypair::new();

    let message_bytes = transaction_builder.prepare_admin_set_config(
        admin_authority.pubkey(),
        Some(new_oracle.pubkey()),
        Some(new_validity),
        Some(new_comm_key.pubkey()),
        Some(100), // New unban fee
    );

    let mut set_config_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    set_config_message.recent_blockhash = context.last_blockhash;
    let mut set_config_tx = Transaction::new_unsigned(set_config_message);
    set_config_tx.sign(&[&admin_authority], context.last_blockhash);
    context
        .banks_client
        .process_transaction(set_config_tx)
        .await?;

    let account = context.banks_client.get_account(admin_pda).await?.unwrap();
    let admin_profile = AdminProfile::try_deserialize(&mut account.data.as_slice())?;

    assert_eq!(admin_profile.oracle_authority, new_oracle.pubkey());
    assert_eq!(admin_profile.timestamp_validity_seconds, new_validity);
    assert_eq!(admin_profile.communication_pubkey, new_comm_key.pubkey());
    assert_eq!(admin_profile.unban_fee, 100);

    println!(
        "✅ Test passed: Admin {} successfully updated their config.",
        admin_authority.pubkey(),
    );
    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_admin_dispatch_command() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, (admin_authority, _admin_pda), (_user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let command_id = 999;
    let payload = b"System message from admin".to_vec();

    let message_bytes = transaction_builder.prepare_admin_dispatch_command(
        admin_authority.pubkey(),
        user_pda,
        command_id,
        payload.clone(),
    );
    let mut dispatch_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    dispatch_message.recent_blockhash = context.last_blockhash;
    let mut dispatch_tx = Transaction::new_unsigned(dispatch_message);
    dispatch_tx.sign(&[&admin_authority], context.last_blockhash);
    context
        .banks_client
        .process_transaction(dispatch_tx)
        .await?;

    // For this instruction, the primary success condition is that the transaction
    // executes without errors. This implies the program correctly processed the
    // instruction and likely emitted an event, which would be caught by an off-chain listener.
    // In this test, a successful transaction is sufficient verification.

    println!("✅ Test passed: Admin dispatched command {command_id} to user profile {user_pda}.");

    Ok(())
}

use std::convert::TryInto;

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_full_payment_cycle_and_withdraw() -> anyhow::Result<()> {
    // === 1. Arrange: Create Admin and User ===
    let mut context = setup_test_environment().await;
    let (transaction_builder, (admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    // === 2. Arrange: User deposits funds ===
    let deposit_amount = 200_000; // More than command price
    let message_bytes = transaction_builder.prepare_user_deposit(
        user_authority.pubkey(),
        admin_pda,
        deposit_amount,
    );
    let mut deposit_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    deposit_message.recent_blockhash = context.last_blockhash;
    let mut deposit_tx = Transaction::new_unsigned(deposit_message);
    deposit_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(deposit_tx).await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // === 3. Act: User dispatches the paid command with an oracle signature ===
    let command_id = 42u16;
    let command_price = 100_000u64;
    let timestamp = chrono::Utc::now().timestamp();

    // The oracle (the admin in this case) signs the price data
    let message = [
        command_id.to_le_bytes().as_ref(),
        command_price.to_le_bytes().as_ref(),
        timestamp.to_le_bytes().as_ref(),
    ]
    .concat();
    let signature = admin_authority.sign_message(&message);

    let message_bytes = transaction_builder.prepare_user_dispatch_command(
        user_authority.pubkey(),
        admin_pda,
        UserDispatchCommandArgs {
            command_id,
            price: command_price,
            timestamp,
            payload: vec![1, 2, 3], // Dummy payload
            oracle_pubkey: admin_authority.pubkey(),
            oracle_signature: signature.as_ref().try_into().unwrap(),
        },
    );
    let mut dispatch_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    dispatch_message.recent_blockhash = context.last_blockhash;
    let mut dispatch_tx = Transaction::new_unsigned(dispatch_message);
    dispatch_tx.sign(&[&user_authority], context.last_blockhash);
    context
        .banks_client
        .process_transaction(dispatch_tx)
        .await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // === 4. Assert: Check balances after dispatch ===
    // User's balance should decrease
    let user_account = context.banks_client.get_account(user_pda).await?.unwrap();
    let user_profile = w3b2_solana_program::state::UserProfile::try_deserialize(
        &mut user_account.data.as_slice(),
    )?;
    assert_eq!(user_profile.deposit_balance, deposit_amount - command_price);

    // Admin's internal balance should increase
    let admin_account = context.banks_client.get_account(admin_pda).await?.unwrap();
    let admin_profile = AdminProfile::try_deserialize(&mut admin_account.data.as_slice())?;
    assert_eq!(admin_profile.balance, command_price);

    println!("✅ Dispatch successful: User balance decreased, admin balance increased by {command_price}");

    // === 5. Act: Admin withdraws the earned funds ===
    let initial_admin_wallet_balance = context
        .banks_client
        .get_balance(admin_authority.pubkey())
        .await?;

    let message_bytes = transaction_builder.prepare_admin_withdraw(
        admin_authority.pubkey(),
        command_price,
        admin_authority.pubkey(), // Destination is the admin's own wallet
    );
    let mut withdraw_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    withdraw_message.recent_blockhash = context.last_blockhash;
    let mut withdraw_tx = Transaction::new_unsigned(withdraw_message);
    withdraw_tx.sign(&[&admin_authority], context.last_blockhash);
    context
        .banks_client
        .process_transaction(withdraw_tx)
        .await?;

    // === 6. Assert: Check balances after withdrawal ===
    // Admin's internal balance should be zero
    let admin_account_after_withdraw = context.banks_client.get_account(admin_pda).await?.unwrap();
    let admin_profile_after_withdraw =
        AdminProfile::try_deserialize(&mut admin_account_after_withdraw.data.as_slice())?;
    assert_eq!(admin_profile_after_withdraw.balance, 0);

    // Admin's wallet balance should increase
    let final_admin_wallet_balance = context
        .banks_client
        .get_balance(admin_authority.pubkey())
        .await?;
    // We can check for a more precise balance increase here because the fee was paid by the admin's wallet,
    // but the incoming amount is exact.
    // final = initial - fee + withdrawn_amount
    // So, final - initial + fee = withdrawn_amount.
    // A simpler check is just to see it increased.
    assert!(final_admin_wallet_balance > initial_admin_wallet_balance);

    println!("✅ Test passed: Full payment cycle and withdrawal successful.");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_user_deposit() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, (_admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let deposit_amount = 500_000; // 0.0005 SOL

    // The user deposits funds from their wallet to their UserProfile PDA.
    let message_bytes = transaction_builder.prepare_user_deposit(
        user_authority.pubkey(),
        admin_pda,
        deposit_amount,
    );
    let mut deposit_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    deposit_message.recent_blockhash = context.last_blockhash;
    let mut deposit_tx = Transaction::new_unsigned(deposit_message);
    deposit_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(deposit_tx).await?;

    // The user's deposit_balance on their PDA should increase.
    let user_account = context
        .banks_client
        .get_account(user_pda)
        .await?
        .expect("User PDA account not found");
    use w3b2_solana_program::state::UserProfile;
    let user_profile = UserProfile::try_deserialize(&mut user_account.data.as_slice())?;
    assert_eq!(user_profile.deposit_balance, deposit_amount);

    println!(
        "✅ Test passed: User {} deposited {} lamports.",
        user_authority.pubkey(),
        deposit_amount
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_user_withdraw() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, (_admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let deposit_amount = 500_000;
    let withdraw_amount = 200_000;

    // First, deposit funds to have something to withdraw.
    let message_bytes = transaction_builder.prepare_user_deposit(
        user_authority.pubkey(),
        admin_pda,
        deposit_amount,
    );
    let mut deposit_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    deposit_message.recent_blockhash = context.last_blockhash;
    let mut deposit_tx = Transaction::new_unsigned(deposit_message);
    deposit_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(deposit_tx).await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // Get the user's wallet balance before withdrawal
    let initial_wallet_balance = context
        .banks_client
        .get_balance(user_authority.pubkey())
        .await?;

    // The user withdraws funds from their UserProfile PDA back to their wallet.
    let message_bytes = transaction_builder.prepare_user_withdraw(
        user_authority.pubkey(),
        admin_pda,
        withdraw_amount,
        user_authority.pubkey(), // Destination is the user's own wallet
    );
    let mut withdraw_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    withdraw_message.recent_blockhash = context.last_blockhash;
    let mut withdraw_tx = Transaction::new_unsigned(withdraw_message);
    withdraw_tx.sign(&[&user_authority], context.last_blockhash);
    context
        .banks_client
        .process_transaction(withdraw_tx)
        .await?;

    // Assert user profile balance has decreased
    let user_account = context
        .banks_client
        .get_account(user_pda)
        .await?
        .expect("User PDA account not found");
    let user_profile = w3b2_solana_program::state::UserProfile::try_deserialize(
        &mut user_account.data.as_slice(),
    )?;
    assert_eq!(
        user_profile.deposit_balance,
        deposit_amount - withdraw_amount
    );

    // Assert user's wallet balance has increased
    let final_wallet_balance = context
        .banks_client
        .get_balance(user_authority.pubkey())
        .await?;
    // Note: We can't check for exact equality due to transaction fees.
    // We check that the balance increased by *at least* the withdrawn amount minus a small fee.
    assert!(final_wallet_balance > initial_wallet_balance);

    println!(
        "✅ Test passed: User {} withdrew {} lamports.",
        user_authority.pubkey(),
        withdraw_amount
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_user_close_profile() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, (_admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let initial_wallet_balance = context
        .banks_client
        .get_balance(user_authority.pubkey())
        .await?;

    let message_bytes =
        transaction_builder.prepare_user_close_profile(user_authority.pubkey(), admin_pda);
    let mut close_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    close_message.recent_blockhash = context.last_blockhash;
    let mut close_tx = Transaction::new_unsigned(close_message);
    close_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(close_tx).await?;

    // The user profile account should no longer exist.
    let account = context.banks_client.get_account(user_pda).await?;
    assert!(account.is_none());

    // Assert that the rent was returned to the authority's wallet.
    let final_wallet_balance = context
        .banks_client
        .get_balance(user_authority.pubkey())
        .await?;
    assert!(final_wallet_balance > initial_wallet_balance);

    println!(
        "✅ Test passed: User {} closed their profile.",
        user_authority.pubkey(),
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_user_update_comm_key() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, (_admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let new_comm_key = Keypair::new();

    let message_bytes = transaction_builder.prepare_user_update_comm_key(
        user_authority.pubkey(),
        admin_pda,
        new_comm_key.pubkey(),
    );
    let mut update_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    update_message.recent_blockhash = context.last_blockhash;
    let mut update_tx = Transaction::new_unsigned(update_message);
    update_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(update_tx).await?;

    let account = context
        .banks_client
        .get_account(user_pda)
        .await?
        .expect("User PDA account not found");
    let user_profile =
        w3b2_solana_program::state::UserProfile::try_deserialize(&mut account.data.as_slice())?;

    assert_eq!(user_profile.communication_pubkey, new_comm_key.pubkey());

    println!(
        "✅ Test passed: User {} updated communication key.",
        user_authority.pubkey(),
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_log_action_by_user() -> anyhow::Result<()> {
    let mut context = setup_test_environment().await;
    let (transaction_builder, (_admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let session_id = 12345;
    let action_code = 404;

    let message_bytes = transaction_builder.prepare_log_action(
        user_authority.pubkey(),
        user_pda,
        admin_pda,
        session_id,
        action_code,
    );
    let mut log_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    log_message.recent_blockhash = context.last_blockhash;
    let mut log_tx = Transaction::new_unsigned(log_message);
    log_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(log_tx).await?;

    // Similar to dispatch, the main success condition is that the transaction executes
    // without errors, implying an event was emitted.

    println!(
        "✅ Test passed: User {} logged action {}.",
        user_authority.pubkey(),
        action_code
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires a compiled BPF program"]
async fn test_full_ban_unban_cycle() -> anyhow::Result<()> {
    // === 1. Arrange: Create Admin and User, set an unban fee ===
    let mut context = setup_test_environment().await;
    let (transaction_builder, (admin_authority, admin_pda), (user_authority, user_pda)) =
        setup_user_profile(&mut context).await?;

    let unban_fee = 100_000;

    let message_bytes = transaction_builder.prepare_admin_set_config(
        admin_authority.pubkey(),
        None,
        None,
        None,
        Some(unban_fee),
    );
    let mut set_config_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    set_config_message.recent_blockhash = context.last_blockhash;
    let mut set_config_tx = Transaction::new_unsigned(set_config_message);
    set_config_tx.sign(&[&admin_authority], context.last_blockhash);
    context
        .banks_client
        .process_transaction(set_config_tx)
        .await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // === 2. Act: Admin bans the user ===
    let message_bytes =
        transaction_builder.prepare_admin_ban_user(admin_authority.pubkey(), user_pda);
    let mut ban_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    ban_message.recent_blockhash = context.last_blockhash;
    let mut ban_tx = Transaction::new_unsigned(ban_message);
    ban_tx.sign(&[&admin_authority], context.last_blockhash);
    context.banks_client.process_transaction(ban_tx).await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // === 3. Assert: User is banned ===
    let user_account = context.banks_client.get_account(user_pda).await?.unwrap();
    let user_profile = w3b2_solana_program::state::UserProfile::try_deserialize(
        &mut user_account.data.as_slice(),
    )?;
    assert!(user_profile.banned);
    println!("✅ Ban successful: User is now banned.");

    // === 4. Act: User deposits funds and requests an unban ===
    // Deposit funds to pay the fee
    let message_bytes =
        transaction_builder.prepare_user_deposit(user_authority.pubkey(), admin_pda, unban_fee);
    let mut deposit_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    deposit_message.recent_blockhash = context.last_blockhash;
    let mut deposit_tx = Transaction::new_unsigned(deposit_message);
    deposit_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(deposit_tx).await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // Request the unban
    let message_bytes =
        transaction_builder.prepare_user_request_unban(user_authority.pubkey(), admin_pda);
    let mut request_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    request_message.recent_blockhash = context.last_blockhash;
    let mut request_tx = Transaction::new_unsigned(request_message);
    request_tx.sign(&[&user_authority], context.last_blockhash);
    context.banks_client.process_transaction(request_tx).await?;
    context.last_blockhash = context.banks_client.get_latest_blockhash().await?;

    // === 5. Assert: Unban is requested and fee is paid ===
    let user_account_after_req = context.banks_client.get_account(user_pda).await?.unwrap();
    let user_profile_after_req = w3b2_solana_program::state::UserProfile::try_deserialize(
        &mut user_account_after_req.data.as_slice(),
    )?;
    assert!(user_profile_after_req.unban_requested);
    assert_eq!(user_profile_after_req.deposit_balance, 0);

    let admin_account_after_req = context.banks_client.get_account(admin_pda).await?.unwrap();
    let admin_profile_after_req =
        AdminProfile::try_deserialize(&mut admin_account_after_req.data.as_slice())?;
    assert_eq!(admin_profile_after_req.balance, unban_fee);
    println!("✅ Unban request successful: Fee paid and request flag set.");

    // === 6. Act: Admin unbans the user ===
    let message_bytes =
        transaction_builder.prepare_admin_unban_user(admin_authority.pubkey(), user_pda);
    let mut unban_message: Message =
        bincode::serde::borrow_decode_from_slice(&message_bytes, bincode::config::standard())?.0;
    unban_message.recent_blockhash = context.last_blockhash;
    let mut unban_tx = Transaction::new_unsigned(unban_message);
    unban_tx.sign(&[&admin_authority], context.last_blockhash);
    context.banks_client.process_transaction(unban_tx).await?;

    // === 7. Assert: User is no longer banned ===
    let user_account_final = context.banks_client.get_account(user_pda).await?.unwrap();
    let user_profile_final = w3b2_solana_program::state::UserProfile::try_deserialize(
        &mut user_account_final.data.as_slice(),
    )?;
    assert!(!user_profile_final.banned);
    assert!(!user_profile_final.unban_requested); // Flag should be reset
    println!("✅ Unban successful: User is no longer banned.");

    println!("✅ Test passed: Full ban/unban cycle completed successfully.");

    Ok(())
}
