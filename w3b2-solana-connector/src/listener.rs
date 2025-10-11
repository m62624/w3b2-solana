//! # Contextual Event Listeners
//!
//! This module provides high-level, contextual event listeners (`UserListener`, `AdminListener`)
//! that abstract away the raw event stream from the `Dispatcher`. These listeners categorize
//! events based on the role of the wallet being monitored, providing clean, purpose-driven
//! event streams for application logic.

use crate::dispatcher::{DispatcherCommand, DispatcherHandle, ListenerChannels};
pub use crate::events::BridgeEvent;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;

/// A type alias for an `EventListener` configured to listen to a user's PDA.
pub type UserListener = EventListener;
/// A type alias for an `EventListener` configured to listen to an admin's PDA.
pub type AdminListener = EventListener;

/// A generic event listener that subscribes to events related to a specific PDA.
///
/// It registers itself with the `Dispatcher` and provides channels to receive
/// both live and historical (catch-up) events. It also handles automatic
/// unsubscription when it is dropped, ensuring clean resource management.
#[derive(Debug)]
pub struct EventListener {
    /// Receives live events from the WebSocket stream.
    live_rx: mpsc::Receiver<BridgeEvent>,
    /// Receives historical events from the catch-up worker.
    catchup_rx: mpsc::Receiver<BridgeEvent>,
    /// Contains the PDA and dispatcher handle needed for unsubscribing.
    /// This is an `Option` to allow for manual unsubscription by taking the value.
    unsubscribe_info: Option<(Pubkey, DispatcherHandle)>,
}

impl EventListener {
    pub fn new(
        pda_to_listen_on: Pubkey,
        dispatcher: DispatcherHandle,
        channel_capacity: usize,
    ) -> Self {
        let (live_tx, live_rx) = mpsc::channel(channel_capacity);
        let (catchup_tx, catchup_rx) = mpsc::channel(channel_capacity);

        let dispatcher_clone = dispatcher.clone();
        tokio::spawn(async move {
            dispatcher_clone
                .command_tx
                .send(DispatcherCommand::Register(
                    pda_to_listen_on,
                    ListenerChannels {
                        live: live_tx,
                        catchup: catchup_tx,
                    },
                ))
                .await
                .ok();
        });

        Self {
            live_rx,
            catchup_rx,
            unsubscribe_info: Some((pda_to_listen_on, dispatcher)),
        }
    }

    /// Receives the next live event. Returns `None` if the stream is closed.
    pub async fn next_live_event(&mut self) -> Option<BridgeEvent> {
        self.live_rx.recv().await
    }

    /// Receives the next catch-up event. Returns `None` if the stream is closed.
    pub async fn next_catchup_event(&mut self) -> Option<BridgeEvent> {
        self.catchup_rx.recv().await
    }

    /// Manually unsubscribes the listener from the event dispatcher.
    ///
    /// This method consumes the listener, preventing further use. After this is called, the
    /// listener will no longer receive events, and the automatic `Drop` implementation will
    /// not attempt to unsubscribe a second time.
    pub async fn unsubscribe(mut self) {
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!("Manual unsubscribe for EventListener on PDA {}", pda);
            let _ = dispatcher
                .command_tx
                .send(DispatcherCommand::Unregister(pda))
                .await;
        }
    }
}

impl Drop for EventListener {
    fn drop(&mut self) {
        // Only perform automatic unsubscription if it hasn't been done manually.
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!(
                "Automatic unsubscribe (on drop) for EventListener on PDA {}",
                pda
            );
            tokio::spawn(async move {
                dispatcher
                    .command_tx
                    .send(DispatcherCommand::Unregister(pda))
                    .await
                    .ok();
            });
        }
    }
}
