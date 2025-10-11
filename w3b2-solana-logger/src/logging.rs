use anyhow::Result;
use serde::Deserialize;
use std::{fs::File, str::FromStr};
use tracing::Level;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, writer::MakeWriterExt},
    prelude::*,
    Registry,
};

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    #[default]
    Plain,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    #[default]
    Stdout,
    File,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LogConfig {
    pub level: String,
    pub format: LogFormat,
    pub output: LogOutput,
    pub file_path: Option<String>,
}

pub fn init(config: &LogConfig) -> Result<()> {
    let log_level = Level::from_str(&config.level).unwrap_or(Level::INFO);
    let level_filter = LevelFilter::from_level(log_level);
    let subscriber = Registry::default().with(level_filter);

    match config.output {
        LogOutput::File => {
            let file_path = config
                .file_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Log output is 'file' but 'file_path' is not specified"))?;
            let log_file = File::create(file_path)?;
            let file_writer = log_file.with_max_level(log_level);

            match config.format {
                LogFormat::Json => {
                    subscriber.with(fmt::layer().with_writer(file_writer).json()).init()
                }
                LogFormat::Plain => subscriber
                    .with(fmt::layer().with_writer(file_writer).pretty())
                    .init(),
            }
        }
        LogOutput::Stdout => {
            let stdout_writer = std::io::stdout.with_max_level(log_level);
            match config.format {
                LogFormat::Json => {
                    subscriber.with(fmt::layer().with_writer(stdout_writer).json()).init()
                }
                LogFormat::Plain => {
                    subscriber.with(fmt::layer().with_writer(stdout_writer).pretty()).init()
                }
            }
        }
    };

    Ok(())
}