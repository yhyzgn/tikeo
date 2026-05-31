use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use tikee_core::ExecutionMode;
use tikee_storage::{CreateJob, CreateJobInstance, UpdateJob};

use crate::tunnel::registry::BroadcastSelector;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, CanaryRoutingSummary, CreateJobRequest, DeleteJobApiResponse, EmptyData,
        ErrorResponse, JobApiResponse, JobInstanceApiResponse, JobInstanceAttemptPage,
        JobInstanceAttemptPageApiResponse, JobInstanceAttemptSummary, JobInstanceCancelApiResponse,
        JobInstanceLogPage, JobInstanceLogPageApiResponse, JobInstanceLogSummary, JobInstancePage,
        JobInstancePageApiResponse, JobInstanceSummary, JobPageApiResponse, JobSummary,
        JobVersionPage, JobVersionPageApiResponse, Page, PageQuery, RollbackJobRequest,
        TriggerJobRequest, UpdateJobRequest,
    },
    error::ApiError,
};

use super::common::{
    audit, defaulted, parse_execution_mode, parse_schedule_type, parse_trigger_path,
    parse_trigger_type,
};

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
    headers: HeaderMap,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobPageApiResponse>, ApiError> {
    let principal = if has_auth_header(&headers) {
        Some(auth::require_permission(&headers, &state, "jobs", "read").await?)
    } else {
        None
    };
    let items = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .filter(|job| {
            principal.as_ref().is_none_or(|principal| {
                crate::http::access_scope::allows_resource(
                    &principal.scope_bindings,
                    &job.namespace,
                    &job.app,
                    None,
                )
            })
        })
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
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let namespace = defaulted(request.namespace, "default");
    let app = defaulted(request.app, "default");
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &namespace,
        &app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let schedule_type = parse_schedule_type(request.schedule_type.as_deref().unwrap_or("api"))?;
    validate_plugin_processor_binding(
        &state,
        request.processor_type.as_deref(),
        request.processor_name.as_deref(),
    )
    .await?;
    if has_concrete_binding(request.processor_name.as_ref())
        && has_concrete_binding(request.script_id.as_ref())
    {
        return Err(ApiError::bad_request(
            "processorName and scriptId are mutually exclusive",
        ));
    }
    let created = state
        .jobs
        .create_job(CreateJob {
            namespace,
            app,
            name: request.name.clone(),
            schedule_type: schedule_type.to_string(),
            schedule_expr: request.schedule_expr.clone(),
            misfire_policy: request
                .misfire_policy
                .clone()
                .unwrap_or_else(|| "fire_once".to_owned()),
            schedule_start_at: request.schedule_start_at.clone(),
            schedule_end_at: request.schedule_end_at.clone(),
            schedule_calendar_json: serialize_schedule_calendar(request.schedule_calendar.as_ref()),
            processor_name: request.processor_name.clone(),
            processor_type: request.processor_type.clone(),
            script_id: request.script_id.clone(),
            enabled: request.enabled.unwrap_or(true),
            canary_job_id: None,
            canary_percent: 0,
            created_by: Some(principal.username.clone()),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    audit(
        &state,
        &principal.username,
        "create",
        "job",
        &created.id,
        Some(format!("name={}", created.name)),
        &headers,
    )
    .await;

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
    let principal = auth::require_permission(&headers, &state, "instances", "execute").await?;
    let job = parse_trigger_path(&job_action)?;

    let job_summary = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &job_summary.namespace,
        &job_summary.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }

    let trigger_type = parse_trigger_type(request.trigger_type.as_deref().unwrap_or("api"))?;
    let execution_mode =
        parse_execution_mode(request.execution_mode.as_deref().unwrap_or("single"))?;
    let canary_routing = resolve_canary_routing(&state, &job_summary).await?;
    let target_job = canary_routing
        .as_ref()
        .filter(|routing| routing.routed)
        .map_or_else(|| job.clone(), |routing| routing.routed_job_id.clone());
    let target_summary = if target_job == job_summary.id {
        job_summary.clone()
    } else {
        state
            .jobs
            .get(&target_job)
            .await
            .map_err(|error| ApiError::storage(&error))?
            .ok_or_else(|| ApiError::not_found(format!("job not found: {target_job}")))?
    };
    let broadcast_selector =
        request
            .broadcast_selector
            .as_ref()
            .map(|selector| BroadcastSelector {
                tags: selector
                    .tags
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|tag| tag.trim().to_owned())
                    .filter(|tag| !tag.is_empty())
                    .collect(),
                region: selector
                    .region
                    .as_ref()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty()),
                cluster: selector
                    .cluster
                    .as_ref()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty()),
                labels: selector.labels.clone().unwrap_or_default(),
            });
    let broadcast_worker_ids = if execution_mode == ExecutionMode::Broadcast {
        let worker_ids = state
            .registry
            .find_eligible_workers_with_broadcast_selector(
                &target_summary.namespace,
                &target_summary.app,
                broadcast_selector.as_ref(),
            )
            .await;
        if worker_ids.is_empty() {
            return Err(ApiError::bad_request(
                "broadcast execution requires at least one eligible online worker matching selector",
            ));
        }
        Some(worker_ids)
    } else {
        None
    };

    let instance = state
        .instances
        .create_pending(CreateJobInstance {
            job_id: target_job.clone(),
            trigger_type,
            execution_mode,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {target_job}")))?;

    if let Some(worker_ids) = broadcast_worker_ids {
        state
            .attempts
            .create_pending_for_workers(&instance.id, &worker_ids)
            .await
            .map_err(|error| ApiError::storage(&error))?;
    }

    audit(
        &state,
        &principal.username,
        "trigger",
        "job",
        &job,
        Some(format!("instance={}", instance.id)),
        &headers,
    )
    .await;

    let mut summary = instance_summary_with_latest_log(&state, instance).await?;
    summary.canary_routing = canary_routing;
    Ok(Json(ApiResponse::success(summary)))
}

/// Update a job.
///
/// # Errors
///
/// Returns validation, authorization, not-found, or storage errors.
#[utoipa::path(
    patch,
    path = "/api/v1/jobs/{job}",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    request_body = UpdateJobRequest,
    responses(
        (status = 200, description = "Updated job", body = JobApiResponse),
        (status = 400, description = "Invalid schedule type", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn update_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
    Json(request): Json<UpdateJobRequest>,
) -> Result<Json<JobApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let current = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &current.namespace,
        &current.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let schedule_type = request
        .schedule_type
        .as_deref()
        .map(parse_schedule_type)
        .transpose()?
        .map(|value| value.to_string());
    if request
        .processor_name
        .as_ref()
        .and_then(std::option::Option::as_ref)
        .is_some_and(|value| has_concrete_binding(Some(value)))
        && request
            .script_id
            .as_ref()
            .and_then(std::option::Option::as_ref)
            .is_some_and(|value| has_concrete_binding(Some(value)))
    {
        return Err(ApiError::bad_request(
            "processorName and scriptId are mutually exclusive",
        ));
    }
    let final_processor_type = request
        .processor_type
        .clone()
        .unwrap_or_else(|| current.processor_type.clone());
    let final_processor_name = request
        .processor_name
        .clone()
        .unwrap_or_else(|| current.processor_name.clone());
    let final_script_id = request
        .script_id
        .clone()
        .unwrap_or_else(|| current.script_id.clone());
    if !has_concrete_binding(final_script_id.as_ref()) {
        validate_plugin_processor_binding(
            &state,
            final_processor_type.as_deref(),
            final_processor_name.as_deref(),
        )
        .await?;
    }
    let updated = state
        .jobs
        .update_job(
            &job,
            UpdateJob {
                name: request.name.clone(),
                schedule_type,
                schedule_expr: request.schedule_expr.clone(),
                misfire_policy: request.misfire_policy.clone(),
                schedule_start_at: request.schedule_start_at.clone(),
                schedule_end_at: request.schedule_end_at.clone(),
                schedule_calendar_json: request
                    .schedule_calendar
                    .as_ref()
                    .map(|value| serialize_schedule_calendar(value.as_ref())),
                processor_name: request.processor_name.clone(),
                processor_type: request.processor_type.clone(),
                script_id: request.script_id.clone(),
                enabled: request.enabled,
                canary_job_id: request.canary_job_id.clone(),
                canary_percent: request.canary_percent,
                updated_by: Some(principal.username.clone()),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    audit(
        &state,
        &principal.username,
        "update",
        "job",
        &job,
        Some(format!("name={}", updated.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(JobSummary::from(updated))))
}

/// List immutable job versions, newest first.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job}/versions",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    responses((status = 200, description = "Job version page", body = JobVersionPageApiResponse))
)]
pub async fn list_job_versions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_action): Path<String>,
) -> Result<Json<JobVersionPageApiResponse>, ApiError> {
    let job = job_action;
    let principal = auth::require_permission(&headers, &state, "jobs", "read").await?;
    let current = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &current.namespace,
        &current.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let items = state
        .jobs
        .versions()
        .list_versions(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(JobVersionPage {
        items,
        next_page_token: None,
    })))
}

