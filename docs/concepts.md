# Core Concepts

The W3B2 toolset is built on a few core principles that enable secure and flexible interaction between Web2 services and the Solana blockchain.

### Admin & User Profiles

The program establishes two primary PDA (Program-Derived Address) account types:

-   **`AdminProfile`**: Represents the service provider (the "Admin"). It holds service-wide configuration, such as the oracle's public key, and serves as a treasury for collected fees. Each service you create will have its own `AdminProfile`.

-   **`UserProfile`**: Represents an end-user's relationship with a specific service. It holds the user's pre-paid deposit balance for that service and tracks their status (e.g., banned). A user will have a separate `UserProfile` for each service they register with.

### Non-Custodial Payments

Users never send funds directly to the service. Instead, they deposit lamports into their own `UserProfile` PDA. This account is controlled by the on-chain program, but the funds can only be transferred to the admin's profile upon the user's explicit, signed approval via the `user_dispatch_command` instruction. This means users always retain control of their funds.

### Off-Chain Oracle for Dynamic Pricing

To avoid hard-coding prices on-chain, the system uses an off-chain oracle pattern. For any paid action, your backend service (the "oracle") signs the payment details (price, command ID, timestamp). The user includes this signature in their transaction. The on-chain program then verifies this signature against the `oracle_authority` stored in the `AdminProfile` before processing the payment. This keeps your business logic flexible and off-chain, while keeping value transfer secure and on-chain.

### Event-Driven Architecture

The program emits detailed events for every significant action (e.g., `UserProfileCreated`, `UserCommandDispatched`, `UserBanned`). Your backend services can listen for these on-chain events using the `w3b2-solana-connector` to synchronize state, update databases, or trigger other business processes.

### The "Request for Review" Unban Model

The unban process is intentionally asynchronous and requires admin intervention. A user pays a fee to *request a review*, not to automatically buy an unban. This design choice ensures the service provider (admin) retains sovereignty over disciplinary actions, preventing malicious users from simply paying a "tax" to continue their behavior. It provides flexibility for the admin to unban users for free (in case of error) or deny a request. The smart contract acts as an incorruptible arbiter, verifiably recording the request and payment, while the final decision remains with the service owner.