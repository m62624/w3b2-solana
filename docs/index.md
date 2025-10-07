# Welcome to the W3B2 Protocol

**W3B2 is a non-custodial protocol and toolset for bridging the gap between Web2 applications and the Solana blockchain.**

It enables developers to build services that leverage the security, transparency, and payment rails of cryptocurrency without needing to migrate their entire business logic on-chain. With W3B2, you can keep your fast, efficient Web2 backend and simply use the blockchain for what it does best: managing ownership, verifying state, and providing an immutable audit trail.

---

## Key Features

-   **Non-Custodial by Design**: Users always control their funds. Our on-chain program ensures that service providers can only debit accounts for services rendered, never directly access or manage user deposits.
-   **Hybrid On/Off-Chain Model**: We use the blockchain as a high-integrity message bus and state machine, not for heavy computation. Your complex business logic remains in your off-chain application, where it's fast and cost-effective.
-   **Turnkey Components**: Get started quickly with our pre-built components:
    -   An audited on-chain program.
    -   A gRPC Gateway service for easy integration.
    -   A Rust Connector library for custom backends.
-   **Developer-Owned Oracles**: We provide the framework for on-chain verification, but you retain full control over your service's business logic and secrets. You run your own oracle, ensuring maximum security and flexibility.
-   **Language Agnostic**: The gRPC gateway allows you to interact with the W3B2 protocol from any language—Python, TypeScript/JavaScript, Go, Rust, and more.

---

## Who is this for?

-   **SaaS businesses** wanting to accept crypto payments without giving up their existing infrastructure.
-   **Game developers** looking to integrate non-custodial player balances or on-chain verifiable actions.
-   **API providers** who want to charge for services in a transparent, auditable way.
-   Any developer who wants to **add Web3 capabilities** to a Web2 service without a complete rewrite.

## Get Started

Ready to dive in? Head over to our **[Getting Started](./getting-started.md)** guide to set up your local environment and run the W3B2 stack in minutes.