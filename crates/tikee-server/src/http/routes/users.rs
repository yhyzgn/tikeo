use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, CreateUserRequest, UpdateUserRequest},
    error::ApiError,
};

use super::common::audit;

/// List all platform users (Admin only).
///
/// # Errors
///
/// Returns unauthorized/forbidden for invalid roles or storage errors when listing users fails.
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    responses(
        (status = 200, description = "User list", body = crate::http::dto::UserListApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<crate::http::dto::UserListApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "users", "read").await?;
    let items = state
        .users
        .list_users()
        .await
        .map_err(|error| ApiError::storage(&error))?;

    Ok(Json(ApiResponse::success(items)))
}

/// Create a new platform user (Admin only).
///
/// # Errors
///
/// Returns validation, authorization, or storage errors when the user cannot be created.
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "Created user", body = crate::http::dto::UserApiResponse),
        (status = 400, description = "Bad request", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<crate::http::dto::CreateUserRequest>,
) -> Result<Json<crate::http::dto::UserApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "users", "manage").await?;

    validate_role(&request.role)?;
    if request.username.trim().is_empty()
        || request.email.trim().is_empty()
        || request.password.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "username, email and password cannot be empty",
        ));
    }
    validate_email(&request.email)?;

    // Hash password with BCrypt
    let hash = bcrypt::hash(request.password, 10)
        .map_err(|_| ApiError::bad_request("failed to hash password"))?;

    let created = state
        .users
        .create_user(tikee_storage::CreateUser {
            username: request.username.trim().to_owned(),
            email: request.email.trim().to_owned(),
            password: hash,
            role: request.role,
            bootstrap_admin: false,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    audit(
        &state,
        &principal.username,
        "create",
        "user",
        &created.id,
        Some(format!("username={}", created.username)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(created)))
}

/// Update user details (Admin only).
///
/// # Errors
///
/// Returns validation, authorization, not-found, or storage errors when the user cannot be updated.
#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}",
    tag = "users",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Updated user", body = crate::http::dto::UserApiResponse),
        (status = 400, description = "Bad request", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<crate::http::dto::UpdateUserRequest>,
) -> Result<Json<crate::http::dto::UserApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "users", "manage").await?;

    let existing = state
        .users
        .get_user(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("user not found: {id}")))?;

    if let Some(role) = request.role.as_deref() {
        validate_role(role)?;
    }
    if let Some(email) = request.email.as_deref() {
        validate_email(email)?;
    }
    let password_changed = request.password.is_some();
    let role_changed = request.role.is_some();
    let password = if let Some(plain) = request.password {
        if plain.trim().is_empty() {
            return Err(ApiError::bad_request("password cannot be empty"));
        }
        let hash = bcrypt::hash(plain, 10)
            .map_err(|_| ApiError::bad_request("failed to hash password"))?;
        Some(hash)
    } else {
        None
    };

    let updated = state
        .users
        .update_user(
            &id,
            tikee_storage::UpdateUser {
                email: request.email.as_deref().map(str::trim).map(str::to_owned),
                password,
                role: request.role.clone(),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("user not found: {id}")))?;

    // Role/password updates invalidate active sessions so principals are refreshed on next login.
    if role_changed || password_changed {
        state
            .sessions
            .revoke_user_sessions(&existing.username)
            .await?;
    }

    audit(
        &state,
        &principal.username,
        "update",
        "user",
        &id,
        Some(format!("username={}", updated.username)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(updated)))
}

/// Delete a platform user (Admin only).
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when the user cannot be deleted.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "users",
    responses(
        (status = 200, description = "Deleted user acknowledged", body = crate::http::dto::EmptyApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<crate::http::dto::EmptyApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "users", "manage").await?;

    let existing = state
        .users
        .get_user(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;

    if let Some(user) = existing.as_ref() {
        if user.role == "admin"
            && state
                .users
                .count_by_role("admin")
                .await
                .map_err(|error| ApiError::storage(&error))?
                <= 1
        {
            return Err(ApiError::bad_request("cannot delete the last admin user"));
        }
    }

    let success = state
        .users
        .delete_user(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;

    if success {
        if let Some(user) = existing {
            state.sessions.revoke_user_sessions(&user.username).await?;
            audit(
                &state,
                &principal.username,
                "delete",
                "user",
                &id,
                Some(format!("username={}", user.username)),
                &headers,
            )
            .await;
        }
        Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
    } else {
        Err(ApiError::not_found(format!("user not found: {id}")))
    }
}

fn validate_role(role: &str) -> Result<(), ApiError> {
    if matches!(role, "admin" | "operator" | "viewer") {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!("unsupported role: {role}")))
    }
}

fn validate_email(email: &str) -> Result<(), ApiError> {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(ApiError::bad_request("valid email is required"));
    };
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(ApiError::bad_request("valid email is required"));
    }
    Ok(())
}
