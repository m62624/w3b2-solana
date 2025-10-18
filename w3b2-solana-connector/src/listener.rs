//! # High-Level Event Listeners
//!
//! This module provides high-level, contextual event listeners, [`UserListener`] and [`AdminListener`],
//! which abstract away the raw event stream from the internal `Dispatcher`.
//!
//! These listeners are the primary mechanism for an application to receive on-chain events
//! related to a specific `AdminProfile` or `UserProfile` PDA. They simplify event consumption
//! by providing two distinct, ordered streams of events:
//!
//! 1.  **Catch-up Stream**: Delivers all historical events for the PDA, from the beginning of
//!     its history up to the point where the listener was created. This ensures that the
//!     application has a complete and consistent view of the PDA's state.
//!
//! 2.  **Live Stream**: Delivers all new events that occur in real-time while the listener is active.

use crate::dispatcher::{DispatcherCommand, DispatcherHandle, ListenerChannels};
pub use crate::events::BridgeEvent;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;

/// A type alias for an [`EventListener`] configured to listen to a `UserProfile` PDA.
pub type UserListener = EventListener;
/// A type alias for an [`EventListener`] configured to listen to an `AdminProfile` PDA.
pub type AdminListener = EventListener;

/// A generic event listener that subscribes to events related to a specific PDA.
///
/// It registers itself with the `Dispatcher` upon creation and provides two `mpsc::Receiver`
/// channels to receive `BridgeEvent`s: one for historical "catch-up" events and one for
/// "live" events from the WebSocket stream.
///
/// # Resource Management
///
/// The `EventListener` automatically handles unsubscription from the `Dispatcher`
/// when it is dropped, ensuring clean resource management without requiring manual cleanup.
#[derive(Debug)]
pub struct EventListener {
    /// A channel receiver for live events pushed from the WebSocket stream.
    live_rx: mpsc::Receiver<BridgeEvent>,
    /// A channel receiver for historical events queried by the `CatchupWorker`.
    catchup_rx: mpsc::Receiver<BridgeEvent>,
    /// Contains the PDA and dispatcher handle needed for unsubscribing on `Drop`.
    /// This is an `Option` to allow for a clean "take" pattern, preventing double-unsubscription.
    unsubscribe_info: Option<(Pubkey, DispatcherHandle)>,
}

impl EventListener {
    /// Creates a new `EventListener` and registers it with the `Dispatcher`.
    ///
    /// This function spawns a Tokio task to send a `Register` command to the central
    /// `Dispatcher`, which will then begin routing events for the specified PDA to
    /// the channels provided by this listener.
    ///
    /// # Arguments
    ///
    /// * `pda_to_listen_on` - The `Pubkey` of the `AdminProfile` or `UserProfile` PDA to monitor.
    /// * `dispatcher` - A handle to the central `Dispatcher` that manages all event subscriptions.
    /// * `channel_capacity` - The buffer capacity of the MPSC channels for live and catch-up events.
    pub fn new(
        pda_to_listen_on: Pubkey,
        dispatcher: DispatcherHandle,
        channel_capacity: usize,
    ) -> Self {
        let (live_tx, live_rx) = mpsc::channel(channel_capacity);
        let (catchup_tx, catchup_rx) = mpsc::channel(channel_capacity);

        // Asynchronously send the registration command to the dispatcher
        let dispatcher_clone = dispatcher.clone();
        tokio::spawn(async move {
            let _ = dispatcher_clone
                .command_tx
                .send(DispatcherCommand::Register(
                    pda_to_listen_on,
                    ListenerChannels {
                        live: live_tx,
                        catchup: catchup_tx,
                    },
                ))
                .await;
        });

        Self {
            live_rx,
            catchup_rx,
            unsubscribe_info: Some((pda_to_listen_on, dispatcher)),
        }
    }

    /// Receives the next live event from the WebSocket stream.
    ///
    /// Returns `None` if the channel is closed, which typically happens when the
    /// `EventManager` is shut down.
    pub async fn next_live_event(&mut self) -> Option<BridgeEvent> {
        self.live_rx.recv().await
    }

    /// Receives the next historical event from the catch-up worker.
    ///
    /// Returns `None` once all historical events have been delivered.
    pub async fn next_catchup_event(&mut self) -> Option<BridgeEvent> {
        self.catchup_rx.recv().await
    }

    /// Manually unsubscribes the listener from the event dispatcher.
    ///
    /// This method consumes the listener, preventing further use. After this is called, the
    /// listener will no longer receive events. The automatic `Drop` implementation will
    /// not attempt to unsubscribe a second time. This is useful for cases where explicit
    /// cleanup is preferred over relying on the `Drop` trait.
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
    /// Automatically unsubscribes the listener from the `Dispatcher` when it goes out of scope.
    fn drop(&mut self) {
        // Only perform automatic unsubscription if it hasn't been done manually via `unsubscribe()`.
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!(
                "Automatic unsubscribe (on drop) for EventListener on PDA {}",
                pda
            );
            // Spawn a new task to send the unregister command without blocking the current thread.
            tokio::spawn(async move {
                let _ = dispatcher
                    .command_tx
                    .send(DispatcherCommand::Unregister(pda))
                    .await;
            });
        }
    }
}
