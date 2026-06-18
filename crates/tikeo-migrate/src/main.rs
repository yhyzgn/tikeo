//! tikeo-migrate binary entrypoint.

#![forbid(unsafe_code)]

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    tikeo_migrate::Cli::parse().run().await
}
