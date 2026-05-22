use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};

use tikee_core::ScriptExecutionPolicy;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, CreateScriptRequest, ErrorResponse, PageQuery, ScriptPage,
        ScriptPageApiResponse, ScriptReleaseRequest, UpdateScriptRequest,
    },
    error::ApiError,
};

use super::common::{audit, client_ip, trace_id};

/// List scripts.
///
/// # Errors
///
/// Returns authorization or storage errors when listing scripts fails.
#[utoipa::path(
    get,
    path = "/api/v1/scripts",
    tag = "scripts",
    params(PageQuery),
    responses(
        (status = 200, description = "Script page", body = ScriptPageApiResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_scripts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(_query): Query<PageQuery>,
) -> Result<Json<ScriptPageApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "scripts", "read").await?;
    let items = state
        .scripts
        .list_scripts()
        .await
        .map_err(|error| ApiError::storage(&error))?;

    Ok(Json(ApiResponse::success(ScriptPage {
        items,
        next_page_token: None,
    })))
}

/// Create a new script definition (Admin only).
///
/// # Errors
///
/// Returns authorization or storage errors when the script cannot be created.
#[utoipa::path(
    post,
    path = "/api/v1/scripts",
    tag = "scripts",
    request_body = CreateScriptRequest,
    responses(
        (status = 200, description = "Created script", body = crate::http::dto::ScriptApiResponse),
        (status = 400, description = "Bad request", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn create_script(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateScriptRequest>,
) -> Result<Json<crate::http::dto::ScriptApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "scripts", "manage").await?;
    let actor = principal.username.clone();

    let created = state
        .scripts
        .create_script(tikee_storage::CreateScript {
            name: request.name,
            language: request.language,
            version: request.version,
            content: request.content,
            created_by: principal.username,
            timeout_seconds: request.timeout_seconds,
            max_memory_bytes: request.max_memory_bytes,
            allow_network: request.allow_network.unwrap_or(false),
            allowed_env_vars: request
                .allowed_env_vars
                .and_then(|vars| serde_json::to_string(&vars).ok()),
            policy_json: validate_policy_json(request.policy)?,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    audit(
        &state,
        &actor,
        "create",
        "script",
        &created.id,
        Some(format!("name={}", created.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(created)))
}

/// Get one script by id (Admin only).
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when lookup fails.
#[utoipa::path(
    get,
    path = "/api/v1/scripts/{id}",
    tag = "scripts",
    params(("id" = String, Path, description = "Script identifier")),
    responses(
        (status = 200, description = "Script", body = crate::http::dto::ScriptApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn get_script(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<crate::http::dto::ScriptApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "scripts", "read").await?;

    let summary = state
        .scripts
        .get(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("script not found: {id}")))?;

    Ok(Json(ApiResponse::success(summary)))
}

/// Update a script definition (Admin only).
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when the script cannot be updated.
#[utoipa::path(
    patch,
    path = "/api/v1/scripts/{id}",
    tag = "scripts",
    params(("id" = String, Path, description = "Script identifier")),
    request_body = UpdateScriptRequest,
    responses(
        (status = 200, description = "Updated script", body = crate::http::dto::ScriptApiResponse),
        (status = 400, description = "Bad request", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn update_script(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateScriptRequest>,
) -> Result<Json<crate::http::dto::ScriptApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "scripts", "manage").await?;

    let updated = state
        .scripts
        .update_script(
            &id,
            tikee_storage::UpdateScript {
                name: request.name,
                language: request.language,
                version: request.version,
                content: request.content,
                status: request.status,
                timeout_seconds: request.timeout_seconds,
                max_memory_bytes: request.max_memory_bytes,
                allow_network: request.allow_network,
                allowed_env_vars: request
                    .allowed_env_vars
                    .and_then(|vars| serde_json::to_string(&vars).ok()),
                policy_json: validate_policy_json(request.policy)?,
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("script not found: {id}")))?;

    audit(
        &state,
        &principal.username,
        "update",
        "script",
        &id,
        Some(format!("name={}", updated.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(updated)))
}

/// Publish the latest or selected immutable script version as the executable release pointer.
#[utoipa::path(
    post,
    path = "/api/v1/scripts/{id}/publish",
    tag = "scripts",
    params(("id" = String, Path, description = "Script identifier")),
    request_body = ScriptReleaseRequest,
    responses(
        (status = 200, description = "Published script", body = crate::http::dto::ScriptApiResponse),
        (status = 400, description = "Bad request", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
///
/// # Errors
///
/// Returns authorization, not-found, bad request, or storage errors.
pub async fn publish_script(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ScriptReleaseRequest>,
) -> Result<Json<crate::http::dto::ScriptApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "scripts", "manage").await?;
    let version_number =
        resolve_release_version_number(&state, &id, request.version_number).await?;
    enforce_release_policy_gate(&state, &principal.username, &id, version_number, &headers).await?;
    let published = state
        .scripts
        .publish_version(&id, version_number)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::not_found(format!("script version not found: {id}@{version_number}"))
        })?;

    audit(
        &state,
        &principal.username,
        "publish",
        "script",
        &id,
        Some(format!("released_version_number={version_number}")),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(published)))
}

async fn enforce_release_policy_gate(
    state: &Arc<AppState>,
    actor: &str,
    id: &str,
    version_number: i64,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    let version = state
        .scripts
        .versions()
        .get_version_by_number(id, version_number)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::not_found(format!("script version not found: {id}@{version_number}"))
        })?;
    let policy: ScriptExecutionPolicy = serde_json::from_value(version.policy.clone())
        .map_err(|error| ApiError::bad_request(format!("invalid script policy: {error}")))?;
    let policy_result = if version.allow_network && !policy.network.enabled {
        Err("script release approval gate blocked legacy allow_network flag".to_owned())
    } else {
        policy
            .validate_default_deny()
            .map_err(|error| format!("script release approval gate blocked: {error}"))
    };
    if let Err(message) = policy_result {
        append_policy_gate_audit(state, actor, id, &message, headers).await;
        return Err(ApiError::bad_request(message));
    }
    Ok(())
}

