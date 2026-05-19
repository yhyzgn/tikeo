//! scheduler server binary entrypoint.

#![forbid(unsafe_code)]

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    scheduler_server::run_cli().await
}
