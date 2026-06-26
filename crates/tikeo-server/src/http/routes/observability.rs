use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, LogSinkStatus, LoggingStatus, ObservabilityStatusApiResponse,
        ObservabilityStatusResponse, TracingStatus,
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
            console: LogSinkStatus {
                enabled: logging.console.enabled,
                level: logging.console.level.clone(),
                target: Some("stdout".to_owned()),
            },
            file: LogSinkStatus {
                enabled: logging.file.enabled,
                level: logging.file.level.clone(),
                target: Some(logging.file.path.clone()),
            },
            error_file: LogSinkStatus {
                enabled: logging.error_file.enabled,
                level: logging.error_file.level.clone(),
                target: Some(logging.error_file.path.clone()),
            },
            elk: LogSinkStatus {
                enabled: logging.elk.enabled,
                level: logging.elk.level.clone(),
                target: Some(logging.elk.servers.clone()),
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
