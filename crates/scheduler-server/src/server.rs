//! Server process orchestration.

use anyhow::{Context, Result};
use scheduler_config::SchedulerConfig;
use tokio::try_join;
use tracing::info;

use crate::{http, tunnel};

/// Run all scheduler server listeners.
///
/// # Errors
///
/// Returns an error when any listener fails to bind or serve.
pub async fn serve(config: SchedulerConfig) -> Result<()> {
    let registry = tunnel::WorkerRegistry::default();
    let http_addr = config.server.listen_addr;
    let tunnel_addr = config.server.worker_tunnel_addr;

    info!(%http_addr, %tunnel_addr, "starting scheduler listeners");

    try_join!(http::serve(http_addr), tunnel::serve(tunnel_addr, registry),)
        .context("scheduler listener failed")?;

    Ok(())
}
