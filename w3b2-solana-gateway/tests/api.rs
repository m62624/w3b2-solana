use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    message::Message,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::{env, str::FromStr, time::Duration};
use tokio::time::timeout;
use tonic::Request;
use w3b2_solana_gateway::grpc::proto::w3b2::protocol::gateway::{
    bridge_event::Event, bridge_gateway_service_client::BridgeGatewayServiceClient,
    EventStreamItem, ListenRequest, PrepareAdminRegisterProfileRequest,
    PrepareUserCreateProfileRequest, SubmitTransactionRequest,
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
    grpc_client: BridgeGatewayServiceClient<tonic::transport::Channel>,
    rpc_client: RpcClient,
    program_id: Pubkey,
}

impl TestHarness {
    async fn new() -> Self {
        Self {
            grpc_client: BridgeGatewayServiceClient::connect(get_gateway_url())
                .await
                .unwrap(),
            rpc_client: RpcClient::new(get_rpc_url()),
            program_id: Pubkey::from_str(&env::var("W3B2_CONNECTOR__PROGRAM_ID").unwrap()).unwrap(),
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

    /// A generic helper to prepare, sign, and submit a transaction.
    /// It takes the unsigned message, signs it, and sends it.
    async fn sign_and_submit(
        &mut self,
        unsigned_tx_msg: Vec<u8>,
        signers: &[&Keypair],
    ) -> anyhow::Result<String> {
        // 1. Decode the message from bytes
        let (message, _): (Message, _) = bincode::serde::borrow_decode_from_slice(
            &unsigned_tx_msg,
            bincode::config::standard(),
        )?;

        // 2. Get a fresh blockhash from the gateway
        let blockhash_bytes = self
            .grpc_client
            .get_latest_blockhash(())
            .await?
            .into_inner()
            .blockhash;
        let blockhash = Hash::new_from_array(blockhash_bytes.try_into().unwrap());

        // 3. Create and sign the transaction
        let tx = Transaction::new(signers, message, blockhash);

        // 4. Serialize and submit
        let signed_tx_bytes = bincode::serde::encode_to_vec(&tx, bincode::config::standard())?;
        let submit_req = Request::new(SubmitTransactionRequest {
            signed_tx: signed_tx_bytes,
        });
        let submit_response = self
            .grpc_client
            .submit_transaction(submit_req)
            .await?
            .into_inner();

        Ok(submit_response.signature)
    }

    /// A high-level helper to create an admin profile.
    async fn create_admin_profile(&mut self, authority: &Keypair) -> anyhow::Result<Pubkey> {
        let prepare_req = Request::new(PrepareAdminRegisterProfileRequest {
            authority_pubkey: authority.pubkey().to_string(),
            communication_pubkey: Keypair::new().pubkey().to_string(),
        });
        let response = self
            .grpc_client
            .prepare_admin_register_profile(prepare_req)
            .await?
            .into_inner();
        self.sign_and_submit(response.unsigned_tx_message, &[authority])
            .await?;
        let (pda, _) = Pubkey::find_program_address(
            &[b"admin", authority.pubkey().as_ref()],
            &self.program_id,
        );
        Ok(pda)
    }

    /// A high-level helper to create a user profile linked to an admin.
    async fn create_user_profile(
        &mut self,
        authority: &Keypair,
        admin_pda: Pubkey,
    ) -> anyhow::Result<Pubkey> {
        let prepare_req = Request::new(PrepareUserCreateProfileRequest {
            authority_pubkey: authority.pubkey().to_string(),
            target_admin_pda: admin_pda.to_string(),
            communication_pubkey: Keypair::new().pubkey().to_string(),
        });
        let response = self
            .grpc_client
            .prepare_user_create_profile(prepare_req)
            .await?
            .into_inner();
        self.sign_and_submit(response.unsigned_tx_message, &[authority])
            .await?;
        let (pda, _) = Pubkey::find_program_address(
            &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
            &self.program_id,
        );
        Ok(pda)
    }
}

/// Helper to listen for the next live event for an Admin PDA.
///
/// It subscribes to the live event stream and waits for a specified duration
/// for the first event to arrive.
async fn listen_for_admin_event(
    mut client: BridgeGatewayServiceClient<tonic::transport::Channel>,
    pda: Pubkey,
) -> Option<Event> {
    let request = Request::new(ListenRequest {
        pda: pda.to_string(),
    });
    let mut stream = client
        .stream_admin_live_events(request)
        .await
        .unwrap()
        .into_inner();
    match timeout(Duration::from_secs(20), stream.message()).await {
        Ok(Ok(Some(EventStreamItem {
            event: Some(bridge_event),
            ..
        }))) => bridge_event.event,
        _ => None,
    }
}

/// Helper to listen for the next live event for a User PDA.
async fn listen_for_user_event(
    mut client: BridgeGatewayServiceClient<tonic::transport::Channel>,
    pda: Pubkey,
) -> Option<Event> {
    let request = Request::new(ListenRequest {
        pda: pda.to_string(),
    });
    let mut stream = client
        .stream_user_live_events(request)
        .await
        .unwrap()
        .into_inner();

    match timeout(Duration::from_secs(20), stream.message()).await {
        Ok(Ok(Some(EventStreamItem {
            event: Some(bridge_event),
            ..
        }))) => bridge_event.event,
        _ => None,
    }
}

#[tokio::test]
#[ignore = "run via docker with the required program id"]
async fn test_create_admin_profile() -> anyhow::Result<()> {
    // === 1. Arrange ===
    println!("--- ARRANGE ---");
    let mut harness = TestHarness::new().await;

    let admin_authority = harness.create_funded_keypair(1.0).await?;

    let (expected_admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", admin_authority.pubkey().as_ref()],
        &harness.program_id,
    );

    let listener_handle = tokio::spawn(listen_for_admin_event(
        harness.grpc_client.clone(),
        expected_admin_pda,
    ));

    println!("\n--- ACT ---");

    let pda = harness.create_admin_profile(&admin_authority).await?;
    assert_eq!(pda, expected_admin_pda);

    println!("   -> Admin profile creation transaction submitted!");

    println!("\n--- ASSERT ---");

    let received_event = listener_handle
        .await?
        .expect("Test timed out waiting for admin event");

    match received_event {
        Event::AdminProfileRegistered(e) => {
            assert_eq!(e.admin_pda, expected_admin_pda.to_string());
            assert_eq!(e.authority, admin_authority.pubkey().to_string());
            println!("✅ Received correct AdminProfileRegistered event.");
        }
        _ => panic!("Received incorrect event type for admin creation"),
    }

    Ok(())
}

#[tokio::test]
#[ignore = "run via docker with the required program id"]
async fn test_create_user_profile() -> anyhow::Result<()> {
    // === 1. Arrange ===
    println!("--- ARRANGE ---");
    let mut harness = TestHarness::new().await;

    // First, create an Admin profile that the user can link to.
    let admin_authority = harness.create_funded_keypair(1.0).await?;
    let admin_pda = harness.create_admin_profile(&admin_authority).await?;
    println!("✅ Admin profile created: {}", admin_pda);

    // Now, create a new funded keypair for the user.
    let user_authority = harness.create_funded_keypair(1.0).await?;

    // Pre-calculate the User PDA we expect to be created.
    let (expected_user_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            user_authority.pubkey().as_ref(),
            admin_pda.as_ref(),
        ],
        &harness.program_id,
    );

    // Spawn a listener for the user event.
    let listener_handle = tokio::spawn(listen_for_user_event(
        harness.grpc_client.clone(),
        expected_user_pda,
    ));

    // === 2. Act ===
    println!("\n--- ACT ---");
    let user_pda = harness
        .create_user_profile(&user_authority, admin_pda)
        .await?;
    assert_eq!(user_pda, expected_user_pda);
    println!("   -> User profile creation transaction submitted!");

    // === 3. Assert ===
    println!("\n--- ASSERT ---");
    let received_event = listener_handle
        .await?
        .expect("Test timed out waiting for user event");

    match received_event {
        Event::UserProfileCreated(e) => {
            assert_eq!(e.user_pda, expected_user_pda.to_string());
            assert_eq!(e.authority, user_authority.pubkey().to_string());
            assert_eq!(e.target_admin_pda, admin_pda.to_string());
            println!("✅ Received correct UserProfileCreated event.");
        }
        _ => panic!("Received incorrect event type for user creation"),
    }

    Ok(())
}
