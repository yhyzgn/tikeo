//! Command-line interface for the tikee server.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tikee_config::load_config;

use crate::server;

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
                init_tracing();
                let config = load_config(config.as_deref())?;
                server::serve(config).await
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

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("tikee=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
}
