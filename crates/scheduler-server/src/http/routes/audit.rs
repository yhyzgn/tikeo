use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, AuditLogExport, AuditLogQuery},
    error::ApiError,
};

const AUDIT_EXPORT_MAX_ROWS: u64 = 500;

/// Export audit logs with governance guardrails (Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs:export",
    tag = "audit",
    params(AuditLogQuery),
    responses(
        (status = 200, description = "Governed audit log export", body = crate::http::dto::AuditLogExportApiResponse),
        (status = 400, description = "Invalid export format", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
#[allow(clippy::missing_errors_doc)]
pub async fn export_audit_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<crate::http::dto::AuditLogExportApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let format = normalize_filter(query.format.clone()).unwrap_or_else(|| "json".to_owned());
    if format != "json" {
        return Err(ApiError::bad_request(
            "audit export currently supports format=json only; CSV needs explicit redaction/content-type governance",
        ));
    }
    let page = state
        .audit
        .list_page(scheduler_storage::AuditLogFilters {
            actor: normalize_filter(query.actor),
            action: normalize_filter(query.action),
            resource_type: normalize_filter(query.resource_type),
            resource_id: normalize_filter(query.resource_id),
            limit: Some(
                query
                    .page_size
                    .map_or(AUDIT_EXPORT_MAX_ROWS, u64::from)
                    .min(AUDIT_EXPORT_MAX_ROWS),
            ),
            offset: query
                .page_token
                .as_deref()
                .and_then(|token| token.parse::<u64>().ok()),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let items = page
        .items
        .into_iter()
        .map(crate::http::dto::AuditLogSummary::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::success(AuditLogExport {
        format,
        exported: items.len() as u64,
        max_rows: AUDIT_EXPORT_MAX_ROWS,
        redacted: false,
        governance: "JSON export is capped at 500 rows, uses the same filters as list, and includes detail/before/after for audit:read operators; review sensitive fields before sharing outside the platform.".to_owned(),
        items,
    })))
}

/// List audit logs (Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs",
    tag = "audit",
    params(AuditLogQuery),
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
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<crate::http::dto::AuditLogPageApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let page = state
        .audit
        .list_page(scheduler_storage::AuditLogFilters {
            actor: normalize_filter(query.actor),
            action: normalize_filter(query.action),
            resource_type: normalize_filter(query.resource_type),
            resource_id: normalize_filter(query.resource_id),
            limit: query.page_size.map(u64::from),
            offset: query
                .page_token
                .as_deref()
                .and_then(|token| token.parse::<u64>().ok()),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    Ok(Json(ApiResponse::success(crate::http::dto::AuditLogPage {
        items: page
            .items
            .into_iter()
            .map(crate::http::dto::AuditLogSummary::from)
            .collect(),
        total: page.total,
        next_page_token: page.next_page_token,
    })))
}

fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}
