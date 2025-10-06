// use anchor_lang::AccountDeserialize;
// use solana_client::nonblocking::rpc_client::RpcClient;
// use solana_sdk::{
//     commitment_config::CommitmentConfig,
//     native_token::LAMPORTS_PER_SOL,
//     pubkey::Pubkey,
//     signature::{Keypair, Signer},
//     transaction::Transaction,
// };
// use tempfile::TempDir;
// use tokio::time::{sleep, Duration};
// use tokio_stream::StreamExt;
// use w3b2_connector::config::ConnectorConfig;
// use w3b2_gateway::{
//     config::{GatewayConfig, GatewaySpecificConfig, GrpcConfig, LogConfig, StreamingConfig},
//     grpc::{
//         proto::w3b2::protocol::gateway::{
//             admin_event_stream, bridge_gateway_service_client::BridgeGatewayServiceClient,
//             ListenAsAdminRequest, PrepareAdminRegisterProfileRequest,
//             PrepareUserCreateProfileRequest, PrepareUserDepositRequest,
//             PrepareUserDispatchCommandRequest, StopListenerRequest, SubmitTransactionRequest,
//         },
//         start,
//     },
// };
// use w3b2_solana_program::state::{AdminProfile, UserProfile};

// const RPC_URL: &str = "http://127.0.0.1:8899";
// const DEFAULT_AIRDROP_AMOUNT: u64 = 10 * LAMPORTS_PER_SOL;

// /// Holds the test environment components, including the TempDir for automatic cleanup.
// struct TestEnvironment {
//     client: BridgeGatewayServiceClient<tonic::transport::Channel>,
//     _temp_dir: TempDir, // Is kept for its Drop implementation, which cleans up the directory.
// }

// /// A helper function to set up a complete test environment.
// ///
// /// This function will:
// /// 1. Find a free TCP port.
// /// 2. Create a default `GatewayConfig` pointing to that port.
// /// 3. Start the gRPC server and all associated `w3b2-solana-connector` background services.
// /// 4. Create and return a gRPC client connected to the server.
// async fn setup_test_environment() -> TestEnvironment {
//     // Find a free port to avoid conflicts during parallel test runs.
//     let port = portpicker::pick_unused_port().expect("No free ports");
//     let addr = format!("127.0.0.1:{}", port);
//     let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

//     // Create a test-specific configuration.
//     let config = GatewayConfig {
//         connector: ConnectorConfig::default(),
//         gateway: GatewaySpecificConfig {
//             db_path: temp_dir.path().to_str().unwrap().to_string(),
//             grpc: GrpcConfig {
//                 host: "127.0.0.1".to_string(),
//                 port,
//             },
//             streaming: StreamingConfig::default(),
//             log: LogConfig::default(),
//         },
//     };

//     // Start the gRPC server and event manager.
//     let _handle = start(&config).await.expect("Failed to start gRPC server");

//     // Allow some time for the server to start up.
//     sleep(Duration::from_millis(200)).await;

//     // Connect a client to the server.
//     let client = BridgeGatewayServiceClient::connect(format!("http://{}", addr))
//         .await
//         .expect("Failed to connect to gRPC server");

//     TestEnvironment {
//         client,
//         _temp_dir: temp_dir,
//     }
// }

// /// Helper to airdrop lamports and wait for confirmation.
// async fn airdrop_and_confirm(rpc_client: &RpcClient, pubkey: &Pubkey, amount: u64) {
//     let sig = rpc_client.request_airdrop(pubkey, amount).await.unwrap();
//     loop {
//         match rpc_client
//             .get_signature_statuses(&[sig])
//             .await
//             .unwrap()
//             .value[0]
//             .as_ref()
//         {
//             Some(status) => {
//                 if status.err.is_some() {
//                     panic!("Airdrop failed with error: {:?}", status.err);
//                 }
//                 if status.confirmation_status.as_ref().map_or(false, |s| {
//                     *s == solana_transaction_status::TransactionConfirmationStatus::Finalized
//                 }) {
//                     break;
//                 }
//             }
//             None => sleep(Duration::from_millis(500)).await,
//         }
//     }
// }

