# W3B2: A Non-Custodial Web3-to-Web2 Bridge Protocol

W3B2 is a protocol and toolset for building non-custodial services on the Solana blockchain that interact with traditional Web2 applications. It provides a bridge that allows Web2 backends (e.g., SaaS platforms, API services, game servers) to leverage blockchain features like security, transparency, and cryptocurrency payments without migrating their core logic on-chain.

## Core Problems Addressed

W3B2 is designed to solve key challenges at the intersection of Web2 and Web3 for both developers and end-users.

### 1. The Custody Problem
*   **Problem**: In traditional online services, users must trust the company with their funds on a centralized balance. If the company fails, the funds are at risk.
*   **Solution**: W3B2 is **non-custodial**. User funds are held in a program-controlled `UserProfile` PDA, which only the user's wallet can deposit to or withdraw from. The service can only debit funds for services rendered according to on-chain rules, never directly control them.

### 2. The Vendor Lock-in Problem
*   **Problem**: A user's balance and relationship with one service are often tied to a single, monolithic account.
*   **Solution**: W3B2 creates a distinct `UserProfile` PDA for each service a user interacts with. This isolates funds and relationships, meaning a user's engagement with "Service A" is cryptographically and financially separate from "Service B". Closing one profile returns all associated funds without affecting others.

### 3. The Auditability Problem
*   **Problem**: In Web2, operations like payments and actions occur on private servers, making independent verification impossible.
*   **Solution**: The blockchain is used as an **immutable audit log**. Every significant state change—profile creation, deposits, command execution (`user_dispatch_command`), or service notifications (`admin_dispatch_command`)—emits a verifiable on-chain event. This creates a transparent and auditable history of all interactions.

### 4. The Integration Complexity Problem
*   **Problem**: Integrating blockchain technology into an existing Web2 business is complex, requiring expertise in smart contracts, key management, and transaction processing.
*   **Solution**: W3B2 provides a suite of turnkey components:
    *   **`w3b2-solana-program`**: A pre-built, audited on-chain program.
    *   **`w3b2-solana-connector`**: A backend library that handles the complexities of tracking historical and real-time on-chain events.
    *   **`w3b2-solana-gateway`**: A ready-to-deploy gRPC server that exposes the protocol's functionality via a simple API, consumable by any language.

### 5. The On-Chain Bloat Problem
*   **Problem**: Storing and processing large amounts of data on-chain is expensive and inefficient.
*   **Solution**: The protocol uses a **hybrid model**. The blockchain is used only for critical state management (ownership, balances) and as a message bus. All heavy business logic and data remain off-chain. The `payload` field in transactions allows applications to pass opaque data through the blockchain, which simply records it in an event without interpretation, using the chain as a verifiable message courier.

## Workspace Components

This monorepo contains the following components:

### 1. `w3b2-solana-program`

The core on-chain Anchor program.

*   **State Management**: Defines the `AdminProfile` (for services) and `UserProfile` (for users) account structures.
*   **Instruction Logic**: Contains all on-chain instructions, such as creating profiles, depositing/withdrawing funds, and dispatching commands.
*   **Event Emission**: Every state change emits a corresponding event, creating an auditable trail of all protocol activity.
*   **Off-Chain Protocol Agnostic**: The `dispatch_command` instructions include an opaque `payload: Vec<u8>` field. This allows off-chain applications to define their own data formats (e.g., Protobuf, JSON) and pass them through the blockchain without the on-chain program needing to understand their contents.

### 2. `w3b2-solana-connector`

A Rust library for building backend services that interact with the on-chain program. It is the ideal choice for building a custom backend if the gateway does not fit your needs.

*   **`TransactionBuilder`**: A non-custodial helper for constructing unsigned transactions for all program instructions.
*   **`EventManager`**: A robust service for synchronizing and dispatching on-chain events in real-time. It handles the complexity of fetching both historical (`catchup`) and live events, ensuring no event is missed.
*   **`UserListener` / `AdminListener`**: High-level event listeners that subscribe to a specific on-chain PDA and provide separate, ordered streams for historical and real-time events.

### 3. `w3b2-solana-gateway`

A ready-to-use, production-grade gRPC service built on top of `w3b2-solana-connector`. It exposes the full functionality of the protocol to clients written in any language (Python, JavaScript, Go, etc.).

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
    *   Service A's backend uses `w3b2-solana-gateway`'s `PrepareAdminRegisterProfile` RPC to create an unsigned transaction.
    *   The backend signs this transaction with its `authority` key and submits it.
    *   The `w3b2-solana-program` creates an `AdminProfile` PDA for Service A and emits an `AdminProfileRegistered` event.

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
anchor build --project w3b2-solana-program

# Build the connector and gateway
cargo build
```

### Run the Gateway

The gateway requires a configuration file. An example is provided in `w3b2-solana-gateway/config.example.toml`.

```bash
# Make sure you have a local Solana validator running
solana-test-validator

# Deploy the program (only needs to be done once)
anchor deploy --provider.cluster localhost --project w3b2-solana-program

# Run the gateway
cargo run --bin w3b2-solana-gateway -- --config ./w3b2-solana-gateway/config.example.toml
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