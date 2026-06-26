use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, HttpLogStatus, LogSinkStatus, LoggingStatus, ObservabilityStatusApiResponse,
        ObservabilityStatusResponse, SqlLogStatus, TracingStatus,
    },
    error::ApiError,
};

#[utoipa::path(get, path = "/api/v1/observability/status", tag = "observability")]
/// Observability status.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn observability_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ObservabilityStatusApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "system", "read").await?;
    let logging = &state.observability.logging;
    let tracing = &state.observability.tracing;
    let mut issues = Vec::new();
    let endpoint_configured = tracing
        .otlp_endpoint
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    if tracing.enabled && !endpoint_configured {
        issues.push(
            "observability.tracing.otlp_endpoint is required when tracing export is enabled"
                .to_owned(),
        );
    }
    Ok(Json(ApiResponse::success(ObservabilityStatusResponse {
        logging: LoggingStatus {
            root_level: logging.root.level.clone(),
            http: HttpLogStatus {
                level: logging.http.level.clone(),
                include_headers: logging.http.include_headers,
                include_body: logging.http.include_body,
                max_body_bytes: logging.http.max_body_bytes,
            },
            sql: SqlLogStatus {
                enabled: logging.sql.enabled,
                level: logging.sql.level.clone(),
                include_values: logging.sql.include_values,
                slow_threshold_ms: logging.sql.slow_threshold_ms,
            },
            console: LogSinkStatus {
                enabled: logging.channels.console.enabled,
                level: logging.channels.console.level.clone(),
                target: Some("stdout".to_owned()),
            },
            file: LogSinkStatus {
                enabled: logging.channels.file.enabled,
                level: logging.channels.file.level.clone(),
                target: Some(logging.channels.file.path.clone()),
            },
            elk: LogSinkStatus {
                enabled: logging.channels.elk.enabled,
                level: logging.channels.elk.level.clone(),
                target: Some(logging.channels.elk.servers.clone()),
            },
        },
        tracing: TracingStatus {
            enabled: tracing.enabled,
            exporter: if tracing.enabled { "otlp" } else { "none" }.to_owned(),
            endpoint_configured,
            header_names: tracing.headers.clone(),
        },
        ready: issues.is_empty(),
        issues,
    })))
}
