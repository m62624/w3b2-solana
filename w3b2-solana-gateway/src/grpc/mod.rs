//! # gRPC Service Implementation
//!
//! This module defines the gRPC server and its implementation of the `BridgeGatewayService`.
//! Its sole responsibility is to provide robust, persistent event streams to off-chain clients.
//!
//! ## Core Components
//!
//! - **[`GatewayServer`]**: The main struct that implements the `BridgeGatewayService` tonic trait.
//!   It holds the application's shared state, [`AppState`].
//!
//! - **[`AppState`]**: A container for shared, thread-safe components needed by the gRPC
//!   service methods, primarily a handle to the `EventManager`.
//!
//! - **[`start`]**: The main entry point for initializing and running the gateway. It sets up
//!   the database, spawns the `EventManager` for background event processing, and starts
//!   the tonic gRPC server.
//!
//! ## Event Streaming Philosophy
//!
//! The gateway provides two distinct types of event streams for both `User` and `Admin` profiles:
//!
//! 1.  **Live Streams (`stream_*_live_events`)**: Opens a persistent, long-lived connection that
//!     forwards events in real-time as they are confirmed on-chain. This is ideal for applications
//!     that need immediate state updates. The stream remains open until the client disconnects or
//!     sends an `unsubscribe` request.
//!
//! 2.  **History Streams (`get_*_event_history`)**: Fetches all historical events for a given
//!     PDA from the beginning of its existence. This is a "one-shot" stream that closes
//!     automatically after the last historical event has been delivered. It is perfect for
//!     hydrating an application's state or running batch analysis.
//!
//! This separation allows clients to build a complete and consistent view of on-chain state by
//! first draining the history stream and then subscribing to the live stream.

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
    ///
    /// This stream remains open indefinitely, pushing events as they are confirmed on-chain.
    /// It does NOT include historical events. For history, use `get_user_event_history`.
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
    ///
    /// This stream remains open indefinitely, pushing events as they are confirmed on-chain.
    /// It does NOT include historical events. For history, use `get_admin_event_history`.
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

    /// Fetches all historical events for a specific UserProfile PDA.
    ///
    /// This is a "one-shot" stream that closes automatically after the last historical
    /// event has been delivered.
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

    /// Fetches all historical events for a specific AdminProfile PDA.
    ///
    /// This is a "one-shot" stream that closes automatically after the last historical
    /// event has been delivered.
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

    /// Manually closes an active **live** event stream subscription.
    ///
    /// This is not needed for history streams, as they close automatically.
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
}
