//! # W3B2 Solana Gateway
//!
//! The `w3b2-solana-gateway` is a gRPC server that provides real-time,
//! persistent event streams from the W3B2 on-chain program to off-chain
//! clients.
//!
//! ## Core Functionality
//!
//! The gateway's sole responsibility is to listen for on-chain events and
//! forward them to subscribed clients. It does **not** handle transaction
//! building, signing, or submission. Clients are expected to construct and
//! send their own transactions directly to a Solana RPC node using standard
//! libraries like `anchor-ts` or `anchorpy`.
//!
//! ## Key Modules
//!
//! - `grpc`: Contains the gRPC server implementation, including the `ListenAsUser`
//!   and `ListenAsAdmin` streaming RPCs.
//! - `storage`: Manages the persistent storage layer used for event catch-up.
//! - `config`: Handles application configuration.
//! - `cli`: Defines the command-line interface for running the gateway.

pub mod cli;
pub mod config;
pub mod error;
pub mod grpc;
pub mod storage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use w3b2_solana_logger::logging;
use config::{load_config, GatewayConfig};
use tokio::signal;

/// The main entry point for running the gateway application logic.
/// This function handles CLI parsing, configuration, and service startup.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let Commands::Run(run_cmd) = cli.command;
    let config = load_config_from_cli(run_cmd)?;
    logging::init(&config.gateway.log)?;
    tracing::info!("Configuration loaded: {:#?}", &config);
    run_server(config).await?;

    Ok(())
}

/// Loads the gateway configuration based on the provided CLI command.
fn load_config_from_cli(run_cmd: cli::RunCmd) -> Result<GatewayConfig> {
    if let Some(config_path) = run_cmd.config {
        println!("Loading configuration from '{}'", &config_path);
        load_config(&config_path)
    } else {
        println!("No config file provided, using default settings.");
        Ok(GatewayConfig::default())
    }
}


/// Starts the gRPC server and handles graceful shutdown.
async fn run_server(config: GatewayConfig) -> Result<()> {
    let event_manager_handle = grpc::start(&config).await?;

    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
            event_manager_handle.stop().await;
            tracing::info!("Shutdown complete.");
        }
        Err(err) => {
            tracing::error!(error = %err, "Failed to listen for shutdown signal.");
        }
    }
    Ok(())
}
