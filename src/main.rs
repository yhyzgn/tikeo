//! tikee server binary entrypoint.

#![forbid(unsafe_code)]

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tikee_server::run_cli().await
}
