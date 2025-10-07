# W3B2 Gateway Service (`w3b2-solana-gateway`)

This crate contains a ready-to-use, production-grade gRPC service built on top of the `w3b2-solana-connector`. It exposes the full functionality of the W3B2 protocol to clients written in any language (Python, TypeScript, Go, etc.).

Its primary features are:
- A non-custodial "prepare-then-submit" flow for all on-chain transactions.
- Server-streaming RPCs for listening to on-chain events.

## Full Documentation

For detailed information on the gRPC methods, configuration, and how to interact with the service, please see our main documentation site.

**--> [Go to the Gateway Service API Reference](../../docs/api/gateway.md)**