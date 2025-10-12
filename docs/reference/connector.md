# Connector Library Reference

The `w3b2-solana-connector` crate is a high-level, asynchronous Rust library for building backend services that interact with the `w3b2-solana-program`. It is the ideal tool for creating custom Rust backends, such as a gRPC gateway or a custom oracle, that need to communicate directly with the on-chain program.

## Core Components

The library offers two primary components that cover the main areas of backend interaction: creating transactions and listening to on-chain state changes.

### 1. `TransactionBuilder`

The `TransactionBuilder` is a utility for backend Rust services to create unsigned transactions for the on-chain program.

-   **Backend-Focused**: It is designed for server-side use where a Rust service (like an admin tool or oracle) needs to create, sign, and submit transactions itself. It is **not** intended for preparing transactions for external clients.
-   **Asynchronous API**: Built on `tokio`, the entire API is `async`, making it easy to integrate into modern Rust applications.
-   **Public Method Coverage**: It provides a dedicated method for every instruction in the on-chain program (e.g., `prepare_admin_ban_user`), ensuring 100% API coverage.

#### Example Usage

```rust
// This example demonstrates a backend service banning a user.

// 1. Prepare the unsigned transaction
let mut unsigned_tx = tx_builder.prepare_admin_ban_user(
    service_wallet.pubkey(), // The service's own wallet
    user_pda_to_ban,
).await?;

// 2. The service signs and submits the transaction itself
let blockhash = rpc_client.get_latest_blockhash().await?;
unsigned_tx.sign(&[&service_wallet], blockhash);
let signature = rpc_client.send_and_confirm_transaction(&unsigned_tx).await?;
```

### 2. `EventListener` (`UserListener` & `AdminListener`)

The `EventListener` provides a robust system for subscribing to on-chain events related to a specific `UserProfile` or `AdminProfile` PDA. This is the foundation of the event-driven architecture.

-   **Catch-up and Live Events**: A key challenge in blockchain development is ensuring state is synchronized. The listener solves this by providing two distinct, ordered streams for every subscription:
    1.  **Catch-up Stream (`next_catchup_event`)**: First, the listener queries all *historical* events for the given PDA and delivers them in order. Your application should process all of these events to build a complete, up-to-date picture of the PDA's state.
    2.  **Live Stream (`next_live_event`)**: Once the catch-up queue is empty, the listener seamlessly transitions to delivering *new* events in real-time as they are emitted on-chain.
-   **Automatic Resource Management**: The listener automatically registers with a central `EventManager` on creation and, more importantly, automatically unsubscribes when it is dropped (goes out of scope). This RAII pattern prevents resource leaks and simplifies application code.

#### Example Usage

```rust
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