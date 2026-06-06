//! Server binary support for tikeo.

#![forbid(unsafe_code)]

pub mod alert;
pub mod cli;
pub mod cluster;
pub mod http;
pub mod observability;
pub mod server;
pub mod tikeo;
pub mod transport_security;
pub mod tunnel;

use anyhow::Result;
use clap::Parser;

/// Parse command-line arguments and run the selected tikeo command.
///
/// # Errors
///
/// Returns an error when the selected command fails to load configuration or run.
pub async fn run_cli() -> Result<()> {
    let cli = cli::Cli::parse();
    Box::pin(cli.run()).await
}
