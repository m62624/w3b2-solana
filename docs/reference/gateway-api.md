# gRPC Gateway API Reference

The gRPC gateway provides a focused, high-performance service for streaming on-chain events from the W3B2 program. Its sole responsibility is to allow clients to subscribe to real-time event feeds for specific on-chain accounts.

## API Methods

The API is focused exclusively on event streaming. For creating and submitting transactions, clients should use a standard library for their language, such as `anchorpy` for Python or `@coral-xyz/anchor` for TypeScript, along with the program's IDL. This is the recommended and most robust approach for interacting with the on-chain program.

### Event Streaming

These are server-side streaming methods that allow a client to subscribe to a persistent stream of on-chain events for a specific PDA. The gateway uses the underlying `EventListener` from the `w3b2-solana-connector`, meaning it provides the same "catch-up then live" event delivery guarantees.

*   **`ListenAsUser(ListenRequest) returns (stream EventStreamItem)`**
    Opens a stream for events related to a specific `UserProfile` PDA.

*   **`ListenAsAdmin(ListenRequest) returns (stream EventStreamItem)`**
    Opens a stream for events related to a specific `AdminProfile` PDA.

*   **`Unsubscribe(UnsubscribeRequest)`**
    Manually closes an active event stream subscription.

#### Client Integration Example

To connect to the gateway's event stream, you should use the provided `.proto` files to generate a gRPC client in your programming language of choice.

1.  **Locate the Protobuf Definitions**: The API definitions are located in the `proto/` directory at the root of the repository. The main service is defined in `gateway.proto`.

2.  **Generate Client Code**: Use your language's standard gRPC code generation tools (e.g., `grpc-tools` for Python, `protoc-gen-go-grpc` for Go) to create client stubs from the `.proto` files.

3.  **Implement the Listener**: Use the generated client to call the `ListenAsUser` or `ListenAsAdmin` methods. These are server-streaming RPCs, so your client will receive a stream of `EventStreamItem` messages.

Your client logic should iterate over this stream to process events as they arrive. The `source` field in `EventStreamItem` allows you to distinguish between historical (`CATCHUP`) and real-time (`LIVE`) events.

#### Conceptual Client Example (Python)

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

**Response Stream:**

The server will stream back `EventStreamItem` messages. The first events will be historical (from the catch-up worker), followed by live events.

```json
{
  "userProfileCreated": {
    "authority": "USER_WALLET_PUBKEY",
    "userPda": "USER_PROFILE_PDA_PUBKEY",
    // ... other fields
  }
}
{
  "userFundsDeposited": {
    "authority": "USER_WALLET_PUBKEY",
    "userProfilePda": "USER_PROFILE_PDA_PUBKEY",
    "amount": "100000000",
    // ... other fields
  }
}
```