async fn append_policy_gate_audit(
    state: &AppState,
    actor: &str,
    id: &str,
    detail: &str,
    headers: &HeaderMap,
) {
    if let Err(error) = state
        .audit
        .append(tikee_storage::CreateAuditLog {
            actor: actor.to_owned(),
            action: "publish_blocked".to_owned(),
            resource_type: "script".to_owned(),
            resource_id: id.to_owned(),
            detail: Some(detail.to_owned()),
            before: None,
            after: None,
            trace_id: Some(trace_id(headers)),
            result: "failed".to_owned(),
            failure_reason: Some("script_policy_approval_required".to_owned()),
            ip_address: client_ip(headers),
        })
        .await
    {
        tracing::warn!(%error, %id, "failed to append script policy gate audit log");
    }
}

/// Roll back the executable release pointer to a selected immutable script version.
#[utoipa::path(
    post,
    path = "/api/v1/scripts/{id}/rollback",
    tag = "scripts",
    params(("id" = String, Path, description = "Script identifier")),
    request_body = ScriptReleaseRequest,
    responses(
        (status = 200, description = "Rolled back script release", body = crate::http::dto::ScriptApiResponse),
        (status = 400, description = "Bad request", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse),
        (status = 500, description = "Storage error", body = crate::http::dto::ErrorResponse)
    )
)]
///
/// # Errors
///
/// Returns authorization, not-found, bad request, or storage errors.
pub async fn rollback_script(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ScriptReleaseRequest>,
) -> Result<Json<crate::http::dto::ScriptApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "scripts", "manage").await?;
    let version_number = request
        .version_number
        .ok_or_else(|| ApiError::bad_request("version_number is required for rollback"))?;
    enforce_release_policy_gate(&state, &principal.username, &id, version_number, &headers).await?;
    let rolled_back = state
        .scripts
        .rollback_release(&id, version_number)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::not_found(format!("script version not found: {id}@{version_number}"))
        })?;

    audit(
        &state,
        &principal.username,
        "rollback",
        "script",
        &id,
        Some(format!("released_version_number={version_number}")),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(rolled_back)))
}

