# Welcome to the W3B2 Documentation

W3B2 is a toolset for building services on the Solana blockchain that need to interact with traditional Web2 applications. It provides an on-chain smart contract for managing state and financial logic, a Rust library for backend integration, and a gRPC gateway for broader API access.

It is designed for developers who want to leverage the security and transparency of Solana for specific tasks without moving their entire application on-chain.

This site provides detailed guides, API references, and architecture diagrams for the entire toolset. Use the navigation to explore the different components.

## What Can You Do With This Toolset?

This project provides the foundation for a variety of hybrid Web2/Web3 use cases:

-   **Non-Custodial Paid APIs**: Charge users in SOL for API calls. Your backend oracle signs the price, and the user approves the payment with their wallet. The on-chain program guarantees the fund transfer.
-   **Verifiable Audit Trails**: Log critical off-chain actions (e.g., "User A deleted file B") to the Solana blockchain as an immutable, permanent record.
-   **User-Managed Deposits**: Allow users to pre-fund an account for your service. All funds remain under the user's control and can only be spent with their explicit, signed approval for a specific action.
-   **On-Chain User Management**: Implement on-chain banning/moderation systems that are transparent and enforced by the smart contract.

## High-Level System Overview

The system is composed of four main parts: the **Client**, the **gRPC Gateway**, the **Solana Connector**, and the **On-Chain Program**. The backend components (Gateway, Connector, and your custom Oracle) are managed by the service provider, while the client interacts with the user's wallet.

For a more detailed breakdown, see the [Architecture Diagram](architecture/architecture-diagram.md) page.