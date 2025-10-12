# gRPC Gateway API Reference

The gRPC gateway provides a ready-to-use service that exposes the functionality of the on-chain program to clients written in any gRPC-compatible language (e.g., Python, TypeScript, Go). It offers a high-level, language-agnostic API that abstracts away the complexities of blockchain interaction.

The full API definition is defined in the `w3b2-solana-gateway` crate and can be referenced from its source code.

## The "Prepare-Then-Submit" Pattern

The gateway is designed around a non-custodial workflow that ensures server-side components **never** handle user private keys. This is achieved through a two-step "prepare-then-submit" pattern:

1.  **Prepare**: The client calls a `prepare_*` method on the gateway (e.g., `PrepareUserDeposit`). The gateway builds the necessary unsigned Solana transaction and returns it to the client as a serialized byte array.
2.  **Sign & Submit**: The client receives the byte array, deserializes it into a transaction, and uses the user's local wallet (e.g., a local keypair, a browser extension) to sign it. The client then sends the now-signed transaction back to the gateway's single `SubmitTransaction` method, which broadcasts it to the Solana network.

This pattern ensures that the user's secret key never leaves their device, which is a critical security practice.

## API Methods

The API provides three main categories of RPC methods.

### 1. Transaction Preparation (`Prepare*`)

This is a suite of unary RPC methods that map one-to-one with the instructions available in the on-chain program. Each method accepts the parameters needed for the specific instruction and returns an `UnsignedTx` message containing the transaction bytes.

#### Example: Preparing a `user_deposit` Transaction

This RPC call asks the gateway to create an unsigned transaction for a user to deposit 0.1 SOL into their profile.

**Request (`grpcurl`):**
```bash
grpcurl -plaintext \
    -d '{
        "authority_pubkey": "USER_WALLET_PUBKEY",
        "admin_profile_pda": "ADMIN_PDA_PUBKEY",
        "amount": 100000000
    }' \
    localhost:50051 w3b2.protocol.gateway.BridgeGatewayService/PrepareUserDeposit
```

**Response:**

The gateway returns a JSON object containing the unsigned transaction, encoded as a base64 string.

```json
{
  "unsignedTx": "AVuD9J...base64_encoded_transaction...m8gQ=="
}
```

#### Complete List of Preparation Methods

**Admin Methods:**
*   `PrepareAdminRegisterProfile`
*   `PrepareAdminCloseProfile`
*   `PrepareAdminSetConfig`
*   `PrepareAdminWithdraw`
*   `PrepareAdminDispatchCommand`
*   `PrepareAdminBanUser`
*   `PrepareAdminUnbanUser`

**User Methods:**
*   `PrepareUserCreateProfile`
*   `PrepareUserUpdateCommKey`
*   `PrepareUserCloseProfile`
*   `PrepareUserDeposit`
*   `PrepareUserWithdraw`
*   `PrepareUserRequestUnban`

**Operational Methods:**
*   `PrepareUserDispatchCommand`
*   `PrepareLogAction`

### 2. Transaction Submission

This is a single unary RPC method for submitting a signed transaction.

*   **`SubmitTransaction(SubmitTransactionRequest) returns (TransactionResponse)`**
    Accepts a message containing the signed transaction bytes. The gateway submits it to the network and returns a message with the resulting transaction signature string.

### 3. Event Streaming

These are server-side streaming methods that allow a client to subscribe to a persistent stream of on-chain events for a specific PDA. The gateway uses the underlying `EventListener` from the `w3b2-solana-connector`, meaning it provides the same "catch-up then live" event delivery guarantees.

*   **`ListenAsUser(ListenRequest) returns (stream EventStreamItem)`**
    Opens a stream for events related to a specific `UserProfile` PDA. The `ListenRequest` contains the public key of the user profile to listen to.

*   **`ListenAsAdmin(ListenRequest) returns (stream EventStreamItem)`**
    Opens a stream for events related to a specific `AdminProfile` PDA. The `ListenRequest` contains the public key of the admin profile to listen to.

*   **`Unsubscribe(UnsubscribeRequest)`**
    Manually closes an active event stream subscription.

#### Example: Listening for User Events

This RPC call opens a persistent stream to listen for events related to a specific `UserProfile` PDA. The server will first send all historical events and then continue to send new events as they happen.

**Request (`grpcurl`):**
```bash
grpcurl -plaintext \
    -d '{
        "pda": "USER_PROFILE_PDA_PUBKEY"
    }' \
    localhost:50051 w3b2.protocol.gateway.BridgeGatewayService/ListenAsUser
```

**Response Stream:**

The server will stream back `EventStreamItem` messages. The first events will be historical (from the catch-up worker), followed by live events.

```json
{
  "userProfileCreated": {
    "authority": "USER_WALLET_PUBKEY",
    "userPda": "USER_PROFILE_PDA_PUBKEY",
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
```