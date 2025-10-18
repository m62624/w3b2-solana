//! # W3B2 Solana Signer (C-ABI)
//!
//! This crate provides a C-compatible foreign function interface (FFI) that allows non-Rust
//! applications to use Solana keypairs for signing transactions. It is the recommended
//! solution for building backend oracles in languages like Python, Node.js, Go, or C# that
//! need to sign data for the `w3b2-solana-program`.
//!
//! The library exposes a simple, secure, and thread-safe API for loading keypairs into memory,
//! signing messages, and managing key lifetimes.
//!
//! ## Key Features
//!
//! - **Secure Key Handling**: Private keys are locked in memory using `mlock` (where available)
//!   to prevent them from being written to swap files. All key material is securely zeroized when unloaded.
//! - **Thread-Safe**: The internal key store is thread-safe, allowing you to load and sign from
//!   multiple threads concurrently.
//! - **Simple API**: The library provides a minimal set of functions for key generation, loading,
//!   signing, and unloading.
//!
//! All FFI-exposed functions are defined in the [`ffi`] module.

pub mod ffi;