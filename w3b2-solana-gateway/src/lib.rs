pub mod cli;
pub mod config;
pub mod error;
pub mod grpc;
pub mod storage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::{load_config, GatewayConfig};
use std::{fs::File, str::FromStr};
use tokio::signal;
use tracing::Level;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, writer::MakeWriterExt},
    prelude::*,
    Registry,
};

/// The main entry point for running the gateway application logic.
/// This function handles CLI parsing, configuration, and service startup.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let Commands::Run(run_cmd) = cli.command;
    let config = load_config_from_cli(run_cmd)?;
    init_logging(&config)?;
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

/// Initializes the logging system based on the provided configuration.
fn init_logging(config: &GatewayConfig) -> Result<()> {
    let log_level = Level::from_str(&config.gateway.log.level).unwrap_or(Level::INFO);
    let level_filter = LevelFilter::from_level(log_level);
    let subscriber = Registry::default().with(level_filter);

    match config.gateway.log.output {
        config::LogOutput::File => {
            let file_path = config
                .gateway
                .log
                .file_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Log output is 'file' but 'file_path' is not specified"))?;
            let log_file = File::create(file_path)?;
            let file_writer = log_file.with_max_level(log_level);

            match config.gateway.log.format {
                config::LogFormat::Json => {
                    subscriber.with(fmt::layer().with_writer(file_writer).json()).init()
                }
                config::LogFormat::Plain => subscriber
                    .with(fmt::layer().with_writer(file_writer).pretty())
                    .init(),
            }
        }
        config::LogOutput::Stdout => {
            let stdout_writer = std::io::stdout.with_max_level(log_level);
            match config.gateway.log.format {
                config::LogFormat::Json => {
                    subscriber.with(fmt::layer().with_writer(stdout_writer).json()).init()
                }
                config::LogFormat::Plain => {
                    subscriber.with(fmt::layer().with_writer(stdout_writer).pretty()).init()
                }
            }
        }
    };

    Ok(())
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
