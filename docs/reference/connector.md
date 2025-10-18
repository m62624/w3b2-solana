# Connector Library Reference

The `w3b2-solana-connector` crate is a high-level, asynchronous Rust library for building backend services that listen to events from the `w3b2-solana-program`. It is the core component that powers the `w3b2-solana-gateway` and is the recommended tool for any Rust-based service that needs to react to on-chain activity.

## Core Purpose: Event Listening

The connector's primary role is to provide a robust and persistent event streaming system. It handles the complexities of fetching both historical and real-time transactions, parsing logs, and dispatching events to the correct listeners.

-   **Persistent Sync State**: The connector uses a storage backend (like `SQLite` via `sled`) to keep track of which transaction signatures have been processed. This ensures that if your service restarts, it can resume fetching events exactly where it left off, preventing missed or duplicated events.
-   **Catch-up and Live Events**: A key challenge in blockchain development is ensuring state is synchronized. The connector solves this by providing two distinct, ordered streams for every subscription:
    1.  **Catch-up Stream (`next_catchup_event`)**: When a listener is created, the connector first queries all *historical* events for the given PDA and delivers them in order. Your application should process all of these to build a complete, up-to-date picture of the PDA's state.
    2.  **Live Stream (`next_live_event`)**: Once the catch-up queue is empty, the listener seamlessly transitions to delivering *new* events in real-time as they are confirmed on-chain.
-   **Automatic Resource Management**: The `EventListener` (`UserListener`/`AdminListener`) automatically registers with a central `EventManager` on creation and, more importantly, automatically unsubscribes when it is dropped (goes out of scope). This RAII pattern prevents resource leaks.

### Example Usage

The main entry point is the `EventManager`, which runs the background workers. You create it, spawn its `run` method, and then use the returned `handle` to create listeners.

```rust,ignore
use w3b2_solana_connector::{
    workers::EventManager,
    config::ConnectorConfig,
    storage::MemoryStorage, // Or SledStorage for persistence
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

// 1. Setup configuration
let config = Arc::new(ConnectorConfig::default());
let rpc_client = Arc::new(RpcClient::new("...".to_string()));
// Use MemoryStorage for simple cases, or SledStorage for file-based persistence
let storage = Arc::new(MemoryStorage::new());

// 2. Create the EventManager and its handle
let (event_manager, handle) = EventManager::new(
    config.clone(),
    rpc_client.clone(),
    storage,
);

// 3. Spawn the workers to run in the background
tokio::spawn(async move {
    event_manager.run().await;
});

// 4. Use the handle to create listeners in your application logic
let user_pda = // ... some user profile PDA
let mut listener = handle.listen_as_user(user_pda);

// 5. Process events
tokio::spawn(async move {
    // First, synchronize state by processing all historical events
    while let Some(event) = listener.next_catchup_event().await {
        // e.g., update a record in your database
    }

    // Then, process new events as they arrive in real-time
    while let Some(event) = listener.next_live_event().await {
        // e.g., send a push notification
    }
});
```

## Secondary Utility: `TransactionBuilder`

The connector also includes a `TransactionBuilder`, a legacy utility for creating unsigned transaction messages in Rust.

-   **Use Case**: This is a helper for **off-chain Rust services** (e.g., an oracle, a custom admin tool) that need to construct instructions programmatically in Rust.
-   **Not for General Use**: It is **not** the primary or recommended way for typical clients to interact with the on-chain program. Standard clients (web, mobile) should use the program's IDL with libraries like `@coral-xyz/anchor` (TypeScript) or `anchorpy` (Python).