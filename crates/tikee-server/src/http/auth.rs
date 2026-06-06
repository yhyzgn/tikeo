//! Authentication and Role-Based Access Control (RBAC) verification.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use bcrypt::{hash, verify};
use std::sync::Arc;
use tikee_storage::{CreateOidcAuthState, OidcAuthStateRepository};
use tracing::warn;

use super::{
    AppState,
    access_scope::validate_scope_bindings,
    dto::{
        ApiResponse, ApiTokenSummary, AuthSession, AuthStatusResponse, BootstrapRegisterRequest,
        BootstrapStatusResponse, CreateApiTokenRequest, CreatedApiToken, LoginRequest, MeResponse,
        OidcAuthorizeResponse, OidcStatus, RotateApiTokenRequest,
    },
    error::ApiError,
    routes::{client_ip, trace_id},
    session::SessionCreate,
};

/// Resolve authentication bearer token from headers.
///
/// # Errors
///
/// Returns unauthorized for missing/invalid bearer tokens or storage errors from the session store.
pub async fn authenticate(headers: &HeaderMap, state: &AppState) -> Result<MeResponse, ApiError> {
    if let Some(principal) = super::sdk_api_keys::authenticate_sdk_api_key(headers, state).await? {
        return Ok(principal);
    }
    let token = bearer_token(headers)?;

    state
        .sessions
        .get_principal(&token)
        .await?
        .ok_or_else(|| ApiError::unauthorized("invalid bearer token"))
}

fn bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    if let Some(value) = headers.get(axum::http::header::AUTHORIZATION) {
        let value = value
            .to_str()
            .map_err(|_| ApiError::unauthorized("invalid authorization header"))?;
        return value
            .strip_prefix("Bearer ")
            .map(str::to_owned)
            .ok_or_else(|| ApiError::unauthorized("authorization scheme must be Bearer"));
    }
    if let Some(value) = headers.get("x-tikee-token") {
        return value
            .to_str()
            .map(str::to_owned)
            .map_err(|_| ApiError::unauthorized("invalid x-tikee-token header"));
    }
    Err(ApiError::unauthorized(
        "missing bearer token or sdk api key",
    ))
}

/// Require the requester to have one of the required roles.
///
/// # Errors
///
/// Returns unauthorized when authentication fails or forbidden when the role is not allowed.
pub async fn require_role(
    headers: &HeaderMap,
    state: &AppState,
    allowed_roles: &[&str],
) -> Result<MeResponse, ApiError> {
    let principal = authenticate(headers, state).await?;
    if principal.bootstrap_admin
        || allowed_roles
            .iter()
            .any(|role| principal.roles.contains(&role.to_string()))
    {
        Ok(principal)
    } else {
        Err(ApiError::forbidden(format!(
            "requires roles: {allowed_roles:?}"
        )))
    }
}

/// Require the requester to have a resource/action permission.
///
/// # Errors
///
/// Returns unauthorized when authentication fails or forbidden when permission is missing.
pub async fn require_permission(
    headers: &HeaderMap,
    state: &AppState,
    resource: &str,
    action: &str,
) -> Result<MeResponse, ApiError> {
    let principal = authenticate(headers, state).await?;
    if state
        .rbac
        .principal_has_permission(&principal, resource, action)
    {
        Ok(principal)
    } else {
        Err(ApiError::forbidden(format!(
            "requires permission: {resource}:{action}"
        )))
    }
}

/// Helper requiring admin role.
///
/// # Errors
///
/// Returns unauthorized or forbidden when the requester is not an admin.
pub async fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<MeResponse, ApiError> {
    require_permission(headers, state, "users", "manage").await
}

/// Return one-time deployment bootstrap registration state.
///
/// # Errors
///
/// Returns a storage error when user lookup fails.
#[utoipa::path(
    get,
    path = "/api/v1/auth/bootstrap",
    tag = "auth",
    responses((status = 200, description = "Bootstrap registration state", body = super::dto::BootstrapStatusApiResponse))
)]
pub async fn bootstrap_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<BootstrapStatusResponse>>, ApiError> {
    let users = state
        .users
        .list_users()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let bootstrap_admin_username = users
        .iter()
        .find(|user| user.bootstrap_admin)
        .map(|user| user.username.clone());
    Ok(Json(ApiResponse::success(BootstrapStatusResponse {
        initialized: !users.is_empty(),
        registration_open: users.is_empty(),
        bootstrap_admin_username,
    })))
}