async fn resolve_release_version_number(
    state: &Arc<AppState>,
    id: &str,
    requested: Option<i64>,
) -> Result<i64, ApiError> {
    if let Some(version_number) = requested {
        return Ok(version_number);
    }
    state
        .scripts
        .versions()
        .list_versions(id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|version| version.version_number)
        .max()
        .ok_or_else(|| ApiError::not_found(format!("script has no versions: {id}")))
}

fn validate_policy_json(value: Option<serde_json::Value>) -> Result<Option<String>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let policy: ScriptExecutionPolicy = serde_json::from_value(value)
        .map_err(|error| ApiError::bad_request(format!("invalid script policy: {error}")))?;
    policy
        .validate_default_deny()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    serde_json::to_string(&policy)
        .map(Some)
        .map_err(|error| ApiError::bad_request(format!("invalid script policy: {error}")))
}

/// Delete a script by id (Admin only).
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when the script cannot be deleted.
#[utoipa::path(
    delete,
    path = "/api/v1/scripts/{id}",
    tag = "scripts",
    params(("id" = String, Path, description = "Script identifier")),
    responses(
        (status = 200, description = "Deleted script acknowledged", body = crate::http::dto::EmptyApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn delete_script(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<crate::http::dto::EmptyApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "scripts", "manage").await?;

    let success = state
        .scripts
        .delete_script(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;

    if success {
        audit(
            &state,
            &principal.username,
            "delete",
            "script",
            &id,
            None,
            &headers,
        )
        .await;
        Ok(Json(ApiResponse::success(crate::http::dto::EmptyData {})))
    } else {
        Err(ApiError::not_found(format!("script not found: {id}")))
    }
}

/// List version history for a script (Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/scripts/{id}/versions",
    tag = "scripts",
    params(("id" = String, Path, description = "Script identifier")),
    responses(
        (status = 200, description = "Version list", body = crate::http::dto::ScriptVersionListApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse)
    )
)]
///
/// # Errors
///
/// Returns authorization or storage errors.
pub async fn list_script_versions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<tikee_storage::ScriptVersionSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "scripts", "read").await?;
    let versions = state
        .scripts
        .versions()
        .list_versions(&id)
        .await
        .map_err(|e| ApiError::storage(&e))?;
    Ok(Json(ApiResponse::success(versions)))
}

/// Diff two versions of a script (Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/scripts/{id}/diff",
    tag = "scripts",
    params(
        ("id" = String, Path, description = "Script identifier"),
        ("v1" = i64, Query, description = "Version number 1"),
        ("v2" = i64, Query, description = "Version number 2")
    ),
    responses(
        (status = 200, description = "Diff result", body = crate::http::dto::ScriptDiffApiResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 404, description = "Version not found", body = crate::http::dto::ErrorResponse)
    )
)]
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors.
pub async fn diff_script_versions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<DiffParams>,
) -> Result<Json<ApiResponse<crate::http::dto::ScriptDiffResult>>, ApiError> {
    auth::require_permission(&headers, &state, "scripts", "read").await?;
    let v1 = state
        .scripts
        .versions()
        .get_version_by_number(&id, params.v1)
        .await
        .map_err(|e| ApiError::storage(&e))?
        .ok_or_else(|| ApiError::not_found(format!("version {} not found", params.v1)))?;
    let v2 = state
        .scripts
        .versions()
        .get_version_by_number(&id, params.v2)
        .await
        .map_err(|e| ApiError::storage(&e))?
        .ok_or_else(|| ApiError::not_found(format!("version {} not found", params.v2)))?;
    let content_diff = unified_diff(&v1.content, &v2.content);
    let policy_diff = policy_diff(&v1, &v2);
    Ok(Json(ApiResponse::success(
        crate::http::dto::ScriptDiffResult {
            content_diff,
            policy_diff,
        },
    )))
}

