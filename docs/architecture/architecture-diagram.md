# High-Level Architecture Diagram

The system is composed of four main parts: the **Client**, the **gRPC Gateway**, the **Solana Connector**, and the **On-Chain Program**. The backend components (Gateway, Connector, and your custom Oracle) are managed by the service provider, while the client interacts with the user's wallet.

The diagram below illustrates the flow for a paid, user-initiated command. The client builds the transaction and sends it directly to the network, while the gateway streams events back.

```mermaid
graph TD
    subgraph "User's Device"
        A["Client App (using anchorpy/anchor-ts)"]
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

    D -- "1. Oracle signs price data for client" --> A

    A -- "2. Builds, signs & sends transaction" --> F
    F -- "3. Processed by program" --> E
    E -- "4. Emits event" --> F

    subgraph "Real-time Event Streaming"
      F -- "5. Event is picked up by" --> C
      C -- "6. Forwards event to" --> B
      B -- "7. Streams event to" --> A
    end

    style D fill:#f9f,stroke:#333,stroke-width:2px
```

### Component Roles

-   **Client**: Any application (web, mobile, desktop) that interacts with the service. It uses a library like `anchorpy` or `@coral-xyz/anchor` to build transactions, manages the user's wallet for signing, and sends them directly to a Solana RPC Node. It can also subscribe to the gRPC gateway for real-time events.
-   **gRPC Gateway**: Provides a persistent, real-time stream of on-chain events to clients. **This is its only role.**
-   **Solana Connector**: A Rust library that powers the gateway's event listening capabilities.
-   **Oracle Service**: A custom backend component, defined by the service provider, responsible for providing and signing dynamic data, such as the price of a command.
-   **On-Chain Program**: The Anchor smart contract that acts as the source of truth, enforcing rules and managing all on-chain state and fund transfers.
-   **Solana RPC Node**: The gateway to the Solana network itself.