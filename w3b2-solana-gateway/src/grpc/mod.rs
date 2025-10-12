//! # gRPC Service Implementation
//!
//! This module defines the gRPC server and its implementation of the `BridgeGatewayService`.
//!
//! Its sole responsibility is to provide a real-time stream of on-chain events.
//!
//! ### Architecture
//!
//! - **Transaction Submission**: Clients are expected to build, sign, and submit
//!   transactions directly to the Solana RPC node using a standard library for
//!   their language (e.g., `anchorpy` for Python, `@coral-xyz/anchor` for TypeScript).
//!   The gateway does **not** handle transaction preparation or submission.
//!
//! - **Event Streaming**: The gateway provides `listen_as_user` and `listen_as_admin`
//!   methods. These methods open a persistent server-side stream of on-chain events
//!   for a specific `UserProfile` or `AdminProfile` PDA, leveraging the underlying
//!   `w3b2-solana-connector` to provide both historical (catch-up) and real-time events.

mod conversions;

use anyhow::Result;
use dashmap::DashMap;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use w3b2_solana_connector::workers::{EventManager, EventManagerHandle};

use crate::grpc::proto::w3b2::protocol::gateway::bridge_gateway_service_server::{
    BridgeGatewayService, BridgeGatewayServiceServer,
};
use crate::{
    config::GatewayConfig,
    error::GatewayError,
    grpc::proto::w3b2::protocol::gateway::{
        self, EventStreamItem, ListenRequest, UnsubscribeRequest,
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

#[tonic::async_trait]
impl BridgeGatewayService for GatewayServer {
    type ListenAsUserStream = ReceiverStream<Result<EventStreamItem, Status>>;

    /// Opens a server-side stream of on-chain events for a specific `UserProfile` PDA.
    ///
    /// The stream first delivers all historical events for the PDA ("catch-up" phase),
    /// and then sends new events in real-time as they occur ("live" phase).
    async fn listen_as_user(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenAsUserStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received ListenAsUser request for PDA: {}", req.pda);

        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;

        let mut listener = self.state.event_manager.listen_as_user(pda);
        let (tx, rx) = mpsc::channel(self.state.config.connector.channels.listener_event_buffer);

        // Create a watch channel to signal termination for this specific stream.
        let (stop_tx, mut stop_rx) = watch::channel(());
        if self
            .state
            .active_subscriptions
            .insert(pda, stop_tx)
            .is_some()
        {
            return Err(Status::already_exists(format!(
                "A listener for PDA {pda} is already active"
            )));
        }

        let tx_clone = tx.clone();
        let active_subscriptions_clone = self.state.active_subscriptions.clone();

        tokio::spawn(async move {
            // Phase 1: Drain all catchup events.
            while let Some(event) = listener.next_catchup_event().await {
                let item = gateway::EventStreamItem::from(event);
                if tx_clone.send(Ok(item)).await.is_err() {
                    tracing::warn!("Client for PDA {} disconnected during catchup.", pda);
                    active_subscriptions_clone.remove(&pda);
                    return;
                }
            }
            tracing::info!("Catchup phase completed for user PDA {}.", pda);

            // Phase 2: Listen for live events and the stop signal.
            loop {
                tokio::select! {
                    // Stop if an unsubscribe signal is received.
                    _ = stop_rx.changed() => {
                        tracing::info!("Unsubscribe signal received for user PDA {}. Closing stream.", pda);
                        break;
                    }
                    // Forward live events.
                    Some(event) = listener.next_live_event() => {
                        let item = gateway::EventStreamItem::from(event);
                        if tx_clone.send(Ok(item)).await.is_err() {
                            tracing::warn!("Client for PDA {} disconnected during live stream.", pda);
                            break;
                        }
                    }
                    // Stop if the event manager shuts down.
                    else => break,
                }
            }

            active_subscriptions_clone.remove(&pda);
            tracing::info!("Event stream for user PDA {} has ended.", pda);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type ListenAsAdminStream = ReceiverStream<Result<EventStreamItem, Status>>;

    /// Opens a server-side stream of on-chain events for a specific `AdminProfile` PDA.
    ///
    /// The stream first delivers all historical events for the PDA ("catch-up" phase),
    /// and then sends new events in real-time as they occur ("live" phase).
    async fn listen_as_admin(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenAsAdminStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received ListenAsAdmin request for PDA: {}", req.pda);

        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;
        let mut listener = self.state.event_manager.listen_as_admin(pda);
        let (tx, rx) = mpsc::channel(self.state.config.connector.channels.listener_event_buffer);

        // Create a watch channel to signal termination.
        let (stop_tx, mut stop_rx) = watch::channel(());
        if self
            .state
            .active_subscriptions
            .insert(pda, stop_tx)
            .is_some()
        {
            return Err(Status::already_exists(format!(
                "A listener for PDA {pda} is already active"
            )));
        }

        let tx_clone = tx.clone();
        let active_subscriptions_clone = self.state.active_subscriptions.clone();

        tokio::spawn(async move {
            // Phase 1: Drain all catchup events.
            while let Some(event) = listener.next_catchup_event().await {
                let item = gateway::EventStreamItem::from(event);
                if tx_clone.send(Ok(item)).await.is_err() {
                    tracing::warn!("Client for admin PDA {} disconnected during catchup.", pda);
                    active_subscriptions_clone.remove(&pda);
                    return;
                }
            }
            tracing::info!("Catchup phase completed for admin PDA {}.", pda);

            // Phase 2: Listen for live events and the stop signal.
            loop {
                tokio::select! {
                    _ = stop_rx.changed() => {
                        tracing::info!("Unsubscribe signal received for admin PDA {}. Closing stream.", pda);
                        break;
                    }

                    Some(event) = listener.next_live_event() => {
                        let item = gateway::EventStreamItem::from(event);
                        if tx_clone.send(Ok(item)).await.is_err() {
                            tracing::warn!("Client for admin PDA {} disconnected during live stream.", pda);
                            break;
                        }
                    }

                    else => break,
                }
            }

            active_subscriptions_clone.remove(&pda);
            tracing::info!("Event stream for admin PDA {} has ended.", pda);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
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
}
