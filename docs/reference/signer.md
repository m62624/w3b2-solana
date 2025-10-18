# Signer Library (C-ABI) Reference

The `w3b2-solana-signer` crate provides a C-compatible foreign function interface (FFI) that allows non-Rust applications to use Solana keypairs for signing transactions.

It is the recommended solution for building backend oracles in languages that do **not** have a mature, well-audited native library for Solana keypairs.

## Core Purpose

Instead of reimplementing keypair and signing logic in your language of choice (e.g., Go, C++, Swift), you can compile this crate into a shared library (`.so`, `.dll`, `.dylib`) and call its secure, performant functions from your existing application.

> **Important**: You should only use `w3b2-solana-signer` if a native, well-audited Solana keypair implementation is not available for your programming language. For mainstream languages like TypeScript (`@solana/web3.js`) and Python (`solana-py`), using their native libraries is the preferred approach.

## Features

- **Secure Key Handling**: Private keys are locked in memory using `mlock` (where available) to prevent them from being written to swap files. All key material is securely zeroized when unloaded.
- **Thread-Safe**: The internal key store is thread-safe, allowing you to load and sign from multiple threads concurrently.
- **Simple API**: The library provides a minimal set of functions for key generation, loading, signing, and unloading.
- **Error Handling**: Functions report errors by returning `NULL` or `0`, with a detailed error message available via a `get_last_error()` function.

## How to Build and Use

### Build

1.  **Clone the Repository** and navigate to its root directory.
2.  **Ensure Crate Type**: Check that `w3b2-solana-signer/Cargo.toml` contains `crate-type = ["cdylib", "rlib"]`.
3.  **Compile**: Run `cargo build --release -p w3b2-solana-signer`.
4.  **Locate Artifact**: The shared library will be in `target/release/` (e.g., `libw3b2_solana_signer.so` on Linux).

### Usage

The `w3b2-solana-signer` library is universal. Since it provides a C-compatible interface (C-ABI), it can be used in almost any modern programming language that can work with external C libraries.

The general workflow is:
1.  Load the compiled shared library.
2.  Define the function signatures for the C-ABI functions you intend to use (e.g., `load_key`, `sign_with_handle`, `free_buffer`).
3.  Call the functions, being careful to manage memory correctly by passing pointers and freeing any returned buffers with the provided `free_buffer` function.

For a complete list of all available C-ABI functions and their specific signatures, please refer to the well-documented source code in `w3b2-solana-signer/src/ffi.rs`.