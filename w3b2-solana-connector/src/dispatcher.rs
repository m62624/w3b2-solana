//! # Event Dispatcher
//!
//! The `Dispatcher` is a background worker that acts as a central router for on-chain events.
//!
//! ## Purpose
//! It subscribes to the single, unified event stream produced by the `Synchronizer` and
//! forwards each event only to the specific listeners that have registered an interest
//! in one of the public keys involved in that event.
//!
//! This architecture prevents each `UserListener` or `AdminListener` from having to
//! process and filter the entire "firehose" of on-chain events, significantly
//! improving efficiency.
use crate::{
    config::ConnectorConfig,
    events::{BridgeEvent, EventSource},
};
use futures::future;
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;

/// A background worker that routes events from a single source to multiple listeners.
///
/// It maintains a map of active listeners and forwards incoming events from the
/// `Synchronizer`'s broadcast channel to the appropriate `mpsc` channels based on
/// the public keys associated with each event.
pub struct Dispatcher {
    listeners: HashMap<Pubkey, ListenerChannels>,
    command_rx: mpsc::Receiver<DispatcherCommand>,
    event_tx: mpsc::Sender<BridgeEvent>,
    event_rx: mpsc::Receiver<BridgeEvent>,
}

/// Defines commands that can be sent to the Dispatcher task.
#[derive(Debug)]
pub enum DispatcherCommand {
    Register(Pubkey, ListenerChannels),
    Unregister(Pubkey),
    Dispatch(BridgeEvent),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct ListenerChannels {
    pub live: mpsc::Sender<BridgeEvent>,
    pub catchup: mpsc::Sender<BridgeEvent>,
}

#[derive(Clone, Debug)]
pub struct DispatcherHandle {
    pub command_tx: mpsc::Sender<DispatcherCommand>,
}

/// A collection of arguments required for the `prepare_user_dispatch_command` method.
///
/// This struct simplifies the method signature by grouping all the parameters
/// related to the oracle-signed command.
pub struct UserDispatchCommandArgs {
    /// The `u16` identifier for the command, as signed by the oracle.
    pub command_id: u16,
    /// The price of the command in lamports, as signed by the oracle.
    pub price: u64,
    /// The Unix timestamp from the oracle's signature, used to prevent replay attacks.
    pub timestamp: i64,
    /// An opaque byte array for application-specific data.
    pub payload: Vec<u8>,
    /// The public key of the oracle that signed the message.
    pub oracle_pubkey: Pubkey,
    /// The 64-byte Ed25519 signature from the oracle.
    pub oracle_signature: [u8; 64],
}

impl DispatcherHandle {
    pub async fn dispatch(&self, event: BridgeEvent) {
        if self
            .command_tx
            .send(DispatcherCommand::Dispatch(event))
            .await
            .is_err()
        {
            tracing::warn!("Failed to dispatch event: dispatcher may be down");
        }
    }

    pub async fn stop(&self) {
        if self
            .command_tx
            .send(DispatcherCommand::Shutdown)
            .await
            .is_err()
        {
            tracing::warn!("Failed to send shutdown to dispatcher: it may already be down");
        }
    }
}

impl Dispatcher {
    /// Creates a new `Dispatcher`.
    pub fn new(
        config: Arc<ConnectorConfig>,
        command_tx: mpsc::Sender<DispatcherCommand>,
        command_rx: mpsc::Receiver<DispatcherCommand>,
    ) -> (Self, DispatcherHandle) {
        let (event_tx, event_rx) = mpsc::channel(config.channels.dispatcher_event_buffer);
        let dispatcher = Self {
            listeners: HashMap::new(),
            command_rx,
            event_tx,
            event_rx,
        };
        let handle = DispatcherHandle { command_tx };
        (dispatcher, handle)
    }

