#![allow(missing_docs, clippy::missing_errors_doc)]

use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::sse::{Event, Sse},
};
use serde::Deserialize;
use tikee_storage::{
    AdvanceWorkflowInput, CompleteWorkflowShardInput, CreateWorkflow, RecoverWorkflowNodeInput,
    UpdateWorkflow, WorkflowDefinition, validate_workflow_definition,
};
use tokio::{sync::mpsc, time};
use tokio_stream::{Stream, wrappers::ReceiverStream};

use super::common::audit;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, WorkflowAdvanceApiResponse, WorkflowApiResponse, WorkflowDryRunApiResponse,
        WorkflowDryRunResponse, WorkflowInstanceApiResponse, WorkflowListApiResponse,
        WorkflowMaterializeApiResponse, WorkflowRecoverApiResponse, WorkflowRunRequest,
        WorkflowShardCompleteApiResponse, WorkflowShardListApiResponse,
        WorkflowValidationApiResponse,
    },
    error::ApiError,
};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkflowRequest {
    pub name: String,
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StreamAuthQuery {
    pub token: Option<String>,
}

#[utoipa::path(post, path = "/api/v1/workflows", tag = "workflows", request_body = CreateWorkflowRequest)]
pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "manage").await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::bad_request("workflow name cannot be empty"));
    }
    let created = state
        .workflows
        .create_workflow(CreateWorkflow {
            name: request.name,
            definition: request.definition,
            created_by: principal.username.clone(),
        })
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    audit(
        &state,
        &principal.username,
        "create",
        "workflow",
        &created.id,
        Some(format!("name={}", created.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

#[utoipa::path(patch, path = "/api/v1/workflows/{id}", tag = "workflows", request_body = UpdateWorkflowRequest)]
pub async fn update_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateWorkflowRequest>,
) -> Result<Json<WorkflowApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "manage").await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::bad_request("workflow name cannot be empty"));
    }
    let updated = state
        .workflows
        .update_workflow(
            &id,
            UpdateWorkflow {
                name: request.name,
                definition: request.definition,
            },
        )
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("workflow not found: {id}")))?;
    audit(
        &state,
        &principal.username,
        "update",
        "workflow",
        &updated.id,
        Some(format!("name={}", updated.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

#[utoipa::path(get, path = "/api/v1/workflows", tag = "workflows")]
pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkflowListApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let items = state
        .workflows
        .list_workflows()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/workflows/dry-run", tag = "workflows", request_body = WorkflowDefinition)]
pub async fn dry_run_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(definition): Json<WorkflowDefinition>,
) -> Result<Json<WorkflowDryRunApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "read").await?;
    let target_nodes: std::collections::HashSet<&str> = definition
        .edges
        .iter()
        .map(|edge| edge.to.as_str())
        .collect();
    let start_nodes = definition
        .nodes
        .iter()
        .filter(|node| !target_nodes.contains(node.key.as_str()))
        .map(|node| node.key.clone())
        .collect();
    let response = WorkflowDryRunResponse {
        validation: validate_workflow_definition(&definition),
        start_nodes,
        node_count: definition.nodes.len(),
        edge_count: definition.edges.len(),
    };
    audit(
        &state,
        &principal.username,
        "dry-run",
        "workflow",
        "definition",
        Some(format!(
            "nodes={} edges={} valid={}",
            response.node_count, response.edge_count, response.validation.valid
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(response)))
}

#[utoipa::path(get, path = "/api/v1/workflows/{id}", tag = "workflows")]
pub async fn get_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<WorkflowApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let item = state
        .workflows
        .get_workflow(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow not found: {id}")))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(post, path = "/api/v1/workflows/{id}/validate", tag = "workflows")]
pub async fn validate_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<WorkflowValidationApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "read").await?;
    let item = state
        .workflows
        .get_workflow(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow not found: {id}")))?;
    let validation = validate_workflow_definition(&item.definition);
    audit(
        &state,
        &principal.username,
        "validate",
        "workflow",
        &item.id,
        Some(format!(
            "valid={} errors={}",
            validation.valid,
            validation.errors.len()
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(validation)))
}

#[utoipa::path(post, path = "/api/v1/workflows/{id}/run", tag = "workflows", request_body = WorkflowRunRequest)]
pub async fn run_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<WorkflowRunRequest>,
) -> Result<Json<WorkflowInstanceApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let trigger_type = request.trigger_type.unwrap_or_else(|| "api".to_owned());
    let item = state
        .workflows
        .run_workflow(&id, &trigger_type)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow not found: {id}")))?;
    audit(
        &state,
        &principal.username,
        "run",
        "workflow",
        &id,
        Some(format!("instance={} trigger_type={trigger_type}", item.id)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(get, path = "/api/v1/workflow-instances/{id}", tag = "workflows")]
pub async fn get_workflow_instance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<WorkflowInstanceApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let item = state
        .workflows
        .get_workflow_instance(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow instance not found: {id}")))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(post, path = "/api/v1/workflow-instances/{id}/advance", tag = "workflows", request_body = AdvanceWorkflowInput)]
pub async fn advance_workflow_instance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<AdvanceWorkflowInput>,
) -> Result<Json<WorkflowAdvanceApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let allowed_statuses = ["queued", "running", "succeeded", "failed", "skipped"];
    if !allowed_statuses.contains(&request.status.as_str()) {
        return Err(ApiError::bad_request(format!(
            "unsupported workflow node status: {}",
            request.status
        )));
    }
    let node_key = request.node_key.clone();
    let status = request.status.clone();
    let item = state
        .workflows
        .advance_workflow(&id, request)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow instance not found: {id}")))?;
    audit(
        &state,
        &principal.username,
        "advance",
        "workflow_instance",
        &id,
        Some(format!(
            "node={node_key} status={status} queued={}",
            item.queued_nodes.join(",")
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(
    post,
    path = "/api/v1/workflow-instances/materialize-next",
    tag = "workflows"
)]
pub async fn materialize_next_workflow_node(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WorkflowMaterializeApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let item = state
        .workflows
        .materialize_next_queued_node()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("no queued workflow node found"))?;
    audit(
        &state,
        &principal.username,
        "materialize",
        "workflow_node_instance",
        &item.node.id,
        Some(format!(
            "workflow_instance={} node={} shards={}",
            item.instance.id,
            item.node.node_key,
            item.shards.len()
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(post, path = "/api/v1/workflow-instances/{id}/recover", tag = "workflows", request_body = RecoverWorkflowNodeInput)]
pub async fn recover_workflow_node(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<RecoverWorkflowNodeInput>,
) -> Result<Json<WorkflowRecoverApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let node_key = request.node_key.clone();
    let action = request.action.clone();
    let item = state
        .workflows
        .recover_workflow_node(&id, request)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow instance not found: {id}")))?;
    audit(
        &state,
        &principal.username,
        "recover",
        "workflow_instance",
        &id,
        Some(format!(
            "node={node_key} action={action} queued={}",
            item.queued_nodes.join(",")
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(
    get,
    path = "/api/v1/workflow-instances/{id}/shards",
    tag = "workflows"
)]
pub async fn list_workflow_shards(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<WorkflowShardListApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let items = state
        .workflows
        .list_workflow_shards(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    post,
    path = "/api/v1/workflow-shards/{id}/complete",
    tag = "workflows",
    request_body = CompleteWorkflowShardInput
)]
pub async fn complete_workflow_shard(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<CompleteWorkflowShardInput>,
) -> Result<Json<WorkflowShardCompleteApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let status = request.status.clone();
    if !matches!(status.as_str(), "succeeded" | "failed") {
        return Err(ApiError::bad_request(format!(
            "unsupported workflow shard status: {status}"
        )));
    }
    let item = state
        .workflows
        .complete_workflow_shard(&id, request)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow shard not found: {id}")))?;
    audit(
        &state,
        &principal.username,
        "complete",
        "workflow_shard",
        &id,
        Some(format!(
            "status={} node_completed={} node_status={}",
            item.shard.status,
            item.node_completed,
            item.node_status
                .clone()
                .unwrap_or_else(|| "pending".to_owned())
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn stream_instance_events(
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<StreamAuthQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    if let Some(token) = query.token
        && !headers.contains_key(axum::http::header::AUTHORIZATION)
    {
        let value = format!("Bearer {token}")
            .parse()
            .map_err(|_| ApiError::unauthorized("invalid stream token"))?;
        headers.insert(axum::http::header::AUTHORIZATION, value);
    }
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let last_event_id = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let workflows = state.workflows.clone();
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        let mut seen = std::collections::HashSet::<String>::new();
        let mut after_last_event = last_event_id.is_none();
        if let Ok(events) = workflows.list_instance_events(&id).await {
            for event in events {
                seen.insert(event.id.clone());
                if !after_last_event {
                    after_last_event = last_event_id.as_deref() == Some(event.id.as_str());
                    continue;
                }
                let sse = Event::default()
                    .id(event.id.clone())
                    .event(event.event_type.clone())
                    .data(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned()));
                if tx.send(Ok::<_, Infallible>(sse)).await.is_err() {
                    return;
                }
            }
        }
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let Ok(events) = workflows.list_instance_events(&id).await else {
                continue;
            };
            for event in events {
                if !seen.insert(event.id.clone()) {
                    continue;
                }
                let sse = Event::default()
                    .id(event.id.clone())
                    .event(event.event_type.clone())
                    .data(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned()));
                if tx.send(Ok::<_, Infallible>(sse)).await.is_err() {
                    return;
                }
            }
        }
    });
    Ok(Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}
