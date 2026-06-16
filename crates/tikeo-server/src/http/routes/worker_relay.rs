use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use tikeo_proto::worker::v1::DispatchTask;
use tonic_prost::prost::Message as _;

use crate::http::{AppState, error::ApiError};

/// Internal worker dispatch relay used by the Raft scheduling leader.
///
/// # Errors
///
/// Returns an API error when the internal transport token is missing or invalid, the
/// protobuf body cannot be decoded, the scheduling leader did not include an
/// assignment token, or this gateway does not own a live stream for the requested
/// Worker.
pub async fn relay_dispatch_to_worker(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(worker_id): Path<String>,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    require_internal_relay_token(&headers, &state)?;
    let task = DispatchTask::decode(body.as_ref()).map_err(|error| {
        ApiError::bad_request(format!("invalid dispatch task protobuf: {error}"))
    })?;
    if task.assignment_token.trim().is_empty() {
        return Err(ApiError::bad_request(
            "assignment_token is required for relayed dispatch",
        ));
    }
    if state
        .registry
        .dispatch_relayed_task_to_local_worker(&worker_id, task)
        .await
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!(
            "worker {worker_id} is not connected to this gateway"
        )))
    }
}

fn require_internal_relay_token(headers: &HeaderMap, state: &AppState) -> Result<(), ApiError> {
    let Some(expected) = state.raft_transport_token.as_deref() else {
        return Err(ApiError::forbidden(
            "internal worker relay requires cluster.transport_token",
        ));
    };
    let authorized = headers
        .get("x-tikeo-raft-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| !actual.is_empty() && actual == expected);
    if authorized {
        Ok(())
    } else {
        Err(ApiError::forbidden("invalid internal worker relay token"))
    }
}
