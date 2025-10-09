# gRPC Gateway (`w3b2-solana-gateway`)

This crate contains a gRPC service built on top of the `w3b2-solana-connector`. It exposes the functionality of the on-chain program to clients written in any gRPC-compatible language (e.g., Python, TypeScript, Go).

Its primary features are a "prepare-then-submit" flow for creating transactions and server-streaming RPCs for listening to on-chain events.

## Documentation

For detailed information on the gRPC methods, configuration, and how to interact with the service, see the main documentation site.

**--> [Gateway Service API Reference](../../docs/api/gateway.md)**