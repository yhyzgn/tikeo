//! App-scoped SDK management API key endpoints and authentication helpers.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tikee_storage::{
    CreateSdkApiKey, PermissionSummary, SdkApiKeyRepository, SdkApiKeySummary,
    ServiceAccountRepository, UpdateSdkApiKey,
};
use utoipa::ToSchema;

use super::{
    AppState, auth,
    dto::{AccessScopeBinding, ApiResponse, EmptyApiResponse, EmptyData, MeResponse},
    error::ApiError,
    opaque_token::generate_base62,
    routes::{client_ip, trace_id},
};

const KEY_PREFIX: &str = "tk-";
const KEY_RANDOM_LENGTH: usize = 64;

/// SDK API key creation request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSdkApiKeyRequest {
    /// Human-readable key name.
    pub name: String,
    /// Namespace scope granted to this SDK key.
    pub namespace: String,
    /// App scope granted to this SDK key.
    pub app: String,
    /// Existing service account id represented by this SDK key.
    pub service_account_id: String,
    /// Permission scopes in `resource:action` form.
    pub scopes: Vec<String>,
    /// Optional RFC3339 expiration timestamp.
    pub expires_at: Option<String>,
}

/// SDK API key creation response. The plaintext API key is returned only once.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedSdkApiKey {
    /// Redacted metadata.
    pub key: SdkApiKeySummary,
    /// Plaintext API key. Copy immediately; only the hash is persisted.
    pub api_key: String,
}

/// SDK API key metadata update request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateSdkApiKeyRequest {
    /// Human-readable key name.
    pub name: String,
    /// Replacement permission scopes in `resource:action` form.
    pub scopes: Vec<String>,
    /// Optional RFC3339 expiration timestamp. `null` means permanent.
    pub expires_at: Option<String>,
}

/// Create an app-scoped SDK management API key.
///
/// # Errors
///
/// Returns unauthorized/forbidden for non-admin callers or storage errors.
#[utoipa::path(
    post,
    path = "/api/v1/management/api-keys",
    tag = "management",
    request_body = CreateSdkApiKeyRequest
)]
pub async fn create_sdk_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateSdkApiKeyRequest>,
) -> Result<Json<ApiResponse<CreatedSdkApiKey>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let request = validate_create_request(request)?;
    let service_account = ServiceAccountRepository::new(state.users.db())
        .get(&request.service_account_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::bad_request("service account not found"))?;
    if service_account.status != "active" {
        return Err(ApiError::bad_request("service account is disabled"));
    }
    if service_account.namespace != request.namespace || service_account.app != request.app {
        return Err(ApiError::bad_request(
            "sdk api key scope must match the selected service account",
        ));
    }
    let plaintext = generate_api_key()?;
    let repository = SdkApiKeyRepository::new(state.users.db());
    let key = repository
        .create_key(CreateSdkApiKey {
            name: request.name,
            key_hash: hash_api_key(&plaintext),
            key_prefix: display_prefix(&plaintext),
            namespace: service_account.namespace,
            app: service_account.app,
            service_account_id: service_account.id,
            service_account_name: service_account.name,
            scopes: request.scopes,
            expires_at: request.expires_at,
            created_by: principal.username.clone(),
            rotated_from: None,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    audit_sdk_api_key(
        &state,
        &principal.username,
        "sdk_api_key_create",
        &key.id,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(CreatedSdkApiKey {
        key,
        api_key: plaintext,
    })))
}

/// List SDK API key metadata without exposing plaintext or hashes.
///
/// # Errors
///
/// Returns unauthorized/forbidden for non-admin callers or storage errors.
#[utoipa::path(get, path = "/api/v1/management/api-keys", tag = "management")]
pub async fn list_sdk_api_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<SdkApiKeySummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let keys = SdkApiKeyRepository::new(state.users.db())
        .list_keys()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(keys)))
}

