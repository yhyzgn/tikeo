use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{Json, extract::State};
use serde_json::Value;

use crate::http::{
    AppState,
    dto::{
        ApiResponse, ClusterApiResponse, ClusterDiagnosticsApiResponse, ClusterDiagnosticsResponse,
        ClusterNodeDiagnostic, ClusterResponse, RaftMemberDiagnostic, RaftMetadataDiagnostic,
        RaftTransportDiagnostic, SmartGatewayDiagnostic, SystemInfoApiResponse, SystemInfoResponse,
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
    let member_summaries = state
        .raft
        .list_members()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let members = member_summaries
        .iter()
        .map(|item| RaftMemberDiagnostic {
            node_id: item.node_id.clone(),
            endpoint: item.endpoint.clone(),
            status: item.status.clone(),
            updated_at: item.updated_at.clone(),
        })
        .collect::<Vec<_>>();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(750))
        .build()
        .map_err(|error| {
            ApiError::bad_request(format!("failed to build diagnostics client: {error}"))
        })?;
    let mut nodes = Vec::new();
    for item in &member_summaries {
        let is_responding_node = item.node_id == status.node_id;
        let probe = if is_responding_node {
            ClusterNodeProbe {
                status: "local".to_owned(),
                observed_role: Some(status.role.as_str().to_owned()),
                observed_can_schedule: Some(status.can_schedule),
                latency_ms: Some(0),
                error: None,
            }
        } else {
            probe_remote_cluster_status(&client, &item.endpoint).await
        };
        nodes.push(ClusterNodeDiagnostic {
            node_id: item.node_id.clone(),
            endpoint: item.endpoint.clone(),
            member_status: item.status.clone(),
            current_term: metadata
                .as_ref()
                .filter(|metadata| metadata.node_id == item.node_id)
                .map(|metadata| metadata.current_term),
            commit_index: metadata
                .as_ref()
                .filter(|metadata| metadata.node_id == item.node_id)
                .map(|metadata| metadata.commit_index),
            applied_index: metadata
                .as_ref()
                .filter(|metadata| metadata.node_id == item.node_id)
                .map(|metadata| metadata.applied_index),
            leader_fencing_token: metadata
                .as_ref()
                .filter(|metadata| metadata.node_id == item.node_id)
                .and_then(|metadata| metadata.leader_fencing_token.clone()),
            is_responding_node,
            can_schedule: is_responding_node && status.can_schedule,
            probe_status: probe.status,
            observed_role: probe.observed_role,
            observed_can_schedule: probe.observed_can_schedule,
            probe_latency_ms: probe.latency_ms,
            probe_error: probe.error,
        });
    }
    let scheduling_gated = !status.can_schedule;
    let raft_runtime_enabled = status.mode == crate::cluster::ClusterMode::Raft;
    let responding_node = cluster_response(status.clone());
    if nodes.is_empty() {
        nodes.push(ClusterNodeDiagnostic {
            node_id: status.node_id.clone(),
            endpoint: String::new(),
            member_status: status.role.as_str().to_owned(),
            current_term: metadata.as_ref().map(|metadata| metadata.current_term),
            commit_index: metadata.as_ref().map(|metadata| metadata.commit_index),
            applied_index: metadata.as_ref().map(|metadata| metadata.applied_index),
            leader_fencing_token: metadata
                .as_ref()
                .and_then(|metadata| metadata.leader_fencing_token.clone()),
            is_responding_node: true,
            can_schedule: status.can_schedule,
            probe_status: "local".to_owned(),
            observed_role: Some(status.role.as_str().to_owned()),
            observed_can_schedule: Some(status.can_schedule),
            probe_latency_ms: Some(0),
            probe_error: None,
        });
    }
    let smart_gateway = smart_gateway_diagnostic(&state).await?;
    Ok(Json(ApiResponse::success(ClusterDiagnosticsResponse {
        responding_node: responding_node.clone(),
        status: responding_node,
        scheduling_gated,
        metadata,
        nodes,
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
        smart_gateway,
    })))
}

