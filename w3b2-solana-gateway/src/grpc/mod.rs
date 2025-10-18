//! # gRPC Service Implementation
//!
//! This module defines the gRPC server and its implementation of the `BridgeGatewayService`.
//!
//! ## Core Components
//!
//! - **[`GatewayServer`]**: The main struct that implements the `BridgeGatewayService` tonic trait.
//!   It holds the application's shared state, [`AppState`].
//!
//! - **[`AppState`]**: A container for shared, thread-safe components needed by the gRPC
//!   service methods, such as the Solana `RpcClient`, a handle to the `EventManager`,
//!   and the gateway's configuration.
//!
//! - **[`start`]**: The main entry point for initializing and running the gateway. It sets up
//!   the database, spawns the `EventManager` for background event processing, and starts
//!   the tonic gRPC server.
//!
//! ## RPC Method Groups
//!
//! The `BridgeGatewayService` implementation is organized into three main categories:
//!
//! 1.  **Transaction Preparation (`prepare_*`)**: A suite of methods that map one-to-one
//!     with the instructions in the on-chain program. Each method takes high-level inputs,
//!     uses the `w3b2-solana-connector`'s `TransactionBuilder` to construct an unsigned
//!     transaction, and returns its serialized `Message` to the client. This enables
//!     a non-custodial workflow where the gateway never handles private keys.
//!
//! 2.  **Transaction Submission (`submit_transaction`)**: A single method that accepts a
//!     signed transaction from a client, deserializes it, and submits it to the Solana network.
//!
//! 3.  **Event Streaming (`listen_as_user`, `listen_as_admin`, `unsubscribe`)**: Methods that
//!     allow clients to open a persistent, server-side stream of on-chain events for a
//!     specific `UserProfile` or `AdminProfile` PDA. The server leverages the `EventManager`
//!     to provide both historical (catch-up) and real-time events.

mod conversions;

use anyhow::Result;
use dashmap::DashMap;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use w3b2_solana_connector::listener::EventListener;
use w3b2_solana_connector::workers::{EventManager, EventManagerHandle};

use w3b2_solana_connector::client::{TransactionBuilder, UserDispatchCommandArgs};

use crate::grpc::proto::w3b2::protocol::gateway::bridge_gateway_service_server::{
    BridgeGatewayService, BridgeGatewayServiceServer,
};
use crate::{
    config::GatewayConfig,
    error::GatewayError,
    grpc::proto::w3b2::protocol::gateway::{
        self, BlockhashResponse, EventStreamItem, ListenRequest, PrepareAdminBanUserRequest,
        PrepareAdminCloseProfileRequest, PrepareAdminDispatchCommandRequest,
        PrepareAdminRegisterProfileRequest, PrepareAdminSetConfigRequest,
        PrepareAdminUnbanUserRequest, PrepareAdminWithdrawRequest, PrepareLogActionRequest,
        PrepareUserCloseProfileRequest, PrepareUserCreateProfileRequest, PrepareUserDepositRequest,
        PrepareUserDispatchCommandRequest, PrepareUserRequestUnbanRequest,
        PrepareUserUpdateCommKeyRequest, PrepareUserWithdrawRequest, SubmitTransactionRequest,
        TransactionResponse, UnsignedTransactionResponse, UnsubscribeRequest,
    },
    storage::SledStorage,
};

/// Generated protobuf code.
pub mod proto {
    pub mod w3b2 {
        pub mod protocol {
            pub mod gateway {
                tonic::include_proto!("w3b2.protocol.gateway");
            }
        }
    }
}

/// A container for the application's shared, thread-safe state.
///
/// An `Arc` of this struct is cloned for each gRPC service instance,
/// allowing all RPC handlers to access the same underlying components.
#[derive(Clone)]
pub struct AppState {
    /// A shared Solana RPC client.
    pub rpc_client: Arc<RpcClient>,
    /// A handle to the central `EventManager` for creating event listeners.
    pub event_manager: EventManagerHandle,
    /// The gateway's configuration.
    pub config: Arc<GatewayConfig>,
    /// A map storing `watch` channel senders to signal termination for active event subscriptions.
    /// The key is the subscribed PDA's `Pubkey`.
    pub active_subscriptions: Arc<DashMap<Pubkey, watch::Sender<()>>>,
}

