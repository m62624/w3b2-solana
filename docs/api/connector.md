# API Reference: Connector Library

The `w3b2-solana-connector` is a high-level, asynchronous Rust library for building backend services that interact with the `w3b2-solana-program`. It is the ideal tool for building custom Rust backends (e.g., a gRPC gateway) that require direct, low-level interaction with the on-chain program.

## Key Features

-   **Non-Custodial by Design**: The `TransactionBuilder` only creates unsigned transactions. It never handles private keys, making it suitable for secure backend services where the signing is delegated to a client.
-   **Asynchronous API**: Built on Tokio, the entire crate is `async` and fits naturally into modern Rust applications.
-   **Robust Event Handling**: The event listening system automatically handles historical event catch-up and real-time event streaming, providing two separate, ordered channels to ensure your application has a complete and consistent view of on-chain state.
-   **Test-Friendly**: The `TransactionBuilder` is generic over an `AsyncRpcClient` trait, allowing for easy mocking with `solana-program-test`'s `BanksClient` in your integration tests.

---

## `TransactionBuilder`

The `TransactionBuilder` is used to prepare an unsigned transaction for any instruction in the on-chain program. A backend service uses it to construct the transaction, which is then sent to a client for signing.

### Example Usage

```rust,ignore
use w3b2_solana_connector::client::{TransactionBuilder, UserDispatchCommandArgs};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

// 1. Initialize the RPC Client and Builder
let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
let tx_builder = TransactionBuilder::new(rpc_client);

// 2. Define transaction parameters
let user_wallet = Pubkey::new_unique();
let admin_pda = Pubkey::new_unique();
let oracle_key = Pubkey::new_unique();
let oracle_signature = [0u8; 64]; // The real signature from the oracle

// 3. Prepare the unsigned transaction
let unsigned_tx = tx_builder.prepare_user_dispatch_command(
    user_wallet,
    admin_pda,
    UserDispatchCommandArgs {
        command_id: 1,
        price: 1000,
        timestamp: 1672531200,
        payload: vec![1, 2, 3],
        oracle_pubkey: oracle_key,
        oracle_signature,
    },
).await?;

// 4. The `unsigned_tx` can now be serialized and sent to the user's wallet for signing.
//    Once signed, it can be submitted to the network via `tx_builder.submit_transaction()`.
```

---

## Event Listeners

The `UserListener` and `AdminListener` provide a powerful way to monitor on-chain activity for a specific PDA. They are created via the `EventManager`, which is the central service that runs in the background to poll the blockchain.

### Consumption Pattern

The listeners are designed for robust state synchronization. The recommended usage pattern is:

1.  **Initialize the `EventManager`**: This is typically done once per application lifetime. It spawns the necessary background workers.
2.  **Create a Listener**: Use the `EventManagerHandle` to create a `UserListener` or `AdminListener` for a specific PDA.
3.  **Process Catch-up Events**: Drain the `catch-up` stream first. This stream contains all historical events for the PDA and will close automatically once the sync is complete. This ensures your local state is consistent with the blockchain's history.
4.  **Process Live Events**: After the catch-up is complete, listen on the `live` stream for new events as they happen in real-time.

### Example Usage

```rust,ignore
use w3b2_solana_connector::workers::EventManager;
use w3b2_solana_connector::listener::UserListener;
use solana_sdk::pubkey::Pubkey;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

// 1. Initialize the EventManager
let rpc_client = Arc::new(RpcClient::new("...".to_string()));
let (runner, handle) = EventManager::new(rpc_client.clone(), "sqlite::memory:".to_string()).await?;

// Run the event manager in the background
tokio::spawn(runner.run());

// 2. Create a listener for a specific UserProfile PDA
let user_pda = Pubkey::new_unique();
let mut user_listener = handle.listen_as_user(user_pda, 100);

// 3. Spawn a task to process events for this listener
tokio::spawn(async move {
    // First, process all historical events
    while let Some(event) = user_listener.next_catchup_event().await {
        println!("Caught up on historical event: {:?}", event);
    }
    println!("State for {} is fully synchronized.", user_pda);

    // Then, process new events as they arrive in real-time
    while let Some(event) = user_listener.next_live_event().await {
        println!("Received live event: {:?}", event);
    }
});

// The listener will automatically unsubscribe from the EventManager when it is dropped.
```