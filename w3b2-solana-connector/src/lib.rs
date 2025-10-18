//! A core Rust library for interacting with the `w3b2-solana-program` on Solana.
//!
//! This crate provides the primary building blocks for creating backend services
//! and clients for the W3B2 protocol. It abstracts away the complexities of
//! blockchain interaction, offering a high-level, asynchronous API.
//!
//! # Key Components
//!
//! *   [`client`]: A non-custodial helper for constructing unsigned transactions for all program instructions.
//! *   [`workers::EventManager`]: The main entry point for the event system. It runs
//!     background services to synchronize with the blockchain and dispatch events.
//! *   [`listener`]: High-level event listeners (`UserListener`, `AdminListener`) that
//!     subscribe to a specific on-chain PDA and provide separate streams for historical
//!     (`catchup`) and real-time (`live`) events.
pub mod client;
/// Defines configuration structures for the connector.
pub mod config;
/// The internal event routing worker (`Dispatcher`).
mod dispatcher;

/// Logic for parsing on-chain events from transaction logs.
pub mod events;
/// High-level, PDA-based event listeners (`UserListener`, `AdminListener`) with
/// separate streams for historical and real-time events.
pub mod listener;
/// A trait and default implementation for persistent synchronization state.
pub mod storage;
/// The background workers responsible for blockchain synchronization.
pub mod workers;