/// Register the first deployment administrator and create a persisted login session.
///
/// # Errors
///
/// Returns validation, conflict, or storage errors when bootstrap registration cannot proceed.
#[utoipa::path(
    post,
    path = "/api/v1/auth/bootstrap/register",
    tag = "auth",
    request_body = BootstrapRegisterRequest,
    responses(
        (status = 200, description = "Bootstrap admin session", body = super::dto::LoginApiResponse),
        (status = 400, description = "Invalid bootstrap registration", body = super::dto::ErrorResponse),
        (status = 403, description = "Registration already closed", body = super::dto::ErrorResponse)
    )
)]
pub async fn register_bootstrap_admin(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<BootstrapRegisterRequest>,
) -> Result<Json<ApiResponse<AuthSession>>, ApiError> {
    let username = request.username.trim();
    let email = request.email.trim();
    if username.is_empty() {
        return Err(ApiError::bad_request("username is required"));
    }
    if !is_valid_email(email) {
        return Err(ApiError::bad_request("valid email is required"));
    }
    if request.password.trim().is_empty() {
        return Err(ApiError::bad_request("password is required"));
    }
    if request.password != request.confirm_password {
        return Err(ApiError::bad_request(
            "password confirmation does not match",
        ));
    }
    if state
        .users
        .count_users()
        .await
        .map_err(|error| ApiError::storage(&error))?
        > 0
    {
        return Err(ApiError::forbidden("bootstrap registration is closed"));
    }

    let password_hash =
        hash(request.password, 10).map_err(|_| ApiError::bad_request("failed to hash password"))?;
    let created = state
        .users
        .create_user(tikee_storage::CreateUser {
            username: username.to_owned(),
            email: email.to_owned(),
            password: password_hash,
            role: "owner".to_owned(),
            bootstrap_admin: true,
        })
        .await
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                ApiError::forbidden("bootstrap registration is closed")
            } else {
                ApiError::storage(&error)
            }
        })?;

    let session = state
        .sessions
        .create_session(SessionCreate {
            user_id: created.id.clone(),
            username: created.username.clone(),
            role: created.role.clone(),
            device_id: None,
            device_name: None,
            token_scopes: Vec::new(),
            scope_bindings: Vec::new(),
            expires_in_seconds: None,
        })
        .await?;

    if let Err(error) = state
        .audit
        .append(tikee_storage::CreateAuditLog {
            actor: created.username,
            action: "bootstrap_admin_register".to_owned(),
            resource_type: "user".to_owned(),
            resource_id: created.id,
            detail: Some("one_time_bootstrap=true;role=owner".to_owned()),
            before: None,
            after: None,
            trace_id: Some(trace_id(&headers)),
            result: "success".to_owned(),
            failure_reason: None,
            ip_address: client_ip(&headers),
        })
        .await
    {
        warn!(%error, "failed to append bootstrap registration audit log");
    }

    Ok(Json(ApiResponse::success(session)))
}

fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.trim().is_empty() && domain.contains('.') && !domain.trim().is_empty()
}

/// Login with secure DB credentials and create a persisted session.
///
/// # Errors
///
/// Returns unauthorized for invalid credentials or storage errors when session creation fails.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated session", body = super::dto::LoginApiResponse),
        (status = 401, description = "Invalid credentials", body = super::dto::ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthSession>>, ApiError> {
    let identifier = request.username.trim();
    let user = resolve_login_user(&state, identifier).await?;

    // Verify hashed password
    let matches = verify(&request.password, &user.password)
        .map_err(|_| ApiError::unauthorized("failed to verify password"))?;

    if !matches {
        return Err(ApiError::unauthorized("invalid username or password"));
    }

    let session = state
        .sessions
        .create_session(SessionCreate {
            user_id: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            device_id: None,
            device_name: None,
            token_scopes: Vec::new(),
            scope_bindings: Vec::new(),
            expires_in_seconds: None,
        })
        .await?;

    if let Err(error) = state
        .audit
        .append(tikee_storage::CreateAuditLog {
            actor: user.username,
            action: "login".to_owned(),
            resource_type: "session".to_owned(),
            resource_id: redact_token_for_audit(&session.token),
            detail: None,
            before: None,
            after: None,
            trace_id: Some(trace_id(&headers)),
            result: "success".to_owned(),
            failure_reason: None,
            ip_address: client_ip(&headers),
        })
        .await
    {
        warn!(%error, "failed to append login audit log");
    }

    Ok(Json(ApiResponse::success(session)))
}

async fn resolve_login_user(
    state: &AppState,
    identifier: &str,
) -> Result<tikee_storage::entities::user::Model, ApiError> {
    if identifier.is_empty() {
        return Err(ApiError::unauthorized("invalid username or password"));
    }
    if let Some(user) = state
        .users
        .get_by_username(identifier)
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        return Ok(user);
    }
    if !identifier.contains('@') {
        return Err(ApiError::unauthorized("invalid username or password"));
    }
    state
        .users
        .get_by_email(identifier)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::unauthorized("invalid username or password"))
}

