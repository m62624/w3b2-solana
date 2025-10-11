# Introduction

This project provides a toolset for integrating Web2 services with the Solana blockchain, allowing developers to manage users, accept payments, and build hybrid business logic. It is not a protocol, but a smart contract and a set of libraries designed to be deployed and controlled by a service provider.

The system is composed of three main components:

1.  **On-Chain Program (`w3b2-solana-program`)**: A Solana smart contract, built with Anchor, that handles the core financial logic. It manages user profiles, service provider accounts, and the interactions between them. It acts as the single source of truth for all on-chain state.

2.  **Connector Library (`w3b2-solana-connector`)**: A low-level, asynchronous Rust library for building backend services that interact with the on-chain program. It provides helpers for creating unsigned transactions and a robust event listener for synchronizing with on-chain events.

3.  **Gateway Service (`w3b2-solana-gateway`)**: A ready-to-use gRPC service built on top of the connector. It exposes the on-chain program's functionality to clients written in any gRPC-compatible language (e.g., Python, TypeScript, Go).

## Core Principles

The system is built on several core principles that ensure security, transparency, and developer control.

### 1. Non-Custodial Design

The service provider **never** has access to users' private keys or direct control over their wallets. All actions that affect a user's funds or on-chain data must be signed by the user in their own wallet (e.g., Phantom, Solflare). The gateway service prepares unsigned transactions, which are then sent to the client for signing. This non-custodial approach is fundamental to the security model.

### 2. Program-Derived Addresses (PDAs)

The system uses PDAs extensively to create on-chain profiles for both the service provider (`AdminProfile`) and its users (`UserProfile`). This has two key benefits:

- **Program-Controlled State**: It allows the on-chain program to "own" and manage these accounts, such as debiting a user's prepaid balance to pay for a command.
- **Verifiable Identity**: It creates a deterministic and verifiable link between a user's wallet, the service they are using, and their on-chain profile.

### 3. Separation of Concerns

The architecture clearly separates on-chain and off-chain responsibilities:

- **On-Chain**:
    - Manages user and admin accounts (`UserProfile`, `AdminProfile`).
    - Handles all value transfer (deposits, withdrawals, payments).
    - Verifies oracle signatures for paid commands.
    - Emits events as an immutable record of actions.

- **Off-Chain (Your Backend)**:
    - Implements all business logic (e.g., what happens after a user pays for a command).
    - Operates the oracle that signs data (e.g., price, command ID).
    - Listens for on-chain events to trigger backend processes.

This hybrid model provides the security and verifiability of the blockchain for financial transactions, while keeping the business logic flexible and scalable in a traditional off-chain environment.

## Next Steps

- Learn about the detailed component interactions in the **[Architecture](./architecture.md)** section.
- Set up your local development environment with the **[Getting Started](./getting-started.md)** guide.