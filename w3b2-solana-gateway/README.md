# w3b2-solana-gateway

This crate provides a focused, high-performance gRPC service for streaming on-chain events from the `w3b2-solana-program`. It is built on top of the `w3b2-solana-connector` and is designed to be the primary event source for off-chain clients.

## Core Functionality

The gateway's API is defined in the `proto/gateway.proto` file and is focused exclusively on event streaming.

**Event Streaming (`listen_as_user`, `listen_as_admin`)**: These server-side streaming methods allow clients to subscribe to a persistent stream of on-chain events for a specific `UserProfile` or `AdminProfile` PDA. The service guarantees "catch-up then live" delivery, ensuring clients have a complete and up-to-date view of an account's event history.

> **Note on Transactions**: This gateway does **not** handle transaction preparation or submission. Clients are expected to build and send transactions directly to a Solana RPC node using standard libraries like `anchorpy` (Python) or `@coral-xyz/anchor` (TypeScript) along with the program's IDL. This separation of concerns makes the overall architecture more robust and standard.

## Client Integration

To connect to the gateway's event stream, you should use the provided `.proto` files to generate a gRPC client in your programming language of choice.

1.  **Locate the Protobuf Definitions**: The API definitions are located in the `proto/` directory at the root of the repository. The main service is defined in `gateway.proto`.

2.  **Generate Client Code**: Use your language's standard gRPC code generation tools (e.g., `grpc-tools` for Python, `protoc-gen-go-grpc` for Go) to create client stubs from the `.proto` files.

3.  **Implement the Listener**: Use the generated client to call the `ListenAsUser` or `ListenAsAdmin` methods. These are server-streaming RPCs, so your client will receive a stream of `EventStreamItem` messages.

Your client logic should iterate over this stream to process events as they arrive. The `source` field in `EventStreamItem` allows you to distinguish between historical (`CATCHUP`) and real-time (`LIVE`) events.

### Conceptual Client Example (Python)

```python
# Assuming 'gateway_pb2' and 'gateway_pb2_grpc' are generated
import gateway_pb2
import gateway_pb2_grpc
import grpc

channel = grpc.insecure_channel('localhost:50051')
stub = gateway_pb2_grpc.BridgeGatewayServiceStub(channel)

user_pda_to_listen = "..." # The PDA of the user profile

request = gateway_pb2.ListenRequest(pda=user_pda_to_listen)

try:
    for item in stub.ListenAsUser(request):
        print(f"Received event from source: {item.source}")
        # Process item.event based on its type
except grpc.RpcError as e:
    print(f"An error occurred: {e.details()}")
```