/// Roll back a job to one immutable version and create a new latest version.
///
/// # Errors
///
/// Returns validation, authorization, not-found, or storage errors.
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{job}/rollback",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    request_body = RollbackJobRequest,
    responses((status = 200, description = "Rolled back job", body = JobApiResponse))
)]
pub async fn rollback_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_action): Path<String>,
    Json(request): Json<RollbackJobRequest>,
) -> Result<Json<JobApiResponse>, ApiError> {
    let job = job_action;
    if request.version_number < 1 {
        return Err(ApiError::bad_request("versionNumber must be positive"));
    }
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let current = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &current.namespace,
        &current.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let updated = state
        .jobs
        .rollback_job(
            &job,
            request.version_number,
            Some(principal.username.clone()),
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "job version not found: {job}#{}",
                request.version_number
            ))
        })?;
    audit(
        &state,
        &principal.username,
        "rollback",
        "job",
        &job,
        Some(format!("version={}", request.version_number)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(JobSummary::from(updated))))
}

/// Delete a job.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors.
#[utoipa::path(
    delete,
    path = "/api/v1/jobs/{job}",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    responses(
        (status = 200, description = "Deleted job", body = DeleteJobApiResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn delete_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
) -> Result<Json<DeleteJobApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let current = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &current.namespace,
        &current.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let deleted = state
        .jobs
        .delete_job(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found(format!("job not found: {job}")));
    }

    audit(
        &state,
        &principal.username,
        "delete",
        "job",
        &job,
        Some(format!("name={}", current.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(EmptyData {})))
}

