#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Extension, Json, extract::State, http::HeaderMap};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, MetricsAlertSummary, MetricsGovernanceSummary, MetricsInstanceSummary,
        MetricsSummaryResponse, MetricsWorkerSummary,
    },
    error::ApiError,
};

#[utoipa::path(get, path = "/api/v1/metrics/summary", tag = "metrics")]
pub async fn metrics_summary(
    State(state): State<Arc<AppState>>,
    Extension(recorder): Extension<Arc<metrics_exporter_prometheus::PrometheusRecorder>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<MetricsSummaryResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "system", "read").await?;
    let workers = state.registry.workers().await;
    let instances = state
        .instances
        .count_by_status()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let alert_counts = state
        .alerts
        .count_events()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let queue = state
        .workflows
        .dispatch_queue_slo_summary()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    metrics::with_local_recorder(&*recorder, || record_dispatch_queue_metrics(&queue));

    Ok(Json(ApiResponse::success(MetricsSummaryResponse {
        workers: MetricsWorkerSummary {
            online: u64::try_from(workers.len()).unwrap_or(u64::MAX),
        },
        instances: MetricsInstanceSummary {
            total: instances.total,
            by_status: instances.by_status,
        },
        alerts: MetricsAlertSummary {
            total_events: alert_counts.total_events,
            by_status: alert_counts.by_status,
        },
        governance: MetricsGovernanceSummary {
            script_failure_events: alert_counts.script_failure_events,
            by_failure_class: alert_counts.by_failure_class,
        },
        queue,
    })))
}

fn record_dispatch_queue_metrics(queue: &tikee_storage::DispatchQueueSloSummary) {
    let oldest = std::time::Duration::from_secs(queue.oldest_pending_age_seconds).as_secs_f64();
    let average = std::time::Duration::from_secs(queue.average_pending_age_seconds).as_secs_f64();
    metrics::histogram!("tikee_dispatch_queue_pending_age_seconds", "stat" => "oldest")
        .record(oldest);
    metrics::histogram!("tikee_dispatch_queue_pending_age_seconds", "stat" => "average")
        .record(average);
    metrics::gauge!("tikee_dispatch_queue_items_total", "status" => "pending")
        .set(u64_metric_value(queue.pending));
    metrics::gauge!("tikee_dispatch_queue_items_total", "status" => "running")
        .set(u64_metric_value(queue.running));
}

#[allow(clippy::cast_precision_loss)]
const fn u64_metric_value(value: u64) -> f64 {
    // The metrics crate exposes gauge values as f64; Prometheus queue gauges are
    // operational signals, so very large u64 counts may be rounded at scrape
    // time rather than rejected or saturated.
    value as f64
}
