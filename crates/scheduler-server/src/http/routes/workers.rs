#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use serde::Deserialize;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, DispatchQueueApiResponse, DispatchQueueClaimApiResponse,
        WorkerListApiResponse, WorkerListResponse, WorkerSummary,
    },
    error::ApiError,
};

use super::common::audit;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ClaimDispatchQueueRequest {
    pub lease_owner: String,
    pub lease_seconds: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/workers", tag = "workers")]
pub async fn list_workers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkerListApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workers", "read").await?;
    let workers = state.registry.workers().await;
    let items = workers
        .into_iter()
        .map(|worker| WorkerSummary {
            worker_id: worker.worker_id,
            app: worker.app,
            namespace: worker.namespace,
            cluster: worker.cluster,
            region: worker.region,
            capabilities: worker.capabilities,
            last_sequence: worker.last_sequence,
        })
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::success(WorkerListResponse {
        online: items.len(),
        items,
    })))
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
    let claim = state
        .workflows
        .claim_next_dispatch_queue_item(&lease_owner, request.lease_seconds.unwrap_or(30))
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
            "lease_owner={} lease_until={}",
            claim.lease_owner, claim.lease_until
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(claim)))
}
