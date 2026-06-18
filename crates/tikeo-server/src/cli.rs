//! Command-line interface for the tikeo server.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tikeo_config::load_config;

use crate::{migration_plan, observability::tracing::TracingRuntime, server};

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
                let config = load_config(config.as_deref())?;
                let mut tracing_runtime =
                    TracingRuntime::start_observability(&config.observability)?;
                let result = Box::pin(server::serve(config)).await;
                tokio::task::spawn_blocking(move || tracing_runtime.shutdown()).await??;
                result
            }
            Command::Migrate(command) => migration_plan::run_migration_command(&command),
        }
    }
}

/// Supported tikeo commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the tikeo server.
    Serve {
        /// Path to a TOML configuration file.
        #[arg(long, env = "TIKEO_CONFIG")]
        config: Option<PathBuf>,
    },
    /// Build a dry-run migration report from an existing scheduler export.
    Migrate(migration_plan::MigrationCommand),
}
