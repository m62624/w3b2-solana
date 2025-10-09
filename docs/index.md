# Introduction

This project provides a set of tools for integrating Solana-based logic into existing applications. It is designed for developers who want to use the Solana blockchain for specific functions, such as managing user deposits or logging auditable events, without moving their entire application on-chain.

The system is composed of three main components:

1.  **On-Chain Program (`w3b2-solana-program`)**: A Solana smart contract, built with Anchor, that handles the core logic. It manages user profiles, service provider accounts, and the financial interactions between them. It acts as the authoritative state machine.

2.  **Connector Library (`w3b2-solana-connector`)**: A Rust library that provides a high-level API for interacting with the on-chain program. It is designed for building custom backend services and includes helpers for creating transactions and listening for on-chain events.

3.  **Gateway Service (`w3b2-solana-gateway`)**: A pre-built gRPC service that exposes the functionality of the on-chain program and connector library. It allows applications written in any gRPC-compatible language (like Python, TypeScript, or Go) to interact with the system.

## Core Design

The architecture is based on a hybrid on/off-chain model. Your application's business logic remains off-chain, while the on-chain program serves as a secure and transparent ledger for value and state.

A key pattern used is the "developer-owned oracle," where a service provider is responsible for signing payment-related data. The on-chain program verifies this signature before executing a transaction, ensuring that the service provider maintains control over their business logic. For more details, see the **[Architecture](./architecture.md)** documentation.

## Get Started

To begin, follow the **[Getting Started](./getting-started.md)** guide to set up the local development environment.