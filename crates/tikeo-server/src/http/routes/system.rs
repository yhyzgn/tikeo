use std::sync::Arc;

use axum::{Json, extract::State};

use crate::http::{
    AppState,
    dto::{
        ApiResponse, ClusterApiResponse, ClusterDiagnosticsApiResponse, ClusterDiagnosticsResponse,
        ClusterResponse, RaftMemberDiagnostic, RaftMetadataDiagnostic, RaftTransportDiagnostic,
        SystemInfoApiResponse, SystemInfoResponse,
    },
    error::ApiError,
};

/// Return tikeo server build and API metadata.
#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    tag = "system",
    responses((status = 200, description = "System info", body = SystemInfoApiResponse))
)]
pub async fn system_info() -> Json<SystemInfoApiResponse> {
    Json(ApiResponse::success(SystemInfoResponse {
        name: "tikeo",
        version: env!("CARGO_PKG_VERSION"),
        target: std::env::consts::OS,
    }))
}

/// Return the current cluster status placeholder.
#[utoipa::path(
    get,
    path = "/api/v1/cluster",
    tag = "system",
    responses((status = 200, description = "Cluster status", body = ClusterApiResponse))
)]
pub async fn cluster_status(State(state): State<Arc<AppState>>) -> Json<ClusterApiResponse> {
    let status = state.cluster.status().await;
    Json(ApiResponse::success(cluster_response(status)))
}

/// Return operator-visible cluster diagnostics.
///
/// # Errors
///
/// Returns a storage error envelope when persisted Raft diagnostics cannot be read.
#[utoipa::path(
    get,
    path = "/api/v1/cluster/diagnostics",
    tag = "system",
    responses((status = 200, description = "Cluster diagnostics", body = ClusterDiagnosticsApiResponse))
)]
pub async fn cluster_diagnostics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ClusterDiagnosticsApiResponse>, ApiError> {
    let status = state.cluster.status().await;
    let metadata = state
        .raft
        .get_metadata(&status.node_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .map(|item| RaftMetadataDiagnostic {
            cluster_id: item.cluster_id,
            node_id: item.node_id,
            current_term: item.current_term,
            voted_for: item.voted_for,
            commit_index: item.commit_index,
            applied_index: item.applied_index,
            leader_fencing_token: item.leader_fencing_token,
            conf_state: item.conf_state,
            updated_at: item.updated_at,
        });
    let members = state
        .raft
        .list_members()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|item| RaftMemberDiagnostic {
            node_id: item.node_id,
            endpoint: item.endpoint,
            status: item.status,
            updated_at: item.updated_at,
        })
        .collect();
    let scheduling_gated = !status.can_schedule;
    let raft_runtime_enabled = status.mode == crate::cluster::ClusterMode::Raft;
    Ok(Json(ApiResponse::success(ClusterDiagnosticsResponse {
        status: cluster_response(status),
        scheduling_gated,
        metadata,
        members,
        transport: RaftTransportDiagnostic {
            append_entries_path: "/api/v1/raft/append-entries",
            mutating: raft_runtime_enabled,
            status: if raft_runtime_enabled {
                "runtime_inbox_enabled"
            } else {
                "standalone_unavailable"
            },
        },
        runtime_boundary:
            "tikv/raft-rs runtime can tick, accept inbound messages, emit gated membership proposals, and apply committed ConfChange with persisted ConfState; leader fencing remains required for scheduling/proposals".to_owned(),
    })))
}

fn cluster_response(status: crate::cluster::ClusterStatus) -> ClusterResponse {
    ClusterResponse {
        mode: status.mode.as_str().to_owned(),
        role: status.role.as_str().to_owned(),
        node_id: status.node_id,
        nodes: status.nodes,
        can_schedule: status.can_schedule,
        leader_fencing_token: status.leader_fencing_token,
        detail: status.detail,
    }
}
