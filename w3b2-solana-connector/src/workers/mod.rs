//! # Event Manager & Background Workers
//!
//! This module defines the `EventManager`, which orchestrates all the background services
//! required for robust on-chain event listening.
//!
//! ## Core Components
//!
//! - [`EventManager`]: The main struct that owns and runs the background workers. It is
//!   consumed when its `run` method is called.
//! - [`EventManagerHandle`]: A clonable, thread-safe handle that provides the public API
//!   for interacting with the running services (e.g., creating listeners, shutting down).
//! - **Workers**:
//!   - `Synchronizer`: Continuously fetches and stores transaction signatures for all PDAs.
//!   - `LiveWorker`: Subscribes to a WebSocket stream for real-time transaction updates.
//!   - `CatchupWorker`: Fetches historical transactions for newly registered listeners.
//!   - `Dispatcher`: Routes events from the workers to the correct listeners.
//!

mod catchup;
mod live;
mod synchronizer;

use crate::{
    config::ConnectorConfig,
    dispatcher::{Dispatcher, DispatcherHandle},
    listener::{AdminListener, UserListener},
    storage::Storage,
    workers::synchronizer::Synchronizer,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tokio::sync::mpsc;

/// A clonable, thread-safe handle for interacting with the `EventManager`'s background services.
///
/// This handle is the primary public entry point for applications using the connector. It is
/// obtained when an [`EventManager`] is created and can be cloned and passed across threads.
#[derive(Debug, Clone)]
pub struct EventManagerHandle {
    dispatcher: DispatcherHandle,
    config: Arc<ConnectorConfig>,
}

impl EventManagerHandle {
    /// Sends a shutdown signal to the `EventManager`'s background services.
    ///
    /// This will cause the `Dispatcher` and `Synchronizer` to gracefully terminate their loops.
    pub async fn stop(&self) {
        self.dispatcher.stop().await;
    }

    /// Creates and returns a contextual listener for a `UserProfile` PDA.
    ///
    /// This is the primary method for applications to listen to events for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user_profile_pda` - The public key of the user's profile PDA to monitor.
    pub fn listen_as_user(&self, user_profile_pda: Pubkey) -> UserListener {
        UserListener::new(
            user_profile_pda,
            self.dispatcher.clone(),
            self.config.channels.listener_event_buffer,
        )
    }

    /// Creates and returns a contextual listener for an `AdminProfile` PDA.
    ///
    /// # Arguments
    ///
    /// * `admin_profile_pda` - The public key of the admin's profile PDA to monitor.
    pub fn listen_as_admin(&self, admin_profile_pda: Pubkey) -> AdminListener {
        AdminListener::new(
            admin_profile_pda,
            self.dispatcher.clone(),
            self.config.channels.listener_event_buffer,
        )
    }
}

/// The main background service manager for the connector.
///
/// This struct orchestrates the `Synchronizer` and `Dispatcher` workers. It is created once,
/// its [`run()`] method is spawned as a background task, and it is then consumed, leaving
/// the [`EventManagerHandle`] as the only way to interact with the running services.
pub struct EventManager {
    synchronizer: Synchronizer,
    dispatcher: Dispatcher,
}

impl EventManager {
    /// Creates a new `EventManager` and its associated [`EventManagerHandle`].
    ///
    /// This method sets up the necessary communication channels between the internal workers
    /// but does not start them. The returned `EventManager` instance must be started by
    /// calling the [`run()`] method.
    ///
    /// # Arguments
    ///
    /// * `config` - The shared connector configuration.
    /// * `rpc_client` - A shared Solana RPC client.
    /// * `storage` - A shared, thread-safe storage backend for persisting sync state.
    ///
    /// # Returns
    ///
    /// A tuple containing the `EventManager` runner and its public `EventManagerHandle`.
    pub fn new(
        config: Arc<ConnectorConfig>,
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage>,
    ) -> (Self, EventManagerHandle) {
        let (dispatcher_cmd_tx, dispatcher_cmd_rx) =
            mpsc::channel(config.channels.dispatcher_command_buffer);

        let (dispatcher, dispatcher_handle) =
            Dispatcher::new(config.clone(), dispatcher_cmd_tx, dispatcher_cmd_rx);

        let synchronizer = Synchronizer::new(
            config.clone(),
            rpc_client,
            storage,
            dispatcher_handle.clone(),
        );

        let runner = Self {
            synchronizer,
            dispatcher,
        };

        let handle = EventManagerHandle {
            dispatcher: dispatcher_handle,
            config,
        };

        (runner, handle)
    }

    /// Runs all background services of the connector.
    ///
    /// This method consumes the `EventManager` and should be spawned as a single, long-running
    /// background task. It will run until a shutdown is initiated via [`EventManagerHandle::stop()`]
    /// or a critical error occurs in one of the workers.
    pub async fn run(self) {
        tracing::info!("Connector is running all background services.");

        tokio::select! {
            res = self.synchronizer.run() => {
                if let Err(e) = res { tracing::error!("Synchronizer exited with an error: {}", e); }
                else { tracing::info!("Synchronizer has shut down."); }
            },
            res = self.dispatcher.run() => {
                if let Err(e) = res { tracing::error!("Dispatcher exited with an error: {}", e); }
                tracing::info!("Dispatcher has shut down.");
            }
        }
    }
}
