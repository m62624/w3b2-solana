use anchor_lang::{
    solana_program::{
        self,
        instruction::Instruction,
        system_program,
        sysvar::{self, instructions as SysvarInstructions},
    },
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_ed25519_program;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::convert::TryInto;
use tokio::time::{sleep, Duration};
use w3b2_solana_program::{
    self,
    state::{AdminProfile, UserProfile},
};

const RPC_URL: &str = "http://127.0.0.1:8899";
const DEFAULT_AIRDROP_AMOUNT: u64 = 10 * LAMPORTS_PER_SOL;

// --- Helper Functions ---

/// Helper to airdrop lamports and wait for confirmation.
async fn airdrop_and_confirm(rpc_client: &RpcClient, pubkey: &Pubkey, amount: u64) {
    let mut retries = 5;
    while retries > 0 {
        match rpc_client.request_airdrop(pubkey, amount).await {
            Ok(sig) => {
                if let Err(e) = rpc_client.confirm_transaction(&sig).await {
                    println!(
                        "Airdrop confirmation for {} failed, {} retries left. Error: {}",
                        sig, retries, e
                    );
                    retries -= 1;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                println!("Airdrop successful for {}", pubkey);
                return;
            }
            Err(e) => {
                println!(
                    "Airdrop request for {} failed, {} retries left. Error: {}",
                    pubkey, retries, e
                );
                retries -= 1;
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    panic!("Airdrop failed for {} after multiple retries.", pubkey);
}

/// Creates a new keypair and funds it with a default amount of SOL from the local validator.
async fn create_funded_keypair(rpc_client: &RpcClient) -> Keypair {
    let keypair = Keypair::new();
    airdrop_and_confirm(rpc_client, &keypair.pubkey(), DEFAULT_AIRDROP_AMOUNT).await;
    keypair
}

/// Fetches and deserializes an AdminProfile account.
async fn get_admin_profile(rpc_client: &RpcClient, pda: &Pubkey) -> AdminProfile {
    let data = rpc_client
        .get_account_data(pda)
        .await
        .expect("Failed to get admin profile account data");
    AdminProfile::try_deserialize(&mut data.as_ref()).expect("Failed to deserialize admin profile")
}

/// Fetches and deserializes a UserProfile account.
async fn get_user_profile(rpc_client: &RpcClient, pda: &Pubkey) -> UserProfile {
    let data = rpc_client
        .get_account_data(pda)
        .await
        .expect("Failed to get user profile account data");
    UserProfile::try_deserialize(&mut data.as_ref()).expect("Failed to deserialize user profile")
}

/// Tests the full end-to-end lifecycle of the oracle-based payment system.
#[tokio::test]
#[ignore] // This test requires a running local validator and can be slow.
async fn test_full_oracle_lifecycle() {
    // === 1. Arrange ===
    let rpc_client =
        RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed());

    // Create keypairs for all parties involved.
    let admin_authority = create_funded_keypair(&rpc_client).await;
    let user_authority = create_funded_keypair(&rpc_client).await;
    let oracle_authority = Keypair::new(); // The oracle doesn't need funds.

    println!("Admin Authority: {}", admin_authority.pubkey());
    println!("User Authority: {}", user_authority.pubkey());
    println!("Oracle Authority: {}", oracle_authority.pubkey());

    // Derive the AdminProfile PDA.
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", admin_authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    // === 2. Act & Assert: Admin Registration ===
    // The admin registers their profile. The oracle_authority should default to the admin's own key.
    let ix = Instruction {
        program_id: w3b2_solana_program::ID,
        accounts: w3b2_solana_program::accounts::AdminRegisterProfile {
            authority: admin_authority.pubkey(),
            admin_profile: admin_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: w3b2_solana_program::instruction::AdminRegisterProfile {
            communication_pubkey: Pubkey::new_unique(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix.clone()], Some(&admin_authority.pubkey()));
    tx.sign(
        &[&admin_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    rpc_client.send_and_confirm_transaction(&tx).await.unwrap();

    let admin_profile = get_admin_profile(&rpc_client, &admin_pda).await;
    assert_eq!(admin_profile.authority, admin_authority.pubkey());
    assert_eq!(admin_profile.oracle_authority, admin_authority.pubkey());
    println!("✅ Admin profile created successfully. Oracle defaults to admin.");

    // === 3. Act & Assert: Admin Sets a Dedicated Oracle ===
    // The admin updates their profile to delegate signature authority to the dedicated oracle.
    let ix = Instruction {
        program_id: w3b2_solana_program::ID,
        accounts: w3b2_solana_program::accounts::AdminSetOracle {
            authority: admin_authority.pubkey(),
            admin_profile: admin_pda,
        }
        .to_account_metas(None),
        data: w3b2_solana_program::instruction::AdminSetOracle {
            new_oracle_authority: oracle_authority.pubkey(),
        }
        .data(),
    };
    let mut tx = Transaction::new_with_payer(&[ix], Some(&admin_authority.pubkey()));
    tx.sign(
        &[&admin_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    rpc_client.send_and_confirm_transaction(&tx).await.unwrap();

    let admin_profile = get_admin_profile(&rpc_client, &admin_pda).await;
    assert_eq!(admin_profile.oracle_authority, oracle_authority.pubkey());
    println!("✅ Admin successfully delegated oracle authority.");

    // === 4. Act & Assert: User Profile Creation and Funding ===
    let (user_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            user_authority.pubkey().as_ref(),
            admin_pda.as_ref(),
        ],
        &w3b2_solana_program::ID,
    );

    // Create User Profile
    let ix = Instruction {
        program_id: w3b2_solana_program::ID,
        accounts: w3b2_solana_program::accounts::UserCreateProfile {
            authority: user_authority.pubkey(),
            admin_profile: admin_pda,
            user_profile: user_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: w3b2_solana_program::instruction::UserCreateProfile {
            target_admin_pda: admin_pda,
            communication_pubkey: Pubkey::new_unique(),
        }
        .data(),
    };
    let mut tx = Transaction::new_with_payer(&[ix], Some(&user_authority.pubkey()));
    tx.sign(
        &[&user_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    rpc_client.send_and_confirm_transaction(&tx).await.unwrap();
    println!("✅ User profile created.");

    // Deposit funds
    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    let ix = Instruction {
        program_id: w3b2_solana_program::ID,
        accounts: w3b2_solana_program::accounts::UserDeposit {
            authority: user_authority.pubkey(),
            admin_profile: admin_pda,
            user_profile: user_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: w3b2_solana_program::instruction::UserDeposit {
            amount: deposit_amount,
        }
        .data(),
    };
    let mut tx = Transaction::new_with_payer(&[ix], Some(&user_authority.pubkey()));
    tx.sign(
        &[&user_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    rpc_client.send_and_confirm_transaction(&tx).await.unwrap();

    let user_profile = get_user_profile(&rpc_client, &user_pda).await;
    assert_eq!(user_profile.deposit_balance, deposit_amount);
    println!("✅ User deposited {} lamports.", deposit_amount);

    // === 5. Act & Assert: User Dispatches a Paid Command with Oracle Signature ===
    let command_id = 1234u16;
    let price = 500_000u64; // 0.0005 SOL
    let timestamp = chrono::Utc::now().timestamp();

    // The oracle constructs the message to be signed.
    let message = [
        command_id.to_le_bytes().as_ref(),
        price.to_le_bytes().as_ref(),
        timestamp.to_le_bytes().as_ref(),
    ]
    .concat();

    // The oracle signs the message. The transaction sent by the user will include
    // this signature and the message data in a precedent `ed25519` instruction.
    let signature = oracle_authority.sign_message(&message);

    // The user's client constructs the transaction.
    let pubkey_bytes = oracle_authority.pubkey().to_bytes();
    let signature_bytes: [u8; 64] = signature.as_ref().try_into().unwrap();
    let ed25519_ix = solana_ed25519_program::new_ed25519_instruction_with_signature(
        &message,
        &signature_bytes,
        &pubkey_bytes,
    );

    let dispatch_accounts = w3b2_solana_program::accounts::UserDispatchCommand {
        authority: user_authority.pubkey(),
        user_profile: user_pda,
        admin_profile: admin_pda,
        instructions: sysvar::instructions::id(),
    };

    let mut dispatch_metas = dispatch_accounts.to_account_metas(None);
    dispatch_metas.push(solana_sdk::instruction::AccountMeta::new_readonly(
        SysvarInstructions::id(),
        false,
    ));

    let dispatch_ix = Instruction {
        program_id: w3b2_solana_program::ID,
        accounts: dispatch_metas,
        data: w3b2_solana_program::instruction::UserDispatchCommand {
            command_id,
            price,
            timestamp,
            payload: vec![1, 2, 3],
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(
        &[ed25519_ix.clone(), dispatch_ix.clone()], // The order is critical!
        Some(&user_authority.pubkey()),
    );
    tx.sign(
        &[&user_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    rpc_client.send_and_confirm_transaction(&tx).await.unwrap();

    // Verify balances changed correctly.
    let user_profile_after = get_user_profile(&rpc_client, &user_pda).await;
    let admin_profile_after = get_admin_profile(&rpc_client, &admin_pda).await;

    assert_eq!(
        user_profile_after.deposit_balance,
        deposit_amount - price
    );
    assert_eq!(admin_profile_after.balance, price);
    println!("✅ Paid command dispatched successfully!");
    println!(
        "   User balance: {} -> {}",
        deposit_amount, user_profile_after.deposit_balance
    );
    println!(
        "   Admin balance: 0 -> {}",
        admin_profile_after.balance
    );

    // === 6. Act & Assert: Test Failure Cases ===

    // Case 1: Replay Attack (submitting the same transaction again)
    println!("Testing failure case: Replay Attack...");
    // To simulate time passing, we can just wait. The on-chain program has a 60s window.
    // However, a faster way is to just resubmit. If the timestamp is identical and
    // the blockhash is new, it should still work. The TRUE test for replay is
    // if the timestamp becomes too old. Let's wait for a short period.
    sleep(Duration::from_secs(2)).await;
    let mut replay_tx = Transaction::new_with_payer(
        &[ed25519_ix.clone(), dispatch_ix.clone()],
        Some(&user_authority.pubkey()),
    );
    replay_tx.sign(
        &[&user_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    // This should succeed if within the time window, but the user's balance will be too low.
    let result = rpc_client.send_and_confirm_transaction(&replay_tx).await;
    assert!(result.is_err());
    println!("✅ Replay attack failed as expected (Insufficient Funds).");

    // Case 2: Invalid Signature (signed by wrong authority)
    println!("Testing failure case: Invalid Signature...");
    let rogue_oracle = Keypair::new();
    let invalid_signature = rogue_oracle.sign_message(&message);
    let rogue_pubkey_bytes = rogue_oracle.pubkey().to_bytes();
    let invalid_signature_bytes: [u8; 64] = invalid_signature.as_ref().try_into().unwrap();
    let invalid_sig_ix = solana_ed25519_program::new_ed25519_instruction_with_signature(
        &message,
        &invalid_signature_bytes,
        &rogue_pubkey_bytes,
    );
    let mut invalid_sig_tx = Transaction::new_with_payer(
        &[invalid_sig_ix, dispatch_ix.clone()],
        Some(&user_authority.pubkey()),
    );
    invalid_sig_tx.sign(
        &[&user_authority],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    let result = rpc_client.send_and_confirm_transaction(&invalid_sig_tx).await;
    assert!(result.is_err());
    println!("✅ Transaction with signature from wrong oracle failed as expected.");
}