use std::sync::Arc;

use sha2::{Digest, Sha256};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};

use tikee_core::{ScriptExecutionPolicy, ScriptReleaseGrantSet};
use tikee_storage::{VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, CreateScriptRequest, ErrorResponse, PageQuery, ScriptPage,
        ScriptPageApiResponse, ScriptReleaseGateApiResponse, ScriptReleaseGateQuery,
        ScriptReleaseGateResponse, ScriptReleaseRequest, UpdateScriptRequest,
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
    let version = resolve_release_version(&state, &id, version_number).await?;
    let release_verification = enforce_release_signature_gate(
        &state,
        &principal.username,
        &id,
        &request,
        &version,
        &headers,
    )
    .await?;
    enforce_release_policy_gate(&state, &principal.username, &id, &version, &headers).await?;
    let published = state
        .scripts
        .publish_version(
            &id,
            version_number,
            release_verification.signature,
            release_verification.grants,
        )
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

/// Preview whether a script version would pass the current release gates.
///
/// # Errors
///
/// Returns authorization, not-found, bad request, or storage errors.
#[utoipa::path(
    get,
    path = "/api/v1/scripts/{id}/release-gate",
    tag = "scripts",
    params(
        ("id" = String, Path, description = "Script identifier"),
        ScriptReleaseGateQuery
    ),
    responses(
        (status = 200, description = "Script release gate preview", body = ScriptReleaseGateApiResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn preview_script_release_gate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ScriptReleaseGateQuery>,
) -> Result<Json<ScriptReleaseGateApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "scripts", "read").await?;
    let version_number = resolve_release_version_number(&state, &id, query.version_number).await?;
    let version = resolve_release_version(&state, &id, version_number).await?;
    let blocking_reasons = release_policy_blocking_reasons(&version)
        .map_err(|error| ApiError::bad_request(format!("invalid script policy: {error}")))?;
    let required_actions = release_gate_required_actions(&blocking_reasons);

    Ok(Json(ApiResponse::success(ScriptReleaseGateResponse {
        script_id: id,
        version_number,
        version_id: version.id,
        content_sha256: version.content_sha256,
        releasable: blocking_reasons.is_empty(),
        blocking_reasons,
        required_actions,
        signature_verification_enabled: state
            .script_governance
            .release_signature_secret_ref
            .is_some(),
    })))
}

async fn enforce_release_signature_gate(
    state: &AppState,
    actor: &str,
    id: &str,
    request: &ScriptReleaseRequest,
    version: &tikee_storage::ScriptVersionSummary,
    headers: &HeaderMap,
) -> Result<VerifiedScriptReleaseVerification, ApiError> {
    let grants = release_grants(request)?;
    let grant_present = grants.as_ref().is_some_and(|value| !value.is_empty());
    let approval_present = request
        .approval_ticket
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let signature_present = request
        .signature
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    if !approval_present && !signature_present && !grant_present {
        return Ok(VerifiedScriptReleaseVerification::default());
    }
    let Some(secret_ref) = state
        .script_governance
        .release_signature_secret_ref
        .as_deref()
    else {
        let message = "script approval/signature metadata was provided, but signature verification is not yet enabled";
        append_release_gate_audit(
            state,
            actor,
            id,
            message,
            "script_signature_verification_required",
            headers,
        )
        .await;
        return Err(ApiError::bad_request(message));
    };
    let Some(approval_ticket) = request
        .approval_ticket
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        let message = "script release signature requires a non-empty approval_ticket";
        append_release_gate_audit(
            state,
            actor,
            id,
            message,
            "script_signature_verification_required",
            headers,
        )
        .await;
        return Err(ApiError::bad_request(message));
    };
    let Some(signature) = request
        .signature
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        let message = "script release approval_ticket requires a matching signature";
        append_release_gate_audit(
            state,
            actor,
            id,
            message,
            "script_signature_verification_required",
            headers,
        )
        .await;
        return Err(ApiError::bad_request(message));
    };
    let secret = resolve_env_secret_ref(secret_ref).ok_or_else(|| {
        ApiError::bad_request("script release signature secret_ref is not resolvable")
    })?;
    let expected = script_release_signature(&secret, id, version, approval_ticket, grants.as_ref());
    if !constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        let message = "script release signature verification failed";
        append_release_gate_audit(
            state,
            actor,
            id,
            message,
            "script_signature_verification_failed",
            headers,
        )
        .await;
        return Err(ApiError::bad_request(message));
    }
    Ok(VerifiedScriptReleaseVerification {
        signature: Some(VerifiedScriptReleaseSignature {
            approval_ticket: approval_ticket.to_owned(),
            signature: signature.to_owned(),
            verified_by: actor.to_owned(),
        }),
        grants: grants
            .filter(|value| !value.is_empty())
            .map(|value| VerifiedScriptReleaseGrants {
                grants: value,
                verified_by: actor.to_owned(),
            }),
    })
}

