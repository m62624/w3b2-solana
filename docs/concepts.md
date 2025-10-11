# Core Concepts

## Developer-Owned Oracle

The fundamental design pattern in this toolset is the "Developer-Owned Oracle."

In a traditional smart contract, all logic, including payment verification, must happen on-chain. This is expensive, slow, and difficult to update.

Our approach separates concerns:

*   **On-Chain (The Program):** Responsible only for verifying a cryptographic signature and executing state changes. It is simple, secure, and rarely needs updates.
*   **Off-Chain (The Gateway):** Responsible for all complex business logic:
    *   User authentication
    *   Payment processing (Stripe, etc.)
    *   Dynamic pricing
    *   Database lookups
    *   Rate limiting

The on-chain program has a hardcoded "oracle authority" public key. It will only accept commands that are signed by the corresponding private key, which is securely held by your Gateway service. This makes the Gateway a trusted "oracle" for the smart contract.

## Transaction Structure

To execute a command, a transaction must contain two instructions in a specific order:

1.  **`Ed25519Program.createInstructionWithPublicKey`:** This is a standard Solana instruction. Its purpose is to load the oracle's signature and the message it signed into the Solana runtime. The program can then access this data.
2.  **`user_dispatch_command`:** This is our custom program instruction. It contains the actual command logic. It reads the data from the previous instruction, re-builds the expected message, and verifies the signature against the known oracle authority.

This two-instruction pattern is a secure and efficient way to pass external data to a Solana program.