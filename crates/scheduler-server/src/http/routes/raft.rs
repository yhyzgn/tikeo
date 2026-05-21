use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, RaftAppendEntriesApiResponse, RaftAppendEntriesRequest, RaftMessageResult},
    error::ApiError,
};

use super::common::client_ip;

/// Receive a Raft `AppendEntries` transport message placeholder.
///
/// This endpoint is intentionally non-mutating until a real consensus runtime is wired.
/// It documents the LB/K8s-safe HTTP transport shape without granting leadership.
///
/// # Errors
///
/// Returns authorization errors when the caller lacks cluster read permission.
#[utoipa::path(
    post,
    path = "/api/v1/raft/append-entries",
    tag = "raft",
    request_body = RaftAppendEntriesRequest,
    responses(
        (status = 200, description = "Raft message accepted as not implemented", body = RaftAppendEntriesApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn append_entries(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RaftAppendEntriesRequest>,
) -> Result<Json<RaftAppendEntriesApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "cluster", "read").await?;
    let local = state.cluster.status().await;
    Ok(Json(ApiResponse::success(RaftMessageResult {
        accepted: false,
        reason: format!(
            "raft-rs {message_type} transport received but event loop is not started",
            message_type = request.message_type
        ),
        local_node_id: local.node_id,
        local_role: local.role.as_str().to_owned(),
        leader_fencing_token: None,
        remote_addr: client_ip(&headers),
        received_term: request.term,
    })))
}
