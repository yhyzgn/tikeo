//! Authentication and Role-Based Access Control (RBAC) verification.

use axum::{Json, extract::State, http::HeaderMap};
use bcrypt::verify;
use std::sync::Arc;
use tracing::warn;

use super::{
    AppState,
    dto::{ApiResponse, AuthSession, AuthStatusResponse, LoginRequest, MeResponse, OidcStatus},
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
    Err(ApiError::unauthorized("missing bearer token"))
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
    if allowed_roles
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
    let user = state
        .users
        .get_by_username(&request.username)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::unauthorized("invalid username or password"))?;

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
    Json(ApiResponse::success(AuthStatusResponse {
        mode: if oidc_ready { "oidc" } else { "local" }.to_owned(),
        local_login_enabled: state.auth_config.local_login_enabled,
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

fn redact_token_for_audit(token: &str) -> String {
    let prefix: String = token.chars().take(8).collect();
    format!("{prefix}…redacted")
}
