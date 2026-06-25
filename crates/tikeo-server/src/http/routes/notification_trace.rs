use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::http::{AppState, auth, dto::ApiResponse, error::ApiError};

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationMessageTraceResponse {
    pub message: tikeo_storage::NotificationMessageSummary,
    pub policy: Option<tikeo_storage::NotificationPolicySummary>,
    pub attempts: Vec<tikeo_storage::NotificationDeliveryAttemptSummary>,
    pub job: Option<NotificationTraceJob>,
    pub instance: Option<NotificationTraceInstance>,
    pub logs: NotificationTraceLogs,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTraceJob {
    pub id: String,
    pub namespace: String,
    pub app: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTraceInstance {
    pub id: String,
    pub job_id: String,
    pub status: String,
    /// Trigger type value.
    pub trigger_type: String,
    /// Execution mode value.
    pub execution_mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub worker_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTraceLogs {
    pub url: Option<String>,
    pub excerpt: Vec<NotificationTraceLogLine>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTraceLogLine {
    pub level: String,
    pub worker_id: String,
    pub sequence: i64,
    pub message: String,
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/public/job-instances/{id}/trace",
    tag = "notifications"
)]
/// Get public job instance trace.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn get_public_job_instance_trace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<NotificationMessageTraceResponse>>, ApiError> {
    let message = state
        .notification_messages
        .list_messages(tikeo_storage::NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(id),
            ..Default::default()
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::not_found("notification message not found"))?;
    notification_message_trace_response(&state, message, true).await
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-messages/{id}/trace",
    tag = "notifications"
)]
/// Get notification message trace.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn get_notification_message_trace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<NotificationMessageTraceResponse>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "read").await?;
    let message = state
        .notification_messages
        .get_message(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification message not found"))?;
    let job = notification_trace_job(&state, &message).await?;
    if let Some(job) = job.as_ref()
        && !crate::http::access_scope::allows_resource(
            &principal.scope_bindings,
            &job.namespace,
            &job.app,
            None,
        )
    {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this notification trace",
        ));
    }
    notification_message_trace_response_with_job(&state, message, job, false).await
}

async fn notification_message_trace_response(
    state: &Arc<AppState>,
    message: tikeo_storage::NotificationMessageSummary,
    public_console: bool,
) -> Result<Json<ApiResponse<NotificationMessageTraceResponse>>, ApiError> {
    let job = notification_trace_job(state, &message).await?;
    notification_message_trace_response_with_job(state, message, job, public_console).await
}

async fn notification_message_trace_response_with_job(
    state: &Arc<AppState>,
    message: tikeo_storage::NotificationMessageSummary,
    job: Option<tikeo_storage::JobSummary>,
    public_console: bool,
) -> Result<Json<ApiResponse<NotificationMessageTraceResponse>>, ApiError> {
    let policy = state
        .notification_policies
        .get_policy(&message.policy_id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let attempts = state
        .notification_delivery_attempts
        .list_attempts(tikeo_storage::NotificationDeliveryAttemptFilters {
            message_id: Some(message.id.clone()),
            ..Default::default()
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let instance_id = trace_instance_id(&message);
    let instance = notification_trace_instance(state, instance_id.as_deref()).await?;
    let logs = if let Some(instance_id) = instance_id.as_deref() {
        notification_trace_logs(state, instance_id, public_console).await?
    } else {
        NotificationTraceLogs {
            url: None,
            excerpt: Vec::new(),
            truncated: false,
        }
    };
    Ok(Json(ApiResponse::success(
        NotificationMessageTraceResponse {
            message,
            policy,
            attempts,
            job: job.map(|item| NotificationTraceJob {
                id: item.id,
                namespace: item.namespace,
                app: item.app,
                name: item.name,
            }),
            instance: instance.map(|item| NotificationTraceInstance {
                id: item.id,
                job_id: item.job_id,
                status: item.status.to_string(),
                trigger_type: item.trigger_type.to_string(),
                execution_mode: item.execution_mode.to_string(),
                created_at: item.created_at,
                updated_at: item.updated_at,
                worker_id: item.result.as_ref().map(|result| result.worker_id.clone()),
            }),
            logs,
        },
    )))
}

async fn notification_trace_job(
    state: &Arc<AppState>,
    message: &tikeo_storage::NotificationMessageSummary,
) -> Result<Option<tikeo_storage::JobSummary>, ApiError> {
    let instance_id = trace_instance_id(message);
    if let Some(instance) = notification_trace_instance(state, instance_id.as_deref()).await? {
        return state
            .jobs
            .get(&instance.job_id)
            .await
            .map_err(|error| ApiError::storage(&error));
    }
    if message.resource_type == "job" {
        return state
            .jobs
            .get(&message.resource_id)
            .await
            .map_err(|error| ApiError::storage(&error));
    }
    Ok(None)
}

async fn notification_trace_instance(
    state: &Arc<AppState>,
    instance_id: Option<&str>,
) -> Result<Option<tikeo_storage::JobInstanceSummary>, ApiError> {
    let Some(instance_id) = instance_id else {
        return Ok(None);
    };
    state
        .instances
        .get(instance_id)
        .await
        .map_err(|error| ApiError::storage(&error))
}

fn trace_instance_id(message: &tikeo_storage::NotificationMessageSummary) -> Option<String> {
    if message.source_type == "job_instance" {
        return Some(message.source_id.clone());
    }
    serde_json::from_str::<serde_json::Value>(&message.payload_json)
        .ok()
        .and_then(|payload| {
            payload
                .get("instanceId")
                .or_else(|| payload.pointer("/instance/id"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
}

async fn notification_trace_logs(
    state: &Arc<AppState>,
    instance_id: &str,
    public_console: bool,
) -> Result<NotificationTraceLogs, ApiError> {
    let logs = state
        .logs
        .list_by_instance(instance_id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let total = logs.len();
    let excerpt = logs
        .into_iter()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|log| NotificationTraceLogLine {
            level: log.level,
            worker_id: log.worker_id,
            sequence: log.sequence,
            message: redact_log_line(&log.message),
            created_at: log.created_at,
        })
        .collect();
    Ok(NotificationTraceLogs {
        url: Some(if public_console {
            format!("/public/instances/{instance_id}/console")
        } else {
            format!("/instances/{instance_id}/logs")
        }),
        excerpt,
        truncated: total > 80,
    })
}

fn redact_log_line(value: &str) -> String {
    let mut redacted = value.to_owned();
    for key in [
        "password",
        "token",
        "secret",
        "authorization",
        "routingKey",
        "signingKey",
    ] {
        redacted = redact_key_value(&redacted, key);
    }
    redacted
}

fn redact_key_value(value: &str, key: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let Some(position) = lower.find(&key.to_ascii_lowercase()) else {
        return value.to_owned();
    };
    let prefix = &value[..position + key.len()];
    let suffix = &value[position + key.len()..];
    if let Some(rest) = suffix.strip_prefix('=') {
        let tail = rest
            .find(|ch: char| ch.is_whitespace())
            .map_or("", |idx| &rest[idx..]);
        return format!("{prefix}=***{tail}");
    }
    if let Some(rest) = suffix.strip_prefix(':') {
        let tail = rest
            .find(|ch: char| ch == ',' || ch == '}' || ch.is_whitespace())
            .map_or("", |idx| &rest[idx..]);
        return format!("{prefix}:***{tail}");
    }
    value.to_owned()
}
