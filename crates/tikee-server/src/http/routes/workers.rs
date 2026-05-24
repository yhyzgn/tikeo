#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use serde::Deserialize;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, DispatchQueueApiResponse, DispatchQueueClaimApiResponse,
        WorkerLifecycleHistoryApiResponse, WorkerLifecycleHistoryResponse, WorkerListApiResponse,
        WorkerListResponse, WorkerSessionEventDto, WorkerSessionHistorySummary, WorkerSummary,
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
    let workers = state.registry.workers().await;
    let items = workers
        .into_iter()
        .filter(|worker| {
            let worker_pool = worker_pool_label(worker).unwrap_or_default();
            crate::http::access_scope::allows_resource(
                &principal.scope_bindings,
                &worker.namespace,
                &worker.app,
                Some(&worker_pool),
            )
        })
        .map(|worker| WorkerSummary {
            worker_id: worker.worker_id,
            logical_instance_id: worker.logical_instance_id,
            client_instance_id: worker.client_instance_id,
            app: worker.app,
            namespace: worker.namespace,
            cluster: worker.cluster,
            region: worker.region,
            capabilities: worker.capabilities,
            generation: worker.generation,
            status: worker.status.as_str().to_owned(),
            status_reason: worker.status_reason,
            replaced_by_worker_id: worker.replaced_by_worker_id,
            last_sequence: worker.last_sequence,
        })
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

fn worker_pool_label(worker: &crate::tunnel::RegisteredWorker) -> Option<String> {
    worker
        .labels
        .get("worker_pool")
        .or_else(|| worker.labels.get("worker-pool"))
        .cloned()
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
