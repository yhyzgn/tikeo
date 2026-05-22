#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};

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
