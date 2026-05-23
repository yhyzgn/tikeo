#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use tikee_storage::ScopeRepository;

use crate::http::{AppState, auth, dto::ApiResponse, error::ApiError};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateNamespaceRequest {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateAppRequest {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateWorkerPoolRequest {
    pub namespace: String,
    pub app: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
pub struct ScopeQuery {
    pub namespace: Option<String>,
    pub app: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/namespaces", tag = "tenancy")]
pub async fn list_namespaces(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<tikee_storage::NamespaceSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = ScopeRepository::new(state.users.db());
    let items = repo
        .list_namespaces()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/namespaces", tag = "tenancy", request_body = CreateNamespaceRequest)]
pub async fn create_namespace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateNamespaceRequest>,
) -> Result<Json<ApiResponse<tikee_storage::NamespaceSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let name = normalize_name(&request.name, "namespace")?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .create_namespace(&name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(get, path = "/api/v1/apps", tag = "tenancy", params(ScopeQuery))]
pub async fn list_apps(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<ApiResponse<Vec<tikee_storage::AppSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = ScopeRepository::new(state.users.db());
    let items = repo
        .list_apps(query.namespace.as_deref())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/apps", tag = "tenancy", request_body = CreateAppRequest)]
pub async fn create_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateAppRequest>,
) -> Result<Json<ApiResponse<tikee_storage::AppSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let namespace = normalize_name(&request.namespace, "namespace")?;
    let name = normalize_name(&request.name, "app")?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .create_app(&namespace, &name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(
    get,
    path = "/api/v1/worker-pools",
    tag = "tenancy",
    params(ScopeQuery)
)]
pub async fn list_worker_pools(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<ApiResponse<Vec<tikee_storage::WorkerPoolSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = ScopeRepository::new(state.users.db());
    let items = repo
        .list_worker_pools(query.namespace.as_deref(), query.app.as_deref())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/worker-pools", tag = "tenancy", request_body = CreateWorkerPoolRequest)]
pub async fn create_worker_pool(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkerPoolRequest>,
) -> Result<Json<ApiResponse<tikee_storage::WorkerPoolSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let namespace = normalize_name(&request.namespace, "namespace")?;
    let app = normalize_name(&request.app, "app")?;
    let name = normalize_name(&request.name, "worker_pool")?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .create_worker_pool(&namespace, &app, &name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(delete, path = "/api/v1/namespaces/{id}", tag = "tenancy", params(("id" = String, Path, description = "Namespace id")))]
pub async fn delete_namespace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    match repo.delete_namespace_if_empty(&id).await {
        Ok(true) => Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {}))),
        Ok(false) => Err(ApiError::not_found("namespace not found")),
        Err(error) if error.to_string().contains("not empty") => {
            Err(ApiError::bad_request("namespace is not empty"))
        }
        Err(error) => Err(ApiError::storage(&error)),
    }
}

#[utoipa::path(delete, path = "/api/v1/apps/{id}", tag = "tenancy", params(("id" = String, Path, description = "App id")))]
pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    match repo.delete_app_if_empty(&id).await {
        Ok(true) => Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {}))),
        Ok(false) => Err(ApiError::not_found("app not found")),
        Err(error) if error.to_string().contains("not empty") => {
            Err(ApiError::bad_request("app is not empty"))
        }
        Err(error) => Err(ApiError::storage(&error)),
    }
}

#[utoipa::path(delete, path = "/api/v1/worker-pools/{id}", tag = "tenancy", params(("id" = String, Path, description = "Worker pool id")))]
pub async fn delete_worker_pool(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    let deleted = repo
        .delete_worker_pool(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("worker pool not found"));
    }
    Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
}

fn normalize_name(value: &str, field: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_owned())
}
