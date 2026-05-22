//! Server binary support for tikee.

#![forbid(unsafe_code)]

pub mod alert;
pub mod cli;
pub mod cluster;
pub mod http;
pub mod server;
pub mod tikee;
pub mod tunnel;

use anyhow::Result;
use clap::Parser;

/// Parse command-line arguments and run the selected tikee command.
///
/// # Errors
///
/// Returns an error when the selected command fails to load configuration or run.
pub async fn run_cli() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.run().await
}
