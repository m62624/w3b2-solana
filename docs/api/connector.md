# API Reference: Connector Library

The `w3b2-solana-connector` is a high-level, asynchronous Rust library for building custom backend services that interact with the on-chain program. It is the foundational layer upon which the Gateway is built and is intended for Rust-native services that require more direct control than the gRPC Gateway provides.

## Key Components

The library provides two primary structs:

1.  **`client::TransactionBuilder`**: Constructs unsigned Solana transactions for the on-chain program's instructions.
2.  **`events::EventManager`**: Listens for on-chain events, manages state synchronization, and handles RPC/WebSocket connection issues.

---

## 1. `client::TransactionBuilder`

The `TransactionBuilder` is the entry point for creating unsigned transactions. A backend service uses it to prepare a transaction, which is then sent to a client (e.g., a browser wallet) for signing. The connector itself does not handle private keys.

### Example Flow

```rust
// 1. The backend service prepares the transaction.
let unsigned_tx = transaction_builder
    .prepare_user_deposit(user_pda, user_authority, amount)
    .await?;

// 2. The service sends the serialized, unsigned transaction
//    to the user's client.
let tx_bytes = bincode::serialize(&unsigned_tx)?;
// ... send bytes to client ...

// 3. The user's client signs the transaction and sends it
//    back to the service.
// ... receive signed bytes from client ...
let signed_tx_bytes = ...;

// 4. The service submits the signed transaction.
let final_tx: solana_sdk::Transaction = bincode::deserialize(&signed_tx_bytes)?;
let signature = transaction_builder.submit_transaction(&final_tx).await?;
```

---

## 2. `events::EventManager` and Listeners

The `EventManager` is used to synchronize an application's state with the blockchain.

### Architecture

The event system consists of three main components:

-   **`EventManager`**: A long-running background task that manages workers for fetching historical events ("catch-up") and streaming live events via WebSockets. One instance is typically run for the lifetime of an application.
-   **`EventManagerHandle`**: A cloneable handle to the `EventManager`, used to create new event listeners.
-   **`UserListener` / `AdminListener`**: High-level abstractions that subscribe to events for a single PDA. They provide separate, ordered channels for `catch-up` and `live` events.

### Consumption Pattern

The listeners are designed for state synchronization. The recommended usage pattern is:

1.  **Create a listener** for the PDA to be monitored.
2.  **Process the `catch-up` stream**. This stream contains all historical events for the PDA and closes automatically once the sync is complete. Draining this stream first brings the local application state up to the present.
3.  **Process the `live` stream**. This stream provides all new events as they are emitted on-chain.

```rust
// Get a handle to the running EventManager
let event_manager_handle = ...;

// Create a listener for a specific user profile
let mut user_listener = event_manager_handle.listen_as_user(user_profile_pda);

// 1. Process all historical events first.
while let Some(event) = user_listener.next_catchup_event().await {
    // Update local database with this past event
}

// 2. Now, process live events indefinitely.
while let Some(event) = user_listener.next_live_event().await {
    // Update state or push a notification for this new event
}
```

This two-phase approach ensures that no events are missed and allows an application to reliably rebuild its state.