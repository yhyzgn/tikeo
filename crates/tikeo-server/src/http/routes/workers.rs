#![allow(missing_docs, clippy::missing_errors_doc)]

use std::{collections::HashMap, sync::Arc};

use axum::{Json, extract::State, http::HeaderMap};
use serde::Deserialize;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, DispatchQueueApiResponse, DispatchQueueClaimApiResponse,
        WorkerCapabilitiesSummary, WorkerLifecycleHistoryApiResponse,
        WorkerLifecycleHistoryResponse, WorkerListApiResponse, WorkerListResponse,
        WorkerMasterSummary, WorkerPluginProcessorSummary, WorkerScriptRunnerSummary,
        WorkerSessionEventDto, WorkerSessionHistorySummary, WorkerSummary,
    },
    error::ApiError,
};

use super::common::audit;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ClaimDispatchQueueRequest {
    pub lease_owner: String,
    pub lease_seconds: Option<i64>,
    pub fencing_token: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/workers", tag = "workers")]
pub async fn list_workers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkerListApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workers", "read").await?;
    let registry_workers = state.registry.workers().await;
    let mut registry_by_worker_id = registry_workers
        .into_iter()
        .map(|worker| (worker.worker_id.clone(), worker))
        .collect::<HashMap<_, _>>();
    let persisted_workers = state
        .worker_lifecycle
        .list_online_workers(500)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut items = Vec::with_capacity(persisted_workers.len().max(registry_by_worker_id.len()));

    for persisted in persisted_workers {
        if let Some(worker) = registry_by_worker_id.remove(&persisted.worker_id) {
            let worker_pool = worker_pool_from_labels(&worker.labels);
            items.push((registry_worker_summary(worker), worker_pool));
        } else {
            let worker_pool = worker_pool_from_labels_json(&persisted.labels_json);
            items.push((persisted_worker_summary(persisted), worker_pool));
        }
    }

    for worker in registry_by_worker_id.into_values() {
        let worker_pool = worker_pool_from_labels(&worker.labels);
        items.push((registry_worker_summary(worker), worker_pool));
    }

    items.retain(|(worker, worker_pool)| {
        crate::http::access_scope::allows_resource(
            &principal.scope_bindings,
            &worker.namespace,
            &worker.app,
            worker_pool.as_deref(),
        )
    });
    let items = items
        .into_iter()
        .map(|(worker, _worker_pool)| worker)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::success(WorkerListResponse {
        online: items.len(),
        items,
    })))
}

#[utoipa::path(get, path = "/api/v1/workers/history", tag = "workers")]
pub async fn worker_lifecycle_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkerLifecycleHistoryApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workers", "read").await?;
    let sessions = state
        .worker_lifecycle
        .list_sessions(200)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|session| WorkerSessionHistorySummary {
            worker_id: session.worker_id,
            logical_instance_id: session.logical_instance_id,
            generation: session.generation,
            status: session.status,
            status_reason: session.status_reason,
            status_evidence: session.status_evidence,
            lease_expires_at: session.lease_expires_at,
            last_heartbeat_at: session.last_heartbeat_at,
            last_sequence: session.last_sequence,
            replaced_by_worker_id: session.replaced_by_worker_id,
        })
        .collect();
    let events = state
        .worker_lifecycle
        .list_recent_events(200)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|event| WorkerSessionEventDto {
            id: event.id,
            worker_id: event.worker_id,
            logical_instance_id: event.logical_instance_id,
            event_type: event.event_type,
            reason: event.reason,
            detail_json: event.detail_json,
            created_at: event.created_at,
        })
        .collect();
    Ok(Json(ApiResponse::success(WorkerLifecycleHistoryResponse {
        sessions,
        events,
    })))
}

fn registry_worker_summary(worker: crate::tunnel::RegisteredWorker) -> WorkerSummary {
    WorkerSummary {
        worker_id: worker.worker_id,
        logical_instance_id: worker.logical_instance_id,
        client_instance_id: worker.client_instance_id,
        app: worker.app,
        namespace: worker.namespace,
        cluster: worker.cluster,
        region: worker.region,
        capabilities: worker.capabilities,
        structured_capabilities: worker_capabilities_summary(&worker.structured_capabilities),
        master: WorkerMasterSummary {
            domain: worker.master.domain,
            is_master: worker.master.is_master,
            master_worker_id: worker.master.master_worker_id,
            term: worker.master.term,
            fencing_token: worker.master.fencing_token,
        },
        generation: worker.generation,
        status: worker.status.as_str().to_owned(),
        status_reason: worker.status_reason,
        replaced_by_worker_id: worker.replaced_by_worker_id,
        last_sequence: worker.last_sequence,
    }
}