// /// Creates a new keypair and funds it with a default amount of SOL.
// async fn create_funded_keypair(rpc_client: &RpcClient) -> Keypair {
//     let keypair = Keypair::new();
//     airdrop_and_confirm(rpc_client, &keypair.pubkey(), DEFAULT_AIRDROP_AMOUNT).await;
//     keypair
// }

// /// Helper to decode, sign, and submit a prepared transaction via gRPC.
// async fn execute_prepared_tx(
//     client: &mut BridgeGatewayServiceClient<tonic::transport::Channel>,
//     unsigned_tx_bytes: Vec<u8>,
//     signers: &[&Keypair],
// ) -> String {
//     let (mut tx, _): (Transaction, _) =
//         bincode::serde::borrow_decode_from_slice(&unsigned_tx_bytes, bincode::config::standard())
//             .unwrap();

//     let blockhash = tx.message.recent_blockhash;
//     tx.sign(signers, blockhash);

//     let signed_tx_bytes = bincode::serde::encode_to_vec(&tx, bincode::config::standard()).unwrap();
//     let sub_req = SubmitTransactionRequest {
//         signed_tx: signed_tx_bytes,
//     };

//     let response = client.submit_transaction(sub_req).await.unwrap();
//     response.into_inner().signature
// }

// /// Tests the full "prepare -> sign -> submit" lifecycle for a series of common operations.
// #[tokio::test]
// #[ignore] // This test requires a running local validator and can be slow.
// async fn test_prepare_and_submit_lifecycle() {
//     // === 1. Arrange ===
//     let env = setup_test_environment().await;
//     let mut client = env.client;
//     let rpc_client =
//         RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed());

//     let admin_authority = create_funded_keypair(&rpc_client).await;
//     let user_authority = create_funded_keypair(&rpc_client).await;

//     // === 2. Act & Assert: Admin Registration ===
//     let prep_req = PrepareAdminRegisterProfileRequest {
//         authority_pubkey: admin_authority.pubkey().to_string(),
//         communication_pubkey: Pubkey::new_unique().to_string(),
//     };
//     let unsigned_tx_resp = client
//         .prepare_admin_register_profile(prep_req)
//         .await
//         .unwrap()
//         .into_inner();

//     let signature = execute_prepared_tx(
//         &mut client,
//         unsigned_tx_resp.unsigned_tx,
//         &[&admin_authority],
//     )
//     .await;
//     println!("Submitted admin registration tx: {}", signature);

//     // Wait a moment for the node to process and for the event manager to catch up.
//     sleep(Duration::from_secs(2)).await;

//     let (admin_pda, _) = Pubkey::find_program_address(
//         &[b"admin", admin_authority.pubkey().as_ref()],
//         &w3b2_solana_program::ID,
//     );
//     let admin_account = rpc_client.get_account(&admin_pda).await.unwrap();
//     let admin_profile = AdminProfile::try_deserialize(&mut admin_account.data.as_slice()).unwrap();
//     assert_eq!(admin_profile.authority, admin_authority.pubkey());
//     println!("✅ Admin profile created successfully.");

//     // === 3. Act & Assert: User Profile Creation and Funding ===
//     let unsigned_tx_resp = client
//         .prepare_user_create_profile(PrepareUserCreateProfileRequest {
//             authority_pubkey: user_authority.pubkey().to_string(),
//             target_admin_authority_pubkey: admin_authority.pubkey().to_string(),
//             communication_pubkey: Pubkey::new_unique().to_string(),
//         })
//         .await
//         .unwrap()
//         .into_inner();

//     let signature = execute_prepared_tx(
//         &mut client,
//         unsigned_tx_resp.unsigned_tx,
//         &[&user_authority],
//     )
//     .await;
//     println!("Submitted user profile creation tx: {}", signature);

