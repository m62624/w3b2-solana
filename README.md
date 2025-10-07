# W3B2: A Non-Custodial Web3-to-Web2 Bridge Protocol

W3B2 is a protocol and toolset for building non-custodial services on the Solana blockchain that interact with traditional Web2 applications. It provides a bridge that allows Web2 backends to leverage blockchain features like security, transparency, and cryptocurrency payments without migrating their core logic on-chain.

## Full Documentation

This repository contains the source code for the W3B2 protocol. For comprehensive documentation, including philosophy, getting started guides, API references, and code examples, please see our full documentation site.

**--> [Go to the full W3B2 Documentation](./docs/index.md) <--**
*Note: The documentation is built with MkDocs. To view it locally, install mkdocs and run `mkdocs serve` from the root of this repository.*

## Core Components

*   **`w3b2-solana-program`**: The core on-chain Anchor program that manages user/admin profiles and financial logic.
*   **`w3b2-solana-connector`**: A Rust library for building backend services that interact with the on-chain program. It handles transaction building and event synchronization.
*   **`w3b2-solana-gateway`**: A ready-to-use gRPC service that exposes the protocol's functionality to clients written in any language.
*   **`proto/`**: Protobuf definitions that serve as the definitive API contract for the entire ecosystem.

## Quick Start

1.  **Install Prerequisites**: Rust, Solana CLI, Anchor CLI.
2.  **Build Program**: `anchor build --project w3b2-solana-program`
3.  **Build Off-chain**: `cargo build`
4.  **Run Validator**: `solana-test-validator`
5.  **Deploy Program**: `anchor deploy --provider.cluster localhost --project w3b2-solana-program`
6.  **Run Gateway**: `cargo run --bin w3b2-solana-gateway -- --config <your-config-file>`

For detailed instructions, please see the [Getting Started](./docs/getting-started.md) section of our documentation.