fn persisted_worker_summary(worker: tikeo_storage::PersistedOnlineWorkerSummary) -> WorkerSummary {
    let domain = format!(
        "{}/{}/{}/{}",
        worker.namespace_name, worker.app_name, worker.cluster, worker.region
    );
    let master = parse_master_summary(&worker.master_json).unwrap_or_else(|| WorkerMasterSummary {
        domain,
        is_master: false,
        master_worker_id: None,
        term: u64::try_from(worker.generation).unwrap_or_default(),
        fencing_token: None,
    });
    WorkerSummary {
        worker_id: worker.worker_id.clone(),
        logical_instance_id: worker.logical_instance_id,
        client_instance_id: worker.client_instance_id,
        app: worker.app_name,
        namespace: worker.namespace_name,
        cluster: worker.cluster,
        region: worker.region,
        capabilities: parse_string_vec(&worker.capabilities_json),
        structured_capabilities: parse_capabilities_summary(&worker.structured_capabilities_json),
        master,
        generation: u64::try_from(worker.generation).unwrap_or_default(),
        status: worker.status,
        status_reason: worker.status_reason,
        replaced_by_worker_id: worker.replaced_by_worker_id,
        last_sequence: u64::try_from(worker.last_sequence).unwrap_or_default(),
    }
}

fn parse_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn parse_capabilities_summary(value: &str) -> WorkerCapabilitiesSummary {
    serde_json::from_str(value).unwrap_or_default()
}

fn parse_master_summary(value: &str) -> Option<WorkerMasterSummary> {
    serde_json::from_str(value).ok()
}

fn worker_pool_from_labels_json(value: &str) -> Option<String> {
    let labels = serde_json::from_str::<HashMap<String, String>>(value).ok()?;
    worker_pool_from_labels(&labels)
}

fn worker_pool_from_labels(labels: &HashMap<String, String>) -> Option<String> {
    labels
        .get("worker_pool")
        .or_else(|| labels.get("worker-pool"))
        .cloned()
}

fn worker_capabilities_summary(
    capabilities: &tikeo_proto::worker::v1::WorkerCapabilities,
) -> WorkerCapabilitiesSummary {
    WorkerCapabilitiesSummary {
        tags: capabilities.tags.clone(),
        sdk_processors: capabilities
            .sdk_processors
            .iter()
            .map(|processor| processor.name.clone())
            .filter(|name| !name.trim().is_empty())
            .collect(),
        script_runners: capabilities
            .script_runners
            .iter()
            .map(|runner| WorkerScriptRunnerSummary {
                language: runner.language.clone(),
                sandbox_backend: runner.sandbox_backend.clone(),
            })
            .collect(),
        plugin_processors: capabilities
            .plugin_processors
            .iter()
            .map(|processor| WorkerPluginProcessorSummary {
                r#type: processor.r#type.clone(),
                processor_names: processor.processor_names.clone(),
            })
            .collect(),
    }
}

#[utoipa::path(get, path = "/api/v1/dispatch-queue", tag = "workers")]
pub async fn dispatch_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<DispatchQueueApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workers", "read").await?;
    let queue = state
        .workflows
        .queue_overview(100)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(queue)))
}

#[utoipa::path(post, path = "/api/v1/dispatch-queue:claim", tag = "workers", request_body = ClaimDispatchQueueRequest)]
pub async fn claim_dispatch_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ClaimDispatchQueueRequest>,
) -> Result<Json<DispatchQueueClaimApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workers", "manage").await?;
    if request.lease_owner.trim().is_empty() {
        return Err(ApiError::bad_request("lease_owner cannot be empty"));
    }
    let lease_owner = request.lease_owner.trim().to_owned();
    let requested_fencing_token = request
        .fencing_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let claim = state
        .workflows
        .claim_next_dispatch_queue_item_with_fencing(
            &lease_owner,
            request.lease_seconds.unwrap_or(30),
            requested_fencing_token,
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("no claimable dispatch queue item found"))?;
    audit(
        &state,
        &principal.username,
        "claim",
        "dispatch_queue",
        &claim.item.id,
        Some(format!(
            "lease_owner={} lease_until={} fencing_token={}",
            claim.lease_owner, claim.lease_until, claim.fencing_token
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(claim)))
}
