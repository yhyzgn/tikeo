use std::str::FromStr;

use axum::http::HeaderMap;
use serde::Deserialize;
use tikeo_core::{ExecutionMode, ScheduleType, TriggerType};
use tracing::warn;

use crate::http::{AppState, error::ApiError, trace};

/// Trace id.
pub fn trace_id(headers: &HeaderMap) -> String {
    trace::resolve_trace_id(headers)
}

/// Query-string authentication fallback for browser `EventSource` clients.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StreamAuthQuery {
    /// Bearer token appended as `?token=...` when `EventSource` cannot set headers.
    pub token: Option<String>,
    /// Page size value for paged streams.
    pub page_size: Option<u32>,
    /// Page token value for paged streams.
    pub page_token: Option<String>,
}

/// Apply stream token.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn apply_stream_token(
    headers: &mut HeaderMap,
    query: &StreamAuthQuery,
) -> Result<(), ApiError> {
    if let Some(token) = query.token.as_deref()
        && !headers.contains_key(axum::http::header::AUTHORIZATION)
    {
        let value = format!("Bearer {token}")
            .parse()
            .map_err(|_| ApiError::unauthorized("invalid stream token"))?;
        headers.insert(axum::http::header::AUTHORIZATION, value);
    }
    Ok(())
}

/// Client ip.
pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim().to_owned())
}

/// Audit.
pub(super) async fn audit(
    state: &AppState,
    actor: &str,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    detail: Option<String>,
    headers: &HeaderMap,
) {
    use tikeo_storage::CreateAuditLog;
    if let Err(error) = state
        .audit
        .append(CreateAuditLog {
            actor: actor.to_owned(),
            action: action.to_owned(),
            resource_type: resource_type.to_owned(),
            resource_id: resource_id.to_owned(),
            detail,
            before: None,
            after: None,
            trace_id: Some(trace_id(headers)),
            result: "success".to_owned(),
            failure_reason: None,
            ip_address: client_ip(headers),
        })
        .await
    {
        warn!(%error, %actor, %action, %resource_type, %resource_id, "failed to append audit log");
    }
}

/// Defaulted.
pub(super) fn defaulted(value: Option<String>, default: &str) -> String {
    value
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

/// Parse schedule type.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn parse_schedule_type(value: &str) -> Result<ScheduleType, ApiError> {
    ScheduleType::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

/// Parse trigger type.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn parse_trigger_type(value: &str) -> Result<TriggerType, ApiError> {
    TriggerType::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

/// Parse execution mode.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn parse_execution_mode(value: &str) -> Result<ExecutionMode, ApiError> {
    ExecutionMode::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

/// Parse trigger path.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn parse_trigger_path(value: &str) -> Result<String, ApiError> {
    value
        .strip_suffix(":trigger")
        .filter(|job| !job.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::not_found(format!("unsupported job action: {value}")))
}
