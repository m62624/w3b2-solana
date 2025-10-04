# W3B2: A Non-Custodial Web3-to-Web2 Bridge Protocol

W3B2 is a protocol and set of tools for building decentralized, non-custodial services on the Solana blockchain. It enables secure, auditable interactions between end-users (clients) and service providers by using the blockchain as a message bus and state machine, rather than for direct data storage.

## Core Concept

The fundamental idea behind W3B2 is to leverage the blockchain for what it does best: providing a secure, decentralized, and auditable log of state transitions. Instead of building entire applications on-chain, W3B2 uses the blockchain to manage user-service relationships, handle payments, and bootstrap secure off-chain communication channels.

**Key Principles:**

1.  **Per-Service Profiles**: A user's identity and funds are not monolithic. For each service they use, a unique `UserProfile` Program-Derived Address (PDA) is created. This isolates funds and interactions, meaning a user's relationship with "Service A" is cryptographically and financially separate from their relationship with "Service B".
2.  **Non-Custodial Authority**: The user's main wallet (`authority`) is only used to sign for transactions that create or manage their profiles. It never gives up control of its main funds. Daily operations and payments are handled by the program-controlled `UserProfile` PDAs.
3.  **Blockchain as a Message Bus**: The primary role of the on-chain program is to emit events. A backend service (like the `w3b2-gateway`) listens *only* to the blockchain for these events. When a user creates a profile or sends a command, the service discovers this by observing the corresponding on-chain event.
4.  **Bootstrapping Secure Off-Chain Communication**: The protocol facilitates, but does not implement, secure off-chain communication. By emitting events containing public keys (e.g., `communication_pubkey`), a user and a service can discover each other's keys from the immutable blockchain log. They can then use these keys to establish a secure, end-to-end encrypted off-chain connection (e.g., via WebSocket, WebRTC) for transmitting large payloads or sensitive data. The `payload` field in `dispatch_command` instructions can be used to exchange initial connection configurations.

## Workspace Components

This monorepo contains three main components:

### 1. `w3b2-program`

The on-chain Anchor program that implements the core logic of the W3B2 protocol.

*   **State Management**: Defines the `AdminProfile` (for services) and `UserProfile` (for users) account structures.
*   **Instruction Logic**: Contains all on-chain instructions, such as creating profiles, depositing/withdrawing funds, and dispatching commands.
*   **Event Emission**: Every state change emits a corresponding event, creating an auditable trail of all protocol activity.
*   **Off-Chain Protocol Agnostic**: The `dispatch_command` instructions include an opaque `payload: Vec<u8>` field. This allows off-chain applications to define their own data formats (e.g., Protobuf, JSON) and pass them through the blockchain without the on-chain program needing to understand their contents.

### 2. `w3b2-connector`

A core Rust library for building backend services that interact with the `w3b2-program`. It is the ideal choice if you need to build a custom backend and the `w3b2-gateway` does not fit your needs.

*   **`TransactionBuilder`**: A non-custodial helper for constructing unsigned transactions for all program instructions.
*   **`EventManager`**: A robust service for synchronizing and dispatching on-chain events in real-time. It handles the complexity of fetching both historical (`catchup`) and live events, ensuring no event is missed.
*   **`UserListener` / `AdminListener`**: High-level event listeners that subscribe to a specific on-chain PDA and provide separate, ordered streams for historical and real-time events.

### 3. `w3b2-gateway`

A ready-to-use, production-grade gRPC service built on top of `w3b2-connector`. It exposes the full functionality of the protocol to clients written in any language (Python, JavaScript, Go, etc.).

*   **Prepare-Then-Submit Flow**: Exposes RPCs for preparing all transactions (e.g., `PrepareUserDeposit`).
*   **Event Streaming**: Provides server-streaming RPCs (`ListenAsUser`, `ListenAsAdmin`) for receiving on-chain events.
*   **Manual Unsubscription**: Allows clients to explicitly unsubscribe from event streams.

### `proto/` Directory

This directory contains the Protobuf definitions that serve as the definitive API contract for the entire ecosystem.

*   **`types.proto`**: Defines all on-chain event structures and shared message types.
*   **`gateway.proto`**: Defines the gRPC `BridgeGatewayService`, including all its RPC methods and request/response messages.

These files can be used with `protoc` or `tonic-build` to generate client and server code in any supported language.

## How It Works: An Example Flow

This example illustrates the interaction between a **User B** and a **Service A**.

1.  **Service Registration (On-Chain)**
    *   Service A's backend uses `w3b2-gateway`'s `PrepareAdminRegisterProfile` RPC to create an unsigned transaction.
    *   The backend signs this transaction with its `authority` key and submits it.
    *   The `w3b2-program` creates an `AdminProfile` PDA for Service A and emits an `AdminProfileRegistered` event.

2.  **User Onboarding (On-Chain)**
    *   User B's client application discovers Service A (e.g., through a Web2 directory).
    *   The client calls `PrepareUserCreateProfile`, providing its `authority` pubkey and Service A's `AdminProfile` PDA.
    *   The gateway returns an unsigned transaction. User B signs it with their wallet.
    *   The signed transaction is submitted. The on-chain program creates a unique `UserProfile` PDA for the pair (User B, Service A) and emits a `UserProfileCreated` event.

3.  **Service Discovery of User (Off-Chain)**
    *   Service A's backend, which is subscribed to its own `AdminProfile` PDA via `ListenAsAdmin`, receives the `UserProfileCreated` event.
    *   Service A now knows that User B is a new customer.

4.  **Bootstrapping Secure Communication (Off-Chain)**
    *   The `UserProfileCreated` event contains User B's `communication_pubkey`.
    *   Service A can now use this key to initiate a secure, end-to-end encrypted off-chain connection with User B. For example, it could send an initial encrypted message via a `dispatch_command` `payload`, containing connection details for a WebSocket server.

5.  **User Interaction (On-Chain Payment, Off-Chain Action)**
    *   User B wants to use a paid feature of Service A.
    *   The client calls `PrepareUserDispatchCommand`. The gateway prepares a transaction that, when executed, will transfer lamports from User B's `UserProfile` PDA to Service A's `AdminProfile` PDA.
    *   User B signs and submits. The program executes the payment and emits a `UserCommandDispatched` event.
    *   Service A's backend sees this event, verifies the payment, and performs the requested off-chain action (e.g., processing data, granting access).

## Getting Started

### Prerequisites

*   Rust toolchain (`rustup`)
*   Solana CLI
*   Anchor CLI
*   Node.js and npm (for Protobuf generation scripts, if needed)

### Build the Project

```bash
# From the root of the workspace

# Build the on-chain program
anchor build --project w3b2-program

# Build the connector and gateway
cargo build
```

### Run the Gateway

The gateway requires a configuration file. An example is provided in `w3b2-gateway/config.example.toml`.

```bash
# Make sure you have a local Solana validator running
solana-test-validator

# Deploy the program (only needs to be done once)
anchor deploy --provider.cluster localhost --project w3b2-program

# Run the gateway
cargo run --bin w3b2-gateway -- --config ./w3b2-gateway/config.example.toml
```

### Interacting with the gRPC Service

You can use any gRPC client tool (like `grpcurl`) or generate a client from the `.proto` files to interact with the running gateway.

**Example using `grpcurl`:**

```bash
# List all services
grpcurl -plaintext localhost:50051 list

# Subscribe to events for a specific PDA (this will hang as it streams events)
grpcurl -plaintext -d '{"pda": "YOUR_PDA_ADDRESS_HERE"}' \
  localhost:50051 w3b2.protocol.gateway.BridgeGatewayService/ListenAsUser
```