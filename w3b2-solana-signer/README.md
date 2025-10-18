# W3B2 Solana Signer

This crate provides a C-compatible foreign function interface (FFI) that allows non-Rust applications to use Solana keypairs for signing transactions. It is the recommended solution for building backend oracles in languages like Python, Node.js, Go, or C# that need to sign data for the `w3b2-solana-program`.

The library exposes a simple, secure, and thread-safe API for loading keypairs into memory, signing messages, and managing key lifetimes.

## The Problem It Solves

While many languages have excellent libraries for interacting with Solana's JSON-RPC API, not all of them have mature, well-audited implementations for handling Solana's `ed25519` keypairs and signing logic. This library solves that problem by exposing Rust's robust and performant `solana_sdk` functionality through a universal C-ABI.

Instead of reimplementing keypair logic in your language of choice, you can compile this crate into a shared library (`.so`, `.dll`, `.dylib`) and call its functions from your existing application.

## Core Features

- **Secure Key Handling**: Private keys are locked in memory using `mlock` (where available) to prevent them from being written to swap files. All key material is securely zeroized when unloaded.
- **Thread-Safe**: The internal key store is thread-safe, allowing you to load and sign from multiple threads concurrently.
- **Simple API**: The library provides a minimal set of functions:
    - Generate a new keypair.
    - Load an existing keypair from bytes.
    - Get the public key from a loaded keypair.
    - Sign a transaction message with a loaded keypair.
    - Unload a keypair to free memory.
- **Error Handling**: Functions report errors by returning `NULL` or `0`, with a detailed error message available via a `get_last_error()` function.
- **Memory Management**: All memory allocated by the library is returned to the caller, who is responsible for freeing it with a provided `free_buffer` function.

## How to Build

You must compile this crate as a C-compatible dynamic library.

1.  **Clone the Repository**:
    ```bash
    git clone https://github.com/your-repo/w3b2-solana.git
    cd w3b2-solana
    ```

2.  **Add Crate Type to `Cargo.toml`**:
    Ensure the `w3b2-solana-signer/Cargo.toml` file specifies the `cdylib` crate type, which produces a shared library:
    ```toml
    [lib]
    name = "w3b2_solana_signer"
    crate-type = ["cdylib", "rlib"]
    ```

3.  **Compile in Release Mode**:
    ```bash
    cargo build --release -p w3b2-solana-signer
    ```

4.  **Locate the Artifact**:
    The compiled library will be in the `target/release` directory.
    -   On Linux: `target/release/libw3b2_solana_signer.so`
    -   On macOS: `target/release/libw3b2_solana_signer.dylib`
    -   On Windows: `target/release/w3b2_solana_signer.dll`

You can now link to this library from your application.

## Usage

The `w3b2-solana-signer` library is universal. Since it provides a C-compatible interface (C-ABI), it can be used in almost any modern programming language that can work with external C libraries (e.g., Python's `ctypes`, Node.js's `ffi-napi`, Go's `cgo`, etc.).

You will need to:
1.  Load the compiled shared library (`.so`, `.dll`, or `.dylib`).
2.  Define the function signatures for the C-ABI functions you intend to use.
3.  Call the functions, being careful to manage memory correctly by passing pointers and freeing returned buffers with `free_buffer`.

> **Important**: You should only use `w3b2-solana-signer` if a native, well-audited Solana keypair implementation is not available for your programming language. For mainstream languages like TypeScript (`@solana/web3.js`) and Python (`solana-py`), using the native libraries is preferred.