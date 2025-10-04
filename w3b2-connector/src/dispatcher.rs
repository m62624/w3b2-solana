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
    /// A receiver for commands to manage the `listeners` map (e.g., adding or
    /// removing subscriptions).
    command_rx: mpsc::Receiver<DispatcherCommand>,
    /// A receiver for events from the synchronizer workers.
    event_tx: mpsc::Sender<BridgeEvent>,
    event_rx: mpsc::Receiver<BridgeEvent>,
}

/// Defines commands that can be sent to the Dispatcher task.
#[derive(Debug)]
pub enum DispatcherCommand {
    /// Registers a new listener for a given public key.
    Register(Pubkey, ListenerChannels),
    /// Unregisters a listener for a given public key.
    Unregister(Pubkey),
    /// An event to be dispatched.
    Dispatch(BridgeEvent),
    /// Signals the dispatcher to shut down gracefully.
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

impl DispatcherHandle {
    pub async fn dispatch(&self, event: BridgeEvent) {
        if self
            .command_tx
            .send(DispatcherCommand::Dispatch(event))
            .await
            .is_err()
        {
            tracing::warn!("Failed to dispatch event, dispatcher may be down");
        }
    }

    pub async fn stop(&self) {
        if self
            .command_tx
            .send(DispatcherCommand::Shutdown)
            .await
            .is_err()
        {
            tracing::warn!("Failed to send shutdown to dispatcher, it may already be down");
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
                Some(event) = self.event_rx.recv() => {
                    let relevant_pdas = extract_pdas_from_event(&event.data);
                    for pda in relevant_pdas {
                        if let Some(channels) = self.listeners.get(&pda) {
                            let target_tx = match event.source {
                                EventSource::Live => &channels.live,
                                EventSource::Catchup => &channels.catchup,
                            };

                            if target_tx.send(event.clone()).await.is_err() {
                                tracing::warn!("Attempted to send to a disconnected listener for PDA {}. Removing.", pda);
                                // The listener is gone, let's remove it.
                                self.listeners.remove(&pda);
                            }
                        }
                    }
                },
                Some(command) = self.command_rx.recv() => {
                    match command {
                        DispatcherCommand::Register(pda, channels) => {
                            tracing::info!("Dispatcher: Registering new listener for PDA {}", pda);
                            self.listeners.insert(pda, channels);
                        },
                        DispatcherCommand::Unregister(pda) => {
                            tracing::info!("Dispatcher: Unregistering listener for PDA {}", pda);
                            self.listeners.remove(&pda);
                        },
                        DispatcherCommand::Dispatch(event) => {
                            if self.event_tx.send(event).await.is_err() {
                                tracing::error!("Event receiver closed. Shutting down dispatcher.");
                                break;
                            }
                        }
                        DispatcherCommand::Shutdown => {
                            tracing::info!("Dispatcher: Received shutdown command. Exiting.");
                            break;
                        }
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
}

/// A helper function that inspects a `BridgeEvent` and returns a `Vec<Pubkey>`
/// of all relevant PDAs.
fn extract_pdas_from_event(event_data: &crate::events::BridgeEventData) -> Vec<Pubkey> {
    use w3b2_program::ID as PROGRAM_ID;

    match event_data {
        crate::events::BridgeEventData::AdminProfileRegistered(e) => {
            vec![Pubkey::find_program_address(&[b"admin", e.authority.as_ref()], &PROGRAM_ID).0]
        }
        crate::events::BridgeEventData::AdminCommKeyUpdated(e) => {
            vec![Pubkey::find_program_address(&[b"admin", e.authority.as_ref()], &PROGRAM_ID).0]
        }
        crate::events::BridgeEventData::AdminPricesUpdated(e) => {
            vec![Pubkey::find_program_address(&[b"admin", e.authority.as_ref()], &PROGRAM_ID).0]
        }
        crate::events::BridgeEventData::AdminFundsWithdrawn(e) => {
            vec![Pubkey::find_program_address(&[b"admin", e.authority.as_ref()], &PROGRAM_ID).0]
        }
        crate::events::BridgeEventData::AdminProfileClosed(e) => vec![e.admin_pda],

        crate::events::BridgeEventData::UserProfileCreated(e) => {
            let (user_pda, _) = Pubkey::find_program_address(
                &[b"user", e.authority.as_ref(), e.target_admin_pda.as_ref()],
                &PROGRAM_ID,
            );
            vec![user_pda, e.target_admin_pda]
        }
        crate::events::BridgeEventData::UserProfileClosed(e) => {
            let (user_pda, _) = Pubkey::find_program_address(
                &[b"user", e.authority.as_ref(), e.admin_pda.as_ref()],
                &PROGRAM_ID,
            );
            vec![user_pda, e.admin_pda]
        }
        crate::events::BridgeEventData::UserCommandDispatched(e) => {
            let (user_pda, _) = Pubkey::find_program_address(
                &[b"user", e.sender.as_ref(), e.target_admin_pda.as_ref()],
                &PROGRAM_ID,
            );
            vec![user_pda, e.target_admin_pda]
        }
        crate::events::BridgeEventData::AdminCommandDispatched(e) => {
            // The event contains the target User PDA. The Admin PDA can be derived
            // from the sender's authority key if needed by a listener.
            let (admin_pda, _) =
                Pubkey::find_program_address(&[b"admin", e.sender.as_ref()], &PROGRAM_ID);
            vec![e.target_user_pda, admin_pda]
        }
        crate::events::BridgeEventData::OffChainActionLogged(e) => {
            // The `actor` is an authority (wallet key), not a PDA.
            // The `target` is the PDA of the other party in the interaction.
            // We only dispatch based on the target PDA.
            vec![e.target]
        }
        crate::events::BridgeEventData::Unknown => vec![],
        // These events now contain the user_profile_pda and can be routed.
        crate::events::BridgeEventData::UserCommKeyUpdated(e) => vec![e.user_profile_pda],
        crate::events::BridgeEventData::UserFundsDeposited(e) => vec![e.user_profile_pda],
        crate::events::BridgeEventData::UserFundsWithdrawn(e) => vec![e.user_profile_pda],
    }
}
