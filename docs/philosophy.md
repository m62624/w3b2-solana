# Our Philosophy: Non-Custodial & Developer-Owned Oracles

The W3B2 protocol is built on a foundation of two core principles: **user sovereignty** over funds and **developer sovereignty** over business logic. This document explains the "why" behind our architecture, focusing on our non-custodial approach and the critical role of the developer-owned oracle.

## The Problem with Centralized Trust

In traditional Web2 services, the relationship between a user and a service is one of trust. The user must trust the service provider to:

1.  **Hold their funds honestly**: Balances are just numbers in a private database.
2.  **Execute actions correctly**: The service's backend logic is a black box.
3.  **Maintain security**: A single breach can compromise user data and assets.

W3B2 is designed to replace this model of trust with one of **on-chain verification**.

## 1. True Non-Custodial Fund Management

At its core, W3B2 ensures that users are always in control of their funds.

- **User-Controlled Deposits**: Funds are held in a `UserProfile` Program Derived Address (PDA) on the Solana blockchain. Only the user's wallet has the authority to deposit funds into this account or withdraw the remaining balance.
- **Program-Controlled Debits**: The service provider (the "Admin") cannot arbitrarily access these funds. The on-chain program only permits the transfer of funds from a user's profile to the admin's profile when the user explicitly signs a `user_dispatch_command` transaction for a service with a predefined price.
- **Transparency**: Every single financial transaction is a public, verifiable event on the blockchain.

This model eliminates the risk of fund mismanagement or loss due to a service provider's failure or malice.

## 2. The Developer-Owned Oracle Pattern

A key design decision in W3B2 is that we do **not** manage your business logic or your secrets. Instead, we provide the tools for you to securely connect your existing Web2 infrastructure to the blockchain. This is achieved through the **developer-owned oracle pattern**.

**The developer, who is the administrator of their own service, must implement a separate, isolated oracle service. This is their responsibility.**

Our system provides the on-chain program that *verifies* signatures and the gateway/connector that *packages* these signatures into transactions. The *creation* of the signature, however, is a core part of the service's business logic and must remain under the developer's control.

### The Full End-to-End Flow

Here is the complete lifecycle of a paid user command, illustrating the separation of concerns:

**Step 1: The Service Admin Creates Their API**

The developer builds their API server (e.g., `MyAwesomeServiceAPI` on Node.js, Go, or Rust). Only this server has access to the **oracle's private key**. This key should be stored securely (e.g., AWS KMS, HashiCorp Vault, or environment variables for development).

**Step 2: The End-User Requests a Service**

A user clicks a button in the web interface (e.g., "Generate AI Image"). This action sends a standard Web2 API request to the developer's server.

```
POST https://myawesomeservice.com/api/generate-image
```

**Step 3: The Developer's Oracle Service Creates a "Quote"**

The developer's API server receives the request and prepares the necessary data for the on-chain transaction.

1.  It identifies the `command_id` for the requested service.
2.  It looks up the `price` for that command from its own configuration or database.
3.  It generates a current `timestamp` to prevent replay attacks.
4.  It constructs the message to be signed: an array of `[command_id, price, timestamp]`.
5.  It **signs this message** using its securely stored **oracle private key**.
6.  It returns a JSON object to the user's client.

```json
{
  "command_id": 123,
  "price": 100000,
  "timestamp": 1678886400,
  "signature": "base64_encoded_signature...", // The generated signature
  "oracle_pubkey": "Pubkey_of_the_oracle..."   // The public key for on-chain verification
}
```

**Step 4: The User's Client Calls the W3B2 Gateway**

The user's client (e.g., the browser) receives this JSON payload. It now has all the information required for the on-chain transaction.

1.  It connects to our `BridgeGatewayService` via gRPC.
2.  It calls the `PrepareUserDispatchCommand` method.
3.  It passes all the fields from the JSON object, including the `signature` and `oracle_pubkey`, as arguments.

**Step 5: The W3B2 Gateway and Connector Do Their Job**

Our infrastructure takes the data and assembles the final Solana transaction.

1.  The gateway and connector **do not create a signature**. They only use the one provided by the developer's oracle.
2.  They construct a transaction that includes two key instructions:
    - An `Ed25519` instruction to verify the provided `signature` against the `oracle_pubkey`.
    - The `user_dispatch_command` instruction to execute the payment and emit the on-chain event.
3.  This unsigned transaction is returned to the user's client for final signing.

### Why This Matters

This "bring your own oracle" model is a deliberate design choice that ensures security and proper separation of roles:

-   **Security**: We never have access to your service's private keys. The responsibility and control remain entirely with you, minimizing the attack surface.
-   **Flexibility**: Your business logic (pricing, command definitions, access control) stays off-chain where it belongs. You can change it anytime without needing to modify an on-chain program.
-   **Clarity**: The roles are clear. You handle your business logic. We handle the on-chain verification and transaction plumbing.

The absence of signature creation methods in our connector and gateway is not an omission; it is a core security feature of the W3B2 protocol.