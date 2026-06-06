use std::str::FromStr;

use axum::http::HeaderMap;
use tikeo_core::{ExecutionMode, ScheduleType, TriggerType};
use tracing::warn;

use crate::http::{AppState, error::ApiError, trace};

pub(crate) fn trace_id(headers: &HeaderMap) -> String {
    trace::resolve_trace_id(headers)
}

pub(crate) fn client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim().to_owned())
}

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

pub(super) fn defaulted(value: Option<String>, default: &str) -> String {
    value
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

pub(super) fn parse_schedule_type(value: &str) -> Result<ScheduleType, ApiError> {
    ScheduleType::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

pub(super) fn parse_trigger_type(value: &str) -> Result<TriggerType, ApiError> {
    TriggerType::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

pub(super) fn parse_execution_mode(value: &str) -> Result<ExecutionMode, ApiError> {
    ExecutionMode::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

pub(super) fn parse_trigger_path(value: &str) -> Result<String, ApiError> {
    value
        .strip_suffix(":trigger")
        .filter(|job| !job.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::not_found(format!("unsupported job action: {value}")))
}
