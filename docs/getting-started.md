# Getting Started

This guide will walk you through setting up the W3B2 development environment using Docker Compose. This is the **highly recommended** approach as it guarantees a consistent and reproducible environment for all components, from the Solana validator to the gRPC gateway, with simple commands.

## Prerequisites

Before you begin, ensure you have the following tools installed on your system:

-   **Docker**: [Install Docker](https://docs.docker.com/get-docker/)
-   **Docker Compose**: Install Docker Compose (usually included with Docker Desktop).
-   **Git**: For cloning the repository.

## 1. Clone the Repository

First, clone the W3B2 monorepo.

```bash
git clone <your-repository-url>
cd w3b2
```

## 2. Build the Project Components

The project is structured as a Cargo workspace, so you can build all the Rust components from the root directory.

### Build the On-Chain Program

The on-chain program must be built first.

```bash
anchor build --project w3b2-solana-program
```

This command compiles the program and creates a `target/deploy/w3b2_solana_program.so` file, which is the binary that will be deployed to the blockchain.

### Build the Off-Chain Components

Build the Connector and Gateway libraries.

```bash
cargo build
```

## 3. Run the Local Environment

To test the full system, you need a local Solana validator and the W3B2 Gateway service.

### Start the Solana Test Validator

Open a new terminal and run the local validator. This command will also airdrop lamports to your default wallet, which you'll need to deploy the program.

```bash
solana-test-validator
```

Keep this validator running in the background.

### Deploy the On-Chain Program

In another terminal, deploy the compiled program to your local validator.

```bash
anchor deploy --provider.cluster localhost --project w3b2-solana-program
```

Make a note of the **Program ID** that is output by this command. You will need it to configure the gateway.

### Configure and Run the Gateway

The gateway needs a configuration file to know which Solana cluster to connect to and which on-chain program to use.

1.  **Copy the example configuration:**
    ```bash
    cp w3b2-solana-gateway/config.example.toml config.dev.toml
    ```

2.  **Edit `config.dev.toml`**:
    -   Set `program_id` to the Program ID you noted after deploying.
    -   Ensure the `rpc_url` and `ws_url` point to your local validator (the defaults are usually correct).

3.  **Run the gateway:**
    ```bash
    cargo run --bin w3b2-solana-gateway -- --config ./config.dev.toml
    ```

The gateway will start and listen for gRPC requests on `localhost:50051` (by default).

## 4. Interact with the Gateway

With the gateway running, you can now interact with the W3B2 protocol using any gRPC-compatible client.

A simple way to test this is with `grpcurl`.

### List the gRPC Services

```bash
grpcurl -plaintext localhost:50051 list
```

This should list the `w3b2.protocol.gateway.BridgeGatewayService`.

### Subscribe to Events

To see the system in action, you can open a new terminal and subscribe to events for a specific account. This command will hang as it waits for new events to be emitted on-chain.

```bash
# Replace YOUR_PDA_ADDRESS_HERE with a real AdminProfile or UserProfile PDA
grpcurl -plaintext -d '{"pda": "YOUR_PDA_ADDRESS_HERE"}' \
  localhost:50051 w3b2.protocol.gateway.BridgeGatewayService/ListenAsUser
```

## Next Steps

You now have a fully functional local instance of the W3B2 protocol.

-   Explore the **API Reference** to understand the available gRPC methods.
-   Check out the **Examples** section to see how to build a client in Rust, TypeScript, or Python to interact with your service.