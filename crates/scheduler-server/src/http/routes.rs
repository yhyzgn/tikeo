//! HTTP route handlers for the management API.

use std::{str::FromStr, sync::Arc};

use axum::{Json, extract::Path, extract::Query, extract::State, http::HeaderMap};
use scheduler_core::{ExecutionMode, ScheduleType, TriggerType};
use scheduler_storage::{CreateJob, CreateJobInstance};

use super::{
    AppState, auth,
    dto::{
        ApiResponse, ClusterApiResponse, ClusterResponse, CreateJobRequest, CreateUserRequest,
        ErrorResponse, JobApiResponse, JobInstanceApiResponse, JobInstanceAttemptPage,
        JobInstanceAttemptPageApiResponse, JobInstanceAttemptSummary, JobInstanceLogPage,
        JobInstanceLogPageApiResponse, JobInstanceLogSummary, JobInstancePage,
        JobInstancePageApiResponse, JobInstanceSummary, JobPageApiResponse, JobSummary, Page,
        PageQuery, SystemInfoApiResponse, SystemInfoResponse, TriggerJobRequest, UpdateUserRequest,
    },
    error::ApiError,
};

/// Return scheduler server build and API metadata.
#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    tag = "system",
    responses((status = 200, description = "System info", body = SystemInfoApiResponse))
)]
pub async fn system_info() -> Json<SystemInfoApiResponse> {
    Json(ApiResponse::success(SystemInfoResponse {
        name: "scheduler",
        version: env!("CARGO_PKG_VERSION"),
        target: std::env::consts::OS,
    }))
}

/// Return the current cluster status placeholder.
#[utoipa::path(
    get,
    path = "/api/v1/cluster",
    tag = "system",
    responses((status = 200, description = "Cluster status", body = ClusterApiResponse))
)]
pub async fn cluster_status() -> Json<ClusterApiResponse> {
    Json(ApiResponse::success(ClusterResponse {
        mode: "standalone",
        role: "leader",
        nodes: 1,
    }))
}

/// List jobs.
///
/// # Errors
///
/// Returns a storage error envelope when the repository query fails.
#[utoipa::path(
    get,
    path = "/api/v1/jobs",
    tag = "jobs",
    params(PageQuery),
    responses(
        (status = 200, description = "Job page", body = JobPageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobPageApiResponse>, ApiError> {
    let items = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(Page {
        items,
        next_page_token: None,
    })))
}

