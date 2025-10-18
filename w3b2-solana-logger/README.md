# W3B2 Solana Logger

This crate provides a simple, reusable logging utility for Rust-based services within the W3B2-Solana toolset, such as the `w3b2-solana-gateway`. It is built on top of the `tracing` and `tracing-subscriber` crates.

## Features

- **Configurable Log Level**: Set the desired verbosity (e.g., `INFO`, `DEBUG`, `TRACE`).
- **Multiple Output Formats**: Supports plain text (`pretty`) and structured `JSON` formats.
- **Multiple Output Sinks**: Can write logs to standard output (`stdout`) or a specified file.
- **Easy Initialization**: A single `init()` function configures the global logger for an application.

## Usage

This logger is intended to be used by other crates in the workspace.

### 1. Define Configuration

The logger is configured via a `LogConfig` struct, which is typically deserialized from a settings file (e.g., a TOML or YAML file) in the consuming application.

**Example `config.toml`:**
```toml
[logging]
level = "info"      # "error", "warn", "info", "debug", or "trace"
format = "json"     # "plain" or "json"
output = "stdout"   # "stdout" or "file"
# file_path = "/var/log/my_app.log" # Required only if output is "file"
```

### 2. Initialize in Your Application

In your application's `main.rs`, initialize the logger early in the startup process.

```rust
use w3b2_solana_logger::{init, LogConfig};
use tracing::info;

// Assume 'config.logging' is the LogConfig struct loaded from a file
let log_config: LogConfig = Default::default();
init(&log_config).expect("Failed to initialize logger");

info!("Logger initialized successfully. The application is starting.");
// ... rest of your application logic ...
```

Once initialized, you can use the standard `tracing` macros (`info!`, `warn!`, `error!`, `debug!`, etc.) throughout your application to generate logs according to the provided configuration.