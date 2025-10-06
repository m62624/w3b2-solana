use anyhow::{Context, Result};
use serde::Deserialize;
use w3b2_solana_connector::config::ConnectorConfig;

/// The top-level configuration for the W3B2 Gateway application.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct GatewayConfig {
    #[serde(default)]
    pub connector: ConnectorConfig,
    #[serde(default)]
    pub gateway: GatewaySpecificConfig,
}

/// Contains settings that are unique to the gateway binary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GatewaySpecificConfig {
    pub db_path: String,
    #[serde(default)]
    pub grpc: GrpcConfig,
    /// Logging configuration.
    #[serde(default)]
    pub log: LogConfig,
}

/// gRPC server connection settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LogConfig {
    /// Log level, e.g., "info", "debug", "trace".
    pub level: String,
    /// Log output format.
    pub format: LogFormat,
    /// Log output destination.
    pub output: LogOutput,
    /// Path to the log file, required if output is "file".
    pub file_path: Option<String>,
}

/// Defines the format for log messages.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LogFormat {
    Plain,
    Json,
}

/// Defines the destination for log output.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LogOutput {
    Stdout,
    File,
}

impl Default for GatewaySpecificConfig {
    fn default() -> Self {
        Self {
            db_path: "./w3b2_gateway.db".to_string(),
            grpc: GrpcConfig::default(),
            log: LogConfig::default(),
        }
    }
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 50051,
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Plain,
            output: LogOutput::Stdout,
            file_path: None,
        }
    }
}

/// Loads the gateway configuration from a specified TOML file.
///
/// It uses the `config` crate to read the file and deserialize it into
/// the `GatewayConfig` struct.
pub fn load_config(path: &str) -> Result<GatewayConfig> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name(path))
        .add_source(config::Environment::with_prefix("W3B2").separator("__"));

    let settings: GatewayConfig = builder
        .build()
        .context(format!("Failed to build configuration from '{}'", path))?
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    Ok(settings)
}