#[derive(Debug, serde::Deserialize)]
/// Diff query parameters.
pub struct DiffParams {
    v1: i64,
    v2: i64,
}

fn unified_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut result = vec!["--- v1".to_owned(), "+++ v2".to_owned()];
    let operations = diff_operations(&old_lines, &new_lines);

    if operations
        .iter()
        .all(|operation| matches!(operation, DiffOp::Equal(_)))
    {
        return result.join("\n");
    }

    result.push(format!(
        "@@ -1,{} +1,{} @@",
        old_lines.len(),
        new_lines.len()
    ));
    for operation in operations {
        match operation {
            DiffOp::Equal(line) => result.push(format!(" {line}")),
            DiffOp::Delete(line) => result.push(format!("-{line}")),
            DiffOp::Insert(line) => result.push(format!("+{line}")),
        }
    }
    result.join("\n")
}

#[derive(Debug, Clone, Copy)]
enum DiffOp<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

fn diff_operations<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<DiffOp<'a>> {
    let mut lengths = vec![vec![0_u32; new.len() + 1]; old.len() + 1];
    for (i, old_line) in old.iter().enumerate() {
        for (j, new_line) in new.iter().enumerate() {
            if old_line == new_line {
                lengths[i + 1][j + 1] = lengths[i][j] + 1;
            } else {
                lengths[i + 1][j + 1] = lengths[i + 1][j].max(lengths[i][j + 1]);
            }
        }
    }

    let mut i = old.len();
    let mut j = new.len();
    let mut operations = Vec::with_capacity(old.len() + new.len());
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            operations.push(DiffOp::Equal(old[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lengths[i][j - 1] >= lengths[i - 1][j]) {
            operations.push(DiffOp::Insert(new[j - 1]));
            j -= 1;
        } else if i > 0 {
            operations.push(DiffOp::Delete(old[i - 1]));
            i -= 1;
        }
    }
    operations.reverse();
    operations
}

fn policy_diff(
    v1: &tikee_storage::ScriptVersionSummary,
    v2: &tikee_storage::ScriptVersionSummary,
) -> Vec<crate::http::dto::FieldChange> {
    let mut changes = Vec::new();
    let mut check = |field: &str, before: &str, after: &str| {
        if before != after {
            changes.push(crate::http::dto::FieldChange {
                field: field.to_owned(),
                before: before.to_owned(),
                after: after.to_owned(),
            });
        }
    };
    check("language", &v1.language, &v2.language);
    check("status", &v1.status, &v2.status);
    check(
        "timeout_seconds",
        &v1.timeout_seconds
            .map_or_else(|| "null".to_owned(), |v| v.to_string()),
        &v2.timeout_seconds
            .map_or_else(|| "null".to_owned(), |v| v.to_string()),
    );
    check(
        "max_memory_bytes",
        &v1.max_memory_bytes
            .map_or_else(|| "null".to_owned(), |v| v.to_string()),
        &v2.max_memory_bytes
            .map_or_else(|| "null".to_owned(), |v| v.to_string()),
    );
    check(
        "allow_network",
        &v1.allow_network.to_string(),
        &v2.allow_network.to_string(),
    );
    check(
        "allowed_env_vars",
        &v1.allowed_env_vars
            .as_ref()
            .map_or_else(|| "null".to_owned(), |v| v.join(",")),
        &v2.allowed_env_vars
            .as_ref()
            .map_or_else(|| "null".to_owned(), |v| v.join(",")),
    );
    check(
        "policy",
        &serde_json::to_string(&v1.policy).unwrap_or_else(|_| "{}".to_owned()),
        &serde_json::to_string(&v2.policy).unwrap_or_else(|_| "{}".to_owned()),
    );
    changes
}
