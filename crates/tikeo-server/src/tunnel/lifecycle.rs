//! Worker lifecycle background maintenance.

use std::time::Duration;

use tikeo_storage::WorkerLifecycleRepository;
use tracing::{info, warn};

const DEFAULT_LEASE_SCAN_BATCH: u64 = 100;

/// Run the persistent worker lease scanner forever.
pub async fn run_lease_scanner(lifecycle: WorkerLifecycleRepository, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        match lifecycle
            .mark_expired_online_sessions(DEFAULT_LEASE_SCAN_BATCH)
            .await
        {
            Ok(expired) if expired.is_empty() => {}
            Ok(expired) => {
                info!(
                    expired_count = expired.len(),
                    "marked worker sessions offline after lease expiry"
                );
            }
            Err(error) => {
                warn!(%error, "worker lease scanner failed");
            }
        }
    }
}
