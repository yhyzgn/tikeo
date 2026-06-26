use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::sse::{Event, Sse},
};
use tikeo_core::ExecutionMode;
use tikeo_storage::{CreateJob, CreateJobInstance, UpdateJob};
use tokio::{sync::mpsc, time};
use tokio_stream::{Stream, wrappers::ReceiverStream};

use crate::{
    notification::{JobNotificationEvent, emit_job_instance_event_best_effort},
    tunnel::registry::BroadcastSelector,
};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, CanaryMetricsGateSummary, CanaryRoutingSummary, CreateJobRequest,
        DeleteJobApiResponse, EmptyData, ErrorResponse, JobApiResponse, JobInstanceApiResponse,
        JobInstanceAttemptPage, JobInstanceAttemptPageApiResponse, JobInstanceAttemptSummary,
        JobInstanceCancelApiResponse, JobInstanceLogPage, JobInstanceLogPageApiResponse,
        JobInstanceLogSummary, JobInstancePage, JobInstancePageApiResponse, JobPageApiResponse,
        JobSummary, JobVersionPage, JobVersionPageApiResponse, Page, PageQuery, RollbackJobRequest,
        TriggerJobRequest, UpdateJobRequest,
    },
    error::ApiError,
};

use super::common::{
    StreamAuthQuery, apply_stream_token, audit, defaulted, parse_execution_mode,
    parse_schedule_type, parse_trigger_path, parse_trigger_type,
};
use views::{
    instance_list_stream_snapshot, instance_log_stream_snapshot, instance_summary_with_latest_log,
};

