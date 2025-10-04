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
use w3b2_connector::{
    client::TransactionBuilder,
    workers::{EventManager, EventManagerHandle},
};
use w3b2_program::state::PriceEntry;

use crate::grpc::proto::w3b2::protocol::gateway::bridge_gateway_service_server::{
    BridgeGatewayService, BridgeGatewayServiceServer,
};
use crate::{
    config::GatewayConfig,
    error::GatewayError,
    grpc::proto::w3b2::protocol::gateway::{
        self, EventStreamItem, ListenRequest, PrepareAdminCloseProfileRequest,
        PrepareAdminDispatchCommandRequest, PrepareAdminRegisterProfileRequest,
        PrepareAdminUpdateCommKeyRequest, PrepareAdminUpdatePricesRequest,
        PrepareAdminWithdrawRequest, PrepareLogActionRequest, PrepareUserCloseProfileRequest,
        PrepareUserCreateProfileRequest, PrepareUserDepositRequest,
        PrepareUserDispatchCommandRequest, PrepareUserUpdateCommKeyRequest,
        PrepareUserWithdrawRequest, SubmitTransactionRequest, TransactionResponse,
        UnsignedTransactionResponse, UnsubscribeRequest,
    },
    storage::SledStorage,
};

pub mod proto {
    pub mod w3b2 {
        pub mod protocol {
            pub mod gateway {
                tonic::include_proto!("w3b2.protocol.gateway");
            }
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<RpcClient>,
    pub event_manager: EventManagerHandle,
    pub config: Arc<GatewayConfig>,
    /// Stores senders to signal termination for active subscriptions.
    pub active_subscriptions: Arc<DashMap<Pubkey, watch::Sender<()>>>,
}

/// gRPC server implementation.
pub struct GatewayServer {
    state: AppState,
}

impl GatewayServer {
    /// Create a new GatewayServer instance.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// The main entry point to start the gRPC server and all background services.
pub async fn start(config: &GatewayConfig) -> Result<EventManagerHandle> {
    // --- 1. Initialize dependencies ---
    let db = sled::open(&config.gateway.db_path)?;
    let storage = Arc::new(SledStorage::new(db));
    let addr = format!("{}:{}", config.gateway.grpc.host, config.gateway.grpc.port).parse()?;
    let rpc_client = Arc::new(RpcClient::new(config.connector.solana.rpc_url.clone()));

    // --- 2. Create and spawn the EventManager service ---

    // `EventManager::new` now returns the runner and its handle.
    let (event_manager_runner, event_manager_handle) = EventManager::new(
        Arc::new(config.connector.clone()),
        rpc_client.clone(),
        storage,
    );

    tokio::spawn(event_manager_runner.run());

    // --- 3. Set up the gRPC server state ---

    // Clone the handle for the gRPC server state. The original will be returned.
    let handle_for_server = event_manager_handle.clone();

    // Create the shared state, storing the lightweight `handle` for the RPCs to use.
    let app_state = AppState {
        rpc_client,
        event_manager: handle_for_server, // Store the cloned handle
        config: Arc::new(config.clone()),
        active_subscriptions: Arc::new(DashMap::new()),
    };

    let gateway_server = GatewayServer::new(app_state);

    tracing::info!(
        "Non-Custodial gRPC Gateway with Event Streaming listening on {}",
        addr
    );

    let grpc_server =
        Server::builder().add_service(BridgeGatewayServiceServer::new(gateway_server));

    tokio::spawn(async move {
        if let Err(e) = grpc_server.serve(addr).await {
            tracing::error!("gRPC server failed: {}", e);
        }
    });

    Ok(event_manager_handle)
}

// helper: parse a Pubkey returning GatewayError
fn parse_pubkey(s: &str) -> Result<Pubkey, GatewayError> {
    Pubkey::from_str(s).map_err(GatewayError::from)
}

#[tonic::async_trait]
impl BridgeGatewayService for GatewayServer {
    type ListenAsUserStream = ReceiverStream<Result<EventStreamItem, Status>>;

    async fn listen_as_user(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenAsUserStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received ListenAsUser request for PDA: {}", req.pda);

        let pda = parse_pubkey(&req.pda).map_err(Status::from)?;

        let mut listener = self.state.event_manager.listen_as_user(pda);
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
                "A listener for PDA {} is already active",
                pda
            )));
        }

