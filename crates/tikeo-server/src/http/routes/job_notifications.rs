#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use tikeo_storage::{
    CreateNotificationPolicy, NotificationPolicySummary, UpdateNotificationPolicy,
};
use utoipa::ToSchema;

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, EmptyData, MeResponse},
    error::ApiError,
};

use super::{common::audit, notification_providers::json_to_string};

const JOB_INSTANCE_EVENT_FAMILY: &str = "job_instance";

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobNotificationBindingRequest {
    pub name: String,
    pub trigger: JobNotificationTrigger,
    #[serde(default)]
    pub event_types: Vec<String>,
    #[serde(default)]
    pub channel_ids: Vec<String>,
    pub template_ref: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default = "default_dedupe_seconds")]
    pub dedupe_seconds: i64,
    #[serde(default = "default_true")]
    pub include_log_link: bool,
    #[serde(default)]
    pub include_log_excerpt: bool,
    #[serde(default = "default_log_excerpt_lines")]
    pub log_excerpt_lines: u64,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[allow(clippy::option_option)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobNotificationBindingRequest {
    pub name: Option<String>,
    pub trigger: Option<JobNotificationTrigger>,
    pub event_types: Option<Vec<String>>,
    pub channel_ids: Option<Vec<String>>,
    pub template_ref: Option<Option<String>>,
    pub enabled: Option<bool>,
    pub severity: Option<String>,
    pub dedupe_seconds: Option<i64>,
    pub include_log_link: Option<bool>,
    pub include_log_excerpt: Option<bool>,
    pub log_excerpt_lines: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobNotificationTrigger {
    Success,
    Failure,
    Always,
    Cancelled,
    RetryScheduled,
    RetryExhausted,
    Advanced,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobNotificationBindingSummary {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub trigger: String,
    pub event_types: Vec<String>,
    pub channel_ids: Vec<String>,
    pub template_ref: Option<String>,
    pub enabled: bool,
    pub severity: String,
    pub dedupe_seconds: i64,
    pub include_log_link: bool,
    pub include_log_excerpt: bool,
    pub log_excerpt_lines: u64,
    pub policy: NotificationPolicySummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobNotificationBindingValidationSummary {
    pub valid: bool,
    pub event_types: Vec<String>,
    pub channel_count: u64,
    pub missing_channel_ids: Vec<String>,
    pub disabled_channel_ids: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobNotificationBindingPreview {
    pub job_id: String,
    pub trigger: String,
    pub event_types: Vec<String>,
    pub sample_context: serde_json::Value,
    pub rendered_template: Option<serde_json::Value>,
    pub validation: JobNotificationBindingValidationSummary,
}

#[utoipa::path(get, path = "/api/v1/jobs/{job}/notification-bindings", tag = "jobs")]
pub async fn list_job_notification_bindings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
) -> Result<Json<ApiResponse<Vec<JobNotificationBindingSummary>>>, ApiError> {
    let (_principal, _job_summary) = require_job_access(&state, &headers, &job, "read").await?;
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let policies = state
        .notification_policies
        .list_policies(tikeo_storage::NotificationPolicyFilters {
            owner_type: Some("job".to_owned()),
            owner_id: Some(job.clone()),
            event_family: Some(JOB_INSTANCE_EVENT_FAMILY.to_owned()),
            enabled: None,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(
        policies.into_iter().map(binding_from_policy).collect(),
    )))
}

#[utoipa::path(post, path = "/api/v1/jobs/{job}/notification-bindings", tag = "jobs", request_body = CreateJobNotificationBindingRequest)]
pub async fn create_job_notification_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
    Json(request): Json<CreateJobNotificationBindingRequest>,
) -> Result<Json<ApiResponse<JobNotificationBindingSummary>>, ApiError> {
    let (principal, _job_summary) = require_job_access(&state, &headers, &job, "write").await?;
    auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let event_types = expand_trigger(&request.trigger, &request.event_types)?;
    let validation = validate_binding_refs(
        &state,
        &request.channel_ids,
        request.template_ref.as_deref(),
        &event_types,
    )
    .await?;
    if !validation.valid {
        return Err(ApiError::bad_request(validation.issues.join("; ")));
    }
    let filters = binding_filter_json(
        &request.trigger,
        &event_types,
        request.include_log_link,
        request.include_log_excerpt,
        request.log_excerpt_lines,
    );
    let channel_refs = channel_refs_json(&request.channel_ids);
    let created = state
        .notification_policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some(job.clone()),
            name: request.name,
            event_family: JOB_INSTANCE_EVENT_FAMILY.to_owned(),
            event_filter_json: json_to_string(&filters),
            channel_refs_json: json_to_string(&channel_refs),
            template_ref: request.template_ref,
            severity: request.severity,
            enabled: request.enabled,
            dedupe_seconds: request.dedupe_seconds,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    audit(
        &state,
        &principal.username,
        "create",
        "job_notification_binding",
        &created.id,
        Some(format!("job={job}")),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(binding_from_policy(created))))
}

#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job}/notification-bindings/{binding}",
    tag = "jobs"
)]
pub async fn get_job_notification_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((job, binding)): Path<(String, String)>,
) -> Result<Json<ApiResponse<JobNotificationBindingSummary>>, ApiError> {
    let (_principal, _job_summary) = require_job_access(&state, &headers, &job, "read").await?;
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let policy = get_job_policy(&state, &job, &binding).await?;
    Ok(Json(ApiResponse::success(binding_from_policy(policy))))
}

