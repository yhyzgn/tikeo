use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use tikeo_storage::ScopeRepository;

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

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkerPoolQuotaRequest {
    /// Max queue depth value.
    pub max_queue_depth: i32,
    /// Max concurrency value.
    pub max_concurrency: i32,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
pub struct ScopeQuery {
    pub namespace: Option<String>,
    pub app: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/namespaces", tag = "tenancy")]
/// List namespaces.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_namespaces(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::NamespaceSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = ScopeRepository::new(state.users.db());
    let items = repo
        .list_namespaces()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/namespaces", tag = "tenancy", request_body = CreateNamespaceRequest)]
/// Create namespace.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn create_namespace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateNamespaceRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NamespaceSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let name = normalize_name(&request.name, "namespace")?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .create_namespace(&name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "namespace",
        &item.id,
        Some(format!("name={}", item.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(get, path = "/api/v1/apps", tag = "tenancy", params(ScopeQuery))]
/// List apps.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_apps(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::AppSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = ScopeRepository::new(state.users.db());
    let items = repo
        .list_apps(query.namespace.as_deref())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/apps", tag = "tenancy", request_body = CreateAppRequest)]
/// Create app.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn create_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateAppRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::AppSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let namespace = normalize_name(&request.namespace, "namespace")?;
    let name = normalize_name(&request.name, "app")?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .create_app(&namespace, &name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "app",
        &item.id,
        Some(format!("{}/{}", item.namespace, item.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(
    get,
    path = "/api/v1/worker-pools",
    tag = "tenancy",
    params(ScopeQuery)
)]
/// List worker pools.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_worker_pools(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::WorkerPoolSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = ScopeRepository::new(state.users.db());
    let items = repo
        .list_worker_pools(query.namespace.as_deref(), query.app.as_deref())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/worker-pools", tag = "tenancy", request_body = CreateWorkerPoolRequest)]
/// Create worker pool.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn create_worker_pool(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkerPoolRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::WorkerPoolSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let namespace = normalize_name(&request.namespace, "namespace")?;
    let app = normalize_name(&request.app, "app")?;
    let name = normalize_name(&request.name, "worker_pool")?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .create_worker_pool(&namespace, &app, &name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "worker_pool",
        &item.id,
        Some(format!("{}/{}/{}", item.namespace, item.app, item.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(patch, path = "/api/v1/worker-pools/{id}/quota", tag = "tenancy", request_body = UpdateWorkerPoolQuotaRequest, params(("id" = String, Path, description = "Worker pool id")))]
/// Update worker pool quota.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn update_worker_pool_quota(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<UpdateWorkerPoolQuotaRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::WorkerPoolSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    let item = repo
        .update_worker_pool_quota(
            &id,
            tikeo_storage::UpdateWorkerPoolQuota {
                max_queue_depth: request.max_queue_depth,
                max_concurrency: request.max_concurrency,
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("worker pool not found"))?;
    super::common::audit(
        &state,
        &principal.username,
        "update",
        "worker_pool",
        &item.id,
        Some(format!(
            "maxQueueDepth={:?} maxConcurrency={:?}",
            item.max_queue_depth, item.max_concurrency
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(delete, path = "/api/v1/namespaces/{id}", tag = "tenancy", params(("id" = String, Path, description = "Namespace id")))]
/// Delete namespace.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn delete_namespace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    match repo.delete_namespace_if_empty(&id).await {
        Ok(true) => {
            super::common::audit(
                &state,
                &principal.username,
                "delete",
                "namespace",
                &id,
                None,
                &headers,
            )
            .await;
            Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
        }
        Ok(false) => Err(ApiError::not_found("namespace not found")),
        Err(error) if error.to_string().contains("not empty") => {
            Err(ApiError::bad_request("namespace is not empty"))
        }
        Err(error) => Err(ApiError::storage(&error)),
    }
}

#[utoipa::path(delete, path = "/api/v1/apps/{id}", tag = "tenancy", params(("id" = String, Path, description = "App id")))]
/// Delete app.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    match repo.delete_app_if_empty(&id).await {
        Ok(true) => {
            super::common::audit(
                &state,
                &principal.username,
                "delete",
                "app",
                &id,
                None,
                &headers,
            )
            .await;
            Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
        }
        Ok(false) => Err(ApiError::not_found("app not found")),
        Err(error) if error.to_string().contains("not empty") => {
            Err(ApiError::bad_request("app is not empty"))
        }
        Err(error) => Err(ApiError::storage(&error)),
    }
}

#[utoipa::path(delete, path = "/api/v1/worker-pools/{id}", tag = "tenancy", params(("id" = String, Path, description = "Worker pool id")))]
/// Delete worker pool.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn delete_worker_pool(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = ScopeRepository::new(state.users.db());
    let deleted = repo
        .delete_worker_pool(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("worker pool not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "worker_pool",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
}

fn normalize_name(value: &str, field: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_owned())
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretRequest {
    pub namespace: String,
    pub app: String,
    pub name: String,
    pub reference: SecretReferenceRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretReferenceRequest {
    Env {
        name: String,
    },
    Vault {
        path: String,
        key: String,
    },
    Secret {
        provider: String,
        id: String,
        key: Option<String>,
    },
}

#[utoipa::path(get, path = "/api/v1/secrets", tag = "tenancy", params(ScopeQuery))]
/// List secrets.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_secrets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::SecretSummary>>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "read").await?;
    let repo = tikeo_storage::SecretRepository::new(state.users.db());
    let mut items = repo
        .list(query.namespace.as_deref(), query.app.as_deref())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    items.retain(|secret| {
        crate::http::access_scope::allows_resource(
            &principal.scope_bindings,
            &secret.namespace,
            &secret.app,
            None,
        )
    });
    super::common::audit(
        &state,
        &principal.username,
        "read",
        "secret",
        query.namespace.as_deref().unwrap_or("*"),
        Some(format!(
            "namespace={} app={} count={}",
            query.namespace.as_deref().unwrap_or("*"),
            query.app.as_deref().unwrap_or("*"),
            items.len()
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/secrets", tag = "tenancy", request_body = CreateSecretRequest)]
/// Create secret.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn create_secret(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateSecretRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::SecretSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let namespace = normalize_name(&request.namespace, "namespace")?;
    let app = normalize_name(&request.app, "app")?;
    let name = normalize_name(&request.name, "secret")?;
    let value_ref = normalize_secret_reference(request.reference)?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &namespace,
        &app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "scope binding does not allow this namespace/app",
        ));
    }
    let repo = tikeo_storage::SecretRepository::new(state.users.db());
    let item = repo
        .create(tikeo_storage::CreateSecret {
            namespace,
            app,
            name,
            value_ref,
            created_by: principal.username.clone(),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "secret",
        &item.id,
        Some(format!("{}/{}:{}", item.namespace, item.app, item.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(delete, path = "/api/v1/secrets/{id}", tag = "tenancy", params(("id" = String, Path, description = "Secret id")))]
/// Delete secret.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn delete_secret(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = tikeo_storage::SecretRepository::new(state.users.db());
    let deleted = repo
        .delete(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("secret not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "secret",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
}

fn normalize_secret_reference(reference: SecretReferenceRequest) -> Result<String, ApiError> {
    let value = serde_json::to_string(&normalize_secret_reference_value(reference)?)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(value)
}

fn normalize_secret_reference_value(
    reference: SecretReferenceRequest,
) -> Result<serde_json::Value, ApiError> {
    match reference {
        SecretReferenceRequest::Env { name } => Ok(serde_json::json!({
            "kind": "env",
            "name": normalize_name(&name, "env secret name")?,
        })),
        SecretReferenceRequest::Vault { path, key } => Ok(serde_json::json!({
            "kind": "vault",
            "path": normalize_non_empty(&path, "vault path")?,
            "key": normalize_name(&key, "vault key")?,
        })),
        SecretReferenceRequest::Secret { provider, id, key } => {
            let mut value = serde_json::json!({
                "kind": "secret",
                "provider": normalize_name(&provider, "secret provider")?,
                "id": normalize_non_empty(&id, "secret id")?,
            });
            if let Some(key) = normalize_optional(key) {
                value["key"] = serde_json::Value::String(key);
            }
            Ok(value)
        }
    }
}

fn normalize_non_empty(value: &str, field: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_owned())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}
