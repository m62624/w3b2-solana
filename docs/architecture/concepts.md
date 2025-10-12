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