/// The gRPC server implementation for the `BridgeGatewayService`.
pub struct GatewayServer {
    /// The shared application state.
    state: AppState,
}

impl GatewayServer {
    /// Creates a new `GatewayServer` instance.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// The main entry point to initialize and start the gRPC server and all background services.
pub async fn start(config: &GatewayConfig) -> Result<EventManagerHandle> {
    // --- 1. Initialize dependencies ---
    let db = sled::open(&config.gateway.db_path)?;
    let storage = Arc::new(SledStorage::new(db));
    let addr = format!("{}:{}", config.gateway.grpc.host, config.gateway.grpc.port).parse()?;
    let rpc_client = Arc::new(RpcClient::new(config.connector.solana.rpc_url.clone()));

    // --- 2. Create and spawn the EventManager service ---
    let (event_manager_runner, event_manager_handle) = EventManager::new(
        Arc::new(config.connector.clone()),
        rpc_client.clone(),
        storage,
    );
    tokio::spawn(event_manager_runner.run());

    // --- 3. Set up the gRPC server state ---
    let handle_for_server = event_manager_handle.clone();
    let app_state = AppState {
        rpc_client,
        event_manager: handle_for_server,
        config: Arc::new(config.clone()),
        active_subscriptions: Arc::new(DashMap::new()),
    };

    let gateway_server = GatewayServer::new(app_state);
    let grpc_server =
        Server::builder().add_service(BridgeGatewayServiceServer::new(gateway_server));

    tracing::info!(
        "Non-Custodial gRPC Gateway with Event Streaming listening on {}",
        addr
    );

    tokio::spawn(async move {
        if let Err(e) = grpc_server.serve(addr).await {
            tracing::error!("gRPC server failed: {}", e);
        }
    });

    // Return the handle so the caller can gracefully shut down the event manager.
    Ok(event_manager_handle)
}

/// A helper function to parse a string into a `Pubkey`, returning a `GatewayError` on failure.
fn parse_pubkey(s: &str) -> Result<Pubkey, GatewayError> {
    Pubkey::from_str(s).map_err(GatewayError::from)
}

/// A helper to handle the logic for streaming **live** events.
///
/// This function registers a persistent listener and spawns a background task that
/// forwards live events to the gRPC stream. It manages the subscription lifecycle,
/// cleaning up when the client disconnects or unsubscribes.
async fn handle_live_stream(
    state: &AppState,
    pda: Pubkey,
    mut listener: EventListener,
) -> Result<Response<ReceiverStream<Result<EventStreamItem, Status>>>, Status> {
    let (tx, rx) = mpsc::channel(state.config.connector.channels.listener_event_buffer);

    // Create a watch channel to signal termination for this specific stream.
    let (stop_tx, mut stop_rx) = watch::channel(());
    if state.active_subscriptions.insert(pda, stop_tx).is_some() {
        return Err(Status::already_exists(format!(
            "A listener for PDA {pda} is already active"
        )));
    }

    let active_subscriptions_clone = state.active_subscriptions.clone();

    tokio::spawn(async move {
        // Listen for live events and the stop signal.
        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    tracing::info!("Unsubscribe signal received for PDA {}. Closing stream.", pda);
                    break;
                }
                Some(event) = listener.next_live_event() => {
                    if tx.send(Ok(gateway::EventStreamItem::from(event))).await.is_err() {
                        tracing::warn!("Client for PDA {} disconnected during live stream.", pda);
                        break;
                    }
                }
                else => {
                    tracing::info!("Event manager shut down for PDA {}. Closing stream.", pda);
                    break;
                }
            }
        }

        active_subscriptions_clone.remove(&pda);
        tracing::info!("Live event stream for PDA {} has ended.", pda);
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}

