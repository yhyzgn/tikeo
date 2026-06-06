#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use tikeo_storage::{CalendarRepository, CalendarWindowSummary, UpsertCalendar};

use crate::http::{AppState, auth, dto::ApiResponse, error::ApiError};

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct CalendarQuery {
    pub namespace: Option<String>,
    pub app: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCalendarRequest {
    pub namespace: String,
    pub app: String,
    pub name: String,
    pub timezone: Option<String>,
    #[serde(default)]
    pub excluded_dates: Vec<String>,
    #[serde(default)]
    pub holidays: Vec<String>,
    #[serde(default)]
    pub maintenance_windows: Vec<CalendarWindowSummary>,
    #[serde(default)]
    pub freeze_windows: Vec<CalendarWindowSummary>,
}

#[utoipa::path(
    get,
    path = "/api/v1/calendars",
    tag = "tenancy",
    params(CalendarQuery)
)]
pub async fn list_calendars(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::CalendarSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let items = CalendarRepository::new(state.users.db())
        .list(query.namespace.as_deref(), query.app.as_deref())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/calendars", tag = "tenancy", request_body = UpsertCalendarRequest)]
pub async fn upsert_calendar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<UpsertCalendarRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::CalendarSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    validate_calendar_request(&request)?;
    let item = CalendarRepository::new(state.users.db())
        .upsert(UpsertCalendar {
            namespace: request.namespace,
            app: request.app,
            name: request.name,
            timezone: request.timezone.unwrap_or_else(|| "UTC".to_owned()),
            excluded_dates: request.excluded_dates,
            holidays: request.holidays,
            maintenance_windows: request.maintenance_windows,
            freeze_windows: request.freeze_windows,
            created_by: principal.username.clone(),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "upsert",
        "calendar",
        &item.id,
        Some(format!("{}/{}/{}", item.namespace, item.app, item.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(delete, path = "/api/v1/calendars/{id}", tag = "tenancy")]
pub async fn delete_calendar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let deleted = CalendarRepository::new(state.users.db())
        .delete(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("calendar not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "calendar",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
}

fn validate_calendar_request(request: &UpsertCalendarRequest) -> Result<(), ApiError> {
    for (label, value) in [
        ("namespace", &request.namespace),
        ("app", &request.app),
        ("name", &request.name),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::bad_request(format!(
                "calendar {label} cannot be empty"
            )));
        }
    }
    for date in request.excluded_dates.iter().chain(request.holidays.iter()) {
        if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return Err(ApiError::bad_request(format!(
                "invalid calendar date: {date}"
            )));
        }
    }
    for window in request
        .maintenance_windows
        .iter()
        .chain(request.freeze_windows.iter())
    {
        let start = chrono::DateTime::parse_from_rfc3339(&window.start).map_err(|_| {
            ApiError::bad_request(format!("invalid calendar window start: {}", window.start))
        })?;
        let end = chrono::DateTime::parse_from_rfc3339(&window.end).map_err(|_| {
            ApiError::bad_request(format!("invalid calendar window end: {}", window.end))
        })?;
        if end <= start {
            return Err(ApiError::bad_request(
                "calendar window end must be after start",
            ));
        }
    }
    Ok(())
}
