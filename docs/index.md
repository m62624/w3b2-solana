# Welcome to the W3B2-Solana Documentation

W3B2-Solana is a toolset for building hybrid services that bridge traditional off-chain infrastructure with the Solana blockchain. It provides an on-chain smart contract, Rust libraries for backend integration, a gRPC gateway for event streaming, and a C-ABI signing library for multi-language support.

The toolset is designed for developers who need to integrate specific on-chain functionality—such as payments, verifiable logging, or state management—into existing applications without migrating their entire stack.

## Core Philosophy: The Developer-Owned Oracle

The central pattern is the **"developer-owned oracle"**. The service provider runs an off-chain oracle that signs business-critical data (e.g., API usage, payment amounts). This signature is consumed by the client, which includes it in a transaction sent to the on-chain program. The program verifies the oracle's signature, ensuring the action is authorized, while the user retains final control via their own signature.

This site provides detailed guides, API references, and architecture diagrams for the entire toolset.

## What Can You Do With This Toolset?

-   **Non-Custodial Paid APIs**: Charge users in SOL for API calls, authorized by your oracle.
-   **Verifiable Audit Trails**: Log critical off-chain actions to the Solana blockchain as an immutable record.
-   **User-Managed Deposits**: Allow users to pre-fund an account for your service, with funds that only they can approve for spending.
-   **On-Chain User Management**: Implement transparent, on-chain banning and moderation systems.

## High-Level System Overview

The system consists of the on-chain program and several off-chain crates that facilitate interaction with it. For a more detailed breakdown, see the [Architecture](architecture/concepts.md) section.