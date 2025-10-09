# W3B2: A Toolset for Solana-Based Services

W3B2 is a toolset for building services on the Solana blockchain that need to interact with traditional Web2 applications. It provides an on-chain smart contract for managing state and financial logic, a Rust library for backend integration, and a gRPC gateway for broader API access.

## Core Components

*   **`w3b2-solana-program`**: The core on-chain Anchor program that manages user and service provider (admin) profiles and handles financial logic.
*   **`w3b2-solana-connector`**: A Rust library for building backend services that interact with the on-chain program. It provides helpers for transaction building and event synchronization.
*   **`w3b2-solana-gateway`**: A ready-to-use gRPC service that exposes the on-chain program's functionality to clients written in any language.
*   **`proto/`**: Protobuf definitions that define the API contract for the gateway and its clients.

## Docker-Based Development Environment

This project provides a full Docker Compose pipeline for building, testing, and deploying all components. The stack is managed via `docker compose` using multiple profiles, allowing you to run only the services you need or the entire stack.

> **Note:** The repository does **not** include any private keys. You must provide your own Solana program keypair (see below). All builds and deployments will use your Program ID, and all components (including the gateway and smart contract) will be built with this ID.

### Prerequisites

*   Docker
*   Docker Compose

### Key Concepts

*   **Profiles**: Services are grouped into profiles (`builder`, `solana-validator`, `deployer`, `gateway`, `docs`, `full`) so you can run only the parts of the stack you need.
*   **Program Keypair**: The build and deploy process requires a Solana keypair for the on-chain program. The scripts will generate one for you if it's not found at the path specified by the `PROGRAM_KEYPAIR_PATH` environment variable (default: `./keys/program-keypair.json`). You should create and manage this file yourself; do **not** commit it to version control.
*   **Program ID Propagation**: During Docker builds, the Program ID is extracted from your keypair and injected into all build artifacts and configuration. The smart contract and gateway are always built and run with your Program ID.
*   **Shared Artifacts**: The `artifacts/` and `target/` directories are mounted into containers so build artifacts (like `.so` and `.json` files) are accessible on your host.


### Usage Examples

**1. Build Artifacts Only**

This command runs only the `builder` service. It compiles the Anchor program and gateway, then places the final artifacts (`w3b2_solana_program.so` and `w3b2_solana_program.json`) into the `./artifacts` directory on your host.

```bash
docker compose --profile builder up --build --force-recreate
```

**2. Run the Validator**

This will start a local Solana test validator with a persistent ledger.

```bash
docker compose --profile validator up --build
```

**3. Build, Deploy, and Run the Gateway**

To build all artifacts, start the validator, deploy the program, and run the gateway:

```bash
# Optional: Define the keypair path
export PROGRAM_KEYPAIR_PATH=./keys/program-keypair.json

# Run the deploy profile (builder, validator, deployer)
docker compose --profile deploy up --build
```

Then, to run the gateway (after deployer has finished):

```bash
docker compose --profile gateway up --build
```

Or, to run the full stack (all services):

```bash
docker compose --profile full up --build
```

**4. View the Documentation**

This command starts a local web server to serve the project documentation.

```bash
docker compose --profile docs up --build
```
Once started, you can access the documentation at **http://localhost:8000**.

## Docker Profiles

The Docker Compose setup supports these profiles:

- **`builder`**: Compiles the Anchor program and gateway with your Program ID, then exits.
- **`solana-validator`**: Runs a local Solana test validator with a persistent ledger.
- **`deploy`**: Runs the build, validator, and then the `deployer` service, which deploys the on-chain program to the validator using your keypair and Program ID (the actual service is named `deployer`).
- **`gateway`**: Runs the gRPC gateway, connecting to the validator and deployed program (runs after deployer).
- **`docs`**: Serves the MkDocs documentation.
- **`full`**: Runs the entire stack (validator, builder, deployer, gateway, docs).

All components are built and run with your Program ID. You must provide your own keypair file (see above). No private keys are included in the repository.

### Service Overview

- **`builder`**: Builds all artifacts and exits.
- **`solana-validator`**: Local Solana test validator.
- **`deployer`**: Deploys the program after builder and validator are ready (used in the `deploy` profile).
- **`gateway`**: Runs the gRPC gateway after deployer is finished.
- **`docs`**: Serves documentation.
- **`full`**: Runs all of the above together.