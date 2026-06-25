//! Server binary support for tikeo.

#![forbid(unsafe_code)]

/// `Alert` module.
pub mod alert;
/// `Cli` module.
pub mod cli;
/// `Cluster` module.
pub mod cluster;
/// `Http` module.
pub mod http;
/// `Notification` module.
pub mod notification;
/// `Observability` module.
pub mod observability;
/// `Server` module.
pub mod server;
/// `Tikeo` module.
pub mod tikeo;
/// Transport security module.
pub mod transport_security;
/// `Tunnel` module.
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