/// A helper to handle the logic for streaming **historical** events.
///
/// This function creates a temporary listener, drains all events from its
/// catch-up channel, and sends them to the client. The stream closes automatically
/// once all historical events have been sent.
async fn handle_history_stream(
    state: &AppState,
    pda: Pubkey,
    mut listener: EventListener,
) -> Result<Response<ReceiverStream<Result<EventStreamItem, Status>>>, Status> {
    let (tx, rx) = mpsc::channel(state.config.connector.channels.listener_event_buffer);

    tokio::spawn(async move {
        // Drain all catchup events and send them to the client.
        while let Some(event) = listener.next_catchup_event().await {
            if tx
                .send(Ok(gateway::EventStreamItem::from(event)))
                .await
                .is_err()
            {
                tracing::warn!("Client for PDA {} disconnected during history stream.", pda);
                // No need to remove from active_subscriptions as this is a temporary listener.
                break;
            }
        }
        // Once the loop finishes, `tx` is dropped, and the client's stream will close gracefully.
        tracing::info!("Event history stream for PDA {} has completed.", pda);
        // The listener will be dropped here, automatically unsubscribing.
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}

#[tonic::async_trait]
impl BridgeGatewayService for GatewayServer {
    type StreamUserLiveEventsStream = ReceiverStream<Result<EventStreamItem, Status>>;

    /// Subscribes to a stream of **live** events for a specific UserProfile PDA.
    /// This stream does NOT include historical events. For history, use
    /// `GetUserEventHistory`.
    async fn stream_user_live_events(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::StreamUserLiveEventsStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received StreamUserLiveEvents request for PDA: {}", req.pda);

        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;

        let listener = self.state.event_manager.listen_as_user(pda);
        handle_live_stream(&self.state, pda, listener).await
    }

    type StreamAdminLiveEventsStream = ReceiverStream<Result<EventStreamItem, Status>>;

    /// Subscribes to a stream of **live** events for a specific AdminProfile PDA.
    /// This stream does NOT include historical events. For history, use
    /// `GetAdminEventHistory`.
    async fn stream_admin_live_events(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::StreamAdminLiveEventsStream>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "Received StreamAdminLiveEvents request for PDA: {}",
            req.pda
        );

        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;

        let listener = self.state.event_manager.listen_as_admin(pda);
        handle_live_stream(&self.state, pda, listener).await
    }

    type GetUserEventHistoryStream = ReceiverStream<Result<EventStreamItem, Status>>;

    async fn get_user_event_history(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::GetUserEventHistoryStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received GetUserEventHistory request for PDA: {}", req.pda);
        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;
        let listener = self.state.event_manager.listen_as_user(pda);
        handle_history_stream(&self.state, pda, listener).await
    }

    type GetAdminEventHistoryStream = ReceiverStream<Result<EventStreamItem, Status>>;

    async fn get_admin_event_history(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::GetAdminEventHistoryStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received GetAdminEventHistory request for PDA: {}", req.pda);
        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;
        let listener = self.state.event_manager.listen_as_admin(pda);
        handle_history_stream(&self.state, pda, listener).await
    }

    /// Manually closes an active event stream subscription.
    async fn unsubscribe(
        &self,
        request: Request<UnsubscribeRequest>,
    ) -> Result<Response<()>, Status> {
        let result: Result<Response<()>, GatewayError> = (async {
            let req = request.into_inner();
            let pda_to_stop = parse_pubkey(&req.pda)?;
            tracing::info!("Received Unsubscribe request for PDA: {}", pda_to_stop);

            // Find the subscription and send a stop signal by dropping the sender.
            if let Some((_, stop_tx)) = self.state.active_subscriptions.remove(&pda_to_stop) {
                let _ = stop_tx.send(());
                tracing::info!("Successfully signaled termination for PDA: {}", pda_to_stop);
            } else {
                tracing::warn!(
                    "Attempted to unsubscribe from a non-existent or already closed subscription for PDA: {}",
                    pda_to_stop
                );
            }

            Ok(Response::new(()))
        })
        .await;

        result.map_err(Status::from)
    }

    // --- Transaction Preparation ---

    /// Prepares an unsigned `user_request_unban` transaction.
    async fn prepare_user_request_unban(
        &self,
        request: Request<PrepareUserRequestUnbanRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserRequestUnban request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_user_request_unban(authority, admin_profile_pda);

            tracing::debug!("Prepared user_request_unban tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_ban_user` transaction.
    async fn prepare_admin_ban_user(
        &self,
        request: Request<PrepareAdminBanUserRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminBanUser request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_user_profile_pda = parse_pubkey(&req.target_user_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_admin_ban_user(authority, target_user_profile_pda);

            tracing::debug!("Prepared admin_ban_user tx for authority {}", authority);

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_unban_user` transaction.
    async fn prepare_admin_unban_user(
        &self,
        request: Request<PrepareAdminUnbanUserRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminUnbanUser request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_user_profile_pda = parse_pubkey(&req.target_user_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_admin_unban_user(authority, target_user_profile_pda);

            tracing::debug!("Prepared admin_unban_user tx for authority {}", authority);

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_register_profile` transaction.
    async fn prepare_admin_register_profile(
        &self,
        request: Request<PrepareAdminRegisterProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminRegisterProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let communication_pubkey = parse_pubkey(&req.communication_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_admin_register_profile(authority, communication_pubkey);

            tracing::debug!(
                "Prepared admin_register_profile tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_set_config` transaction.
    async fn prepare_admin_set_config(
        &self,
        request: Request<PrepareAdminSetConfigRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminSetConfig request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let new_oracle_authority = req
                .new_oracle_authority
                .map(|s| parse_pubkey(&s))
                .transpose()?;
            let new_communication_pubkey = req
                .new_communication_pubkey
                .map(|s| parse_pubkey(&s))
                .transpose()?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_admin_set_config(
                authority,
                new_oracle_authority,
                req.new_timestamp_validity,
                new_communication_pubkey,
                req.new_unban_fee,
            );
            tracing::debug!("Prepared admin_set_config tx for authority {}", authority);

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_withdraw` transaction.
    async fn prepare_admin_withdraw(
        &self,
        request: Request<PrepareAdminWithdrawRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminWithdraw request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let destination = parse_pubkey(&req.destination)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_admin_withdraw(authority, req.amount, destination);

            tracing::debug!("Prepared admin_withdraw tx for authority {}", authority);

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_close_profile` transaction.
    async fn prepare_admin_close_profile(
        &self,
        request: Request<PrepareAdminCloseProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminCloseProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_admin_close_profile(authority);

            tracing::debug!(
                "Prepared admin_close_profile tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `admin_dispatch_command` transaction.
    async fn prepare_admin_dispatch_command(
        &self,
        request: Request<PrepareAdminDispatchCommandRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminDispatchCommand request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_user_profile_pda = parse_pubkey(&req.target_user_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_admin_dispatch_command(
                authority,
                target_user_profile_pda,
                req.command_id,
                req.payload,
            );
            tracing::debug!(
                "Prepared admin_dispatch_command tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `user_create_profile` transaction.
    async fn prepare_user_create_profile(
        &self,
        request: Request<PrepareUserCreateProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserCreateProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_admin_pda = parse_pubkey(&req.target_admin_pda)?;
            let communication_pubkey = parse_pubkey(&req.communication_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_user_create_profile(
                authority,
                target_admin_pda,
                communication_pubkey,
            );

            tracing::debug!(
                "Prepared user_create_profile tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `user_update_comm_key` transaction.
    async fn prepare_user_update_comm_key(
        &self,
        request: Request<PrepareUserUpdateCommKeyRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserUpdateCommKey request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;
            let new_key = parse_pubkey(&req.new_key)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_user_update_comm_key(authority, admin_profile_pda, new_key);

            tracing::debug!(
                "Prepared user_update_comm_key tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `user_deposit` transaction.
    async fn prepare_user_deposit(
        &self,
        request: Request<PrepareUserDepositRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserDeposit request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_user_deposit(authority, admin_profile_pda, req.amount);

            tracing::debug!("Prepared user_deposit tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `user_withdraw` transaction.
    async fn prepare_user_withdraw(
        &self,
        request: Request<PrepareUserWithdrawRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserWithdraw request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;
            let destination = parse_pubkey(&req.destination)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_user_withdraw(
                authority,
                admin_profile_pda,
                req.amount,
                destination,
            );

            tracing::debug!("Prepared user_withdraw tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `user_close_profile` transaction.
    async fn prepare_user_close_profile(
        &self,
        request: Request<PrepareUserCloseProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserCloseProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message =
                builder.prepare_user_close_profile(authority, admin_profile_pda);

            tracing::debug!("Prepared user_close_profile tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `user_dispatch_command` transaction.
    async fn prepare_user_dispatch_command(
        &self,
        request: Request<PrepareUserDispatchCommandRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserDispatchCommand request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_admin_pda = parse_pubkey(&req.target_admin_pda)?;
            let oracle_pubkey = parse_pubkey(&req.oracle_pubkey)?;

            // Convert the signature from Vec<u8> to [u8; 64]
            let oracle_signature: [u8; 64] = req.oracle_signature.try_into().map_err(|_| {
                GatewayError::InvalidArgument("Oracle signature must be 64 bytes".to_string())
            })?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_user_dispatch_command(
                authority,
                target_admin_pda,
                UserDispatchCommandArgs {
                    command_id: req.command_id as u16,
                    price: req.price,
                    timestamp: req.timestamp,
                    payload: req.payload,
                    oracle_pubkey,
                    oracle_signature,
                },
            );
            tracing::debug!(
                "Prepared user_dispatch_command tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Prepares an unsigned `log_action` transaction.
    async fn prepare_log_action(
        &self,
        request: Request<PrepareLogActionRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!("Received PrepareLogAction request: {:?}", request.get_ref());

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let user_profile_pda = parse_pubkey(&req.user_profile_pda)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let unsigned_tx_message = builder.prepare_log_action(
                authority,
                user_profile_pda,
                admin_profile_pda,
                req.session_id,
                req.action_code as u16,
            );
            tracing::debug!("Prepared log_action tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse {
                unsigned_tx_message,
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    /// Submits a signed transaction to the network.
    async fn submit_transaction(
        &self,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<TransactionResponse>, Status> {
        let result: Result<Response<TransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received SubmitTransaction request with {} bytes",
                request.get_ref().signed_tx.len()
            );

            let req = request.into_inner();
            let tx_bytes = req.signed_tx;

            let (transaction, _len): (Transaction, usize) =
                bincode::serde::borrow_decode_from_slice(
                    tx_bytes.as_slice(),
                    bincode::config::standard(),
                )
                .map_err(GatewayError::from)?;

            tracing::debug!("Deserialized transaction: {:?}", transaction);

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let signature = builder
                .submit_transaction(&transaction)
                .await
                .map_err(|e| GatewayError::Connector(Box::new(e)))?;
            tracing::info!("Submitted transaction, signature: {}", signature);

            Ok(Response::new(TransactionResponse {
                signature: signature.to_string(),
            }))
        })
        .await;

        result.map_err(Status::from)
    }

    // --- Utility RPCs ---

    /// Fetches the latest blockhash from the Solana network.
    async fn get_latest_blockhash(
        &self,
        _request: Request<()>,
    ) -> Result<Response<BlockhashResponse>, Status> {
        let result: Result<Response<BlockhashResponse>, GatewayError> = (async {
            tracing::info!("Received GetLatestBlockhash request");

            let blockhash = self
                .state
                .rpc_client
                .get_latest_blockhash()
                .await
                .map_err(|e| GatewayError::Connector(Box::new(e)))?;

            Ok(Response::new(BlockhashResponse {
                blockhash: blockhash.to_bytes().to_vec(),
            }))
        })
        .await;
        result.map_err(Status::from)
    }
}