/// Create a durable API token for the current principal.
///
/// # Errors
///
/// Returns unauthorized, forbidden, bad request, or storage errors.
#[utoipa::path(
    post,
    path = "/api/v1/auth/api-tokens",
    tag = "auth",
    request_body = CreateApiTokenRequest
)]
pub async fn create_api_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateApiTokenRequest>,
) -> Result<Json<ApiResponse<CreatedApiToken>>, ApiError> {
    let principal = authenticate(&headers, &state).await?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("api token name is required"));
    }
    let user = state
        .users
        .get_by_username(&principal.username)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::unauthorized("authenticated user no longer exists"))?;
    let roles = vec![user.role.clone()];
    let permissions = state.rbac.permissions_for_roles(&roles).await?;
    let token_scopes = validate_api_token_scopes(request.scopes.unwrap_or_default(), &permissions)?;
    let scope_bindings = validate_scope_bindings(
        request.scope_bindings.unwrap_or_default(),
        &principal.scope_bindings,
    )?;
    let expires_in_seconds =
        validate_api_token_ttl(request.expires_in_seconds, &state.auth_config.api_tokens)?;
    let created = state
        .sessions
        .create_api_token(SessionCreate {
            user_id: user.id,
            username: user.username.clone(),
            role: user.role,
            device_id: None,
            device_name: Some(name.to_owned()),
            token_scopes,
            scope_bindings,
            expires_in_seconds: Some(expires_in_seconds),
        })
        .await?;
    audit_api_token(
        &state,
        &user.username,
        "api_token_create",
        &created.token.id,
        Some(format!("name={}", created.token.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

/// List durable API token metadata for the current principal.
///
/// # Errors
///
/// Returns unauthorized or storage errors.
#[utoipa::path(get, path = "/api/v1/auth/api-tokens", tag = "auth")]
pub async fn list_api_tokens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ApiTokenSummary>>>, ApiError> {
    let principal = authenticate(&headers, &state).await?;
    let tokens = state.sessions.list_api_tokens(&principal.username).await?;
    Ok(Json(ApiResponse::success(tokens)))
}

/// Rotate one durable API token while preserving its scopes.
///
/// # Errors
///
/// Returns unauthorized, not found, bad request, or storage errors.
#[utoipa::path(
    post,
    path = "/api/v1/auth/api-tokens/{id}/rotate",
    tag = "auth",
    request_body = RotateApiTokenRequest
)]
pub async fn rotate_api_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<RotateApiTokenRequest>,
) -> Result<Json<ApiResponse<CreatedApiToken>>, ApiError> {
    let principal = authenticate(&headers, &state).await?;
    let existing = state
        .sessions
        .list_api_tokens(&principal.username)
        .await?
        .into_iter()
        .find(|token| token.id == id)
        .ok_or_else(|| ApiError::not_found("api token not found"))?;
    let user = state
        .users
        .get_by_username(&principal.username)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::unauthorized("authenticated user no longer exists"))?;
    let roles = vec![user.role.clone()];
    let permissions = state.rbac.permissions_for_roles(&roles).await?;
    let token_scopes = validate_api_token_scopes(existing.scopes, &permissions)?;
    let scope_bindings =
        validate_scope_bindings(existing.scope_bindings, &principal.scope_bindings)?;
    let name = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.name.as_str())
        .to_owned();
    let expires_in_seconds =
        validate_api_token_ttl(request.expires_in_seconds, &state.auth_config.api_tokens)?;

    let created = state
        .sessions
        .create_api_token(SessionCreate {
            user_id: user.id,
            username: user.username.clone(),
            role: user.role,
            device_id: None,
            device_name: Some(name),
            token_scopes,
            scope_bindings,
            expires_in_seconds: Some(expires_in_seconds),
        })
        .await?;
    let revoked = state
        .sessions
        .revoke_api_token(&principal.username, &id)
        .await?;
    if !revoked {
        return Err(ApiError::not_found("api token not found"));
    }
    audit_api_token(
        &state,
        &principal.username,
        "api_token_rotate",
        &created.token.id,
        Some(format!("rotated_from={id}")),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

/// Revoke one durable API token owned by the current principal.
///
/// # Errors
///
/// Returns unauthorized, not found, or storage errors.
#[utoipa::path(delete, path = "/api/v1/auth/api-tokens/{id}", tag = "auth")]
pub async fn revoke_api_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<super::dto::EmptyApiResponse>, ApiError> {
    let principal = authenticate(&headers, &state).await?;
    let revoked = state
        .sessions
        .revoke_api_token(&principal.username, &id)
        .await?;
    if !revoked {
        return Err(ApiError::not_found("api token not found"));
    }
    audit_api_token(
        &state,
        &principal.username,
        "api_token_revoke",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(super::dto::EmptyData {})))
}