        let tx_clone = tx.clone();
        let active_subscriptions_clone = self.state.active_subscriptions.clone();

        tokio::spawn(async move {
            // Phase 1: Drain all catchup events. This loop will naturally end when the
            // catchup worker has sent all historical events and closes its side of the channel.
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
                    // Always listen for the stop signal.
                    _ = stop_rx.changed() => {
                        tracing::info!("Unsubscribe signal received for user PDA {}. Closing stream.", pda);
                        break;
                    }

                    // Listen for live events.
                    Some(event) = listener.next_live_event() => {
                        let item = gateway::EventStreamItem::from(event);
                        if tx_clone.send(Ok(item)).await.is_err() {
                            tracing::warn!("Client for PDA {} disconnected during live stream.", pda);
                            break;
                        }
                    }

                    else => {
                        // Live channel closed, which means the system is shutting down.
                        break;
                    }
                }
            }

            active_subscriptions_clone.remove(&pda);
            tracing::info!("Event stream for user PDA {} has ended.", pda);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type ListenAsAdminStream = ReceiverStream<Result<EventStreamItem, Status>>;

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
                "A listener for PDA {} is already active",
                pda
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

                    else => {
                        // Live channel closed, system is shutting down.
                        break;
                    }
                }
            }

            active_subscriptions_clone.remove(&pda);
            tracing::info!("Event stream for admin PDA {} has ended.", pda);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn unsubscribe(
        &self,
        request: Request<UnsubscribeRequest>,
    ) -> Result<Response<()>, Status> {
        let result: Result<Response<()>, GatewayError> = (async {
            let req = request.into_inner();
            let pda_to_stop = parse_pubkey(&req.pda)?;
            tracing::info!("Received Unsubscribe request for PDA: {}", pda_to_stop);

            // Find the subscription and send a stop signal.
            // The `remove` also drops the sender, ensuring the watch channel closes.
            if let Some((_, stop_tx)) = self.state.active_subscriptions.remove(&pda_to_stop) {
                // The `send` will notify the receiver. It might return an error if the
                // receiver is already gone, which is fine.
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
            let transaction = builder
                .prepare_admin_register_profile(authority, communication_pubkey)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_register_profile tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_update_comm_key(
        &self,
        request: Request<PrepareAdminUpdateCommKeyRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminUpdateCommKey request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let new_key = parse_pubkey(&req.new_key)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_update_comm_key(authority, new_key)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_update_comm_key tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_update_prices(
        &self,
        request: Request<PrepareAdminUpdatePricesRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminUpdatePrices request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;

            let new_prices = req
                .new_prices
                .into_iter()
                .map(|p| PriceEntry {
                    command_id: p.command_id as u16,
                    price: p.price,
                })
                .collect::<Vec<PriceEntry>>();

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_update_prices(authority, new_prices)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_update_prices tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_admin_withdraw(authority, req.amount, destination)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared admin_withdraw tx for authority {}", authority);

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_admin_close_profile(authority)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_close_profile tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_admin_dispatch_command(
                    authority,
                    target_user_profile_pda,
                    req.command_id,
                    req.payload,
                )
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_dispatch_command tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_user_create_profile(authority, target_admin_pda, communication_pubkey)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared user_create_profile tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_user_update_comm_key(authority, admin_profile_pda, new_key)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared user_update_comm_key tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_user_deposit(authority, admin_profile_pda, req.amount)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared user_deposit tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_user_withdraw(authority, admin_profile_pda, req.amount, destination)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared user_withdraw tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_user_close_profile(authority, admin_profile_pda)
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared user_close_profile tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_dispatch_command(
                    authority,
                    target_admin_pda,
                    req.command_id as u16,
                    req.payload,
                )
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared user_dispatch_command tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
            let transaction = builder
                .prepare_log_action(
                    authority,
                    user_profile_pda,
                    admin_profile_pda,
                    req.session_id,
                    req.action_code as u16,
                )
                .await
                .map_err(GatewayError::Connector)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared log_action tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

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
                .map_err(GatewayError::Connector)?;
            tracing::info!("Submitted transaction, signature: {}", signature);

            Ok(Response::new(TransactionResponse {
                signature: signature.to_string(),
            }))
        })
        .await;

        result.map_err(Status::from)
    }
}
