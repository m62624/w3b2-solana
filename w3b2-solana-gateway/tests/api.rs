use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    message::Message,
    native_token::LAMPORTS_PER_SOL,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::{env, time::Duration};
use tonic::Request;
use w3b2_solana_gateway::grpc::proto::w3b2::protocol::gateway::{
    bridge_gateway_service_client::BridgeGatewayServiceClient, PrepareAdminRegisterProfileRequest,
    SubmitTransactionRequest,
};

/// Constructs the gateway URL from environment variables, with fallbacks for Docker.
fn get_gateway_url() -> String {
    let host = env::var("GATEWAY_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("GATEWAY_PORT_EXTERNAL").unwrap_or_else(|_| "50051".to_string());
    format!("http://{}:{}", host, port)
}

/// Constructs the Solana RPC URL from environment variables.
fn get_rpc_url() -> String {
    let host = env::var("SOLANA_RPC_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port =
        env::var("SOLANA_VALIDATOR_RPC_PORT_EXTERNAL").unwrap_or_else(|_| "8899".to_string());
    format!("http://{}:{}", host, port)
}

/// A helper struct to manage test state and provide common utilities.
struct TestHarness {
    rpc_client: RpcClient,
}

impl TestHarness {
    fn new() -> Self {
        Self {
            rpc_client: RpcClient::new(get_rpc_url()),
        }
    }

    /// Creates a new keypair and reliably funds it by waiting for the balance to update.
    async fn create_funded_keypair(&self, sol_amount: f64) -> anyhow::Result<Keypair> {
        let keypair = Keypair::new();
        println!("🔑 New Keypair: {}", keypair.pubkey());

        let lamports = (sol_amount * LAMPORTS_PER_SOL as f64) as u64;

        println!(
            "💸 Requesting airdrop of {} SOL to {}...",
            sol_amount,
            keypair.pubkey()
        );
        let signature = self
            .rpc_client
            .request_airdrop(&keypair.pubkey(), lamports)
            .await?;
        self.rpc_client
            .confirm_transaction_with_commitment(&signature, self.rpc_client.commitment())
            .await?;
        println!("✅ Airdrop confirmed.");

        // Reliably wait for the validator's state to reflect the new balance.
        println!("   -> Waiting for balance to update on-chain...");
        loop {
            let balance = self.rpc_client.get_balance(&keypair.pubkey()).await?;
            if balance >= lamports {
                println!("   -> Balance is now {} lamports. Proceeding.", balance);
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        Ok(keypair)
    }
}

#[tokio::test]
#[ignore = "run via docker with the required program id"]
async fn test_create_admin_profile_from_scratch() -> anyhow::Result<()> {
    // === 1. Arrange ===
    println!("--- ARRANGE ---");

    let harness = TestHarness::new();
    let mut grpc_client = BridgeGatewayServiceClient::connect(get_gateway_url()).await?;

    // Use the harness to create a new, reliably funded keypair.
    let admin_authority = harness.create_funded_keypair(1.0).await?;

    println!("\n--- ACT ---");

    let prepare_req = Request::new(PrepareAdminRegisterProfileRequest {
        authority_pubkey: admin_authority.pubkey().to_string(),
        communication_pubkey: Keypair::new().pubkey().to_string(),
    });

    println!("1. Calling 'prepare_admin_register_profile'...");
    let response = grpc_client
        .prepare_admin_register_profile(prepare_req)
        .await?
        .into_inner();

    let unsigned_tx_message_bytes = response.unsigned_tx_message;
    println!(
        "   -> Received {} bytes of an unsigned transaction message.",
        unsigned_tx_message_bytes.len()
    );

    let (message, _): (Message, usize) = bincode::serde::borrow_decode_from_slice(
        &unsigned_tx_message_bytes,
        bincode::config::standard(),
    )?;

    println!("2. Calling 'get_latest_blockhash'...");
    let blockhash_response = grpc_client.get_latest_blockhash(()).await?.into_inner();
    let blockhash = Hash::new_from_array(blockhash_response.blockhash.try_into().unwrap());
    println!("   -> Received blockhash: {}", blockhash);

    println!("3. Signing the transaction locally...");
    let tx = Transaction::new(&[&admin_authority], message, blockhash);

    println!("4. Calling 'submit_transaction'...");
    let signed_tx_bytes = bincode::serde::encode_to_vec(&tx, bincode::config::standard())?;
    let submit_req = Request::new(SubmitTransactionRequest {
        signed_tx: signed_tx_bytes,
    });
    let submit_response = grpc_client
        .submit_transaction(submit_req)
        .await?
        .into_inner();
    println!(
        "   -> Transaction submitted! Signature: {}",
        submit_response.signature
    );

    println!("\n--- ASSERT ---");

    println!("✅ Test completed successfully!");

    Ok(())
}