async fn resolve_canary_routing(
    state: &AppState,
    job: &tikee_storage::JobSummary,
) -> Result<Option<CanaryRoutingSummary>, ApiError> {
    let percent = job.canary_percent.clamp(0, 100);
    let Some(canary_job_id) = job
        .canary_job_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if percent <= 0 {
        return Ok(Some(CanaryRoutingSummary {
            enabled: false,
            routed: false,
            original_job_id: job.id.clone(),
            routed_job_id: job.id.clone(),
            percent,
        }));
    }
    let canary = state
        .jobs
        .get(canary_job_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("canary job not found: {canary_job_id}")))?;
    if job.namespace != canary.namespace || job.app != canary.app {
        return Err(ApiError::bad_request(
            "canary job must belong to the same namespace/app",
        ));
    }
    let routed = canary_sample(&job.id, percent);
    Ok(Some(CanaryRoutingSummary {
        enabled: true,
        routed,
        original_job_id: job.id.clone(),
        routed_job_id: if routed { canary.id } else { job.id.clone() },
        percent,
    }))
}

fn canary_sample(_job_id: &str, percent: i32) -> bool {
    if percent >= 100 {
        return true;
    }
    if percent <= 0 {
        return false;
    }
    let bucket = rand::random::<u8>() % 100;
    bucket < u8::try_from(percent).unwrap_or(0)
}

async fn validate_plugin_processor_binding(
    state: &Arc<AppState>,
    processor_type: Option<&str>,
    processor_name: Option<&str>,
) -> Result<(), ApiError> {
    let Some(processor_type) = processor_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    if processor_type == "sdk" || processor_type == "script" {
        return Ok(());
    }
    let Some(processor) = state
        .plugins
        .resolve_processor_type(processor_type)
        .await
        .map_err(|error| ApiError::storage(&error))?
    else {
        return Err(ApiError::bad_request(format!(
            "plugin processor type is not registered or enabled: {processor_type}"
        )));
    };
    if processor.processor_names.is_empty() {
        return Err(ApiError::bad_request(
            "plugin processor type has no processorNames; maintain candidates in plugin management first",
        ));
    }
    let Some(processor_name) = processor_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(ApiError::bad_request(
            "plugin processorName is required and must come from plugin processorNames",
        ));
    };
    if !processor
        .processor_names
        .iter()
        .any(|candidate| candidate == processor_name)
    {
        return Err(ApiError::bad_request(format!(
            "plugin processorName is not declared for processorType {processor_type}: {processor_name}"
        )));
    }
    Ok(())
}

fn has_auth_header(headers: &HeaderMap) -> bool {
    headers.contains_key(axum::http::header::AUTHORIZATION)
        || headers.contains_key("x-tikee-token")
        || headers.contains_key("x-tikee-api-key")
}

fn has_concrete_binding(value: Option<&String>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
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
    let mut items = Vec::new();
    for instance in state
        .instances
        .list_by_job(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        items.push(instance_summary_with_latest_log(&state, instance).await?);
    }

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

    Ok(Json(ApiResponse::success(
        instance_summary_with_latest_log(&state, summary).await?,
    )))
}

