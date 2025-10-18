# W3B2 Solana Connector

This crate provides a high-level, asynchronous Rust library for building backend services that interact with the `w3b2-solana-program`. It is the primary tool for Rust-based services (like oracles, administrative tools, or the gRPC gateway) to listen for on-chain events.

The connector's main responsibility is to provide a robust and persistent event streaming system that allows an application to monitor the activity of any `AdminProfile` or `UserProfile` PDA.

## Key Features

- **Asynchronous API**: Built on `tokio`, the entire crate is `async` and designed for use in modern, high-performance Rust applications.
- **Robust Event Handling**: The event listening system automatically handles historical event catch-up and real-time event streaming, providing a consistent and gap-free view of on-chain state.
- **Persistent State**: It uses a lightweight `SQLite` database to keep track of processed transaction signatures, ensuring that event processing can resume from the correct point after a restart, preventing missed or duplicated events.
- **Specific Listeners**: Provides dedicated listener types (`AdminListener`, `UserListener`) for subscribing to events related to a specific on-chain account.

## Usage

The primary use case is to listen for events emitted by the on-chain program. The `EventManager` is the central component that manages the connection and event dispatching.

### Listening for Events

The `UserListener` and `AdminListener` provide a powerful way to monitor on-chain activity for a specific account.

```rust,ignore
use w3b2_solana_connector::{
    event_manager::EventManager,
    listener::{AdminListener, UserListener},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

// 1. Initialize the RPC Client
let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));

// 2. Initialize the EventManager (typically once per application)
//    It requires a path to a SQLite database for persistence.
let event_manager = EventManager::new(
    rpc_client.clone(),
    "sqlite://./event_store.db?mode=rwc".to_string(),
)
.await?;
let dispatcher = event_manager.dispatcher();

// 3. Create a listener for a specific UserProfile PDA
let user_pda = Pubkey::new_unique();
let mut user_listener = UserListener::new(user_pda, dispatcher.clone(), 100);

// 4. Spawn a task to process events for the user
tokio::spawn(async move {
    // First, process all historical events to ensure state is synchronized
    while let Some(event) = user_listener.next_catchup_event().await {
        println!("[User: {}] Caught up on historical event: {:?}", user_pda, event);
    }
    println!("[User: {}] State is fully synchronized.", user_pda);

    // Then, process new events as they arrive in real-time
    while let Some(event) = user_listener.next_live_event().await {
        println!("[User: {}] Received live event: {:?}", user_pda, event);
    }
});

// The listener will automatically unsubscribe when it is dropped.
```

For more detailed information, please refer to the Rustdoc comments within the source code.