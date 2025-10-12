# Getting Started

This project provides a full Docker Compose pipeline for building, testing, and deploying all components. This is the recommended way to get started with local development.

> **Note:** The repository does **not** include any private keys. You must provide your own Solana program keypair. All builds and deployments will use your Program ID, and all components will be configured to work with it.

### Prerequisites

*   Docker & Docker Compose

### Quickstart

1.  **Generate a Program Keypair**: The on-chain program requires a keypair to be deployed. The `builder` service in the Docker Compose setup is equipped with the Solana CLI tools needed to generate one.

    Run the following command from the root of the repository:
    ```bash
    # This creates ./keys/program-keypair.json on your host machine.
    # The directory will be created if it doesn't exist.
    docker compose run --rm builder solana-keygen new --outfile /keys/program-keypair.json
    ```

2.  **Run the Full Stack**: Use the `full` profile to build, deploy, and run all services simultaneously.

    ```bash
    docker compose --profile full up --build
    ```

    This single command orchestrates the entire development environment:
    -   It builds the on-chain program (`.so`) and its IDL (`.json`).
    -   It builds the gRPC gateway.
    -   It starts a local Solana validator instance with the necessary accounts and configuration.
    -   It deploys the newly built program to the local validator.
    -   It runs the gRPC gateway, connecting it to the validator.
    -   It serves this documentation site locally.

### What's Running?

Once the `full` profile is up, the following services will be available:

-   **gRPC Gateway**: Listening on `0.0.0.0:50051`. Your client applications will connect to this port.
-   **Documentation Site**: Served at `http://localhost:8000`. You are likely reading this page from that local instance right now.
-   **Solana Validator**: The local RPC endpoint is available at `http://localhost:8899`.