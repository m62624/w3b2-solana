# API Reference: Gateway Service

The `w3b2-solana-gateway` is a gRPC service that provides a language-agnostic API for interacting with the W3B2 on-chain program. It handles transaction building and event handling.

## Transaction Workflow: Prepare, Sign, Submit

The gateway uses a non-custodial workflow for all state-changing operations. Private keys are never sent over the network. Instead, interactions follow a three-step process:

1.  **Prepare**: The client calls a `Prepare...` RPC method (e.g., `PrepareUserDeposit`). The gateway constructs the Solana transaction and returns it to the client, unsigned.
2.  **Sign**: The client signs the transaction locally using the appropriate private key (e.g., in a browser wallet or on a backend server).
3.  **Submit**: The client sends the signed transaction to the `SubmitTransaction` RPC endpoint, which broadcasts it to the Solana network.

## Service: `w3b2.protocol.gateway.BridgeGatewayService`

The following methods are available on this service.

### Event Streaming

RPCs for subscribing to on-chain events.

---

**`ListenAsUser(ListenRequest) returns (stream EventStreamItem)`**

Subscribes to all events for a specific `UserProfile` PDA. This server-streaming RPC first sends all historical events (`catch-up`) and then streams live events as they occur.

-   **Request**: `ListenRequest { pda: string }`
-   **Stream Item**: `EventStreamItem { event: Oneof<AllEvents> }`

---

**`ListenAsAdmin(ListenRequest) returns (stream EventStreamItem)`**

Subscribes to all events for a specific `AdminProfile` PDA. This includes events from all users who have created a profile with that admin.

-   **Request**: `ListenRequest { pda: string }`
-   **Stream Item**: `EventStreamItem { event: Oneof<AllEvents> }`

---

**`Unsubscribe(UnsubscribeRequest) returns (google.protobuf.Empty)`**

Closes an active event stream using its `subscription_id`.

-   **Request**: `UnsubscribeRequest { subscription_id: string }`

---

### Transaction Preparation

RPCs that prepare unsigned transactions. All methods in this section return an `UnsignedTransactionResponse { transaction: bytes }`.

#### **Admin Methods**

-   `PrepareAdminRegisterProfile`: Prepares a transaction to create an `AdminProfile`.
-   `PrepareAdminSetConfig`: Prepares a transaction to configure an admin's oracle settings.
-   `PrepareAdminWithdraw`: Prepares a transaction to withdraw earned funds.
-   `PrepareAdminCloseProfile`: Prepares a transaction to close an `AdminProfile`.
-   `PrepareAdminDispatchCommand`: Prepares a transaction for an admin to send a command to a user.

#### **User Methods**

-   `PrepareUserCreateProfile`: Prepares a transaction to create a `UserProfile`.
-   `PrepareUserUpdateCommKey`: Prepares a transaction to update a user's communication key.
-   `PrepareUserDeposit`: Prepares a transaction to deposit funds into a `UserProfile`.
-   `PrepareUserWithdraw`: Prepares a transaction to withdraw funds from a `UserProfile`.
-   `PrepareUserCloseProfile`: Prepares a transaction to close a `UserProfile`.
-   `PrepareUserDispatchCommand`: Prepares a transaction for a user to execute a service command. The developer-owned oracle signature is passed in this method's request.

#### **Operational Methods**

-   `PrepareLogAction`: Prepares a transaction to log a generic action on-chain for auditing.

---

### Transaction Submission

**`SubmitTransaction(SubmitTransactionRequest) returns (TransactionResponse)`**

Submits a signed transaction to the Solana network.

-   **Request**: `SubmitTransactionRequest { signed_transaction: bytes }`
-   **Response**: `TransactionResponse { signature: string }` (The transaction signature).