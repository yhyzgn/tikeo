use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
    response::sse::{Event, Sse},
};
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, time};
use tokio_stream::{Stream, wrappers::ReceiverStream};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, DispatchQueueApiResponse, DispatchQueueClaimApiResponse, MeResponse,
        WorkerCapabilitiesSummary, WorkerLifecycleHistoryApiResponse,
        WorkerLifecycleHistoryResponse, WorkerListApiResponse, WorkerListResponse,
        WorkerMasterSummary, WorkerSessionEventDto, WorkerSessionHistorySummary, WorkerSummary,
    },
    error::ApiError,
};

use super::common::{StreamAuthQuery, apply_stream_token, audit};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ClaimDispatchQueueRequest {
    /// Lease owner value.
    pub lease_owner: String,
    /// Lease seconds value.
    pub lease_seconds: Option<i64>,
    /// Fencing token value.
    pub fencing_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStreamSnapshot {
    pub workers: WorkerListResponse,
    pub history: WorkerLifecycleHistoryResponse,
}

#[utoipa::path(get, path = "/api/v1/workers", tag = "workers")]
/// List workers.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_workers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkerListApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workers", "read").await?;
    Ok(Json(ApiResponse::success(
        worker_list_response(&state, &principal).await?,
    )))
}

async fn worker_list_response(
    state: &AppState,
    principal: &MeResponse,
) -> Result<WorkerListResponse, ApiError> {
    let persisted_workers = state
        .worker_lifecycle
        .list_online_workers(500)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut items = Vec::with_capacity(persisted_workers.len());

    for persisted in persisted_workers {
        let worker_pool = worker_pool_from_labels_json(&persisted.labels_json);
        items.push((persisted_worker_summary(persisted), worker_pool));
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
    Ok(WorkerListResponse {
        online: items.len(),
        items,
    })
}

#[utoipa::path(get, path = "/api/v1/workers/history", tag = "workers")]
/// Worker lifecycle history.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn worker_lifecycle_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkerLifecycleHistoryApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workers", "read").await?;
    Ok(Json(ApiResponse::success(
        worker_lifecycle_history_response(&state).await?,
    )))
}

async fn worker_lifecycle_history_response(
    state: &AppState,
) -> Result<WorkerLifecycleHistoryResponse, ApiError> {
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
    Ok(WorkerLifecycleHistoryResponse { sessions, events })
}

/// Stream workers.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn stream_workers(
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    Query(query): Query<StreamAuthQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    apply_stream_token(&mut headers, &query)?;
    let principal = auth::require_permission(&headers, &state, "workers", "read").await?;
    let (tx, rx) = mpsc::channel(16);

    tokio::spawn(async move {
        let mut last_snapshot_json: Option<String> = None;
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            if let Ok(snapshot) = worker_stream_snapshot(&state, &principal).await
                && let Ok(snapshot_json) = serde_json::to_string(&snapshot)
                && last_snapshot_json.as_deref() != Some(snapshot_json.as_str())
            {
                last_snapshot_json = Some(snapshot_json.clone());
                if tx
                    .send(Ok::<_, Infallible>(
                        Event::default()
                            .event("workers.snapshot")
                            .data(snapshot_json),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            interval.tick().await;
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}

async fn worker_stream_snapshot(
    state: &AppState,
    principal: &MeResponse,
) -> Result<WorkerStreamSnapshot, ApiError> {
    Ok(WorkerStreamSnapshot {
        workers: worker_list_response(state, principal).await?,
        history: worker_lifecycle_history_response(state).await?,
    })
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
    let mut summary: WorkerCapabilitiesSummary = serde_json::from_str(value).unwrap_or_default();
    for plugin in &mut summary.plugin_processors {
        if plugin.processors.is_empty() {
            plugin.processors = plugin
                .processor_names
                .iter()
                .map(|name| crate::http::dto::WorkerProcessorSummary {
                    name: name.clone(),
                    description: String::new(),
                })
                .collect();
        }
    }
    summary
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

#[utoipa::path(get, path = "/api/v1/dispatch-queue", tag = "workers")]
/// Dispatch queue.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn dispatch_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<DispatchQueueApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workers", "read").await?;
    Ok(Json(ApiResponse::success(
        dispatch_queue_response(&state).await?,
    )))
}

async fn dispatch_queue_response(
    state: &AppState,
) -> Result<tikeo_storage::QueueOverview, ApiError> {
    let queue = state
        .workflows
        .queue_overview(100)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(queue)
}

/// Stream dispatch queue.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn stream_dispatch_queue(
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    Query(query): Query<StreamAuthQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    apply_stream_token(&mut headers, &query)?;
    auth::require_permission(&headers, &state, "workers", "read").await?;
    let (tx, rx) = mpsc::channel(16);

    tokio::spawn(async move {
        let mut last_snapshot_json: Option<String> = None;
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            if let Ok(queue) = dispatch_queue_response(&state).await
                && let Ok(snapshot_json) = serde_json::to_string(&queue)
                && last_snapshot_json.as_deref() != Some(snapshot_json.as_str())
            {
                last_snapshot_json = Some(snapshot_json.clone());
                if tx
                    .send(Ok::<_, Infallible>(
                        Event::default()
                            .event("dispatchQueue.snapshot")
                            .data(snapshot_json),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            interval.tick().await;
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}

#[utoipa::path(post, path = "/api/v1/dispatch-queue:claim", tag = "workers", request_body = ClaimDispatchQueueRequest)]
/// Claim dispatch queue.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
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
