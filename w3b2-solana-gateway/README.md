# w3b2-solana-gateway

This crate provides a ready-to-use gRPC service that exposes the functionality of the `w3b2-solana-program` to clients written in any gRPC-compatible language (e.g., Python, TypeScript, Go). It is built on top of the `w3b2-solana-connector` and handles the complexities of blockchain interaction, offering a high-level, language-agnostic API.

## Core Functionality

The gateway provides three main categories of RPC methods, defined in the `proto/w3b2/protocol/gateway/bridge_gateway_service.proto` file:

1.  **Transaction Preparation (`prepare_*`)**: A suite of methods that map one-to-one with the instructions in the on-chain program. Each method constructs an **unsigned** transaction and returns it to the client as a serialized byte array. This enables a non-custodial workflow where the gateway never handles private keys.
2.  **Transaction Submission (`submit_transaction`)**: A single method that accepts a signed transaction from a client, deserializes it, and submits it to the Solana network, returning the transaction signature.
3.  **Event Streaming (`listen_as_user`, `listen_as_admin`)**: Server-side streaming methods that allow clients to subscribe to a persistent stream of on-chain events for a specific `UserProfile` or `AdminProfile` PDA.

## Usage Example with `grpcurl`

The following example demonstrates how a client would interact with the gateway to prepare a transaction, and separately, how to listen for events.

### Prerequisites

- `grpcurl` installed.
- The gateway server running locally on `localhost:9090`.

### 1. Prepare a `user_deposit` Transaction

This RPC call asks the gateway to create an unsigned transaction for a user to deposit 0.1 SOL into their profile.

**Request:**

```bash
grpcurl -plaintext \
    -d '{
        "authority_pubkey": "USER_WALLET_PUBKEY",
        "admin_profile_pda": "ADMIN_PDA_PUBKEY",
        "amount": 100000000
    }' \
    localhost:9090 w3b2.protocol.gateway.BridgeGatewayService/PrepareUserDeposit
```

- `USER_WALLET_PUBKEY`: The user's public key (e.g., `5x...`). This wallet will be the fee-payer and signer.
- `ADMIN_PDA_PUBKEY`: The public key of the admin service the user is depositing funds for.

**Response:**

The gateway returns a JSON object containing the unsigned transaction, encoded as a base64 string.

```json
{
  "unsignedTx": "AVuD9J...base64_encoded_transaction...m8gQ=="
}
```

The client would then decode this transaction, sign it with the user's private key, and send it back to the `submit_transaction` endpoint.

### 2. Listen for User Events

This RPC call opens a persistent stream to listen for events related to a specific `UserProfile` PDA. The server will first send all historical events and then continue to send new events as they happen.

**Request:**

```bash
grpcurl -plaintext \
    -d '{
        "pda": "USER_PROFILE_PDA_PUBKEY"
    }' \
    localhost:9090 w3b2.protocol.gateway.BridgeGatewayService/ListenAsUser
```

- `USER_PROFILE_PDA_PUBKEY`: The public key of the user's profile account.

**Response Stream:**

The server will stream back `EventStreamItem` messages. The first events will be historical (from the catch-up worker), followed by live events.

```json
{
  "userProfileCreated": {
    "authority": "USER_WALLET_PUBKEY",
    "userPda": "USER_PROFILE_PDA_PUBKEY",
    "targetAdminPda": "ADMIN_PDA_PUBKEY",
    // ... other fields
  }
}
{
  "userFundsDeposited": {
    "authority": "USER_WALLET_PUBKEY",
    "userProfilePda": "USER_PROFILE_PDA_PUBKEY",
    "amount": "100000000",
    // ... other fields
  }
}
...
```

The client can maintain this stream to keep its own state synchronized with the blockchain in real-time.