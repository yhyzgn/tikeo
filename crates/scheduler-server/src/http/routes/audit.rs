use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, PageQuery},
    error::ApiError,
};

/// List audit logs (Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs",
    tag = "audit",
    params(PageQuery),
    responses(
        (status = 200, description = "Audit log page", body = crate::http::dto::AuditLogPageApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
#[allow(clippy::missing_errors_doc)]
pub async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(_query): Query<PageQuery>,
) -> Result<Json<crate::http::dto::AuditLogPageApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let items = state
        .audit
        .list(scheduler_storage::AuditLogFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(crate::http::dto::AuditLogSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(crate::http::dto::AuditLogPage {
        items,
        next_page_token: None,
    })))
}
