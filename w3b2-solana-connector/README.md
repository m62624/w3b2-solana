# W3B2 Bridge Connector

A core Rust library for interacting with the `w3b2-solana-program` on the Solana blockchain.

## Overview

`w3b2-solana-connector` is a high-level, asynchronous library that provides all the necessary tools to build backend services (like the `w3b2-solana-gateway`) on top of the W3B2 protocol. It handles the complexities of on-chain interaction, allowing developers to focus on their application's business logic.

The library is designed to be the foundational layer for any application that needs to:
-   Listen to on-chain events in real-time with guaranteed delivery.
-   Prepare non-custodial transactions for remote signing.
-   Ensure reliable and resilient synchronization with the blockchain.

## Core Modules & Usage

The library's functionality is primarily exposed through three modules: `client`, `workers`, and `listener`.

### 1. `client::TransactionBuilder`

This module provides a non-custodial builder for creating unsigned Solana transactions. Your service uses the builder to construct a transaction, serializes it, and sends it to a client (e.g., a mobile app). The client signs it with their local wallet and sends it back for submission.

**This library never handles private keys.**

#### Example: Preparing and Submitting a Transaction

```rust
use std::sync::Arc;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use w3b2_connector::client::TransactionBuilder;

async fn transaction_example() -> anyhow::Result<()> {
    let rpc_client = Arc::new(RpcClient::new("http://127.0.0.1:8899".to_string()));
    let builder = TransactionBuilder::new(rpc_client.clone());

    // --- On the Server (e.g., w3b2-solana-gateway) ---

    // 1. Prepare an unsigned transaction.
    let admin_authority_pubkey = Pubkey::new_unique();
    let comm_pubkey = Pubkey::new_unique();
    let mut unsigned_tx = builder
        .prepare_admin_register_profile(admin_authority_pubkey, comm_pubkey)
        .await?;

    // 2. Serialize it to send to the client.
    // (You would also need to fetch and set the recent_blockhash before sending)
    let blockhash = rpc_client.get_latest_blockhash().await?;
    unsigned_tx.message.recent_blockhash = blockhash;
    let tx_bytes = bincode::serialize(&unsigned_tx)?;

    // --- On the Client (e.g., a mobile app) ---

    // 3. The client deserializes, signs, and sends it back.
    let admin_keypair = Keypair::new(); // The client holds this securely.
    let mut received_tx: solana_sdk::Transaction = bincode::deserialize(&tx_bytes)?;
    received_tx.sign(&[&admin_keypair], blockhash);
    let signed_tx_bytes = bincode::serialize(&received_tx)?;

    // --- Back on the Server ---

    // 4. The server deserializes the signed transaction and submits it.
    let final_tx: solana_sdk::Transaction = bincode::deserialize(&signed_tx_bytes)?;
    let signature = builder.submit_transaction(&final_tx).await?;

    println!("Transaction successful with signature: {}", signature);

    Ok(())
}
```

### 2. `workers` and `listener` Modules

These modules work together to provide real-time event streaming.

*   The `workers::EventManager` runs background tasks to sync with the blockchain. You create it once and run it in a spawned task.
*   The `workers::EventManagerHandle` is your clonable handle to interact with the running manager.
*   The `listener` module provides high-level `UserListener` and `AdminListener` structs that consume the raw event stream and categorize events into clean, purpose-driven channels.

#### Architecture

```
                                     [Your Service / Gateway]
                                                ^
                                                | (PDA-specific events)
                              +-----------------+-----------------+
                              |                                 |
                        [UserListener]                    [AdminListener]
                        (live_rx, catchup_rx)             (live_rx, catchup_rx)
                              ^                                 ^
                              | (Events for specific PDA)       |
                              +-------[Dispatcher]----------------+
                                                ^
                                                | (Unified BridgeEvent Stream)
                                       [Synchronizer]
                                       /            \
                              (WebSocket)        (RPC)
                              /                    \
                      [LiveWorker]             [CatchupWorker]
                          ^                          ^
                          |                          |
                  +-------+--------------------------+
                  |
          [Solana Blockchain]
```

#### Example: Using a `UserListener` with Catch-up and Live Events

This example shows the recommended pattern for consuming events: first, process all historical ("catch-up") events to sync state, then switch to processing real-time ("live") events.

```rust
use std::sync::Arc;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use w3b2_connector::{
    config::ConnectorConfig,
    storage::SledStorage, // Example storage
    workers::EventManager,
};

async fn listener_example() -> anyhow::Result<()> {
    // --- 1. Setup Phase (in your main application logic) ---
    let config = Arc::new(ConnectorConfig::default());
    let rpc_client = Arc::new(RpcClient::new(config.solana.rpc_url.clone()));
    let db = sled::Config::new().temporary(true).open()?;
    let storage = Arc::new(SledStorage::new(db));

    // Get the manager runner and its handle. Channel capacities are now read from config.
    let (event_manager_runner, event_manager_handle) =
        EventManager::new(config, rpc_client, storage);

    // Spawn the manager as a long-running background task
    tokio::spawn(event_manager_runner.run());

    // --- 2. Usage Phase (in your request handlers) ---

    let user_profile_pda = Pubkey::new_unique(); // The PDA of the user profile to monitor.

    // Create a high-level listener for the user's PDA.
    // The listener automatically subscribes to the Dispatcher.
    let mut user_listener = event_manager_handle.listen_as_user(user_profile_pda);

    // A. First, process all historical events to bring the application state up to date.
    // The `catchup` channel will close automatically once all historical events are processed.
    println!("Processing historical events for {}...", user_profile_pda);
    while let Some(event) = user_listener.next_catchup_event().await {
        println!("[Catchup] Received event: {:?}", event.data);
        // The `Dispatcher` ensures you receive all relevant events, including
        // actions taken by other parties (e.g., an Admin sending you a command).
    }
    println!("Historical sync complete.");

    // B. Now, listen for live events from the WebSocket stream.
    // This loop will run indefinitely.
    println!("Listening for live events for {}...", user_profile_pda);
    while let Some(event) = user_listener.next_live_event().await {
        println!("[Live] Received event: {:?}", event.data);
    }

    // The listener will automatically unsubscribe when it goes out of scope (via Drop),
    // or you can call `user_listener.unsubscribe().await` for manual control.

    Ok(())
}
```
