use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, EmptyApiResponse, EmptyData},
    error::ApiError,
};

use super::common::audit;

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountRequest {
    pub name: String,
    pub description: Option<String>,
    pub namespace: String,
    pub app: String,
    /// Worker pool value.
    pub worker_pool: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServiceAccountRequest {
    pub name: String,
    pub description: Option<String>,
    pub namespace: String,
    pub app: String,
    /// Worker pool value.
    pub worker_pool: Option<String>,
    pub status: String,
}

/// `ServiceAccountApiResponse` type alias.
pub type ServiceAccountApiResponse = ApiResponse<tikeo_storage::ServiceAccountSummary>;
/// `ServiceAccountListApiResponse` type alias.
pub type ServiceAccountListApiResponse = ApiResponse<Vec<tikeo_storage::ServiceAccountSummary>>;

/// List service accounts.
///
/// # Errors
///
/// Returns authorization or storage errors.
#[utoipa::path(get, path = "/api/v1/management/service-accounts", tag = "management")]
/// List service accounts.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_service_accounts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ServiceAccountListApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let items = tikeo_storage::ServiceAccountRepository::new(state.users.db())
        .list()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

/// Create a service account.
///
/// # Errors
///
/// Returns validation, authorization, or storage errors.
#[utoipa::path(
    post,
    path = "/api/v1/management/service-accounts",
    tag = "management",
    request_body = CreateServiceAccountRequest
)]
/// Create service account.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn create_service_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateServiceAccountRequest>,
) -> Result<Json<ServiceAccountApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let request = validate_create_request(request)?;
    validate_worker_pool_scope(
        &state,
        &request.namespace,
        &request.app,
        request.worker_pool.as_deref(),
    )
    .await?;
    let created = tikeo_storage::ServiceAccountRepository::new(state.users.db())
        .create(tikeo_storage::CreateServiceAccount {
            name: request.name,
            description: request.description,
            namespace: request.namespace,
            app: request.app,
            worker_pool: request.worker_pool,
            created_by: principal.username.clone(),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    audit(
        &state,
        &principal.username,
        "service_account_create",
        "service_account",
        &created.id,
        Some(format!("scope={}/{}", created.namespace, created.app)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

/// Update a service account.
///
/// # Errors
///
/// Returns validation, authorization, not-found, or storage errors.
#[utoipa::path(
    patch,
    path = "/api/v1/management/service-accounts/{id}",
    tag = "management",
    request_body = UpdateServiceAccountRequest
)]
/// Update service account.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn update_service_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateServiceAccountRequest>,
) -> Result<Json<ServiceAccountApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let request = validate_update_request(request)?;
    validate_worker_pool_scope(
        &state,
        &request.namespace,
        &request.app,
        request.worker_pool.as_deref(),
    )
    .await?;
    let Some(updated) = tikeo_storage::ServiceAccountRepository::new(state.users.db())
        .update(
            &id,
            tikeo_storage::UpdateServiceAccount {
                name: request.name,
                description: request.description,
                namespace: request.namespace,
                app: request.app,
                worker_pool: request.worker_pool,
                status: request.status,
                updated_by: principal.username.clone(),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
    else {
        return Err(ApiError::not_found("service account not found"));
    };
    let api_keys = tikeo_storage::SdkApiKeyRepository::new(state.users.db());
    if updated.status == "disabled" {
        api_keys
            .revoke_keys_for_service_account(&id, &principal.username)
            .await
            .map_err(|error| ApiError::storage(&error))?;
    } else {
        api_keys
            .sync_keys_for_service_account(&id, &updated.namespace, &updated.app, &updated.name)
            .await
            .map_err(|error| ApiError::storage(&error))?;
    }
    audit(
        &state,
        &principal.username,
        "service_account_update",
        "service_account",
        &id,
        Some(format!("status={}", updated.status)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

/// Disable a service account and revoke all active API keys bound to it.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors.
#[utoipa::path(
    delete,
    path = "/api/v1/management/service-accounts/{id}",
    tag = "management"
)]
/// Disable service account.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn disable_service_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<EmptyApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let repo = tikeo_storage::ServiceAccountRepository::new(state.users.db());
    let disabled = repo
        .disable(&id, &principal.username)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !disabled {
        return Err(ApiError::not_found("service account not found"));
    }
    tikeo_storage::SdkApiKeyRepository::new(state.users.db())
        .revoke_keys_for_service_account(&id, &principal.username)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    audit(
        &state,
        &principal.username,
        "service_account_disable",
        "service_account",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

fn validate_create_request(
    mut request: CreateServiceAccountRequest,
) -> Result<CreateServiceAccountRequest, ApiError> {
    request.name = request.name.trim().to_owned();
    request.namespace = request.namespace.trim().to_owned();
    request.app = request.app.trim().to_owned();
    request.description = trim_optional(request.description);
    request.worker_pool = trim_optional(request.worker_pool);
    if request.name.is_empty() || request.namespace.is_empty() || request.app.is_empty() {
        return Err(ApiError::bad_request(
            "service account name, namespace and app are required",
        ));
    }
    Ok(request)
}

fn validate_update_request(
    mut request: UpdateServiceAccountRequest,
) -> Result<UpdateServiceAccountRequest, ApiError> {
    request.name = request.name.trim().to_owned();
    request.namespace = request.namespace.trim().to_owned();
    request.app = request.app.trim().to_owned();
    request.status = request.status.trim().to_owned();
    request.description = trim_optional(request.description);
    request.worker_pool = trim_optional(request.worker_pool);
    if request.name.is_empty() || request.namespace.is_empty() || request.app.is_empty() {
        return Err(ApiError::bad_request(
            "service account name, namespace and app are required",
        ));
    }
    if !matches!(request.status.as_str(), "active" | "disabled") {
        return Err(ApiError::bad_request(
            "service account status must be active or disabled",
        ));
    }
    Ok(request)
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

async fn validate_worker_pool_scope(
    state: &AppState,
    namespace: &str,
    app: &str,
    worker_pool: Option<&str>,
) -> Result<(), ApiError> {
    let Some(worker_pool) = worker_pool.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let exists = state
        .jobs
        .scopes()
        .list_worker_pools(Some(namespace), Some(app))
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .any(|pool| pool.name == worker_pool);
    if !exists {
        return Err(ApiError::bad_request(
            "workerPool must belong to the target namespace/app",
        ));
    }
    Ok(())
}
