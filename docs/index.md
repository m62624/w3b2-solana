# W3B2 Toolset Documentation

Welcome to the developer documentation for the W3B2 Toolset. This system enables on-chain actions that are authorized and paid for via a trusted, off-chain service.

The core pattern is the **Developer-Owned Oracle**. The on-chain program does not handle payments directly. Instead, it trusts a specific off-chain authority (the "oracle," which is your gateway service) to verify payments and other business logic. The program simply verifies a cryptographic signature from this oracle to execute a command.

## Core Components

*   **Solana Program (Rust):** The on-chain smart contract that holds state and executes signed commands.
*   **Gateway (Python):** The off-chain service that handles business logic (e.g., user accounts, payments) and acts as the trusted oracle by signing messages.
*   **Client (TypeScript):** A library to simplify interaction between a user's wallet, the Gateway, and the Solana Program.

## Get Started

To understand how these pieces fit together, follow our step-by-step guide:

*   **[Tutorial: End-to-End Workflow](./end-to-end-workflow.md)**