#[utoipa::path(patch, path = "/api/v1/jobs/{job}/notification-bindings/{binding}", tag = "jobs", request_body = UpdateJobNotificationBindingRequest)]
pub async fn update_job_notification_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((job, binding)): Path<(String, String)>,
    Json(request): Json<UpdateJobNotificationBindingRequest>,
) -> Result<Json<ApiResponse<JobNotificationBindingSummary>>, ApiError> {
    let (principal, _job_summary) = require_job_access(&state, &headers, &job, "write").await?;
    auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let existing = get_job_policy(&state, &job, &binding).await?;
    let existing_binding = binding_from_policy(existing.clone());
    let trigger = request
        .trigger
        .clone()
        .unwrap_or_else(|| trigger_from_name(&existing_binding.trigger));
    let requested_events = request
        .event_types
        .clone()
        .unwrap_or_else(|| existing_binding.event_types.clone());
    let event_types = expand_trigger(&trigger, &requested_events)?;
    let channel_ids = request
        .channel_ids
        .clone()
        .unwrap_or_else(|| existing_binding.channel_ids.clone());
    let template_ref = request
        .template_ref
        .clone()
        .unwrap_or_else(|| existing_binding.template_ref.clone());
    let include_log_link = request
        .include_log_link
        .unwrap_or(existing_binding.include_log_link);
    let include_log_excerpt = request
        .include_log_excerpt
        .unwrap_or(existing_binding.include_log_excerpt);
    let log_excerpt_lines = request
        .log_excerpt_lines
        .unwrap_or(existing_binding.log_excerpt_lines);
    let validation =
        validate_binding_refs(&state, &channel_ids, template_ref.as_deref(), &event_types).await?;
    if !validation.valid {
        return Err(ApiError::bad_request(validation.issues.join("; ")));
    }
    let updated = state
        .notification_policies
        .update_policy(
            &binding,
            UpdateNotificationPolicy {
                owner_type: None,
                owner_id: None,
                name: request.name,
                event_family: None,
                event_filter_json: Some(json_to_string(&binding_filter_json(
                    &trigger,
                    &event_types,
                    include_log_link,
                    include_log_excerpt,
                    log_excerpt_lines,
                ))),
                channel_refs_json: Some(json_to_string(&channel_refs_json(&channel_ids))),
                template_ref: Some(template_ref),
                severity: request.severity,
                enabled: request.enabled,
                dedupe_seconds: request.dedupe_seconds,
                throttle_json: None,
                quiet_hours_json: None,
                escalation_json: None,
                updated_by: Some(Some(principal.username.clone())),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("job notification binding not found"))?;
    audit(
        &state,
        &principal.username,
        "update",
        "job_notification_binding",
        &updated.id,
        Some(format!("job={job}")),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(binding_from_policy(updated))))
}