    /// Runs the main event loop for the dispatcher.
    pub async fn run(mut self) -> anyhow::Result<()> {
        tracing::info!("Dispatcher started. Waiting for events and commands...");
        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => self.handle_event(event).await,
                Some(command) = self.command_rx.recv() => {
                    if self.handle_command(command).await {
                        break;
                    }
                },
                else => {
                    tracing::info!("All channels closed. Dispatcher shutting down.");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Handles an incoming event by dispatching it to all relevant listeners.
    async fn handle_event(&mut self, event: BridgeEvent) {
        let pdas = extract_pdas_from_event(&event.data);
        let sends = pdas
            .iter()
            .filter_map(|pda| self.listeners.get(pda).map(|channels| (pda, channels)))
            .map(|(pda, channels)| {
                let target_tx = match event.source {
                    EventSource::Live => &channels.live,
                    EventSource::Catchup => &channels.catchup,
                };
                let event_clone = event.clone();
                async move {
                    if target_tx.send(event_clone).await.is_err() {
                        tracing::warn!(
                            "Listener for PDA {} disconnected. It will be removed.",
                            pda
                        );
                        return Some(*pda);
                    }
                    None
                }
            });

        let results = future::join_all(sends).await;
        for pda_to_remove in results.into_iter().flatten() {
            self.listeners.remove(&pda_to_remove);
        }
    }

    /// Handles an incoming command. Returns `true` if the dispatcher should shut down.
    async fn handle_command(&mut self, command: DispatcherCommand) -> bool {
        match command {
            DispatcherCommand::Register(pda, channels) => {
                tracing::info!("Registering new listener for PDA {}", pda);
                self.listeners.insert(pda, channels);
            }
            DispatcherCommand::Unregister(pda) => {
                tracing::info!("Unregistering listener for PDA {}", pda);
                self.listeners.remove(&pda);
            }
            DispatcherCommand::Dispatch(event) => {
                if self.event_tx.send(event).await.is_err() {
                    tracing::error!("Event receiver closed. Shutting down dispatcher.");
                    return true; // Signal shutdown
                }
            }
            DispatcherCommand::Shutdown => {
                tracing::info!("Received shutdown command. Exiting.");
                return true; // Signal shutdown
            }
        }
        false
    }
}

/// A helper function that inspects a `BridgeEvent` and returns a `Vec<Pubkey>`
/// of all relevant PDAs.
fn extract_pdas_from_event(event_data: &crate::events::BridgeEventData) -> Vec<Pubkey> {
    match event_data {
        // Admin-only events
        crate::events::BridgeEventData::AdminProfileRegistered(e) => vec![e.admin_pda],
        crate::events::BridgeEventData::AdminConfigUpdated(e) => vec![e.admin_pda],
        crate::events::BridgeEventData::AdminFundsWithdrawn(e) => vec![e.admin_pda],
        crate::events::BridgeEventData::AdminProfileClosed(e) => vec![e.admin_pda],

        // User-only events
        crate::events::BridgeEventData::UserCommKeyUpdated(e) => vec![e.user_profile_pda],
        crate::events::BridgeEventData::UserFundsDeposited(e) => vec![e.user_profile_pda],
        crate::events::BridgeEventData::UserFundsWithdrawn(e) => vec![e.user_profile_pda],

        // Events relevant to both User and Admin
        crate::events::BridgeEventData::UserProfileCreated(e) => {
            vec![e.user_pda, e.target_admin_pda]
        }
        crate::events::BridgeEventData::UserProfileClosed(e) => vec![e.user_pda, e.admin_pda],
        crate::events::BridgeEventData::UserCommandDispatched(e) => {
            vec![e.sender_user_pda, e.target_admin_pda]
        }
        crate::events::BridgeEventData::AdminCommandDispatched(e) => {
            vec![e.target_user_pda, e.sender_admin_pda]
        }
        crate::events::BridgeEventData::OffChainActionLogged(e) => {
            vec![e.user_profile_pda, e.admin_profile_pda]
        }
        crate::events::BridgeEventData::AdminUnbanFeeUpdated(e) => vec![e.admin_pda],
        crate::events::BridgeEventData::UserBanned(e) => {
            vec![e.user_profile_pda, e.admin_pda]
        }
        crate::events::BridgeEventData::UserUnbanned(e) => {
            vec![e.user_profile_pda, e.admin_pda]
        }
        crate::events::BridgeEventData::UserUnbanRequested(e) => {
            vec![e.user_profile_pda, e.admin_pda]
        }
        crate::events::BridgeEventData::Unknown => vec![],
    }
}
