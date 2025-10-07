# API Reference: Connector Library

The `w3b2-solana-connector` is a high-level, asynchronous Rust library for building custom backend services on top of the W3B2 protocol. It is the foundational layer upon which the Gateway is built.

You should use the connector directly if you are building a Rust-native service and require more control, performance, or custom logic than the generic gRPC Gateway can provide.

## Core Features

The library provides two primary capabilities:

1.  **Non-Custodial Transaction Building**: A safe, fluent API to construct any on-chain instruction without ever handling a private key.
2.  **Resilient Event Streaming**: A robust system for listening to on-chain events, with a guarantee of "exactly once" delivery and a clear separation between historical and real-time events.

---

## 1. `client::TransactionBuilder`

The `TransactionBuilder` is the entry point for creating unsigned transactions. Your service uses it to prepare a transaction, which is then sent to the end-user's client (e.g., a browser wallet) for signing.

**Key Principle**: The connector library is strictly non-custodial. It never touches private keys.

### Example Flow

```rust
// 1. Your backend service prepares the transaction.
let unsigned_tx = transaction_builder
    .prepare_user_deposit(user_pda, user_authority, amount)
    .await?;

// 2. Your service sends the serialized, unsigned transaction
//    to the user's client.
let tx_bytes = bincode::serialize(&unsigned_tx)?;
// ... send bytes to client ...

// 3. The user's client (e.g., browser) signs the transaction
//    and sends it back to your server.
// ... receive signed bytes from client ...
let signed_tx_bytes = ...;

// 4. Your service submits the signed transaction.
let final_tx: solana_sdk::Transaction = bincode::deserialize(&signed_tx_bytes)?;
let signature = transaction_builder.submit_transaction(&final_tx).await?;
```

---

## 2. Event Listening (`EventManager` & Listeners)

The connector provides a powerful event manager for synchronizing your application's state with the blockchain. It is designed for resilience, handling RPC failures and WebSocket disconnects gracefully.

### Architecture

The system consists of a few key components:

-   **`EventManager`**: A long-running background task that manages workers for fetching historical events (catch-up) and streaming live events (via WebSockets). You create one instance and run it for the lifetime of your application.
-   **`EventManagerHandle`**: A cheap, cloneable handle to the `EventManager`. You use this handle to create new event listeners.
-   **`UserListener` / `AdminListener`**: High-level abstractions that subscribe to events for a *single* PDA. They provide separate, ordered channels for `catch-up` and `live` events.

### Recommended Consumption Pattern

The listeners are designed to make state synchronization simple and correct. The recommended pattern is:

1.  **Create a listener** for the PDA you want to monitor.
2.  **Drain the `catch-up` stream completely**. This brings your application's state for that entity up to the present moment. The stream will close automatically when the historical sync is done.
3.  **Begin processing the `live` stream**. This stream will provide all new events as they happen.

```rust
// Get a handle to the running EventManager
let event_manager_handle = ...;

// Create a listener for a specific user profile
let mut user_listener = event_manager_handle.listen_as_user(user_profile_pda);

// 1. Process all historical events first.
while let Some(event) = user_listener.next_catchup_event().await {
    // Update your database with this past event
}

// 2. Now, process live events indefinitely.
while let Some(event) = user_listener.next_live_event().await {
    // Update state or push a notification for this new event
}
```

This two-phase approach ensures that you never miss an event and that your application can reliably rebuild its state at any time.