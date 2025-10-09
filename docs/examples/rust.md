# Rust Example

This example demonstrates how to interact with the W3B2 Gateway service from a Rust application using the `tonic` gRPC framework.

## Prerequisites

Add the necessary dependencies to your `Cargo.toml`. You will need `tonic`, `prost`, `tokio`, and the protobuf definitions from the W3B2 `proto` directory.

```toml
[dependencies]
tonic = "0.8"
prost = "0.11"
tokio = { version = "1", features = ["full"] }
w3b2-protocol-proto = { path = "../proto" } # Assuming relative path
solana-sdk = "1.18"
bs58 = "0.5"
```

You'll also need to set up `tonic-build` in your `build.rs` to generate the client code from the `.proto` files.

## 1. The Oracle Service (Your Backend)

First, let's look at a simplified version of the **developer's oracle service**. This is the server you would write. It has one job: to receive a command request and return a signed quote.

```rust
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use std::time::{SystemTime, UNIX_EPOCH};

// In a real app, load this from a secure vault or env var.
// DO NOT hardcode private keys.
fn get_oracle_keypair() -> Keypair {
    // Example keypair
    Keypair::from_bytes(&[...]).unwrap()
}

// This struct represents the JSON response your oracle sends.
#[derive(serde::Serialize)]
struct OracleQuote {
    command_id: u16,
    price: u64,
    timestamp: u64,
    signature: String, // base64 encoded
    oracle_pubkey: String, // base58 encoded
}

// This function would be part of your web server (e.g., Axum, Actix-web)
async fn handle_quote_request(command_id: u16) -> OracleQuote {
    let oracle_keypair = get_oracle_keypair();
    let price = 50000; // Look up price for command_id
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    // Construct the message exactly as the on-chain program expects
    let message = [
        &command_id.to_le_bytes()[..],
        &price.to_le_bytes()[..],
        &timestamp.to_le_bytes()[..],
    ].concat();

    // Sign the message
    let signature = oracle_keypair.sign_message(&message);

    OracleQuote {
        command_id,
        price,
        timestamp,
        signature: base64::encode(signature.as_ref()),
        oracle_pubkey: oracle_keypair.pubkey().to_string(),
    }
}
```

## 2. The Client (Interacting with W3B2 Gateway)

Now, here is the full client-side logic for interacting with the W3B2 Gateway.

```rust
use tonic::transport::Channel;
use w3b2_protocol_proto::gateway::{
    bridge_gateway_service_client::BridgeGatewayServiceClient,
    PrepareUserDispatchCommandRequest, SubmitTransactionRequest,
};
use solana_sdk::{
    transaction::Transaction,
    signature::{Keypair, Signer},
    pubkey::Pubkey,
    bs58,
};
use std::str::FromStr;

// This would be the user's keypair, managed by their wallet.
fn get_user_keypair() -> Keypair {
    Keypair::from_bytes(&[...]).unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the W3B2 Gateway
    let mut client = BridgeGatewayServiceClient::connect("http://localhost:50051").await?;

    // 2. Get the signed quote from your oracle service (as shown in Part 1)
    let quote: OracleQuote = reqwest::get("http://api.ai-art-generator.com/v1/generate-quote")
        .await?
        .json()
        .await?;

    // 3. Prepare the transaction by calling the gateway
    let user_keypair = get_user_keypair();
    let user_profile_pda = Pubkey::from_str("...").unwrap(); // User's profile PDA for this service

    let request = PrepareUserDispatchCommandRequest {
        user_profile_pda: user_profile_pda.to_string(),
        user_authority: user_keypair.pubkey().to_string(),
        oracle_authority: quote.oracle_pubkey,
        command_id: quote.command_id as u32,
        price: quote.price,
        timestamp: quote.timestamp,
        signature: bs58::decode(quote.signature).into_vec()?,
        payload: vec![], // Optional payload
    };

    let unsigned_tx_response = client.prepare_user_dispatch_command(request).await?.into_inner();

    // 4. Sign the transaction locally
    let mut transaction: Transaction = bincode::deserialize(&unsigned_tx_response.transaction)?;
    let recent_blockhash = ...; // You would fetch this from the RPC node
    transaction.sign(&[&user_keypair], recent_blockhash);

    // 5. Submit the signed transaction
    let submit_request = SubmitTransactionRequest {
        signed_transaction: bincode::serialize(&transaction)?,
    };

    let tx_response = client.submit_transaction(submit_request).await?.into_inner();

    println!("Transaction submitted successfully!");
    println!("Signature: {}", tx_response.signature);

    Ok(())
}
```

## 3. Listening for Events

To be notified when the transaction is confirmed (or when any other on-chain event happens), you can use the `ListenAsAdmin` or `ListenAsUser` streaming RPCs.

```rust
use w3b2_protocol_proto::gateway::ListenRequest;
use tokio_stream::StreamExt;

async fn listen_for_events(
    mut client: BridgeGatewayServiceClient<Channel>,
    pda_to_listen: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = ListenRequest {
        pda: pda_to_listen.to_string(),
    };

    let mut stream = client.listen_as_admin(request).await?.into_inner();

    println!("Listening for events on {}...", pda_to_listen);

    // The stream will first deliver historical "catch-up" events, then live ones.
    while let Some(item) = stream.next().await {
        let event_item = item?;
        println!("Received event: {:?}", event_item.event);
        // Here you would process the event, e.g., update your database
    }

    Ok(())
}
```