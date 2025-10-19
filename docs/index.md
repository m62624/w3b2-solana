# Welcome to the W3B2-Solana Documentation

W3B2-Solana is a toolset for developers looking to integrate their existing Web2 services with the security, transparency, and non-custodial nature of the Solana blockchain. It provides the on-chain programs and off-chain libraries to seamlessly blend high-performance, traditional backend infrastructure with the power of Web3.

The core value is enabling **two powerful interaction models** within a single, unified framework:

1.  **Direct On-Chain Transactions**: For simple, low-data interactions like micropayments or logging critical audit data, your application can interact directly with the on-chain program. This is the classic Web3 model.

2.  **Secure Off-Chain Handshake for Heavy Traffic**: For high-bandwidth Web2 services (e.g., private data feeds, session-based services), this toolset allows you to use the blockchain as a **secure message bus** to negotiate a direct, off-chain connection between your service and the user.

This hybrid approach allows you to use the blockchain for what it's best at—security and asset transfer—while leveraging your existing Web2 infrastructure for performance and scale.

This site provides detailed guides, API references, and architecture diagrams for the entire toolset. Use the navigation to explore the different components and concepts.

---

## Core Use Cases

-   **Non-Custodial Paid APIs**: Charge users in SOL for API calls, authorized by your service's oracle.
-   **Verifiable Audit Trails**: Log critical off-chain actions to the Solana blockchain as an immutable record using the `log_action` instruction.
-   **Secure Connection Brokering**: Use the on-chain `dispatch` commands to securely establish direct, high-throughput off-chain communication channels with your users.
-   **On-Chain User Management**: Implement transparent, on-chain banning and moderation systems.

---

To dive deeper, start with the [Core Concepts](architecture/concepts.md).