# gRPC Gateway API Reference

The gRPC gateway provides a focused, high-performance service for streaming on-chain events from the `w3b2-solana-program`. Its sole responsibility is to allow clients to subscribe to event feeds for specific on-chain accounts.

**Note:** This gateway **only** handles event streaming. For creating and submitting transactions, clients should use a standard library for their language, such as `anchorpy` for Python or `@coral-xyz/anchor` for TypeScript, along with the program's IDL.

## API Philosophy: Live vs. History

The gateway provides two distinct types of event streams for both `User` and `Admin` profiles to enable robust state synchronization:

1.  **Live Streams (`stream_*_live_events`)**: Opens a persistent, long-lived connection that forwards events in real-time as they are confirmed on-chain. This is ideal for applications that need immediate updates.
2.  **History Streams (`get_*_event_history`)**: Fetches all historical events for a given PDA from its creation. This is a "one-shot" stream that closes automatically after the last historical event has been delivered.

A typical client would first drain the history stream to build its initial state, and then subscribe to the live stream for ongoing updates.

## API Methods

The full Protobuf definition can be found in `proto/gateway.proto`.

---

### Live Event Streams

#### `StreamUserLiveEvents(ListenRequest) returns (stream EventStreamItem)`
Subscribes to a stream of **live** events for a specific `UserProfile` PDA. The stream remains open until the client disconnects or an `Unsubscribe` request is sent.

#### `StreamAdminLiveEvents(ListenRequest) returns (stream EventStreamItem)`
Subscribes to a stream of **live** events for a specific `AdminProfile` PDA.

---

### Historical Event Streams

#### `GetUserEventHistory(ListenRequest) returns (stream EventStreamItem)`
Fetches all historical events for a specific `UserProfile` PDA. The stream closes automatically once the full history has been delivered.

#### `GetAdminEventHistory(ListenRequest) returns (stream EventStreamItem)`
Fetches all historical events for a specific `AdminProfile` PDA. The stream closes automatically once the full history has been delivered.

---

### Utility

#### `Unsubscribe(UnsubscribeRequest) returns (google.protobuf.Empty)`
Manually closes an active **live** event stream subscription. This is not needed for history streams.

## Example Client Workflow (Conceptual Python)

```python
# Assuming 'gateway_pb2' and 'gateway_pb2_grpc' are generated from .proto files
import gateway_pb2
import gateway_pb2_grpc
import grpc

# --- Setup ---
channel = grpc.insecure_channel('localhost:50051')
stub = gateway_pb2_grpc.BridgeGatewayServiceStub(channel)
pda_to_listen = "Hq1q2y3..." # Base-58 encoded PDA of the UserProfile

# --- 1. Hydrate state with historical events ---
print(f"Fetching event history for {pda_to_listen}...")
history_request = gateway_pb2.ListenRequest(pda=pda_to_listen)
try:
    for item in stub.GetUserEventHistory(history_request):
        # Process historical event to build local state
        print(f"  [History] Event received: {item.event}")
    print("History sync complete.")
except grpc.RpcError as e:
    print(f"An RPC error occurred during history fetch: {e.details()}")


# --- 2. Subscribe to live events for real-time updates ---
print(f"Subscribing to live events for {pda_to_listen}...")
live_request = gateway_pb2.ListenRequest(pda=pda_to_listen)
try:
    for item in stub.StreamUserLiveEvents(live_request):
        # Process live event
        print(f"  [Live] Event received: {item.event}")
except grpc.RpcError as e:
    # This will be hit if the stream is closed by the server or a network error occurs
    print(f"Live stream ended: {e.details()}")

# To close the live stream from the client:
# unsubscribe_request = gateway_pb2.UnsubscribeRequest(pda=pda_to_listen)
# stub.Unsubscribe(unsubscribe_request)
```