//     sleep(Duration::from_secs(2)).await;

//     let (user_pda, _) = Pubkey::find_program_address(
//         &[
//             b"user",
//             user_authority.pubkey().as_ref(),
//             admin_pda.as_ref(),
//         ],
//         &w3b2_solana_program::ID,
//     );
//     assert!(rpc_client.get_account(&user_pda).await.is_ok());
//     println!("✅ User profile created successfully.");

//     // Deposit funds
//     let deposit_amount = 1 * LAMPORTS_PER_SOL;
//     let unsigned_tx_resp = client
//         .prepare_user_deposit(PrepareUserDepositRequest {
//             authority_pubkey: user_authority.pubkey().to_string(),
//             admin_profile_pda: admin_pda.to_string(),
//             amount: deposit_amount,
//         })
//         .await
//         .unwrap()
//         .into_inner();

//     let signature = execute_prepared_tx(
//         &mut client,
//         unsigned_tx_resp.unsigned_tx,
//         &[&user_authority],
//     )
//     .await;
//     println!("Submitted user deposit tx: {}", signature);

//     sleep(Duration::from_secs(2)).await;

//     let user_account = rpc_client.get_account(&user_pda).await.unwrap();
//     let user_profile = UserProfile::try_deserialize(&mut user_account.data.as_slice()).unwrap();
//     assert_eq!(user_profile.deposit_balance, deposit_amount);
//     println!("✅ User deposit successful.");
// }

// /// Tests the `ListenAsAdmin` streaming RPC.
// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// #[ignore] // This test requires a running local validator and can be slow.
// async fn test_listen_as_admin_stream() {
//     // === 1. Arrange ===
//     let env = setup_test_environment().await;
//     let mut client = env.client;
//     let rpc_client =
//         RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed());

//     // Create an admin and a user on-chain.
//     let admin_authority = create_funded_keypair(&rpc_client).await;
//     let user_authority = create_funded_keypair(&rpc_client).await;

//     // Use the gRPC client to create the admin profile
//     let prep_req = PrepareAdminRegisterProfileRequest {
//         authority_pubkey: admin_authority.pubkey().to_string(),
//         communication_pubkey: Pubkey::new_unique().to_string(),
//     };
//     let unsigned_tx_resp = client
//         .prepare_admin_register_profile(prep_req)
//         .await
//         .unwrap()
//         .into_inner();
//     execute_prepared_tx(
//         &mut client,
//         unsigned_tx_resp.unsigned_tx,
//         &[&admin_authority],
//     )
//     .await;
//     println!("Admin profile created for streaming test.");

//     let (admin_pda, _) = Pubkey::find_program_address(
//         &[b"admin", admin_authority.pubkey().as_ref()],
//         &w3b2_solana_program::ID,
//     );

//     // === 2. Act: Start listening ===
//     let req = ListenAsAdminRequest {
//         admin_pubkey: admin_authority.pubkey().to_string(),
//     };
//     let mut stream = client.listen_as_admin(req).await.unwrap().into_inner();
//     println!("Listening for admin events...");

//     // === 3. Act: Trigger events ===

//     // Give the listener a moment to establish connection
//     sleep(Duration::from_secs(3)).await;

//     // Trigger a `NewUserProfile` event.
//     let prep_user_req = PrepareUserCreateProfileRequest {
//         authority_pubkey: user_authority.pubkey().to_string(),
//         target_admin_authority_pubkey: admin_authority.pubkey().to_string(),
//         communication_pubkey: Pubkey::new_unique().to_string(),
//     };
//     let unsigned_tx_resp = client
//         .prepare_user_create_profile(prep_user_req)
//         .await
//         .unwrap()
//         .into_inner();
//     execute_prepared_tx(
//         &mut client,
//         unsigned_tx_resp.unsigned_tx,
//         &[&user_authority],
//     )
//     .await;
//     println!("Triggered NewUserProfile event.");

