# Getting Started

This guide provides a step-by-step walkthrough for setting up a complete local development environment using Docker Compose. This is the recommended approach as it handles all dependencies, networking, and deployment automatically.

## Prerequisites

- **Docker**: [Install Docker](httpshttps://docs.docker.com/get-docker/)
- **Docker Compose**: Included with Docker Desktop. For Linux, you may need to [install it separately](https://docs.docker.com/compose/install/).
- **`grpcurl`** (Optional but recommended): A command-line tool for interacting with gRPC services. [Installation instructions](https://github.com/fullstorydev/grpcurl/blob/master/README.md#installation).

## 1. Generate a Program Keypair

The on-chain program requires a Solana keypair to determine its public key (Program ID). The deployment scripts expect this keypair to exist at a specific path.

First, create a directory to store your keys:
```bash
mkdir keys
```

Next, use the `solana-keygen` tool (which is included in the `builder` Docker image) to generate a new keypair.

```bash
docker compose run --rm builder solana-keygen new --outfile /keys/program-keypair.json
```
- `docker compose run --rm builder`: This command runs the `builder` service defined in `docker-compose.yml`. The `--rm` flag ensures the container is removed after the command completes.
- `solana-keygen ...`: This is the command executed *inside* the container.
- `--outfile /keys/program-keypair.json`: This tells `solana-keygen` where to save the file *inside the container's filesystem*. Because `docker-compose.yml` mounts the host's `./keys` directory to `/keys` in the container, the generated file will appear at `./keys/program-keypair.json` on your host machine.

**Important**: Your `./keys/` directory is included in `.gitignore`. **Do not** commit your keypair to version control.

## 2. Build, Deploy, and Run the Full Stack

With the keypair in place, you can now bring up the entire local development stack using the `full` Docker Compose profile.

```bash
docker compose --profile full up --build
```

This single command performs the following steps in the correct order:
1.  **`builder`**:
    -   Reads your `program-keypair.json` to get the Program ID.
    -   Injects this Program ID into the on-chain program's source code.
    -   Compiles the on-chain program (`.so` artifact and IDL).
    -   Builds the gRPC gateway binary.
2.  **`solana-validator`**: Starts a local Solana test validator with a persistent ledger stored in `./solana-ledger`.
3.  **`deployer`**: Waits for the `builder` and `validator` to be ready, then deploys the compiled on-chain program to the local validator.
4.  **`gateway`**: Starts the gRPC gateway, configured to connect to the local validator and use your Program ID.
5.  **`docs`**: Starts a local web server to serve this documentation site.

You will see logs from all services in your terminal.

## 3. Verify the Setup

Once all services are running, you can verify that the gateway is operational using `grpcurl`.

#### List gRPC Services

In a new terminal, list the services exposed by the gateway:

```bash
grpcurl -plaintext localhost:9090 list
```

You should see `w3b2.protocol.gateway.BridgeGatewayService` in the output.

#### Prepare a Transaction

You can also test the `prepare` methods. Since no `AdminProfile` exists yet, you can't create a real transaction, but you can still see the gateway respond. This command attempts to prepare a registration transaction.

```bash
grpcurl -plaintext \
    -d '{
        "authority_pubkey": "5x2g37tGztWvYJkdeYc2n1f2g4v3p5q6r7s8t9u0v1w2",
        "communication_pubkey": "5x2g37tGztWvYJkdeYc2n1f2g4v3p5q6r7s8t9u0v1w2"
    }' \
    localhost:9090 w3b2.protocol.gateway.BridgeGatewayService/PrepareAdminRegisterProfile
```

You should receive a JSON response containing a base64-encoded `unsignedTx`, confirming that the gateway is running and connected to the underlying components.

```json
{
  "unsignedTx": "AVuD9J...base64_encoded_transaction...m8gQ=="
}
```

Your local development environment is now fully operational. You can proceed to the **[Examples](./examples/full-flow.md)** section to see how to interact with the system programmatically.