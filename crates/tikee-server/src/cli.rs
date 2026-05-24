//! Command-line interface for the tikee server.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tikee_config::load_config;

use crate::{observability::tracing::TracingRuntime, server};

/// tikee command-line entrypoint.
#[derive(Debug, Parser)]
#[command(
    name = "tikee",
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
                let config = load_config(config.as_deref())?;
                let mut tracing_runtime = TracingRuntime::start(&config.observability.tracing)?;
                let result = Box::pin(server::serve(config)).await;
                tokio::task::spawn_blocking(move || tracing_runtime.shutdown()).await??;
                result
            }
        }
    }
}

/// Supported tikee commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the tikee server.
    Serve {
        /// Path to a TOML configuration file.
        #[arg(long, env = "TIKEE_CONFIG")]
        config: Option<PathBuf>,
    },
}
