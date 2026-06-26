//! Command-line interface for the tikeo server.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tikeo_config::load_config;
use tracing::{error, info};

use crate::{observability::tracing::TracingRuntime, server};

/// tikeo command-line entrypoint.
#[derive(Debug, Parser)]
#[command(
    name = "tikeo",
    version,
    about = "Distributed task scheduling platform"
)]
pub struct Cli {
    /// Command to execute.
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// Execute the parsed command.
    ///
    /// # Errors
    ///
    /// Returns an error when configuration loading or server startup fails.
    pub async fn run(self) -> Result<()> {
        match self.command {
            Command::Serve { config } => {
                let config_path = config.clone();
                let config = load_config(config.as_deref())?;
                let mut tracing_runtime = TracingRuntime::start_from_config(&config)?;
                info!(config_path = ?config_path, "loaded tikeo server configuration");
                let result = Box::pin(server::serve(config)).await;
                if let Err(error) = &result {
                    error!(%error, "tikeo server runtime exited with error");
                } else {
                    info!("tikeo server runtime exited cleanly");
                }
                tokio::task::spawn_blocking(move || tracing_runtime.shutdown()).await??;
                result
            }
        }
    }
}

/// Supported tikeo commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the tikeo server.
    Serve {
        /// Path to a YAML configuration file.
        #[arg(long, env = "TIKEO_CONFIG")]
        config: Option<PathBuf>,
    },
}
