use anyhow::{Context, Result};
use futures::stream::StreamExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tracing::info;
use w3b2_solana_connector::client::TransactionBuilder;
use w3b2_solana_proto::services::gateway::{
    bridge_gateway_service_client::BridgeGatewayServiceClient, ListenRequest,
    PrepareAdminDispatchCommandRequest, SubmitTransactionRequest,
};

const RPC_URL: &str = "http://127.0.0.1:8899";
const GATEWAY_URL: &str = "http://127.0.0.1:50051";

#[tokio::main]
async fn main() -> Result<()> {
    // --- 0. Setup ---
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        RPC_URL.to_string(),
        CommitmentConfig::confirmed(),
    ));
    let builder = TransactionBuilder::new(rpc_client.clone());

    // Create keypairs for admin and user
    let admin_authority = Keypair::new();
    let user_authority = Keypair::new();

    // Fund them so they can pay for transactions
    // In a real scenario, these would already have funds.
    // This part requires a local validator running.
    // await_funding(&rpc_client, &admin_authority.pubkey()).await?;
    // await_funding(&rpc_client, &user_authority.pubkey()).await?;

    // Create profiles on-chain (this part uses the builder directly for simplicity)
    let (admin_pda, user_pda) =
        setup_profiles(&builder, &admin_authority, &user_authority).await?;

    info!("Admin PDA: {}", admin_pda);
    info!("User PDA: {}", user_pda);

    // --- Interaction via gRPC Gateway ---
    let command_id = 101;
    let payload = b"Welcome to the platform!".to_vec();

    let mut grpc_client = BridgeGatewayServiceClient::connect(GATEWAY_URL).await?;

    // 1. Client requests the gateway to prepare a message
    let prepare_req = PrepareAdminDispatchCommandRequest {
        authority_pubkey: admin_authority.pubkey().to_string(),
        target_user_profile_pda: user_pda.to_string(),
        command_id,
        payload,
    };

    let prepare_res = grpc_client
        .prepare_admin_dispatch_command(prepare_req)
        .await?
        .into_inner();

    // 2. Client gets the latest blockhash from the gateway
    let blockhash_res = grpc_client.get_latest_blockhash(()).await?.into_inner();
    let blockhash = Hash::new(&blockhash_res.blockhash);

    // 3. Client patches the message with the real blockhash
    let mut message_bytes = prepare_res.unsigned_tx_message;
    let offset = prepare_res.blockhash_placeholder_offset as usize;
    message_bytes[offset..offset + 32].copy_from_slice(&blockhash.to_bytes());

    // 4. Client deserializes the patched message and signs it
    let message = bincode::deserialize(&message_bytes).context("Failed to deserialize message")?;
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(&[&admin_authority], blockhash);

    // 5. Client submits the signed transaction through the gateway
    let submit_req = SubmitTransactionRequest {
        signed_tx: bincode::serialize(&tx).context("Failed to serialize transaction")?,
    };
    let submit_res = grpc_client.submit_transaction(submit_req).await?.into_inner();
    let signature = submit_res.signature;

    info!(
        "Admin dispatch transaction sent successfully! Signature: {}",
        signature
    );

    Ok(())
}

/// Helper function to create and fund admin and user profiles for the example.
/// In a real application, this would likely be done through a UI or separate setup script.
async fn setup_profiles(
    builder: &TransactionBuilder<RpcClient>,
    admin: &Keypair,
    user: &Keypair,
) -> Result<(Pubkey, Pubkey)> {
    // Admin registers a profile
    let admin_comm_key = Keypair::new().pubkey();
    let (admin_message_bytes, admin_offset) = builder
        .prepare_admin_register_profile(admin.pubkey(), admin_comm_key)
        .await?;
    let blockhash = builder.rpc_client().get_latest_blockhash().await?;
    let mut final_admin_msg_bytes = admin_message_bytes;
    final_admin_msg_bytes[admin_offset as usize..admin_offset as usize + 32]
        .copy_from_slice(&blockhash.to_bytes());
    let admin_message = bincode::deserialize(&final_admin_msg_bytes)?;
    let mut admin_tx = Transaction::new_unsigned(admin_message);
    admin_tx.sign(&[admin], blockhash);
    builder.submit_transaction(&admin_tx).await?;

    let (admin_pda, _) =
        Pubkey::find_program_address(&[b"admin", admin.pubkey().as_ref()], &w3b2_solana_program::ID);

    // User creates a profile linked to the admin
    let user_comm_key = Keypair::new().pubkey();
    let (user_message_bytes, user_offset) = builder
        .prepare_user_create_profile(user.pubkey(), admin_pda, user_comm_key)
        .await?;
    let blockhash = builder.rpc_client().get_latest_blockhash().await?;
    let mut final_user_msg_bytes = user_message_bytes;
    final_user_msg_bytes[user_offset as usize..user_offset as usize + 32]
        .copy_from_slice(&blockhash.to_bytes());
    let user_message = bincode::deserialize(&final_user_msg_bytes)?;
    let mut user_tx = Transaction::new_unsigned(user_message);
    user_tx.sign(&[user], blockhash);
    builder.submit_transaction(&user_tx).await?;

    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", user.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    Ok((admin_pda, user_pda))
}