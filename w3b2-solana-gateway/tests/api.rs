use solana_sdk::{
    hash::Hash,
    message::Message,
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
    PrepareUserCreateProfileRequest, SubmitTransactionRequest, UnsubscribeRequest,
};

/// Constructs the gateway URL from environment variables, with fallbacks for Docker.
fn get_gateway_url() -> String {
    let host = env::var("GATEWAY_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("GATEWAY_PORT").unwrap_or_else(|_| "50051".to_string());
    format!("http://{}:{}", host, port)
}

async fn get_client() -> BridgeGatewayServiceClient<tonic::transport::Channel> {
    BridgeGatewayServiceClient::connect(get_gateway_url())
        .await
        .expect("Failed to connect to gateway")
}

/// A helper struct to simplify transaction preparation and submission in tests.
struct TestHarness {
    client: BridgeGatewayServiceClient<tonic::transport::Channel>,
    program_id: Pubkey,
}

impl TestHarness {
    async fn new() -> Self {
        let program_id_str = env::var("W3B2_CONNECTOR__PROGRAM_ID").expect(
            "W3B2_CONNECTOR__PROGRAM_ID environment variable not set. \
            Please set it before running integration tests.",
        );
        let program_id = Pubkey::from_str(&program_id_str).unwrap();
        Self {
            client: get_client().await,
            program_id,
        }
    }

    /// Prepares, signs, and submits a transaction from a raw message buffer.
    async fn sign_and_submit(
        &mut self,
        unsigned_tx_msg: Vec<u8>,
        signer: &Keypair,
    ) -> anyhow::Result<()> {
        let blockhash_bytes = self
            .client
            .get_latest_blockhash(())
            .await?
            .into_inner()
            .blockhash;
        let blockhash_array: [u8; 32] = blockhash_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Blockhash is not 32 bytes long"))?;
        let blockhash = Hash::new_from_array(blockhash_array);

        let mut message: Message = bincode::serde::borrow_decode_from_slice(
            &unsigned_tx_msg,
            bincode::config::standard(),
        )?
        .0;
        message.recent_blockhash = blockhash;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[signer], blockhash);

        let submit_req = Request::new(SubmitTransactionRequest {
            signed_tx: bincode::serde::encode_to_vec(&tx, bincode::config::standard())?,
        });
        self.client.submit_transaction(submit_req).await?;
        Ok(())
    }
}

/// Helper to listen for the next live event for an Admin PDA.
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
#[ignore = "run via docker with the required program id"] // These are integration tests, run them explicitly.
async fn test_connection_and_unsubscribe() {
    let mut client = get_client().await;
    let pda = Pubkey::new_unique();

    // 1. Test connection by opening a stream
    let listen_request = Request::new(ListenRequest {
        pda: pda.to_string(),
    });
    let stream_result = client.stream_user_live_events(listen_request).await;
    assert!(stream_result.is_ok(), "Should successfully open a stream");
    println!("✅ Stream opened successfully.");

    // 2. Test unsubscribe
    let unsubscribe_request = Request::new(UnsubscribeRequest {
        pda: pda.to_string(),
    });
    let unsubscribe_result = client.unsubscribe(unsubscribe_request).await;
    assert!(
        unsubscribe_result.is_ok(),
        "Should successfully unsubscribe"
    );
    println!("✅ Unsubscribe call successful.");
}

#[tokio::test]
#[ignore = "run via docker with the required program id"]
async fn test_profile_creation_events() -> anyhow::Result<()> {
    // === 1. Arrange ===
    let mut harness = TestHarness::new().await;

    // --- Admin Profile Creation ---
    let admin_authority = Keypair::new();
    let admin_comm_key = Keypair::new();
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", admin_authority.pubkey().as_ref()],
        &harness.program_id,
    );

    // Spawn a listener task *before* performing the action.
    let listener_handle = tokio::spawn(listen_for_admin_event(harness.client.clone(), admin_pda));

    // === 2. Act: Create Admin Profile ===
    let prepare_req = Request::new(PrepareAdminRegisterProfileRequest {
        authority_pubkey: admin_authority.pubkey().to_string(),
        communication_pubkey: admin_comm_key.pubkey().to_string(),
    });
    let unsigned_tx_msg = harness
        .client
        .prepare_admin_register_profile(prepare_req)
        .await?
        .into_inner()
        .unsigned_tx_message;

    harness
        .sign_and_submit(unsigned_tx_msg, &admin_authority)
        .await?;

    // === 3. Assert Admin Event ===
    let received_event = listener_handle
        .await?
        .expect("Test timed out waiting for admin event");

    match received_event {
        Event::AdminProfileRegistered(e) => {
            assert_eq!(e.admin_pda, admin_pda.to_string());
            assert_eq!(e.authority, admin_authority.pubkey().to_string());
        }
        _ => panic!("Received incorrect event type for admin creation"),
    }
    println!("✅ Received AdminProfileRegistered event successfully.");

    // === 4. Arrange: User Profile Creation ===
    let user_authority = Keypair::new();
    let user_comm_key = Keypair::new();
    let (user_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            user_authority.pubkey().as_ref(),
            admin_pda.as_ref(),
        ],
        &harness.program_id,
    );

    let user_listener_handle =
        tokio::spawn(listen_for_user_event(harness.client.clone(), user_pda));

    // === 5. Act: Create User Profile ===
    let prepare_req = Request::new(PrepareUserCreateProfileRequest {
        authority_pubkey: user_authority.pubkey().to_string(),
        target_admin_pda: admin_pda.to_string(),
        communication_pubkey: user_comm_key.pubkey().to_string(),
    });
    let unsigned_tx_msg = harness
        .client
        .prepare_user_create_profile(prepare_req)
        .await?
        .into_inner()
        .unsigned_tx_message;

    harness
        .sign_and_submit(unsigned_tx_msg, &user_authority)
        .await?;

    // === 6. Assert User Event ===
    let received_user_event = user_listener_handle
        .await?
        .expect("Test timed out waiting for user event");

    match received_user_event {
        Event::UserProfileCreated(e) => {
            assert_eq!(e.user_pda, user_pda.to_string());
            assert_eq!(e.authority, user_authority.pubkey().to_string());
            assert_eq!(e.target_admin_pda, admin_pda.to_string());
        }
        _ => panic!("Received incorrect event type for user creation"),
    }
    println!("✅ Received UserProfileCreated event successfully.");

    Ok(())
}