#[utoipa::path(
    delete,
    path = "/api/v1/jobs/{job}/notification-bindings/{binding}",
    tag = "jobs"
)]
pub async fn delete_job_notification_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((job, binding)): Path<(String, String)>,
) -> Result<Json<ApiResponse<EmptyData>>, ApiError> {
    let (principal, _job_summary) = require_job_access(&state, &headers, &job, "write").await?;
    auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let _existing = get_job_policy(&state, &job, &binding).await?;
    let deleted = state
        .notification_policies
        .delete_policy(&binding)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("job notification binding not found"));
    }
    audit(
        &state,
        &principal.username,
        "delete",
        "job_notification_binding",
        &binding,
        Some(format!("job={job}")),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

#[utoipa::path(post, path = "/api/v1/jobs/{job}/notification-bindings:validate", tag = "jobs", request_body = CreateJobNotificationBindingRequest)]
pub async fn validate_job_notification_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
    Json(request): Json<CreateJobNotificationBindingRequest>,
) -> Result<Json<ApiResponse<JobNotificationBindingValidationSummary>>, ApiError> {
    let (_principal, _job_summary) = require_job_access(&state, &headers, &job, "read").await?;
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let event_types = expand_trigger(&request.trigger, &request.event_types)?;
    let validation = validate_binding_refs(
        &state,
        &request.channel_ids,
        request.template_ref.as_deref(),
        &event_types,
    )
    .await?;
    Ok(Json(ApiResponse::success(validation)))
}

#[utoipa::path(post, path = "/api/v1/jobs/{job}/notification-bindings:preview", tag = "jobs", request_body = CreateJobNotificationBindingRequest)]
pub async fn preview_job_notification_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
    Json(request): Json<CreateJobNotificationBindingRequest>,
) -> Result<Json<ApiResponse<JobNotificationBindingPreview>>, ApiError> {
    let (_principal, job_summary) = require_job_access(&state, &headers, &job, "read").await?;
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let event_types = expand_trigger(&request.trigger, &request.event_types)?;
    let validation = validate_binding_refs(
        &state,
        &request.channel_ids,
        request.template_ref.as_deref(),
        &event_types,
    )
    .await?;
    let sample_context = sample_job_notification_context(
        &job_summary,
        event_types
            .first()
            .map_or("job_instance.failed", String::as_str),
    );
    let rendered_template = if let Some(template_ref) = request.template_ref.as_deref() {
        let template = state
            .notification_templates
            .get_template(template_ref)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        template.map(|item| {
            let body = serde_json::from_str::<serde_json::Value>(&item.body_json)
                .unwrap_or_else(|_| serde_json::json!({}));
            crate::notification::render_notification_template_preview(&body, &sample_context)
        })
    } else {
        None
    };
    Ok(Json(ApiResponse::success(JobNotificationBindingPreview {
        job_id: job,
        trigger: trigger_name(&request.trigger).to_owned(),
        event_types,
        sample_context,
        rendered_template,
        validation,
    })))
}

async fn require_job_access(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    job: &str,
    action: &str,
) -> Result<(MeResponse, tikeo_storage::JobSummary), ApiError> {
    let principal = auth::require_permission(headers, state, "jobs", action).await?;
    let item = state
        .jobs
        .get(job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &item.namespace,
        &item.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    Ok((principal, item))
}

async fn get_job_policy(
    state: &Arc<AppState>,
    job: &str,
    binding: &str,
) -> Result<NotificationPolicySummary, ApiError> {
    let policy = state
        .notification_policies
        .get_policy(binding)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("job notification binding not found"))?;
    if policy.owner_type != "job"
        || policy.owner_id.as_deref() != Some(job)
        || policy.event_family != JOB_INSTANCE_EVENT_FAMILY
    {
        return Err(ApiError::not_found("job notification binding not found"));
    }
    Ok(policy)
}