/// Return authentication mode and SSO configuration status.
#[utoipa::path(
    get,
    path = "/api/v1/auth/status",
    tag = "auth",
    responses((status = 200, description = "Authentication mode/status", body = super::dto::AuthStatusApiResponse))
)]
pub async fn status(State(state): State<Arc<AppState>>) -> Json<ApiResponse<AuthStatusResponse>> {
    let oidc = &state.auth_config.oidc;
    let oidc_ready = oidc.enabled
        && oidc
            .issuer_url
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        && oidc
            .client_id
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty());
    let initialized = state.users.count_users().await.unwrap_or(0) > 0;
    Json(ApiResponse::success(AuthStatusResponse {
        mode: if oidc_ready { "oidc" } else { "local" }.to_owned(),
        local_login_enabled: state.auth_config.local_login_enabled,
        bootstrap_required: !initialized,
        registration_open: !initialized,
        oidc: OidcStatus {
            enabled: oidc.enabled,
            issuer_url: oidc
                .issuer_url
                .clone()
                .filter(|value| !value.trim().is_empty()),
            client_id: oidc
                .client_id
                .clone()
                .filter(|value| !value.trim().is_empty()),
            client_secret_configured: oidc
                .client_secret
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty()),
            scopes: oidc.scopes.clone(),
        },
    }))
}

/// Build an OIDC authorization URL without contacting the provider.
///
/// # Errors
///
/// Returns a bad request when OIDC is disabled or required provider metadata is missing.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/authorize",
    tag = "auth",
    responses((status = 200, description = "OIDC authorization bootstrap", body = super::dto::OidcAuthorizeApiResponse))
)]
pub async fn oidc_authorize(
    State(state): State<Arc<AppState>>,
    Query(query): Query<super::oidc::OidcAuthorizeQuery>,
) -> Result<Json<ApiResponse<OidcAuthorizeResponse>>, ApiError> {
    let oidc = &state.auth_config.oidc;
    if !oidc.enabled {
        return Err(ApiError::bad_request("OIDC login is not enabled"));
    }
    let issuer = configured_value(oidc.issuer_url.as_ref(), "auth.oidc.issuer_url")?;
    let client_id = configured_value(oidc.client_id.as_ref(), "auth.oidc.client_id")?;
    let redirect_uri = query
        .redirect_uri
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "/api/v1/auth/oidc/callback".to_owned());
    let state_value = super::oidc::generate_state();
    OidcAuthStateRepository::new(state.users.db())
        .create_state(CreateOidcAuthState {
            state_hash: super::oidc::hash_state(&state_value),
            redirect_uri: redirect_uri.clone(),
            ttl_seconds: 600,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut authorization_url = url::Url::parse(&format!(
        "{}/protocol/openid-connect/auth",
        issuer.trim_end_matches('/')
    ))
    .map_err(|_| ApiError::bad_request("auth.oidc.issuer_url must be a valid URL"))?;
    authorization_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", &oidc.scopes.join(" "));
    authorization_url
        .query_pairs_mut()
        .append_pair("state", &state_value);
    Ok(Json(ApiResponse::success(OidcAuthorizeResponse {
        provider: "oidc".to_owned(),
        authorization_url: authorization_url.to_string(),
        client_id: client_id.to_owned(),
        scopes: oidc.scopes.clone(),
        state_required: true,
        pkce_required: true,
    })))
}

/// OIDC callback token exchange boundary; it never accepts unverified identity locally.
///
/// # Errors
///
/// Returns a bad request when OIDC is disabled, callback data is malformed, token exchange fails,
/// or until external identity is mapped to a local opaque session.
#[utoipa::path(get, path = "/api/v1/auth/oidc/callback", tag = "auth")]
pub async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<super::oidc::OidcCallbackQuery>,
) -> Result<Json<ApiResponse<AuthSession>>, ApiError> {
    let session = super::oidc_session::complete_oidc_callback(&state, &headers, &query).await?;
    Ok(Json(ApiResponse::success(session)))
}

