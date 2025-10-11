# Getting Started

This project provides a full Docker Compose pipeline for building, testing, and deploying all components. This is the recommended way to get started with local development.

## Prerequisites

*   Docker & Docker Compose

## Quickstart

1.  **Generate a Program Keypair**

    The build process requires a keypair for the on-chain program. If you don't have one, use the `builder` service to generate it. This command will create the keypair file at `./keys/program-keypair.json` on your host machine.

    ```bash
    docker compose run --rm builder solana-keygen new --outfile /keys/program-keypair.json
    ```

2.  **Run the Full Stack**

    Use the `full` profile to build, deploy, and run all services, including the documentation site.

    ```bash
    docker compose --profile full up --build
    ```

    This command will:
    -   Build the on-chain program and the gRPC gateway.
    -   Start a local Solana validator.
    -   Deploy the program to the local validator.
    -   Run the gRPC gateway, making it available for client connections.
    -   Serve this documentation site on `http://localhost:8000`.
    -   Run an example Python client that continuously interacts with the system.