/// Update SDK API key permissions and expiration without changing the key value.
///
/// # Errors
///
/// Returns unauthorized/forbidden for non-admin callers, not found, bad request, or storage errors.
#[utoipa::path(
    patch,
    path = "/api/v1/management/api-keys/{id}",
    tag = "management",
    request_body = UpdateSdkApiKeyRequest
)]
pub async fn update_sdk_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateSdkApiKeyRequest>,
) -> Result<Json<ApiResponse<SdkApiKeySummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let request = validate_update_request(request)?;
    let Some(updated) = SdkApiKeyRepository::new(state.users.db())
        .update_key(
            &id,
            UpdateSdkApiKey {
                name: request.name,
                scopes: request.scopes,
                expires_at: request.expires_at,
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
    else {
        return Err(ApiError::not_found("active sdk api key not found"));
    };
    audit_sdk_api_key(
        &state,
        &principal.username,
        "sdk_api_key_update",
        &id,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

/// Revoke one SDK API key.
///
/// # Errors
///
/// Returns unauthorized/forbidden for non-admin callers, not found, or storage errors.
#[utoipa::path(delete, path = "/api/v1/management/api-keys/{id}", tag = "management")]
pub async fn revoke_sdk_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<EmptyApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let revoked = SdkApiKeyRepository::new(state.users.db())
        .revoke_key(&id, &principal.username)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !revoked {
        return Err(ApiError::not_found("sdk api key not found"));
    }
    audit_sdk_api_key(
        &state,
        &principal.username,
        "sdk_api_key_revoke",
        &id,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

pub(super) async fn authenticate_sdk_api_key(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<Option<MeResponse>, ApiError> {
    let Some(api_key) = sdk_api_key_header(headers)? else {
        return Ok(None);
    };
    if !api_key.starts_with(KEY_PREFIX) {
        return Err(ApiError::unauthorized("invalid sdk api key"));
    }
    let repository = SdkApiKeyRepository::new(state.users.db());
    let Some(summary) = repository
        .get_active_by_hash(&hash_api_key(&api_key))
        .await
        .map_err(|error| ApiError::storage(&error))?
    else {
        return Err(ApiError::unauthorized("invalid sdk api key"));
    };
    let service_account = ServiceAccountRepository::new(state.users.db())
        .get(&summary.service_account_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .filter(|account| account.status == "active")
        .ok_or_else(|| ApiError::unauthorized("invalid sdk api key"))?;
    repository
        .mark_used(&summary.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    audit_sdk_api_key(
        state,
        &format!("service_account:{}", summary.service_account_id),
        "sdk_api_key_authenticate",
        &summary.id,
        headers,
    )
    .await;
    Ok(Some(sdk_principal(summary, service_account)))
}

fn validate_create_request(
    mut request: CreateSdkApiKeyRequest,
) -> Result<CreateSdkApiKeyRequest, ApiError> {
    request.name = request.name.trim().to_owned();
    request.namespace = request.namespace.trim().to_owned();
    request.app = request.app.trim().to_owned();
    request.service_account_id = request.service_account_id.trim().to_owned();
    request.scopes = validate_scopes(request.scopes)?;
    request.expires_at = validate_expires_at(request.expires_at)?;
    if request.name.is_empty() {
        return Err(ApiError::bad_request("sdk api key name is required"));
    }
    if request.namespace.is_empty() || request.app.is_empty() {
        return Err(ApiError::bad_request(
            "sdk api key namespace and app are required",
        ));
    }
    if request.service_account_id.is_empty() {
        return Err(ApiError::bad_request(
            "sdk api key service_account_id is required",
        ));
    }
    Ok(request)
}

fn validate_update_request(
    mut request: UpdateSdkApiKeyRequest,
) -> Result<UpdateSdkApiKeyRequest, ApiError> {
    request.name = request.name.trim().to_owned();
    request.scopes = validate_scopes(request.scopes)?;
    request.expires_at = validate_expires_at(request.expires_at)?;
    if request.name.is_empty() {
        return Err(ApiError::bad_request("sdk api key name is required"));
    }
    Ok(request)
}

fn validate_scopes(scopes: Vec<String>) -> Result<Vec<String>, ApiError> {
    let scopes = scopes
        .into_iter()
        .map(|scope| scope.trim().to_owned())
        .filter(|scope| !scope.is_empty())
        .collect::<Vec<_>>();
    if scopes.is_empty() {
        return Err(ApiError::bad_request("sdk api key scopes are required"));
    }
    Ok(scopes)
}

fn validate_expires_at(expires_at: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(expires_at) = expires_at
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    time::OffsetDateTime::parse(&expires_at, &time::format_description::well_known::Rfc3339)
        .map_err(|_| ApiError::bad_request("expires_at must be RFC3339"))?;
    Ok(Some(expires_at))
}

fn sdk_api_key_header(headers: &HeaderMap) -> Result<Option<String>, ApiError> {
    headers
        .get("x-tikee-api-key")
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .map_err(|_| ApiError::unauthorized("invalid x-tikee-api-key header"))
        })
        .transpose()
}

fn sdk_principal(
    summary: SdkApiKeySummary,
    service_account: tikee_storage::ServiceAccountSummary,
) -> MeResponse {
    let permissions = permissions_from_scopes(&summary.scopes);
    MeResponse {
        username: format!("service_account:{}", summary.service_account_id),
        roles: vec![
            "service_account".to_owned(),
            "sdk_api_key".to_owned(),
            "app_service".to_owned(),
        ],
        permissions,
        bootstrap_admin: false,
        scope_limited: true,
        token_scopes: summary.scopes,
        scope_bindings: vec![AccessScopeBinding {
            namespace: Some(service_account.namespace),
            app: Some(service_account.app),
            worker_pool: None,
        }],
        menu_keys: Vec::new(),
        ui_action_keys: Vec::new(),
    }
}

fn permissions_from_scopes(scopes: &[String]) -> Vec<PermissionSummary> {
    scopes
        .iter()
        .filter_map(|scope| {
            let (resource, action) = scope.split_once(':')?;
            Some(PermissionSummary {
                resource: resource.to_owned(),
                action: action.to_owned(),
            })
        })
        .collect()
}

fn generate_api_key() -> Result<String, ApiError> {
    let mut random = String::with_capacity(KEY_PREFIX.len() + KEY_RANDOM_LENGTH);
    random.push_str(KEY_PREFIX);
    random.push_str(&generate_base62(KEY_RANDOM_LENGTH)?);
    Ok(random)
}

fn display_prefix(api_key: &str) -> String {
    let prefix: String = api_key.chars().take(12).collect();
    let suffix = api_key
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}••••••••••••{suffix}")
}

fn hash_api_key(api_key: &str) -> String {
    hex::encode(Sha256::digest(api_key.as_bytes()))
}

async fn audit_sdk_api_key(
    state: &AppState,
    actor: &str,
    action: &str,
    key_id: &str,
    headers: &HeaderMap,
) {
    if let Err(error) = state
        .audit
        .append(tikee_storage::CreateAuditLog {
            actor: actor.to_owned(),
            action: action.to_owned(),
            resource_type: "sdk_api_key".to_owned(),
            resource_id: key_id.to_owned(),
            detail: None,
            before: None,
            after: None,
            trace_id: Some(trace_id(headers)),
            result: "success".to_owned(),
            failure_reason: None,
            ip_address: client_ip(headers),
        })
        .await
    {
        tracing::warn!(%error, "failed to append sdk api key audit log");
    }
}

#[cfg(test)]
mod tests {
    use super::{display_prefix, generate_api_key};

    #[test]
    fn generated_api_key_uses_tk_base62_shape() {
        let key =
            generate_api_key().unwrap_or_else(|error| panic!("key should generate: {error:?}"));
        assert!(key.starts_with("tk-"));
        assert_eq!(key.len(), 67);
        assert!(
            key[3..]
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        );
    }

    #[test]
    fn display_prefix_masks_middle_and_keeps_both_ends() {
        let display =
            display_prefix("tk-ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789AB");
        assert_eq!(display, "tk-ABCDEFGHI••••••••••••456789AB");
    }
}