//     // Trigger an `IncomingUserCommand` event.
//     let command_payload = vec![1, 2, 3, 4, 5];
//     // Note: This request now correctly expects the admin's *authority* pubkey, not the PDA.
//     let prep_dispatch_req = PrepareUserDispatchCommandRequest {
//         authority_pubkey: user_authority.pubkey().to_string(),
//         target_admin_authority_pubkey: admin_authority.pubkey().to_string(),
//         command_id: 123,
//         payload: command_payload.clone(),
//     };
//     let unsigned_tx_resp = client
//         .prepare_user_dispatch_command(prep_dispatch_req)
//         .await
//         .unwrap()
//         .into_inner();
//     execute_prepared_tx(
//         &mut client,
//         unsigned_tx_resp.unsigned_tx,
//         &[&user_authority],
//     )
//     .await;
//     println!("Triggered IncomingUserCommand event.");

//     // IMPORTANT: Wait for a moment to allow the event synchronizer (especially the
//     // CatchupWorker) to process the transactions and dispatch the events.
//     sleep(Duration::from_secs(4)).await;

//     // === 4. Assert ===

//     // Wait for and validate the NewUserProfile event.
//     let event_1 = tokio::time::timeout(Duration::from_secs(10), stream.next())
//         .await
//         .expect("Timed out waiting for NewUserProfile event")
//         .unwrap()
//         .unwrap();
//     if let Some(category) = event_1.event_category {
//         match category {
//             admin_event_stream::EventCategory::NewUserProfile(e) => {
//                 assert_eq!(e.authority, user_authority.pubkey().to_string());
//                 assert_eq!(e.target_admin, admin_pda.to_string());
//                 println!("✅ Received correct NewUserProfile event.");
//             }
//             _ => panic!("Expected NewUserProfile event, got {:?}", category),
//         }
//     }

//     // Wait for and validate the IncomingUserCommand event.
//     let event_2 = tokio::time::timeout(Duration::from_secs(10), stream.next())
//         .await
//         .expect("Timed out waiting for IncomingUserCommand event")
//         .unwrap()
//         .unwrap();
//     if let Some(category) = event_2.event_category {
//         match category {
//             admin_event_stream::EventCategory::IncomingUserCommand(e) => {
//                 assert_eq!(e.sender, user_authority.pubkey().to_string());
//                 assert_eq!(e.command_id, 123);
//                 assert_eq!(e.payload, command_payload);
//                 println!("✅ Received correct IncomingUserCommand event.");
//             }
//             _ => panic!("Expected IncomingUserCommand event, got {:?}", category),
//         }
//     }
// }

// /// Tests that the `StopListener` RPC correctly terminates an active stream.
// #[tokio::test]
// #[ignore] // This test can be run standalone.
// async fn test_stop_listener() {
//     // === 1. Arrange ===
//     let env = setup_test_environment().await;
//     let mut client = env.client;
//     let admin_pubkey = Pubkey::new_unique();

//     // === 2. Act: Start listening ===
//     let req = ListenAsAdminRequest {
//         admin_pubkey: admin_pubkey.to_string(),
//     };
//     let mut stream = client.listen_as_admin(req).await.unwrap().into_inner();
//     println!("Stream started for {}", admin_pubkey);

//     // === 3. Act: Stop the listener ===
//     let stop_req = StopListenerRequest {
//         pubkey_to_stop: admin_pubkey.to_string(),
//     };
//     client.stop_listener(stop_req).await.unwrap();
//     println!("StopListener request sent.");

//     // === 4. Assert ===
//     // The stream should now be closed. The next call should return None.
//     let result = tokio::time::timeout(Duration::from_secs(2), stream.next()).await;

//     assert!(
//         result.is_ok(),
//         "Stream did not close within the timeout period"
//     );
//     assert!(
//         result.unwrap().is_none(),
//         "Stream should be closed and return None"
//     );

//     println!("✅ Stream closed successfully after StopListener call.");
// }
