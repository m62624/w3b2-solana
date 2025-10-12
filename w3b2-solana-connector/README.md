# W3B2 Solana Connector

This crate provides a high-level, asynchronous Rust library for building backend services that interact with the `w3b2-solana-program`. It offers two primary components:

1.  **[`TransactionBuilder`](src/client.rs)**: A utility for backend Rust services to create unsigned transactions for the on-chain program.
2.  **Event Management ([`EventManager`](src/workers/mod.rs))**: A robust system for subscribing to on-chain events for specific `UserProfile` or `AdminProfile` PDAs.

It is the ideal tool for building custom Rust backends (e.g., oracles, admin tools) that require direct interaction with the on-chain program.

## Key Features

- **Backend-Focused**: The `TransactionBuilder` is designed for server-side use where a Rust service needs to create transactions.
- **Asynchronous API**: Built on Tokio, the entire crate is `async` and fits naturally into modern Rust applications.
- **Robust Event Handling**: The `EventManager` automatically handles historical event catch-up and real-time event streaming, providing a consistent view of on-chain state.
- **Test-Friendly**: The `TransactionBuilder` is generic over an `AsyncRpcClient` trait, allowing for easy mocking with `solana-program-test`'s `BanksClient` in your integration tests.

## Usage Example

### 1. Building a Transaction

The `TransactionBuilder` is used by a backend Rust service to prepare a transaction. The service would then sign and submit it.

```rust,ignore
use w3b2_solana_connector::client::{TransactionBuilder, UserDispatchCommandArgs};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::Keypair;
use std::sync::Arc;

// 1. Initialize the RPC Client and Builder
let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
let tx_builder = TransactionBuilder::new(rpc_client.clone());

// 2. Define transaction parameters
let service_wallet = Keypair::new(); // The service's wallet
let admin_pda = Pubkey::new_unique();
let user_pda_to_ban = Pubkey::new_unique();

// 3. Prepare the unsigned transaction
let mut unsigned_tx = tx_builder.prepare_admin_ban_user(
    service_wallet.pubkey(),
    user_pda_to_ban,
).await?;

// 4. The service can now sign and send the transaction itself.
let blockhash = rpc_client.get_latest_blockhash().await?;
unsigned_tx.sign(&[&service_wallet], blockhash);
let signature = rpc_client.send_and_confirm_transaction(&unsigned_tx).await?;
```

### 2. Listening for Events

The `UserListener` and `AdminListener` provide a powerful way to monitor on-chain activity for a specific account.

```rust,ignore
use w3b2_solana_connector::workers::EventManager;
use w3b2_solana_connector::listener::UserListener;
use solana_sdk::pubkey::Pubkey;

// 1. Initialize the EventManager (typically done once per application)
let event_manager = EventManager::new(rpc_client.clone(), "sqlite::memory:".to_string()).await?;
let dispatcher = event_manager.dispatcher();

// 2. Create a listener for a specific UserProfile PDA
let user_pda = Pubkey::new_unique();
let mut user_listener = UserListener::new(user_pda, dispatcher, 100);

// 3. Spawn a task to process events
tokio::spawn(async move {
    // First, process all historical events to ensure state is synchronized
    while let Some(event) = user_listener.next_catchup_event().await {
        println!("Caught up on historical event: {:?}", event);
    }
    println!("State for {} is fully synchronized.", user_pda);

    // Then, process new events as they arrive in real-time
    while let Some(event) = user_listener.next_live_event().await {
        println!("Received live event: {:?}", event);
    }
});

// The listener will automatically unsubscribe when it is dropped.
```

For more detailed information, please refer to the Rustdoc comments within the source code.