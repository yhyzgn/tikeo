//! tikee server binary entrypoint.

#![forbid(unsafe_code)]

use anyhow::Result;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    tikee_server::run_cli().await
}