fn configured_value<'a>(value: Option<&'a String>, field: &str) -> Result<&'a str, ApiError> {
    value
        .map(String::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("{field} is required when OIDC is enabled")))
}

/// Return the current authenticated principal.
///
/// # Errors
///
/// Returns unauthorized for missing or invalid bearer tokens.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current principal", body = super::dto::MeApiResponse),
        (status = 401, description = "Missing or invalid bearer token", body = super::dto::ErrorResponse)
    )
)]
pub async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<MeResponse>>, ApiError> {
    let principal = authenticate(&headers, &state).await?;
    Ok(Json(ApiResponse::success(principal)))
}

/// Logout endpoint by revoking the current session.
///
/// # Errors
///
/// Returns a storage error envelope if session revocation fails.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    responses((status = 200, description = "Logout acknowledged", body = super::dto::EmptyApiResponse))
)]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<super::dto::EmptyApiResponse>, ApiError> {
    let Some(value) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Ok(Json(ApiResponse::success(super::dto::EmptyData {})));
    };
    let Ok(value) = value.to_str() else {
        return Ok(Json(ApiResponse::success(super::dto::EmptyData {})));
    };
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Ok(Json(ApiResponse::success(super::dto::EmptyData {})));
    };

    let principal = authenticate(&headers, &state).await.ok();
    state.sessions.revoke_token(token).await?;

    if let Some(p) = &principal
        && let Err(error) = state
            .audit
            .append(tikee_storage::CreateAuditLog {
                actor: p.username.clone(),
                action: "logout".to_owned(),
                resource_type: "session".to_owned(),
                resource_id: redact_token_for_audit(token),
                detail: None,
                before: None,
                after: None,
                trace_id: Some(trace_id(&headers)),
                result: "success".to_owned(),
                failure_reason: None,
                ip_address: client_ip(&headers),
            })
            .await
    {
        warn!(%error, "failed to append logout audit log");
    }

    Ok(Json(ApiResponse::success(super::dto::EmptyData {})))
}

pub(super) fn redact_token_for_audit(token: &str) -> String {
    let prefix: String = token.chars().take(8).collect();
    format!("{prefix}…redacted")
}

fn validate_api_token_scopes(
    scopes: Vec<String>,
    permissions: &[tikee_storage::PermissionSummary],
) -> Result<Vec<String>, ApiError> {
    let mut normalized = Vec::new();
    for scope in scopes {
        let scope = scope.trim();
        if scope.is_empty() {
            continue;
        }
        let Some((resource, action)) = scope.split_once(':') else {
            return Err(ApiError::bad_request(
                "api token scopes must use resource:action format",
            ));
        };
        if resource.trim().is_empty()
            || action.trim().is_empty()
            || resource.contains(',')
            || action.contains(',')
        {
            return Err(ApiError::bad_request(
                "api token scopes must use non-empty resource:action values without commas",
            ));
        }
        let normalized_scope = format!("{}:{}", resource.trim(), action.trim());
        let allowed = permissions.iter().any(|permission| {
            permission.resource == resource.trim()
                && (permission.action == action.trim() || permission.action == "manage")
        });
        if !allowed {
            return Err(ApiError::forbidden(format!(
                "api token scope is not granted to the current principal: {normalized_scope}"
            )));
        }
        normalized.push(normalized_scope);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn validate_api_token_ttl(
    requested: Option<i64>,
    policy: &tikee_config::ApiTokenConfig,
) -> Result<i64, ApiError> {
    let ttl = requested.unwrap_or(policy.default_ttl_seconds);
    if ttl < policy.min_ttl_seconds || ttl > policy.max_ttl_seconds {
        return Err(ApiError::bad_request(format!(
            "expires_in_seconds must be between {} and {}",
            policy.min_ttl_seconds, policy.max_ttl_seconds
        )));
    }
    Ok(ttl)
}

async fn audit_api_token(
    state: &AppState,
    actor: &str,
    action: &str,
    token_id: &str,
    detail: Option<String>,
    headers: &HeaderMap,
) {
    if let Err(error) = state
        .audit
        .append(tikee_storage::CreateAuditLog {
            actor: actor.to_owned(),
            action: action.to_owned(),
            resource_type: "api_token".to_owned(),
            resource_id: token_id.to_owned(),
            detail,
            before: None,
            after: None,
            trace_id: Some(trace_id(headers)),
            result: "success".to_owned(),
            failure_reason: None,
            ip_address: client_ip(headers),
        })
        .await
    {
        warn!(%error, "failed to append api token audit log");
    }
}
