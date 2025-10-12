# Connector Library Reference

The `w3b2-solana-connector` crate is a high-level, asynchronous Rust library for building backend services that interact with the `w3b2-solana-program`. It is the ideal tool for creating custom Rust backends, such as a gRPC gateway or a custom oracle, that need to communicate directly with the on-chain program.

## Core Components

The library offers two primary components that cover the main areas of backend interaction: creating transactions and listening to on-chain state changes.

### 1. `TransactionBuilder`

The `TransactionBuilder` is a non-custodial helper for creating unsigned transactions for every instruction in the on-chain program.

-   **Non-Custodial by Design**: Its most important feature is that it **never handles private keys**. It builds a complete, unsigned `VersionedTransaction`, which can then be serialized and sent to a client (e.g., a browser wallet) for signing. This design makes it safe to use in a backend environment, as the server never has access to user keys.
-   **Asynchronous API**: Built on `tokio`, the entire API is `async`, making it easy to integrate into modern Rust applications.
-   **Public Method Coverage**: It provides a dedicated method for every instruction in the on-chain program (e.g., `prepare_user_dispatch_command`, `prepare_admin_ban_user`), ensuring 100% API coverage.

#### Example Usage

```rust,ignore
// This example demonstrates the "prepare-then-submit" flow.

// 1. Prepare the unsigned transaction on the backend
let unsigned_tx = tx_builder.prepare_user_dispatch_command(
    user_wallet,
    admin_pda,
    // ... args
).await?;

// 2. Serialize and send to the client for signing
let serialized_tx = bincode::serialize(&unsigned_tx)?;
// ... send to client ...

// 3. Client signs the transaction and sends it back
let signed_tx = client.sign_transaction(serialized_tx);

// 4. Submit the signed transaction to the network
let signature = tx_builder.submit_transaction(&signed_tx).await?;
```

### 2. `EventListener` (`UserListener` & `AdminListener`)

The `EventListener` provides a robust system for subscribing to on-chain events related to a specific `UserProfile` or `AdminProfile` PDA. This is the foundation of the event-driven architecture.

-   **Catch-up and Live Events**: A key challenge in blockchain development is ensuring state is synchronized. The listener solves this by providing two distinct, ordered streams for every subscription:
    1.  **Catch-up Stream (`next_catchup_event`)**: First, the listener queries all *historical* events for the given PDA and delivers them in order. Your application should process all of these events to build a complete, up-to-date picture of the PDA's state.
    2.  **Live Stream (`next_live_event`)**: Once the catch-up queue is empty, the listener seamlessly transitions to delivering *new* events in real-time as they are emitted on-chain.
-   **Automatic Resource Management**: The listener automatically registers with a central `EventManager` on creation and, more importantly, automatically unsubscribes when it is dropped (goes out of scope). This RAII pattern prevents resource leaks and simplifies application code.

#### Example Usage

```rust,ignore
// Create a listener for a specific UserProfile PDA
let mut user_listener = UserListener::new(user_pda, dispatcher, 100);

// 1. Synchronize state by processing all historical events
while let Some(event) = user_listener.next_catchup_event().await {
    // e.g., update a record in your database
    db.apply_historical_event(event);
}

// 2. Process new events as they arrive in real-time
while let Some(event) = user_listener.next_live_event().await {
    // e.g., send a push notification
    notifications.send_for_live_event(event);
}
```