async fn smart_gateway_diagnostic(state: &AppState) -> Result<SmartGatewayDiagnostic, ApiError> {
    let local_gateway_node_id = state.registry.gateway_node_id().to_owned();
    let online_workers = state
        .worker_lifecycle
        .list_online_workers(500)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let local_gateway_workers = online_workers
        .iter()
        .filter(|worker| worker.gateway_node_id == local_gateway_node_id)
        .count();
    let online_worker_count = u64::try_from(online_workers.len()).unwrap_or(u64::MAX);
    let local_gateway_worker_count = u64::try_from(local_gateway_workers).unwrap_or(u64::MAX);
    let remote_gateway_worker_count =
        online_worker_count.saturating_sub(local_gateway_worker_count);
    let outbox = state
        .worker_dispatch_outbox
        .summary()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let queued_or_reroute_pending = outbox
        .by_status
        .get("queued")
        .copied()
        .unwrap_or(0)
        .saturating_add(
            outbox
                .by_status
                .get("reroute_pending")
                .copied()
                .unwrap_or(0),
        );
    let status = if queued_or_reroute_pending == 0 && online_worker_count == 0 {
        "idle"
    } else if queued_or_reroute_pending > 0
        && (online_worker_count == 0 || outbox.oldest_queued_age_seconds > 300)
    {
        "degraded"
    } else {
        "ready"
    };

    Ok(SmartGatewayDiagnostic {
        mode: "diagnostic_safe_optimization",
        status,
        local_gateway_node_id,
        online_workers: online_worker_count,
        local_gateway_workers: local_gateway_worker_count,
        remote_gateway_workers: remote_gateway_worker_count,
        outbox_total: outbox.total,
        queued_or_reroute_pending,
        oldest_queued_age_seconds: outbox.oldest_queued_age_seconds,
        safety_boundary: "Smart Gateway optimizes Worker Tunnel locality and operator diagnosis only; Raft fencing, shard ownership, and the durable outbox remains the source of truth for dispatch correctness.",
    })
}

struct ClusterNodeProbe {
    status: String,
    observed_role: Option<String>,
    observed_can_schedule: Option<bool>,
    latency_ms: Option<u64>,
    error: Option<String>,
}

async fn probe_remote_cluster_status(client: &reqwest::Client, endpoint: &str) -> ClusterNodeProbe {
    let Ok(mut url) = url::Url::parse(endpoint.trim()) else {
        return ClusterNodeProbe {
            status: "invalid_endpoint".to_owned(),
            observed_role: None,
            observed_can_schedule: None,
            latency_ms: None,
            error: Some("member endpoint is not a valid URL".to_owned()),
        };
    };
    url.set_path("/api/v1/cluster");
    url.set_query(None);
    let started = Instant::now();
    match client.get(url).send().await {
        Ok(response) => {
            let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            if !response.status().is_success() {
                return ClusterNodeProbe {
                    status: "http_error".to_owned(),
                    observed_role: None,
                    observed_can_schedule: None,
                    latency_ms: Some(latency_ms),
                    error: Some(format!("remote status {}", response.status())),
                };
            }
            match response.json::<Value>().await {
                Ok(payload) => {
                    let data = payload.get("data").unwrap_or(&Value::Null);
                    ClusterNodeProbe {
                        status: "ok".to_owned(),
                        observed_role: data
                            .get("role")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        observed_can_schedule: data
                            .get("can_schedule")
                            .or_else(|| data.get("canSchedule"))
                            .and_then(Value::as_bool),
                        latency_ms: Some(latency_ms),
                        error: None,
                    }
                }
                Err(error) => ClusterNodeProbe {
                    status: "invalid_json".to_owned(),
                    observed_role: None,
                    observed_can_schedule: None,
                    latency_ms: Some(latency_ms),
                    error: Some(error.to_string()),
                },
            }
        }
        Err(error) => ClusterNodeProbe {
            status: "unreachable".to_owned(),
            observed_role: None,
            observed_can_schedule: None,
            latency_ms: Some(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)),
            error: Some(error.to_string()),
        },
    }
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
