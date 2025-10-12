# High-Level Architecture Diagram

The system is composed of four main parts: the **Client**, the **gRPC Gateway**, the **Solana Connector**, and the **On-Chain Program**. The backend components (Gateway, Connector, and your custom Oracle) are managed by the service provider, while the client interacts with the user's wallet.

The diagram below illustrates the typical flow for a paid, user-initiated command.

```mermaid
graph TD
    subgraph "User's Device"
        A[Client Browser/App]
    end

    subgraph "Service Provider's Backend"
        B[gRPC Gateway]
        C[Solana Connector]
        D[Your Oracle Service]
    end

    subgraph "Solana Network"
        E[On-Chain Program]
        F[Solana RPC Node]
    end

    A -- "1. Prepare Tx (gRPC)" --> B
    B -- "2. Build Unsigned Tx" --> C
    C -- "3. Return Unsigned Tx" --> B
    B -- "4. Return Unsigned Tx (gRPC)" --> A
    A -- "5. Sign Tx w/ Wallet" --> A
    A -- "6. Submit Signed Tx (gRPC)" --> B
    B -- "7. Submit to Network" --> C
    C -- "8. Send to RPC" --> F
    F -- "9. Processed by" --> E

    D -- "Signs Price Data" --> A

    C -- "Listens for Events" --> F
    B -- "Receives Events" --> C

    style D fill:#f9f,stroke:#333,stroke-width:2px
```

### Component Roles

-   **Client**: Any application (web, mobile, desktop) that interacts with the service. It manages the user's wallet for signing transactions.
-   **gRPC Gateway**: The primary, language-agnostic entry point to the backend. It exposes the API for preparing and submitting transactions.
-   **Solana Connector**: A Rust library that handles the low-level details of building Solana transactions and listening for on-chain events.
-   **Oracle Service**: A custom backend component, defined by the service provider, responsible for providing and signing dynamic data, such as the price of a command.
-   **On-Chain Program**: The Anchor smart contract that acts as the source of truth, enforcing rules and managing all on-chain state and fund transfers.
-   **Solana RPC Node**: The gateway to the Solana network itself.