mod views;

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
                    job.worker_pool.as_deref(),
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
        request.worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app/worker pool",
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
    validate_worker_pool_scope(&state, &namespace, &app, request.worker_pool.as_deref()).await?;
    validate_canary_target_scope(&state, request.canary_job_id.as_deref(), &namespace, &app)
        .await?;
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
            worker_pool: request.worker_pool.clone(),
            script_id: request.script_id.clone(),
            enabled: request.enabled.unwrap_or(true),
            canary_job_id: request.canary_job_id.clone(),
            canary_percent: request.canary_percent.unwrap_or(0),
            canary_policy: request.canary_policy.clone(),
            created_by: Some(principal.username.clone()),
            retry_policy: request.retry_policy.clone(),
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
        job_summary.worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app/worker pool",
        ));
    }

    let trigger_type = parse_trigger_type(request.trigger_type.as_deref().unwrap_or("api"))?;
    let execution_mode =
        parse_execution_mode(request.execution_mode.as_deref().unwrap_or("single"))?;
    let canary_routing = resolve_canary_routing(&state, &job_summary).await?;
    if canary_routing
        .as_ref()
        .is_some_and(|routing| routing.rolled_back)
    {
        audit(
            &state,
            &principal.username,
            "canary_auto_rollback",
            "job",
            &job,
            Some("canary metrics gate set canaryPercent=0".to_owned()),
            &headers,
        )
        .await;
    }
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
            .find_persisted_broadcast_workers(
                &target_summary.namespace,
                &target_summary.app,
                target_summary.worker_pool.as_deref(),
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
        current.worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app/worker pool",
        ));
    }
    let target_namespace =
        update_scope_or_current(request.namespace.as_deref(), &current.namespace);
    let target_app = update_scope_or_current(request.app.as_deref(), &current.app);
    let target_worker_pool = request
        .worker_pool
        .clone()
        .unwrap_or_else(|| current.worker_pool.clone());
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &target_namespace,
        &target_app,
        target_worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow target namespace/app/worker pool",
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
    let final_canary_job_id = request
        .canary_job_id
        .clone()
        .unwrap_or_else(|| current.canary_job_id.clone());
    validate_worker_pool_scope(
        &state,
        &target_namespace,
        &target_app,
        target_worker_pool.as_deref(),
    )
    .await?;
    validate_canary_target_scope(
        &state,
        final_canary_job_id.as_deref(),
        &target_namespace,
        &target_app,
    )
    .await?;
    let updated = state
        .jobs
        .update_job(
            &job,
            UpdateJob {
                namespace: request.namespace.clone(),
                app: request.app.clone(),
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
                worker_pool: request.worker_pool.clone(),
                script_id: request.script_id.clone(),
                enabled: request.enabled,
                canary_job_id: request.canary_job_id.clone(),
                canary_percent: request.canary_percent,
                canary_policy: request.canary_policy.clone(),
                retry_policy: request.retry_policy.clone(),
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
        current.worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app/worker pool",
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
        current.worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app/worker pool",
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
        current.worker_pool.as_deref(),
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app/worker pool",
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
    job: &tikeo_storage::JobSummary,
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
            rolled_back: false,
            metrics_gate: None,
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
    if let Some(metrics_gate) = evaluate_canary_metrics_gate(state, job, &canary).await?
        && metrics_gate.status == "rollback"
        && job.canary_policy.auto_rollback
    {
        let rolled_back = state
            .jobs
            .update_job(
                &job.id,
                UpdateJob {
                    canary_percent: Some(0),
                    canary_policy: None,
                    updated_by: Some("system:canary-metrics-gate".to_owned()),
                    ..UpdateJob::default()
                },
            )
            .await
            .map_err(|error| ApiError::storage(&error))?
            .ok_or_else(|| ApiError::not_found(format!("job not found: {}", job.id)))?;
        return Ok(Some(CanaryRoutingSummary {
            enabled: false,
            routed: false,
            original_job_id: job.id.clone(),
            routed_job_id: rolled_back.id,
            percent: 0,
            rolled_back: true,
            metrics_gate: Some(metrics_gate),
        }));
    }
    let routed = canary_sample(&job.id, percent);
    Ok(Some(CanaryRoutingSummary {
        enabled: true,
        routed,
        original_job_id: job.id.clone(),
        routed_job_id: if routed {
            canary.id.clone()
        } else {
            job.id.clone()
        },
        percent,
        rolled_back: false,
        metrics_gate: evaluate_canary_metrics_gate(state, job, &canary).await?,
    }))
}

async fn evaluate_canary_metrics_gate(
    state: &AppState,
    job: &tikeo_storage::JobSummary,
    canary: &tikeo_storage::JobSummary,
) -> Result<Option<CanaryMetricsGateSummary>, ApiError> {
    let policy = job.canary_policy.clone().normalized();
    if !policy.metrics_gate_enabled {
        return Ok(None);
    }
    let instances = state
        .instances
        .list_by_job(&canary.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut inspected_samples = 0_u64;
    let mut failed_samples = 0_u64;
    for instance in instances {
        if inspected_samples >= policy.evaluation_window {
            break;
        }
        let status = instance.status.to_string();
        if !matches!(
            status.as_str(),
            "succeeded" | "failed" | "partial_failed" | "cancelled"
        ) {
            continue;
        }
        inspected_samples = inspected_samples.saturating_add(1);
        if matches!(status.as_str(), "failed" | "partial_failed" | "cancelled") {
            failed_samples = failed_samples.saturating_add(1);
        }
    }
    let failure_rate = failure_rate_ratio(failed_samples, inspected_samples);
    if inspected_samples < policy.minimum_samples {
        return Ok(Some(CanaryMetricsGateSummary {
            status: "insufficient_samples".to_owned(),
            inspected_samples,
            failed_samples,
            failure_rate,
            threshold: policy.max_failure_rate,
            reason: format!(
                "needs at least {} terminal canary samples",
                policy.minimum_samples
            ),
        }));
    }
    let should_rollback = failure_rate > policy.max_failure_rate;
    Ok(Some(CanaryMetricsGateSummary {
        status: if should_rollback { "rollback" } else { "pass" }.to_owned(),
        inspected_samples,
        failed_samples,
        failure_rate,
        threshold: policy.max_failure_rate,
        reason: if should_rollback {
            format!(
                "failure rate {:.2} exceeded threshold {:.2}",
                failure_rate, policy.max_failure_rate
            )
        } else {
            format!(
                "failure rate {:.2} within threshold {:.2}",
                failure_rate, policy.max_failure_rate
            )
        },
    }))
}

fn failure_rate_ratio(failed_samples: u64, inspected_samples: u64) -> f64 {
    if inspected_samples == 0 {
        return 0.0;
    }
    let failed = u32::try_from(failed_samples).unwrap_or(u32::MAX);
    let inspected = u32::try_from(inspected_samples).unwrap_or(u32::MAX).max(1);
    f64::from(failed) / f64::from(inspected)
}

async fn validate_canary_target_scope(
    state: &AppState,
    canary_job_id: Option<&str>,
    namespace: &str,
    app: &str,
) -> Result<(), ApiError> {
    let Some(canary_job_id) = canary_job_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let canary = state
        .jobs
        .get(canary_job_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("canary job not found: {canary_job_id}")))?;
    if canary.namespace != namespace || canary.app != app {
        return Err(ApiError::bad_request(
            "canary job must belong to the target namespace/app",
        ));
    }
    Ok(())
}

async fn validate_worker_pool_scope(
    state: &AppState,
    namespace: &str,
    app: &str,
    worker_pool: Option<&str>,
) -> Result<(), ApiError> {
    let Some(worker_pool) = worker_pool.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let exists = state
        .jobs
        .scopes()
        .list_worker_pools(Some(namespace), Some(app))
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .any(|pool| pool.name == worker_pool);
    if !exists {
        return Err(ApiError::bad_request(
            "workerPool must belong to the target namespace/app",
        ));
    }
    Ok(())
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

fn update_scope_or_current(value: Option<&str>, current: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(current)
        .to_owned()
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
        || headers.contains_key("x-tikeo-token")
        || headers.contains_key("x-tikeo-api-key")
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

/// Stream the visible job instance list via Server-Sent Events.
///
/// # Errors
///
/// Returns authentication or storage errors before opening the stream.
pub async fn stream_instances(
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    Query(query): Query<StreamAuthQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    apply_stream_token(&mut headers, &query)?;
    let principal = auth::require_permission(&headers, &state, "instances", "read").await?;
    let (tx, rx) = mpsc::channel(16);

    tokio::spawn(async move {
        let mut last_snapshot_json: Option<String> = None;
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            if let Ok(snapshot) = instance_list_stream_snapshot(&state, &principal).await
                && let Ok(snapshot_json) = serde_json::to_string(&snapshot)
                && last_snapshot_json.as_deref() != Some(snapshot_json.as_str())
            {
                last_snapshot_json = Some(snapshot_json.clone());
                if tx
                    .send(Ok::<_, Infallible>(
                        Event::default()
                            .event("instances.snapshot")
                            .data(snapshot_json),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            interval.tick().await;
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
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
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors.
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
    if summary.status == tikeo_core::InstanceStatus::Cancelled {
        let notifications = state.notification_center();
        emit_job_instance_event_best_effort(
            &notifications,
            &summary,
            JobNotificationEvent::Cancelled,
            Some("instance cancelled through management API"),
        )
        .await;
    }
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

/// Stream one job instance's logs and status snapshots via Server-Sent Events.
///
/// # Errors
///
/// Returns authentication, not-found, or storage errors before opening the stream.
pub async fn stream_instance_logs(
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    Path(instance): Path<String>,
    Query(query): Query<StreamAuthQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    apply_stream_token(&mut headers, &query)?;
    auth::require_permission(&headers, &state, "instances", "read").await?;

    let _ = state
        .instances
        .get(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("instance not found: {instance}")))?;

    let last_event_id = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(-1);
    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        let mut after_sequence = last_event_id;
        let mut last_snapshot_json: Option<String> = None;
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            if let Ok(snapshot) = instance_log_stream_snapshot(&state, &instance).await
                && let Ok(snapshot_json) = serde_json::to_string(&snapshot)
                && last_snapshot_json.as_deref() != Some(snapshot_json.as_str())
            {
                last_snapshot_json = Some(snapshot_json.clone());
                if tx
                    .send(Ok::<_, Infallible>(
                        Event::default()
                            .event("instance.snapshot")
                            .data(snapshot_json),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }

            if let Ok(logs) = state
                .logs
                .list_by_instance_after_sequence(&instance, after_sequence)
                .await
            {
                for log in logs {
                    let item = JobInstanceLogSummary::from(log);
                    after_sequence = after_sequence.max(item.sequence);
                    let Ok(log_json) = serde_json::to_string(&item) else {
                        continue;
                    };
                    if tx
                        .send(Ok::<_, Infallible>(
                            Event::default()
                                .id(item.sequence.to_string())
                                .event("instance.log")
                                .data(log_json),
                        ))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }

            interval.tick().await;
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
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
