# Python Example

This example provides a complete guide for interacting with the W3B2 protocol using Python, featuring a FastAPI oracle service and a client using the `grpcio` library.

## Prerequisites

-   Python 3.8+
-   `grpcio` and `grpcio-tools` for gRPC communication.
-   `pysolana` and `solders` for Solana-specific types.
-   `pynacl` for cryptographic signing.
-   `fastapi` and `uvicorn` for the oracle web service.

First, you must generate the Python gRPC code from the `.proto` files in the `proto/` directory.

```bash
python -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/gateway.proto ../proto/types.proto
```

## 1. The Oracle Service (Your FastAPI Backend)

This is the backend service you are responsible for building and securing. It holds the oracle's private key and provides signed quotes to authorize user transactions.

```python
import time
import base64
from fastapi import FastAPI
from pydantic import BaseModel
from solders.keypair import Keypair
from solders.pubkey import Pubkey
import nacl.signing

app = FastAPI()

# In a real application, load this from a secure vault (e.g., HashiCorp Vault)
# or an environment variable. DO NOT hardcode private keys.
ORACLE_KEYPAIR = Keypair() # Example: generate a new one for demonstration
ORACLE_SIGNING_KEY = nacl.signing.SigningKey(ORACLE_KEYPAIR.secret_key()[:32])

class QuoteRequest(BaseModel):
    command_id: int

class QuoteResponse(BaseModel):
    command_id: int
    price: int
    timestamp: int
    signature: str  # base64 encoded
    oracle_pubkey: str # base58 encoded

@app.post("/api/quote", response_model=QuoteResponse)
def get_quote(request: QuoteRequest):
    """Creates and signs a payment quote for a client."""
    price = 50000  # Business logic: look up the price for the command
    timestamp = int(time.time())

    # Construct the message exactly as the on-chain program expects (little-endian bytes)
    command_id_bytes = request.command_id.to_bytes(2, 'little')
    price_bytes = price.to_bytes(8, 'little')
    timestamp_bytes = timestamp.to_bytes(8, 'little')

    message = command_id_bytes + price_bytes + timestamp_bytes

    # Sign the message with the oracle's private key
    signed = ORACLE_SIGNING_KEY.sign(message)
    signature = signed.signature

    return QuoteResponse(
        command_id=request.command_id,
        price=price,
        timestamp=timestamp,
        signature=base64.b64encode(signature).decode('ascii'),
        oracle_pubkey=str(ORACLE_KEYPAIR.pubkey()),
    )

# To run: uvicorn your_file_name:app --reload
```

## 2. The Client (Interacting with W3B2 Gateway)

This script shows how a Python application can get a quote, prepare a transaction via the W3B2 Gateway, sign it, and submit it.

```python
import grpc
import requests
import base64
from solders.keypair import Keypair
from solders.pubkey import Pubkey
from solders.transaction import Transaction
from solders.hash import Hash
from pysolana.rpc.api import Client

# Import generated gRPC files
import gateway_pb2
import gateway_pb2_grpc

# This would be the user's keypair
USER_KEYPAIR = Keypair()

def main():
    # --- Step 1: Get quote from your oracle ---
    quote_res = requests.post(
        "http://127.0.0.1:8000/api/quote",
        json={"command_id": 42}
    )
    quote = quote_res.json()

    # --- Step 2: Prepare transaction via W3B2 Gateway ---
    with grpc.insecure_channel('localhost:50051') as channel:
        stub = gateway_pb2_grpc.BridgeGatewayServiceStub(channel)
        user_profile_pda = Pubkey.new_unique() # User's profile PDA for this service

        prepare_req = gateway_pb2.PrepareUserDispatchCommandRequest(
            user_profile_pda=str(user_profile_pda),
            user_authority=str(USER_KEYPAIR.pubkey()),
            oracle_authority=quote['oracle_pubkey'],
            command_id=quote['command_id'],
            price=quote['price'],
            timestamp=quote['timestamp'],
            signature=base64.b64decode(quote['signature']),
        )

        unsigned_tx_res = stub.PrepareUserDispatchCommand(prepare_req)

        # --- Step 3: Sign the transaction locally ---
        tx = Transaction.from_bytes(unsigned_tx_res.transaction)

        # The client must fetch and set the blockhash
        solana_client = Client("http://127.0.0.1:8899")
        recent_blockhash = solana_client.get_latest_blockhash().value.blockhash

        tx.sign([USER_KEYPAIR], recent_blockhash)

        # --- Step 4: Submit the signed transaction ---
        submit_req = gateway_pb2.SubmitTransactionRequest(
            signed_transaction=tx.to_bytes()
        )

        submit_res = stub.SubmitTransaction(submit_req)

        print(f"Transaction successful! Signature: {submit_res.signature}")


if __name__ == '__main__':
    main()
```

## 3. Listening for Events (Python Backend)

Your backend service can listen for events to confirm that a user's action was successful.

```python
def listen_for_events(admin_profile_pda: str):
    """Listens for events for a given Admin PDA."""
    with grpc.insecure_channel('localhost:50051') as channel:
        stub = gateway_pb2_grpc.BridgeGatewayServiceStub(channel)

        listen_req = gateway_pb2.ListenRequest(pda=admin_profile_pda)

        print(f"Listening for events on {admin_profile_pda}...")

        try:
            for item in stub.ListenAsAdmin(listen_req):
                event_type = item.event.WhichOneof('event')
                if event_type == 'user_command_dispatched':
                    event_data = item.event.user_command_dispatched
                    print(f"[SUCCESS] User {event_data.user_authority} dispatched command {event_data.command_id}")
                    # TODO: Trigger business logic here
                else:
                    print(f"Received event of type: {event_type}")

        except grpc.RpcError as e:
            print(f"An error occurred: {e.status()}: {e.details()}")

```