/// Create a job.
///
/// # Errors
///
/// Returns a storage error envelope when the job cannot be created.
#[utoipa::path(
    post,
    path = "/api/v1/jobs",
    tag = "jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 200, description = "Created job", body = JobApiResponse),
        (status = 400, description = "Invalid schedule type", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn create_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<JobApiResponse>, ApiError> {
    auth::require_admin(&headers, &state).await?;
    let schedule_type = parse_schedule_type(request.schedule_type.as_deref().unwrap_or("api"))?;
    let created = state
        .jobs
        .create_job(CreateJob {
            namespace: defaulted(request.namespace, "default"),
            app: defaulted(request.app, "default"),
            name: request.name,
            schedule_type: schedule_type.to_string(),
            schedule_expr: request.schedule_expr,
            enabled: request.enabled.unwrap_or(true),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    Ok(Json(ApiResponse::success(JobSummary::from(created))))
}

/// Trigger a job and create a pending instance.
///
/// # Errors
///
/// Returns a validation, not-found, or storage error envelope when triggering fails.
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{job}:trigger",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    request_body = TriggerJobRequest,
    responses(
        (status = 200, description = "Created pending job instance", body = JobInstanceApiResponse),
        (status = 400, description = "Invalid trigger or execution mode", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn trigger_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_action): Path<String>,
    Json(request): Json<TriggerJobRequest>,
) -> Result<Json<JobInstanceApiResponse>, ApiError> {
    auth::require_admin(&headers, &state).await?;
    let job = parse_trigger_path(&job_action)?;

    let job_summary = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    let trigger_type = parse_trigger_type(request.trigger_type.as_deref().unwrap_or("api"))?;
    let execution_mode =
        parse_execution_mode(request.execution_mode.as_deref().unwrap_or("single"))?;
    let broadcast_worker_ids = if execution_mode == ExecutionMode::Broadcast {
        let worker_ids = state
            .registry
            .find_eligible_workers(&job_summary.namespace, &job_summary.app)
            .await;
        if worker_ids.is_empty() {
            return Err(ApiError::bad_request(
                "broadcast execution requires at least one eligible online worker",
            ));
        }
        Some(worker_ids)
    } else {
        None
    };

    let instance = state
        .instances
        .create_pending(CreateJobInstance {
            job_id: job.clone(),
            trigger_type,
            execution_mode,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    if let Some(worker_ids) = broadcast_worker_ids {
        state
            .attempts
            .create_pending_for_workers(&instance.id, &worker_ids)
            .await
            .map_err(|error| ApiError::storage(&error))?;
    }

    Ok(Json(ApiResponse::success(JobInstanceSummary::from(
        instance,
    ))))
}

/// List instances for a job.
///
/// # Errors
///
/// Returns a storage error envelope when repository access fails.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job}/instances",
    tag = "jobs",
    params(
        ("job" = String, Path, description = "Job identifier"),
        PageQuery,
    ),
    responses(
        (status = 200, description = "Job instance page", body = JobInstancePageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_job_instances(
    State(state): State<Arc<AppState>>,
    Path(job): Path<String>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobInstancePageApiResponse>, ApiError> {
    let items = state
        .instances
        .list_by_job(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(JobInstancePage {
        items,
        next_page_token: None,
    })))
}

/// Get one job instance.
///
/// # Errors
///
/// Returns a not-found or storage error envelope when lookup fails.
#[utoipa::path(
    get,
    path = "/api/v1/instances/{instance}",
    tag = "jobs",
    params(("instance" = String, Path, description = "Instance identifier")),
    responses(
        (status = 200, description = "Job instance", body = JobInstanceApiResponse),
        (status = 404, description = "Instance not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn get_job_instance(
    State(state): State<Arc<AppState>>,
    Path(instance): Path<String>,
) -> Result<Json<JobInstanceApiResponse>, ApiError> {
    let summary = state
        .instances
        .get(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("instance not found: {instance}")))?;

    Ok(Json(ApiResponse::success(JobInstanceSummary::from(
        summary,
    ))))
}

/// List broadcast attempts for one job instance.
///
/// # Errors
///
/// Returns a storage error envelope when repository access fails.
#[utoipa::path(
    get,
    path = "/api/v1/instances/{instance}/attempts",
    tag = "jobs",
    params(
        ("instance" = String, Path, description = "Instance identifier"),
        PageQuery,
    ),
    responses(
        (status = 200, description = "Job instance attempt page", body = JobInstanceAttemptPageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_instance_attempts(
    State(state): State<Arc<AppState>>,
    Path(instance): Path<String>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobInstanceAttemptPageApiResponse>, ApiError> {
    let items = state
        .attempts
        .list_by_instance(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceAttemptSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(JobInstanceAttemptPage {
        items,
        next_page_token: None,
    })))
}

/// List logs for one job instance.
///
/// # Errors
///
/// Returns a storage error envelope when repository access fails.
#[utoipa::path(
    get,
    path = "/api/v1/instances/{instance}/logs",
    tag = "jobs",
    params(
        ("instance" = String, Path, description = "Instance identifier"),
        PageQuery,
    ),
    responses(
        (status = 200, description = "Job instance log page", body = JobInstanceLogPageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_instance_logs(
    State(state): State<Arc<AppState>>,
    Path(instance): Path<String>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobInstanceLogPageApiResponse>, ApiError> {
    let items = state
        .logs
        .list_by_instance(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceLogSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(JobInstanceLogPage {
        items,
        next_page_token: None,
    })))
}

impl From<scheduler_storage::JobSummary> for JobSummary {
    fn from(value: scheduler_storage::JobSummary) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            schedule_type: value.schedule_type,
            schedule_expr: value.schedule_expr,
            enabled: value.enabled,
        }
    }
}

impl From<scheduler_storage::JobInstanceSummary> for JobInstanceSummary {
    fn from(value: scheduler_storage::JobInstanceSummary) -> Self {
        Self {
            id: value.id,
            job_id: value.job_id,
            status: value.status.to_string(),
            trigger_type: value.trigger_type.to_string(),
            execution_mode: value.execution_mode.to_string(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<scheduler_storage::JobInstanceAttemptSummary> for JobInstanceAttemptSummary {
    fn from(value: scheduler_storage::JobInstanceAttemptSummary) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            status: value.status.to_string(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<scheduler_storage::JobInstanceLogSummary> for JobInstanceLogSummary {
    fn from(value: scheduler_storage::JobInstanceLogSummary) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            level: value.level,
            message: value.message,
            sequence: value.sequence,
            created_at: value.created_at,
        }
    }
}

fn defaulted(value: Option<String>, default: &str) -> String {
    value
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn parse_schedule_type(value: &str) -> Result<ScheduleType, ApiError> {
    ScheduleType::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn parse_trigger_type(value: &str) -> Result<TriggerType, ApiError> {
    TriggerType::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn parse_execution_mode(value: &str) -> Result<ExecutionMode, ApiError> {
    ExecutionMode::from_str(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn parse_trigger_path(value: &str) -> Result<String, ApiError> {
    value
        .strip_suffix(":trigger")
        .filter(|job| !job.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::not_found(format!("unsupported job action: {value}")))
}

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
        (status = 200, description = "User list", body = super::dto::UserListApiResponse),
        (status = 401, description = "Unauthorized", body = super::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = super::dto::ErrorResponse)
    )
)]
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<super::dto::UserListApiResponse>, ApiError> {
    auth::require_admin(&headers, &state).await?;
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
        (status = 200, description = "Created user", body = super::dto::UserApiResponse),
        (status = 400, description = "Bad request", body = super::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = super::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = super::dto::ErrorResponse)
    )
)]
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<super::dto::CreateUserRequest>,
) -> Result<Json<super::dto::UserApiResponse>, ApiError> {
    auth::require_admin(&headers, &state).await?;

    if request.username.trim().is_empty() || request.password.trim().is_empty() {
        return Err(ApiError::bad_request(
            "username and password cannot be empty",
        ));
    }

    // Hash password with BCrypt
    let hash = bcrypt::hash(request.password, 10)
        .map_err(|_| ApiError::bad_request("failed to hash password"))?;

    let created = state
        .users
        .create_user(scheduler_storage::CreateUser {
            username: request.username,
            password: hash,
            role: request.role,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

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
        (status = 200, description = "Updated user", body = super::dto::UserApiResponse),
        (status = 400, description = "Bad request", body = super::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = super::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = super::dto::ErrorResponse),
        (status = 404, description = "Not found", body = super::dto::ErrorResponse)
    )
)]
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<super::dto::UpdateUserRequest>,
) -> Result<Json<super::dto::UserApiResponse>, ApiError> {
    auth::require_admin(&headers, &state).await?;

    let existing = state
        .users
        .get_user(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("user not found: {id}")))?;

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
            scheduler_storage::UpdateUser {
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
        (status = 200, description = "Deleted user acknowledged", body = super::dto::EmptyApiResponse),
        (status = 401, description = "Unauthorized", body = super::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = super::dto::ErrorResponse),
        (status = 404, description = "Not found", body = super::dto::ErrorResponse)
    )
)]
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<super::dto::EmptyApiResponse>, ApiError> {
    auth::require_admin(&headers, &state).await?;

    let existing = state
        .users
        .get_user(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;

    let success = state
        .users
        .delete_user(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;

    if success {
        if let Some(user) = existing {
            state.sessions.revoke_user_sessions(&user.username).await?;
        }
        Ok(Json(ApiResponse::success(super::dto::EmptyData {})))
    } else {
        Err(ApiError::not_found(format!("user not found: {id}")))
    }
}
