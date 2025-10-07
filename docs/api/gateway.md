# API Reference: Gateway Service

The `w3b2-solana-gateway` is a production-ready gRPC service that provides a simple, language-agnostic API for interacting with the W3B2 on-chain program. It abstracts away the complexities of transaction building and event handling, allowing you to focus on your application's logic.

## Core Flow: Prepare, Sign, Submit

The gateway enforces a non-custodial workflow for all state-changing operations. Private keys are never sent over the network. Instead, interactions follow a three-step process:

1.  **Prepare**: Your client calls a `Prepare...` RPC method on the gateway (e.g., `PrepareUserDeposit`). The gateway constructs the necessary Solana transaction and returns it to your client, **unsigned**.
2.  **Sign**: Your client signs the transaction locally using the appropriate private key (e.g., in the user's browser wallet or on your backend server).
3.  **Submit**: Your client sends the signed transaction to the universal `SubmitTransaction` RPC endpoint, which broadcasts it to the Solana network.

## Service: `BridgeGatewayService`

The following methods are available on the `w3b2.protocol.gateway.BridgeGatewayService`.

### Event Streaming

These RPCs allow you to subscribe to on-chain events in real-time.

---

**`ListenAsUser(ListenRequest) returns (stream EventStreamItem)`**

Subscribes to all events related to a specific `UserProfile` PDA. This is a server-streaming RPC that will first send all historical events (`catch-up`) and then stream live events as they occur.

-   **Request**: `ListenRequest { pda: string }`
-   **Stream Item**: `EventStreamItem { event: Oneof<AllEvents> }`

---

**`ListenAsAdmin(ListenRequest) returns (stream EventStreamItem)`**

Subscribes to all events related to a specific `AdminProfile` PDA. This includes events from all users who have created a profile with that admin.

-   **Request**: `ListenRequest { pda: string }`
-   **Stream Item**: `EventStreamItem { event: Oneof<AllEvents> }`

---

**`Unsubscribe(UnsubscribeRequest) returns (google.protobuf.Empty)`**

Manually closes an active event stream. Each stream response includes a `subscription_id` that can be used here.

-   **Request**: `UnsubscribeRequest { subscription_id: string }`

---

### Transaction Preparation (Step 1)

These RPCs prepare unsigned transactions. All of them return an `UnsignedTransactionResponse { transaction: bytes }`.

#### **Admin Methods**

-   `PrepareAdminRegisterProfile(PrepareAdminRegisterProfileRequest)`: Creates a new service provider (`AdminProfile`).
-   `PrepareAdminSetConfig(PrepareAdminSetConfigRequest)`: Configures an admin's oracle settings.
-   `PrepareAdminWithdraw(PrepareAdminWithdrawRequest)`: Withdraws earned funds.
-   `PrepareAdminCloseProfile(PrepareAdminCloseProfileRequest)`: Closes the admin profile.
-   `PrepareAdminDispatchCommand(PrepareAdminDispatchCommandRequest)`: Sends a notification/command to a user.

#### **User Methods**

-   `PrepareUserCreateProfile(PrepareUserCreateProfileRequest)`: Creates a `UserProfile` linked to a service.
-   `PrepareUserUpdateCommKey(PrepareUserUpdateCommKeyRequest)`: Updates a user's communication key.
-   `PrepareUserDeposit(PrepareUserDepositRequest)`: Deposits funds into a `UserProfile`.
-   `PrepareUserWithdraw(PrepareUserWithdrawRequest)`: Withdraws funds from a `UserProfile`.
-   `PrepareUserCloseProfile(PrepareUserCloseProfileRequest)`: Closes a `UserProfile`.
-   `PrepareUserDispatchCommand(PrepareUserDispatchCommandRequest)`: The core method for a user to pay for and execute a service command. This is where the **developer-owned oracle signature** is passed in.

#### **Operational Methods**

-   `PrepareLogAction(PrepareLogActionRequest)`: Prepares a transaction to log a generic action on-chain for auditing.

---

### Transaction Submission (Step 2)

This is the final step for all on-chain actions.

**`SubmitTransaction(SubmitTransactionRequest) returns (TransactionResponse)`**

Submits a signed transaction to the Solana network and waits for it to be processed.

-   **Request**: `SubmitTransactionRequest { signed_transaction: bytes }`
-   **Response**: `TransactionResponse { signature: string }` (The transaction signature).