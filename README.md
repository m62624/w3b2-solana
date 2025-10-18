# W3B2-Solana: Bridge Your Web2 Service with Web3 Security

W3B2-Solana is a toolset for developers looking to integrate their existing Web2 services with the security, transparency, and non-custodial nature of the Solana blockchain. It provides the on-chain programs and off-chain libraries to seamlessly blend high-performance, traditional backend infrastructure with the power of Web3.

The core value is enabling **two powerful interaction models** within a single, unified framework:

1.  **Direct On-Chain Transactions**: For simple, low-data interactions like micropayments, voting, or logging critical audit data, your application can interact directly with the on-chain program. This is the classic Web3 model, providing maximum transparency and security for well-defined, atomic operations.

2.  **Secure Off-Chain Handshake for Heavy Traffic**: For high-bandwidth Web2 services (e.g., video streaming, large file transfers, real-time data feeds), using the blockchain for every packet of data is inefficient and costly. This toolset allows you to use the blockchain as a **secure message bus** to negotiate a direct, off-chain connection between your service and the user. The on-chain transaction becomes a verifiable, auditable "handshake" that establishes a secure, private communication channel, while the heavy data lifting happens off-chain.

This hybrid approach allows you to use the blockchain for what it's best at—security, auditability, and asset transfer—while leveraging your existing Web2 infrastructure for performance and scale.

## How the Secure Handshake Works

The on-chain program provides the instruments for this secure negotiation:
-   **`communication_pubkey`**: Both admins and users store a public key on-chain for secure, hybrid encryption.
-   **`dispatch` commands**: The `admin_dispatch_command` and `user_dispatch_command` instructions contain a flexible `payload` field.
-   **The Flow**:
    1.  A party (e.g., the user) initiates a connection by calling a `dispatch` command.
    2.  The `payload` of this command contains a connection configuration, encrypted for the recipient using their on-chain `communication_pubkey`.
    3.  The recipient's backend service, listening for on-chain events via the `w3b2-solana-connector`, receives this encrypted config.
    4.  After decrypting the config, the service can establish a direct, off-chain connection (e.g., a WebSocket, TCP socket) with the user, completely bypassing the blockchain for the actual data transfer.

The `w3b2-solana-program/src/protocols.rs` file provides a reference implementation for a configuration payload, but developers are free to implement any protocol they need.

## Crate Overview

This workspace contains the following crates:

-   `w3b2-solana-program`: The core on-chain Anchor program.
-   `w3b2-solana-connector`: A Rust library for listening to on-chain events.
-   `w3b2-solana-gateway`: An optional gRPC server that streams on-chain events to clients.
-   `w3b2-solana-signer`: A C-ABI compatible library for signing messages from any programming language, useful for building oracles.
-   `w3b2-solana-logger`: A simple logging utility for the Rust services.

> **Note**: For detailed guides, API references, and architecture diagrams, please see the **Full Documentation Site**. Instructions to run it locally are in the [Local Development](#local-development-with-docker) section.

## Local Development with Docker

The recommended development environment is managed via Docker and Docker Compose. See the full documentation for details on getting started.