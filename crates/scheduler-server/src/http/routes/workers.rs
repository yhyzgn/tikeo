#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, DispatchQueueApiResponse, WorkerListApiResponse, WorkerListResponse,
        WorkerSummary,
    },
    error::ApiError,
};

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