async fn validate_binding_refs(
    state: &Arc<AppState>,
    channel_ids: &[String],
    template_ref: Option<&str>,
    event_types: &[String],
) -> Result<JobNotificationBindingValidationSummary, ApiError> {
    let channels = state
        .notification_channels
        .list_channels(tikeo_storage::NotificationChannelFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut missing_channel_ids = Vec::new();
    let mut disabled_channel_ids = Vec::new();
    let mut providers = Vec::<String>::new();
    for channel_id in channel_ids {
        match channels.iter().find(|item| item.id == *channel_id) {
            Some(channel) if channel.enabled => providers.push(channel.provider.clone()),
            Some(_) => disabled_channel_ids.push(channel_id.clone()),
            None => missing_channel_ids.push(channel_id.clone()),
        }
    }
    providers.sort();
    providers.dedup();
    let mut issues = Vec::new();
    if channel_ids.is_empty() {
        issues.push("at least one notification channel is required".to_owned());
    }
    if event_types.is_empty() {
        issues.push("at least one job instance event type is required".to_owned());
    }
    if !missing_channel_ids.is_empty() {
        issues.push(format!(
            "missing notification channels: {}",
            missing_channel_ids.join(",")
        ));
    }
    if !disabled_channel_ids.is_empty() {
        issues.push(format!(
            "disabled notification channels: {}",
            disabled_channel_ids.join(",")
        ));
    }
    if let Some(template_ref) = template_ref.filter(|value| !value.trim().is_empty()) {
        match state
            .notification_templates
            .get_template(template_ref)
            .await
            .map_err(|error| ApiError::storage(&error))?
        {
            Some(template) if !template.enabled => {
                issues.push(format!("notification template is disabled: {template_ref}"));
            }
            Some(template)
                if !providers.is_empty()
                    && !providers.iter().any(|item| item == &template.provider) =>
            {
                issues.push(format!(
                    "notification template provider {} does not match selected channels",
                    template.provider
                ));
            }
            Some(_) => {}
            None => issues.push(format!("notification template not found: {template_ref}")),
        }
    }
    Ok(JobNotificationBindingValidationSummary {
        valid: issues.is_empty(),
        event_types: event_types.to_vec(),
        channel_count: providers.len() as u64,
        missing_channel_ids,
        disabled_channel_ids,
        issues,
    })
}

fn binding_from_policy(policy: NotificationPolicySummary) -> JobNotificationBindingSummary {
    let filter = parse_json(&policy.event_filter_json);
    let trigger = filter
        .get("triggerPreset")
        .or_else(|| filter.get("trigger_preset"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("advanced")
        .to_owned();
    let event_types = event_types_from_filter(&filter);
    let channel_ids = channel_ids_from_refs(&policy.channel_refs_json);
    let include_log_link = filter
        .get("includeLogLink")
        .or_else(|| filter.get("include_log_link"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let include_log_excerpt = filter
        .get("includeLogExcerpt")
        .or_else(|| filter.get("include_log_excerpt"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let log_excerpt_lines = filter
        .get("logExcerptLines")
        .or_else(|| filter.get("log_excerpt_lines"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(default_log_excerpt_lines);
    JobNotificationBindingSummary {
        id: policy.id.clone(),
        job_id: policy.owner_id.clone().unwrap_or_default(),
        name: policy.name.clone(),
        trigger,
        event_types,
        channel_ids,
        template_ref: policy.template_ref.clone(),
        enabled: policy.enabled,
        severity: policy.severity.clone(),
        dedupe_seconds: policy.dedupe_seconds,
        include_log_link,
        include_log_excerpt,
        log_excerpt_lines,
        policy,
    }
}

fn expand_trigger(
    trigger: &JobNotificationTrigger,
    event_types: &[String],
) -> Result<Vec<String>, ApiError> {
    let events = match trigger {
        JobNotificationTrigger::Success => vec!["job_instance.succeeded"],
        JobNotificationTrigger::Failure => vec![
            "job_instance.failed",
            "job_instance.partial_failed",
            "job_instance.retry_exhausted",
            "job_instance.no_eligible_worker",
            "job_instance.script_governance_failure",
        ],
        JobNotificationTrigger::Always => vec![
            "job_instance.running",
            "job_instance.succeeded",
            "job_instance.failed",
            "job_instance.partial_failed",
            "job_instance.retry_exhausted",
            "job_instance.no_eligible_worker",
            "job_instance.script_governance_failure",
            "job_instance.cancelled",
        ],
        JobNotificationTrigger::Cancelled => vec!["job_instance.cancelled"],
        JobNotificationTrigger::RetryScheduled => vec!["job_instance.retry_scheduled"],
        JobNotificationTrigger::RetryExhausted => vec!["job_instance.retry_exhausted"],
        JobNotificationTrigger::Advanced => event_types.iter().map(String::as_str).collect(),
    };
    let mut normalized = Vec::new();
    for event in events {
        let Some(canonical) = canonical_job_event(event) else {
            return Err(ApiError::bad_request(format!(
                "unsupported job notification event type: {event}"
            )));
        };
        if !normalized.iter().any(|item| item == canonical) {
            normalized.push(canonical.to_owned());
        }
    }
    Ok(normalized)
}

fn canonical_job_event(value: &str) -> Option<&'static str> {
    match value.trim() {
        "job_instance.running" | "job_instance.运行中" => Some("job_instance.running"),
        "job_instance.succeeded" | "job_instance.success" | "job_instance.成功" => {
            Some("job_instance.succeeded")
        }
        "job_instance.failed" | "job_instance.failure" | "job_instance.失败" => {
            Some("job_instance.failed")
        }
        "job_instance.partial_failed" | "job_instance.部分失败" => {
            Some("job_instance.partial_failed")
        }
        "job_instance.cancelled" | "job_instance.canceled" | "job_instance.取消" => {
            Some("job_instance.cancelled")
        }
        "job_instance.retry_scheduled" | "job_instance.重试中" => {
            Some("job_instance.retry_scheduled")
        }
        "job_instance.retry_exhausted" | "job_instance.重试耗尽" => {
            Some("job_instance.retry_exhausted")
        }
        "job_instance.no_eligible_worker" | "job_instance.无可用执行节点" => {
            Some("job_instance.no_eligible_worker")
        }
        "job_instance.script_governance_failure" | "job_instance.脚本治理失败" => {
            Some("job_instance.script_governance_failure")
        }
        _ => None,
    }
}

fn binding_filter_json(
    trigger: &JobNotificationTrigger,
    event_types: &[String],
    include_log_link: bool,
    include_log_excerpt: bool,
    log_excerpt_lines: u64,
) -> serde_json::Value {
    serde_json::json!({
        "triggerPreset": trigger_name(trigger),
        "eventTypes": event_types,
        "statuses": event_types.iter().filter_map(|event| event.strip_prefix("job_instance.")).collect::<Vec<_>>(),
        "includeLogLink": include_log_link,
        "includeLogExcerpt": include_log_excerpt,
        "logExcerptLines": log_excerpt_lines.clamp(1, 500),
    })
}

fn channel_refs_json(channel_ids: &[String]) -> Vec<serde_json::Value> {
    channel_ids
        .iter()
        .filter(|item| !item.trim().is_empty())
        .map(|channel_id| serde_json::json!({"channelId": channel_id}))
        .collect()
}

fn channel_ids_from_refs(raw: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.get("channelId")
                .or_else(|| item.get("channel_id"))
                .or_else(|| item.get("id"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn event_types_from_filter(filter: &serde_json::Value) -> Vec<String> {
    filter
        .get("eventTypes")
        .or_else(|| filter.get("event_types"))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(|event| canonical_job_event(event).unwrap_or(event).to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_json(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn trigger_from_name(value: &str) -> JobNotificationTrigger {
    match value {
        "success" => JobNotificationTrigger::Success,
        "failure" => JobNotificationTrigger::Failure,
        "always" => JobNotificationTrigger::Always,
        "cancelled" => JobNotificationTrigger::Cancelled,
        "retry_scheduled" => JobNotificationTrigger::RetryScheduled,
        "retry_exhausted" => JobNotificationTrigger::RetryExhausted,
        _ => JobNotificationTrigger::Advanced,
    }
}

const fn trigger_name(trigger: &JobNotificationTrigger) -> &'static str {
    match trigger {
        JobNotificationTrigger::Success => "success",
        JobNotificationTrigger::Failure => "failure",
        JobNotificationTrigger::Always => "always",
        JobNotificationTrigger::Cancelled => "cancelled",
        JobNotificationTrigger::RetryScheduled => "retry_scheduled",
        JobNotificationTrigger::RetryExhausted => "retry_exhausted",
        JobNotificationTrigger::Advanced => "advanced",
    }
}

fn sample_job_notification_context(
    job: &tikeo_storage::JobSummary,
    event_type: &str,
) -> serde_json::Value {
    serde_json::json!({
        "subject": format!("Tikeo job {}: {}", job.name, event_type),
        "body": format!("Job {} sample notification for {}", job.name, event_type),
        "eventType": event_type,
        "resourceType": "job",
        "resourceId": job.id,
        "severity": "critical",
        "messageId": "preview-message",
        "policyId": "preview-policy",
        "dedupeKey": format!("preview:{}:{event_type}", job.id),
        "triggeredAt": "2026-06-13T00:00:00Z",
        "jobId": job.id,
        "jobName": job.name,
        "namespace": job.namespace,
        "app": job.app,
        "instanceId": "preview-instance",
        "status": event_type.strip_prefix("job_instance.").unwrap_or(event_type),
        "logsUrl": "/public/instances/preview-instance/console",
        "consoleUrl": "/public/instances/preview-instance/console",
    })
}

const fn default_enabled() -> bool {
    true
}
const fn default_true() -> bool {
    true
}
fn default_severity() -> String {
    "critical".to_owned()
}
const fn default_dedupe_seconds() -> i64 {
    300
}
const fn default_log_excerpt_lines() -> u64 {
    80
}
