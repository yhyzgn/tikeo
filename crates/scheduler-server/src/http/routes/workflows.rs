#![allow(missing_docs, clippy::missing_errors_doc)]

use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::sse::{Event, Sse},
};
use scheduler_storage::{
    AdvanceWorkflowInput, CreateWorkflow, WorkflowDefinition, validate_workflow_definition,
};
use serde::Deserialize;
use tokio_stream::Stream;
use tokio_stream::iter;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, WorkflowAdvanceApiResponse, WorkflowApiResponse, WorkflowDryRunApiResponse,
        WorkflowDryRunResponse, WorkflowInstanceApiResponse, WorkflowListApiResponse,
        WorkflowRunRequest, WorkflowValidationApiResponse,
    },
    error::ApiError,
};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateWorkflowRequest {
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
            created_by: principal.username,
        })
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(ApiResponse::success(created)))
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
    auth::require_permission(&headers, &state, "workflows", "read").await?;
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
    Ok(Json(ApiResponse::success(WorkflowDryRunResponse {
        validation: validate_workflow_definition(&definition),
        start_nodes,
        node_count: definition.nodes.len(),
        edge_count: definition.edges.len(),
    })))
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
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let item = state
        .workflows
        .get_workflow(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow not found: {id}")))?;
    Ok(Json(ApiResponse::success(validate_workflow_definition(
        &item.definition,
    ))))
}

#[utoipa::path(post, path = "/api/v1/workflows/{id}/run", tag = "workflows", request_body = WorkflowRunRequest)]
pub async fn run_workflow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<WorkflowRunRequest>,
) -> Result<Json<WorkflowInstanceApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let trigger_type = request.trigger_type.unwrap_or_else(|| "api".to_owned());
    let item = state
        .workflows
        .run_workflow(&id, &trigger_type)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow not found: {id}")))?;
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
    auth::require_permission(&headers, &state, "workflows", "execute").await?;
    let allowed_statuses = ["queued", "running", "succeeded", "failed", "skipped"];
    if !allowed_statuses.contains(&request.status.as_str()) {
        return Err(ApiError::bad_request(format!(
            "unsupported workflow node status: {}",
            request.status
        )));
    }
    let item = state
        .workflows
        .advance_workflow(&id, request)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow instance not found: {id}")))?;
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
    let events = state
        .workflows
        .list_instance_events(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let stream = iter(events.into_iter().map(|event| {
        Ok(Event::default()
            .event(event.event_type.clone())
            .data(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned())))
    }));
    Ok(Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}
