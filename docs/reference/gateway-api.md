# gRPC Gateway API Reference

The gRPC gateway provides a ready-to-use service that exposes the functionality of the on-chain program to clients written in any gRPC-compatible language (e.g., Python, TypeScript, Go). It offers a high-level, language-agnostic API that abstracts away the complexities of blockchain interaction.

> **Note:** The underlying `.proto` file for this service was not found in the repository. This documentation is based on the information provided in the `w3b2-solana-gateway/README.md` file and describes the API conceptually.

## The "Prepare-Then-Submit" Pattern

The gateway is designed around a non-custodial workflow that ensures server-side components **never** handle user private keys. This is achieved through a two-step "prepare-then-submit" pattern:

1.  **Prepare**: The client calls a `prepare_*` method on the gateway (e.g., `PrepareUserDeposit`). The gateway builds the necessary unsigned Solana transaction and returns it to the client as a serialized byte array.
2.  **Sign & Submit**: The client receives the byte array, deserializes it into a transaction, and uses the user's local wallet (e.g., Phantom, a Ledger, a local keypair) to sign it. The client then sends the now-signed transaction back to the gateway's single `SubmitTransaction` method, which broadcasts it to the Solana network.

This pattern ensures that the user's secret key never leaves their device, which is a critical security practice.

## API Methods

The API provides three main categories of RPC methods.

### 1. Transaction Preparation (`Prepare*`)

This is a suite of unary RPC methods that map one-to-one with the instructions available in the on-chain program. Each method accepts the parameters needed for the specific instruction and returns an `UnsignedTx` message containing the transaction bytes.

**Conceptual List of Methods:**
- `PrepareAdminRegisterProfile`
- `PrepareAdminCloseProfile`
- `PrepareAdminSetConfig`
- `PrepareAdminWithdraw`
- `PrepareAdminDispatchCommand`
- `PrepareAdminBanUser`
- `PrepareAdminUnbanUser`
- `PrepareUserCreateProfile`
- `PrepareUserUpdateCommKey`
- `PrepareUserCloseProfile`
- `PrepareUserDeposit`
- `PrepareUserWithdraw`
- `PrepareUserRequestUnban`
- `PrepareUserDispatchCommand`
- `PrepareLogAction`

### 2. Transaction Submission

This is a single unary RPC method for submitting a signed transaction.

-   **`SubmitTransaction(SignedTx) returns (TxSignature)`**
    Accepts a message containing the signed transaction bytes. The gateway submits it to the network and returns a message with the resulting transaction signature string.

### 3. Event Streaming

These are server-side streaming methods that allow a client to subscribe to a persistent stream of on-chain events for a specific PDA. The gateway uses the underlying `EventListener` from the `w3b2-solana-connector`, meaning it provides the same "catch-up then live" event delivery guarantees.

-   **`ListenAsUser(ListenRequest) returns (stream EventStreamItem)`**
    Opens a stream for events related to a specific `UserProfile` PDA. The `ListenRequest` should contain the public key of the user profile to listen to.
-   **`ListenAsAdmin(ListenRequest) returns (stream EventStreamItem)`**
    Opens a stream for events related to a specific `AdminProfile` PDA. The `ListenRequest` should contain the public key of the admin profile to listen to.