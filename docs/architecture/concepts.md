# Core Concepts

The W3B2 toolset is built on a set of fundamental principles designed to provide a secure, flexible, and non-custodial bridge between Web2 services and the Solana blockchain. Understanding these concepts is key to effectively integrating and using the system.

## Asynchronous, Event-Driven Architecture

The entire system is designed around an asynchronous, event-driven model. The on-chain program's primary role is to validate state transitions, enforce financial rules, and emit verifiable events. It does not directly "call" or "trigger" off-chain systems.

Instead, backend services (like the `w3b2-solana-connector`) listen for these on-chain events (e.g., `UserCommandDispatched`, `UserUnbanRequested`) and react to them. This decoupled architecture provides several advantages:
- **Resilience**: If a backend service is temporarily down, it can catch up on missed events once it comes back online.
- **Scalability**: Multiple, independent services can listen to the same stream of on-chain events and perform different tasks in parallel.
- **Verifiability**: The on-chain program serves as the single source of truth. Backend systems synchronize their state based on the immutable log of events on the Solana ledger.

## Admin & User Profiles (PDAs)

The system revolves around two main Program-Derived Addresses (PDAs):

-   **`AdminProfile`**: Represents the service provider (the "Admin"). This on-chain account holds configuration like the oracle's public key, the treasury for collected fees, and settings like the fee for unban requests. It is controlled by the admin's authority wallet.
-   **`UserProfile`**: Represents an end-user's relationship with a specific service. It holds the user's pre-paid deposit balance and is verifiably linked to a single `AdminProfile`. Crucially, it is controlled by the user's authority wallet.

This structure creates a clear separation of ownership and control on-chain.

## Non-Custodial Payments

A core principle of W3B2 is that users never lose custody of their funds.
1.  **Deposits**: Users deposit SOL into their *own* `UserProfile` PDA. These funds are still effectively owned by the user, but are now controlled by the rules of the on-chain program.
2.  **Payments**: When a user wants to pay for a service, they sign a transaction that authorizes the program to transfer a specific amount from their `UserProfile` PDA to the service's `AdminProfile` PDA.

The service provider can **never** unilaterally withdraw funds from a user's profile. Every payment requires the user's explicit, cryptographic signature for a specific, oracle-verified command.

## Off-Chain Oracle for Dynamic Pricing

To maintain business logic flexibility, the on-chain program does not contain any hardcoded prices. Instead, it uses an off-chain oracle pattern.

1.  **The Oracle**: The service provider runs a backend service (the "Oracle") which has its own keypair. The public key of this oracle is stored in the `AdminProfile`.
2.  **Signing**: When a user requests to perform a paid action, the backend constructs a message containing the `command_id`, the `price`, and a current `timestamp`. The Oracle signs this message with its private key.
3.  **Verification**: The client sends a transaction to Solana that includes two instructions:
    a. An `ed25519_instruction` that verifies the oracle's signature against the message.
    b. The `user_dispatch_command` instruction for the W3B2 program.
4.  **Execution**: The W3B2 program first checks that the preceding instruction in the transaction is a valid signature verification from the authorized oracle. It then verifies that the `price`, `command_id`, and `timestamp` in the `user_dispatch_command` instruction match what the oracle signed.

This pattern keeps the pricing logic off-chain and easy to change, while the on-chain program focuses on what it does best: securely verifying the outcome and transferring value.

## The "Request for Review" Unban Model

The unban process is a prime example of the toolset's design philosophy. It is an **asynchronous process that requires admin intervention**, not an automatic, on-chain function.

-   **Why?**: A ban is a disciplinary tool. If a user could instantly unban themselves by paying a fee, it would just be a "tax on bad behavior" and lose its power as a deterrent. The service provider must have the final say.
-   **The Process**:
    1.  The admin sets an optional `unban_fee` in their `AdminProfile`.
    2.  A banned user calls the `user_request_unban` instruction. If a fee is set, it is paid from their deposit.
    3.  The program emits a `UserUnbanRequested` event. It does **not** automatically unban the user.
    4.  The admin's backend listens for this event and can alert the admin (e.g., in a dashboard) that a user has paid to have their case reviewed.
    5.  The admin makes an off-chain decision and, if they choose to, calls the `admin_unban_user` instruction to lift the ban.

The `unban_fee` is not a payment for an unban; it's a **fee for the admin's time to review the appeal**. The smart contract's job is to verifiably record the request and the payment, not to make the business decision.

---

## Secure Handshake for High-Traffic Off-Chain Services

While the blockchain is excellent for simple, atomic transactions, it is not suitable for high-throughput data transfer (e.g., video streaming, large file sharing, real-time game data). For these use cases, W3B2-Solana provides the tools to use the blockchain as a **secure message bus** to negotiate a direct, off-chain communication channel.

This pattern allows you to leverage your existing high-performance Web2 infrastructure while using the blockchain for what it excels at: authentication, authorization, and auditing.

### The Mechanism

The core of this pattern lies in using the `dispatch` instructions (`user_dispatch_command` and `admin_dispatch_command`) not just for simple commands, but as a secure envelope for establishing an off-chain connection.

1.  **On-Chain Keys for Off-Chain Encryption**: Both `AdminProfile` and `UserProfile` accounts have a `communication_pubkey` field. This key is stored on-chain and is publicly visible, making it the perfect public key for use in a hybrid encryption scheme (like ECIES).

2.  **The Encrypted Payload**: A party wishing to initiate a direct connection (e.g., a user wants to download a large file from the service) constructs a configuration message. This message might contain details like a one-time access token, an IP address, a port, or a dedicated WebSocket URL. This entire configuration is then encrypted using the recipient's `communication_pubkey`.

3.  **The Dispatch "Handshake"**: The encrypted configuration is placed into the `payload` field of a `dispatch` command and sent as an on-chain transaction.

4.  **Off-Chain Event Listening**: The recipient's backend service, using `w3b2-solana-connector`, is constantly listening for `...CommandDispatched` events. When it sees the handshake transaction, it receives the encrypted payload.

5.  **Decryption and Connection**: The backend service uses its corresponding private key to decrypt the payload. Now possessing the connection details, it can grant the user access to the high-throughput, off-chain service.

### Benefits of this Approach

-   **Audit Trail**: The on-chain transaction serves as an immutable, verifiable record that a specific user requested and was granted access at a specific time.
-   **Security**: The initial handshake is secured by on-chain credentials. You are not relying on traditional Web2 authentication methods alone.
-   **Performance & Cost-Effectiveness**: The actual data transfer happens off-chain, avoiding the high cost and low throughput of storing large amounts of data on the blockchain.
-   **Flexibility**: The `payload` is an opaque byte array. You can implement any off-chain protocol you wish. The `protocols.rs` file in the on-chain program provides a useful, serializable `CommandConfig` struct as a starting point, but you are free to define your own.