#[derive(Default)]
struct VerifiedScriptReleaseVerification {
    signature: Option<VerifiedScriptReleaseSignature>,
    grants: Option<VerifiedScriptReleaseGrants>,
}

fn release_grants(
    request: &ScriptReleaseRequest,
) -> Result<Option<ScriptReleaseGrantSet>, ApiError> {
    let Some(grants) = request.grants.clone() else {
        return Ok(None);
    };
    let grants: ScriptReleaseGrantSet = grants.into();
    grants
        .validate_values()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Some(grants))
}

async fn enforce_release_policy_gate(
    state: &Arc<AppState>,
    actor: &str,
    id: &str,
    version: &tikee_storage::ScriptVersionSummary,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    let blocking_reasons = release_policy_blocking_reasons(version)
        .map_err(|error| ApiError::bad_request(format!("invalid script policy: {error}")))?;
    if let Some(message) = blocking_reasons.first() {
        append_release_gate_audit(
            state,
            actor,
            id,
            message,
            "script_policy_approval_required",
            headers,
        )
        .await;
        return Err(ApiError::bad_request(message));
    }
    Ok(())
}

fn release_policy_blocking_reasons(
    version: &tikee_storage::ScriptVersionSummary,
) -> Result<Vec<String>, serde_json::Error> {
    let policy: ScriptExecutionPolicy = serde_json::from_value(version.policy.clone())?;
    let mut reasons = Vec::new();
    if version.allow_network && !policy.network.enabled {
        reasons.push("script release approval gate blocked legacy allow_network flag".to_owned());
    }
    if let Err(error) = policy.validate_default_deny() {
        reasons.push(format!("script release approval gate blocked: {error}"));
    }
    Ok(reasons)
}

fn release_gate_required_actions(blocking_reasons: &[String]) -> Vec<String> {
    if blocking_reasons.is_empty() {
        return Vec::new();
    }
    vec![
        "remove dangerous URL/File/Secret grants or wait for verified approval/signature support"
            .to_owned(),
        "publish a new safe script version before moving the release pointer".to_owned(),
    ]
}

async fn append_release_gate_audit(
    state: &AppState,
    actor: &str,
    id: &str,
    detail: &str,
    failure_reason: &str,
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
            failure_reason: Some(failure_reason.to_owned()),
            ip_address: client_ip(headers),
        })
        .await
    {
        tracing::warn!(%error, %id, %failure_reason, "failed to append script release gate audit log");
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
    let version = resolve_release_version(&state, &id, version_number).await?;
    let release_verification = enforce_release_signature_gate(
        &state,
        &principal.username,
        &id,
        &request,
        &version,
        &headers,
    )
    .await?;
    enforce_release_policy_gate(&state, &principal.username, &id, &version, &headers).await?;
    let rolled_back = state
        .scripts
        .rollback_release(
            &id,
            version_number,
            release_verification.signature,
            release_verification.grants,
        )
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

async fn resolve_release_version(
    state: &Arc<AppState>,
    id: &str,
    version_number: i64,
) -> Result<tikee_storage::ScriptVersionSummary, ApiError> {
    state
        .scripts
        .versions()
        .get_version_by_number(id, version_number)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::not_found(format!("script version not found: {id}@{version_number}"))
        })
}

fn resolve_env_secret_ref(secret_ref: &str) -> Option<String> {
    secret_ref
        .strip_prefix("env:")
        .and_then(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
}

fn script_release_signature(
    secret: &str,
    script_id: &str,
    version: &tikee_storage::ScriptVersionSummary,
    approval_ticket: &str,
    grants: Option<&ScriptReleaseGrantSet>,
) -> String {
    let grants_json = canonical_release_grants_json(grants);
    let payload = format!(
        "tikee-script-release-v1\nscript_id={script_id}\nversion_number={}\ncontent_sha256={}\napproval_ticket={approval_ticket}\ngrants={grants_json}",
        version.version_number, version.content_sha256
    );
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b"\n");
    hasher.update(payload.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn canonical_release_grants_json(grants: Option<&ScriptReleaseGrantSet>) -> String {
    let grants = grants.cloned().unwrap_or_default();
    serde_json::to_string(&grants).unwrap_or_else(|_| "{}".to_owned())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
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
