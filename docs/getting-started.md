# Getting Started

This guide walks through setting up the development environment manually. For a simpler, containerized setup, see the Docker Compose instructions in the main `README.md` file.

## Prerequisites

Ensure the following tools are installed on your system:

-   **Rust & Cargo**: [Install Rust](https://www.rust-lang.org/tools/install)
-   **Solana Tool Suite**: [Install Solana](https://docs.solana.com/cli/install-solana-cli-tools)
-   **Anchor**: [Install Anchor](https://www.anchor-lang.com/docs/installation)
-   **Git**: For cloning the repository.

## 1. Clone the Repository

```bash
git clone <your-repository-url>
cd w3b2
```

## 2. Build the Project Components

The project is structured as a Cargo workspace.

### Build the On-Chain Program

The on-chain program must be built first.

```bash
anchor build --project w3b2-solana-program
```

This command compiles the program and creates a `target/deploy/w3b2_solana_program.so` file, which is the binary to be deployed to the blockchain.

### Build the Off-Chain Components

Build the connector and gateway libraries.

```bash
cargo build
```

## 3. Run the Local Environment

To test the full system, you need a local Solana validator and the Gateway service.

### Start the Solana Test Validator

Open a new terminal and run the local validator. This command will also airdrop lamports to your default wallet, which is needed to deploy the program.

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

The gateway requires a configuration file to specify the Solana cluster connection and the on-chain program ID.

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

The gateway will start and listen for gRPC requests on `localhost:50051` by default.

## 4. Interact with the Gateway

With the gateway running, you can interact with the system using any gRPC-compatible client. A simple way to test this is with `grpcurl`.

### List the gRPC Services

```bash
grpcurl -plaintext localhost:50051 list
```

This should list the `w3b2.protocol.gateway.BridgeGatewayService`.

### Subscribe to Events

To monitor on-chain events, you can open a new terminal and subscribe to events for a specific account. This command will maintain an open connection and stream events as they are emitted.

```bash
# Replace YOUR_PDA_ADDRESS_HERE with a real AdminProfile or UserProfile PDA
grpcurl -plaintext -d '{"pda": "YOUR_PDA_ADDRESS_HERE"}' \
  localhost:50051 w3b2.protocol.gateway.BridgeGatewayService/ListenAsUser
```

## Docker Profiles & Program ID

If you use the Docker Compose environment, you can run any part of the stack using profiles:

- `builder`: Build all artifacts (smart contract and gateway) with your Program ID.
- `solana-validator`: Local Solana test validator.
- `deploy`: Runs the build, validator, and then the `deployer` service, which deploys the smart contract to the validator (the actual service is named `deployer`).
- `gateway`: Runs the gRPC gateway (after deployer is finished).
- `docs`: Serves documentation.
- `full`: Runs the entire stack.

**How it works:**  
- The `builder` service builds all artifacts and exits.
- The `solana-validator` service starts the local validator.
- The `deployer` service (used in the `deploy` profile) deploys the program after builder and validator are ready.
- The `gateway` service starts after deployer is finished and uses your Program ID.
- The `full` profile runs all of the above together.

**Important:**  
The repository does **not** include any private keys. You must provide your own Solana program keypair (see `PROGRAM_KEYPAIR_PATH` in `.env`). The Program ID is automatically extracted from your keypair and injected into all builds and runtime configs.  
The `program-id` field in `config.docker.toml` is a placeholder and is overridden at runtime via the `W3B2_CONNECTOR__PROGRAM_ID` environment variable.

This ensures all components (listener, gateway, smart contract) are built and run with your Program ID.