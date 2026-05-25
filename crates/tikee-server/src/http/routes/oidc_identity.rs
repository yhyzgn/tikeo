#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use serde::Deserialize;
use tikee_storage::{OidcIdentityRepository, UpsertOidcIdentity};

use crate::http::{AppState, auth, dto::ApiResponse, error::ApiError};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpsertOidcIdentityRequest {
    pub issuer: String,
    pub subject: String,
    pub username: String,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/oidc-identities", tag = "auth")]
pub async fn list_oidc_identities(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<tikee_storage::OidcIdentitySummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let items = OidcIdentityRepository::new(state.users.db())
        .list_identities()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/oidc-identities", tag = "auth", request_body = UpsertOidcIdentityRequest)]
pub async fn upsert_oidc_identity(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<UpsertOidcIdentityRequest>,
) -> Result<Json<ApiResponse<tikee_storage::OidcIdentitySummary>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let input = normalize_request(request)?;
    let item = OidcIdentityRepository::new(state.users.db())
        .upsert_identity(input)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(delete, path = "/api/v1/oidc-identities/{id}", tag = "auth", params(("id" = String, Path, description = "OIDC identity mapping id")))]
pub async fn delete_oidc_identity(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<crate::http::dto::EmptyData>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let deleted = OidcIdentityRepository::new(state.users.db())
        .delete_identity(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("OIDC identity mapping not found"));
    }
    Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
}

fn normalize_request(request: UpsertOidcIdentityRequest) -> Result<UpsertOidcIdentity, ApiError> {
    Ok(UpsertOidcIdentity {
        issuer: required(&request.issuer, "issuer")?,
        subject: required(&request.subject, "subject")?,
        username: required(&request.username, "username")?,
        namespace: optional_part(request.namespace, "namespace")?,
        app: optional_part(request.app, "app")?,
        worker_pool: optional_part(request.worker_pool, "worker_pool")?,
    })
}

fn required(value: &str, field: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_owned())
}

fn optional_part(value: Option<String>, field: &str) -> Result<Option<String>, ApiError> {
    value.map_or(Ok(None), |value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if trimmed
            .chars()
            .any(|character| matches!(character, ',' | ';' | '|'))
        {
            return Err(ApiError::bad_request(format!(
                "OIDC {field} binding cannot contain ',', ';', or '|'"
            )));
        }
        Ok(Some(trimmed.to_owned()))
    })
}
