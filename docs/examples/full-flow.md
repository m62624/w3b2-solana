# Example: The Full Interaction Flow

This document illustrates the end-to-end process of a user interacting with a paid service built on the W3B2 protocol. This flow involves four parties:

1.  **The User's Client**: The frontend application (e.g., a web app) that the end-user interacts with.
2.  **The Developer's Oracle Service**: The backend API created by the service provider. This is **your** server.
3.  **The W3B2 Gateway**: Our gRPC service that bridges to the on-chain program.
4.  **The Solana Blockchain**: The ultimate source of truth.

Let's imagine a service called "AI Art Generator" that charges a small fee to generate an image.

---

### Step 1: User Initiates Action

The user is on the "AI Art Generator" website and clicks the "Generate Image" button.

-   **Action**: The user's client (a TypeScript web app) prepares to make an API call. It knows the `command_id` for this action is `42`.

---

### Step 2: Client Requests a "Quote" from the Developer's Oracle

The user's client does **not** call the W3B2 Gateway directly yet. First, it needs a signature from the service's own backend to authorize the payment.

-   **Action**: The client sends a `POST` request to the developer's API.

    ```http
    POST https://api.ai-art-generator.com/v1/generate-quote
    Content-Type: application/json

    {
      "command_id": 42
    }
    ```

---

### Step 3: Developer's Oracle Service Creates and Signs the Quote

The "AI Art Generator" backend (the oracle) receives this request. Its job is to define the terms of the transaction and cryptographically sign them.

-   **Action**: The oracle server (e.g., a Python/FastAPI app) performs the following steps:
    1.  Validates the `command_id`.
    2.  Looks up the current `price` for command `42`. Let's say it's `50000` lamports.
    3.  Gets the current Unix `timestamp`.
    4.  Constructs the message to be signed: `[command_id, price, timestamp]`.
    5.  **Signs the message** using its securely stored **oracle private key**.
    6.  Returns the signed quote to the user's client.

-   **Response from Oracle to Client**:

    ```json
    {
      "command_id": 42,
      "price": 50000,
      "timestamp": 1680000000,
      "signature": "a_base64_encoded_ed25519_signature_...",
      "oracle_pubkey": "oracle_public_key_base58_encoded..."
    }
    ```

---

### Step 4: Client Prepares the On-Chain Transaction via W3B2 Gateway

The user's client now has everything it needs to build the real on-chain transaction.

-   **Action**: The client uses a gRPC client to call the W3B2 Gateway.

    -   **Service**: `w3b2.protocol.gateway.BridgeGatewayService`
    -   **Method**: `PrepareUserDispatchCommand`
    -   **Request Payload**:
        -   `user_profile_pda`: The user's profile PDA for this service.
        -   `user_authority`: The user's wallet public key.
        -   `command_id`: `42`
        -   `price`: `50000`
        -   `timestamp`: `1680000000`
        -   `signature`: `"a_base64_encoded_ed25519_signature_..."`
        -   `oracle_authority`: `"oracle_public_key_base58_encoded..."`
        -   `payload`: An optional byte array for extra data.

-   **Response from W3B2 Gateway**: The gateway returns an `UnsignedTransactionResponse` containing the raw bytes of an unsigned Solana transaction.

---

### Step 5: User Signs the Transaction

The transaction is now on the client side. The gateway and the oracle have done their parts. The final authority rests with the user.

-   **Action**: The client uses a wallet adapter (e.g., Phantom, Solflare) to present the transaction to the user for signing. The user approves it.
-   **Result**: The client now holds a **signed** transaction.

---

### Step 6: Client Submits the Signed Transaction

-   **Action**: The client calls the final W3B2 Gateway method.

    -   **Service**: `w3b2.protocol.gateway.BridgeGatewayService`
    -   **Method**: `SubmitTransaction`
    -   **Request Payload**:
        -   `signed_transaction`: The raw bytes of the transaction signed by the user.

-   **Response from W3B2 Gateway**: `TransactionResponse { signature: "solana_tx_signature..." }`

---

### Step 7: Developer's Backend is Notified of Success

The developer's backend needs to know that the user has successfully paid and executed the command so it can perform the actual work (generating the AI art).

-   **Action**: The developer's service uses the `w3b2-solana-connector` or a gRPC client to listen for events on its `AdminProfile` PDA.
-   **Event Received**: The backend receives a `UserCommandDispatched` event containing all the details of the transaction.
-   **Result**: The backend verifies the event details, credits the user's account in its internal database, and kicks off the AI art generation job.

This completes the full, non-custodial, and verifiable interaction loop. The user paid, the service was notified, and the entire financial transaction is recorded permanently on the blockchain.