/// Cancel a pending/running job instance and close its dispatch queue item.
#[utoipa::path(
    post,
    path = "/api/v1/instances/{instance}/cancel",
    tag = "jobs",
    params(("instance" = String, Path, description = "Instance identifier")),
    responses(
        (status = 200, description = "Cancelled job instance", body = JobInstanceCancelApiResponse),
        (status = 404, description = "Instance not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn cancel_job_instance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(instance): Path<String>,
) -> Result<Json<JobInstanceCancelApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "instances", "execute").await?;
    let cancelled = state
        .workflows
        .cancel_job_instance(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !cancelled {
        let exists = state
            .instances
            .get(&instance)
            .await
            .map_err(|error| ApiError::storage(&error))?
            .is_some();
        if !exists {
            return Err(ApiError::not_found(format!(
                "instance not found: {instance}"
            )));
        }
    }
    let summary = state
        .instances
        .get(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("instance not found: {instance}")))?;
    audit(
        &state,
        &principal.username,
        "cancel",
        "job_instance",
        &instance,
        Some(format!("cancelled={cancelled}")),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(
        instance_summary_with_latest_log(&state, summary).await?,
    )))
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
    Query(query): Query<PageQuery>,
) -> Result<Json<JobInstanceLogPageApiResponse>, ApiError> {
    let items = state
        .logs
        .list_by_instance(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceLogSummary::from)
        .filter(|log| {
            query.page_token.as_deref() != Some("script_execution_governance")
                || log.governance_event.as_deref() == Some("script_execution_governance")
        })
        .collect();

    Ok(Json(ApiResponse::success(JobInstanceLogPage {
        items,
        next_page_token: None,
    })))
}

fn serialize_schedule_calendar(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|value| {
        if value.is_null() {
            None
        } else {
            serde_json::to_string(value).ok()
        }
    })
}

impl From<tikee_storage::JobSummary> for JobSummary {
    fn from(value: tikee_storage::JobSummary) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            schedule_type: value.schedule_type,
            schedule_expr: value.schedule_expr,
            misfire_policy: value.misfire_policy,
            schedule_start_at: value.schedule_start_at,
            schedule_end_at: value.schedule_end_at,
            schedule_calendar: value
                .schedule_calendar_json
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok()),
            processor_name: value.processor_name,
            processor_type: value.processor_type,
            script_id: value.script_id,
            version_number: value.version_number,
            enabled: value.enabled,
            canary_job_id: value.canary_job_id,
            canary_percent: value.canary_percent,
        }
    }
}

async fn instance_summary_with_latest_log(
    state: &AppState,
    value: tikee_storage::JobInstanceSummary,
) -> Result<JobInstanceSummary, ApiError> {
    let log_count = state
        .logs
        .count_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let latest_log = state
        .logs
        .latest_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .map(JobInstanceLogSummary::from);
    let worker_id = state
        .logs
        .latest_worker_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(JobInstanceSummary {
        id: value.id,
        job_id: value.job_id,
        status: value.status.to_string(),
        trigger_type: value.trigger_type.to_string(),
        execution_mode: value.execution_mode.to_string(),
        created_at: value.created_at,
        updated_at: value.updated_at,
        log_count,
        latest_log,
        worker_id,
        canary_routing: None,
    })
}

impl From<tikee_storage::JobInstanceAttemptSummary> for JobInstanceAttemptSummary {
    fn from(value: tikee_storage::JobInstanceAttemptSummary) -> Self {
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

impl From<tikee_storage::JobInstanceLogSummary> for JobInstanceLogSummary {
    fn from(value: tikee_storage::JobInstanceLogSummary) -> Self {
        let governance = parse_log_governance(&value.message);
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            level: value.level,
            message: governance
                .as_ref()
                .and_then(|parsed| parsed.message.clone())
                .unwrap_or(value.message),
            governance_event: governance.as_ref().map(|parsed| parsed.event.clone()),
            governance_failure_class: governance
                .as_ref()
                .and_then(|parsed| parsed.failure_class.clone()),
            governance_message: governance.and_then(|parsed| parsed.message),
            sequence: value.sequence,
            created_at: value.created_at,
        }
    }
}

struct ParsedLogGovernance {
    event: String,
    failure_class: Option<String>,
    message: Option<String>,
}

fn parse_log_governance(message: &str) -> Option<ParsedLogGovernance> {
    let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
    let event = value.get("event")?.as_str()?.to_owned();
    if event != "script_execution_governance" {
        return None;
    }
    Some(ParsedLogGovernance {
        event,
        failure_class: value
            .get("failure_class")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        message: value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}
