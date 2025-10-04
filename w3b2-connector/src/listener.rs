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

// --- User Listener ---

/// Manages and categorizes event streams from a user's perspective.
#[derive(Debug)]
pub struct UserListener {
    /// Receives all events relevant to the user's profile PDA.
    live_rx: mpsc::Receiver<BridgeEvent>,
    catchup_rx: mpsc::Receiver<BridgeEvent>,
    /// Contains the PDA and dispatcher handle needed for unsubscribing.
    unsubscribe_info: Option<(Pubkey, DispatcherHandle)>,
}

impl UserListener {
    pub fn new(
        user_profile_pda: Pubkey,
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
                    user_profile_pda,
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
            unsubscribe_info: Some((user_profile_pda, dispatcher)),
        }
    }

    /// Receives the next live event for this user. Returns `None` if the stream is closed.
    pub async fn next_live_event(&mut self) -> Option<BridgeEvent> {
        self.live_rx.recv().await
    }

    /// Receives the next catch-up event for this user. Returns `None` if the stream is closed.
    pub async fn next_catchup_event(&mut self) -> Option<BridgeEvent> {
        self.catchup_rx.recv().await
    }

    /// Manually unsubscribes the listener from the event dispatcher.
    ///
    /// This method consumes the listener, preventing further use. After this is called,
    /// the listener will no longer receive events, and the automatic `Drop` implementation
    /// will not attempt to unsubscribe a second time.
    pub async fn unsubscribe(mut self) {
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!("Manual unsubscribe for UserListener on PDA {}", pda);
            let _ = dispatcher
                .command_tx
                .send(DispatcherCommand::Unregister(pda))
                .await;
        }
    }
}

impl Drop for UserListener {
    fn drop(&mut self) {
        // Only perform automatic unsubscription if it hasn't been done manually.
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!(
                "Automatic unsubscribe (on drop) for UserListener on PDA {}",
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

// --- Admin Listener ---

#[derive(Debug)]
pub struct AdminListener {
    /// Receives all events relevant to the admin's profile PDA.
    live_rx: mpsc::Receiver<BridgeEvent>,
    catchup_rx: mpsc::Receiver<BridgeEvent>,
    /// Contains the PDA and dispatcher handle needed for unsubscribing.
    unsubscribe_info: Option<(Pubkey, DispatcherHandle)>,
}

impl AdminListener {
    pub fn new(
        admin_profile_pda: Pubkey,
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
                    admin_profile_pda,
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
            unsubscribe_info: Some((admin_profile_pda, dispatcher)),
        }
    }

    /// Receives the next live event for this admin. Returns `None` if the stream is closed.
    pub async fn next_live_event(&mut self) -> Option<BridgeEvent> {
        self.live_rx.recv().await
    }

    /// Receives the next catch-up event for this admin. Returns `None` if the stream is closed.
    pub async fn next_catchup_event(&mut self) -> Option<BridgeEvent> {
        self.catchup_rx.recv().await
    }

    /// Manually unsubscribes the listener from the event dispatcher.
    ///
    /// This method consumes the listener, preventing further use. After this is called,
    /// the listener will no longer receive events, and the automatic `Drop` implementation
    /// will not attempt to unsubscribe a second time.
    pub async fn unsubscribe(mut self) {
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!("Manual unsubscribe for AdminListener on PDA {}", pda);
            let _ = dispatcher
                .command_tx
                .send(DispatcherCommand::Unregister(pda))
                .await;
        }
    }
}

impl Drop for AdminListener {
    fn drop(&mut self) {
        // Only perform automatic unsubscription if it hasn't been done manually.
        if let Some((pda, dispatcher)) = self.unsubscribe_info.take() {
            tracing::debug!(
                "Automatic unsubscribe (on drop) for AdminListener on PDA {}",
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
