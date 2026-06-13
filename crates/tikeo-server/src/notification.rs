//! Notification Center message materialization and delivery ledger helpers.

mod alert_events;
mod delivery;
mod provider_templates;
mod signing;
mod workflow_events;

pub use alert_events::{
    AlertRuleNotificationBackfillSummary, backfill_alert_rule_notification_policies,
    ensure_alert_rule_notification_policy_from_channels,
};
pub use workflow_events::{
    emit_workflow_notification_node_requested,
    emit_workflow_notification_node_requested_best_effort,
};

use serde::{Deserialize, Serialize};
use tikeo_core::InstanceStatus;
use tikeo_storage::{
    CreateNotificationMessage, JobInstanceSummary, JobRepository, NotificationChannelFilters,
    NotificationChannelRepository, NotificationDeliveryAttemptRepository,
    NotificationMessageRepository, NotificationMessageSummary, NotificationPolicyRepository,
    NotificationPolicySummary, NotificationTemplateRepository, NotificationTemplateSummary,
    RecordNotificationDeliveryAttempt,
};
use tracing::warn;

use delivery::dedupe_window_elapsed;
use provider_templates::{render_template_value, validate_template_tokens};

/// Stable event emitted from a job instance lifecycle transition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobNotificationEvent {
    /// A retry has been scheduled after a failed attempt.
    RetryScheduled,
    /// All retry attempts have been exhausted.
    RetryExhausted,
    /// Terminal success.
    Succeeded,
    /// Terminal failure.
    Failed,
    /// Broadcast parent completed with at least one failed child.
    PartialFailed,
    /// User/system cancelled the instance.
    Cancelled,
    /// Dispatcher could not find an eligible worker.
    NoEligibleWorker,
    /// Script governance failure was materialized.
    ScriptGovernanceFailure,
}

impl JobNotificationEvent {
    /// Stable event type used by policies/messages.
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::RetryScheduled => "job_instance.retry_scheduled",
            Self::RetryExhausted => "job_instance.retry_exhausted",
            Self::Succeeded => "job_instance.succeeded",
            Self::Failed => "job_instance.failed",
            Self::PartialFailed => "job_instance.partial_failed",
            Self::Cancelled => "job_instance.cancelled",
            Self::NoEligibleWorker => "job_instance.no_eligible_worker",
            Self::ScriptGovernanceFailure => "job_instance.script_governance_failure",
        }
    }

    const fn filter_status(&self) -> &'static str {
        match self {
            Self::RetryScheduled => "retry_scheduled",
            Self::RetryExhausted => "retry_exhausted",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::PartialFailed => "partial_failed",
            Self::Cancelled => "cancelled",
            Self::NoEligibleWorker => "no_eligible_worker",
            Self::ScriptGovernanceFailure => "script_governance_failure",
        }
    }

    const fn default_severity(&self) -> &'static str {
        match self {
            Self::Succeeded => "info",
            Self::Cancelled | Self::RetryScheduled => "warning",
            Self::Failed
            | Self::PartialFailed
            | Self::RetryExhausted
            | Self::NoEligibleWorker
            | Self::ScriptGovernanceFailure => "critical",
        }
    }

    /// Derive a terminal event from an instance status when applicable.
    #[must_use]
    pub const fn from_terminal_status(status: InstanceStatus) -> Option<Self> {
        match status {
            InstanceStatus::Succeeded => Some(Self::Succeeded),
            InstanceStatus::Failed => Some(Self::Failed),
            InstanceStatus::PartialFailed => Some(Self::PartialFailed),
            InstanceStatus::Cancelled => Some(Self::Cancelled),
            InstanceStatus::Pending | InstanceStatus::Dispatching | InstanceStatus::Running => None,
        }
    }
}

/// Result of materializing one job notification event.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEmitSummary {
    /// Number of policies that matched the event.
    pub matched_policies: u64,
    /// Number of normalized messages inserted.
    pub messages_created: u64,
    /// Number of delivery attempts inserted.
    pub delivery_attempts_created: u64,
}

/// Retry policy for generic notification delivery attempts.
#[derive(Debug, Clone, Copy)]
pub struct NotificationDeliveryPolicy {
    /// Maximum attempts before a pending attempt moves to dead-letter state.
    pub max_attempts: i32,
    /// Backoff in seconds before another retry may run.
    pub backoff_seconds: i64,
}

impl Default for NotificationDeliveryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_seconds: 300,
        }
    }
}

/// Result of one generic notification delivery scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryProcessSummary {
    /// Due attempts inspected.
    pub scanned: u64,
    /// Attempts delivered in this scan.
    pub delivered: u64,
    /// New retry attempt records appended.
    pub retried: u64,
    /// Attempts moved to dead-letter state.
    pub dead_lettered: u64,
    /// Attempts skipped because their message/channel context was unavailable.
    pub skipped: u64,
}

/// Repository-backed Notification Center materializer.
#[derive(Debug, Clone)]
pub struct NotificationCenter {
    channels: NotificationChannelRepository,
    policies: NotificationPolicyRepository,
    messages: NotificationMessageRepository,
    attempts: NotificationDeliveryAttemptRepository,
    templates: NotificationTemplateRepository,
    jobs: JobRepository,
}

impl NotificationCenter {
    /// Build a Notification Center from storage repositories.
    #[must_use]
    pub const fn new(
        channels: NotificationChannelRepository,
        policies: NotificationPolicyRepository,
        messages: NotificationMessageRepository,
        attempts: NotificationDeliveryAttemptRepository,
        templates: NotificationTemplateRepository,
        jobs: JobRepository,
    ) -> Self {
        Self {
            channels,
            policies,
            messages,
            attempts,
            templates,
            jobs,
        }
    }

    /// Materialize notification messages and delivery attempt ledger rows for a job instance event.
    ///
    /// # Errors
    ///
    /// Returns storage errors from repository operations.
    pub async fn emit_job_instance_event(
        &self,
        instance: &JobInstanceSummary,
        event: JobNotificationEvent,
        reason: Option<&str>,
    ) -> Result<NotificationEmitSummary, tikeo_storage::DbErr> {
        let Some(job) = self.jobs.get(&instance.job_id).await? else {
            return Ok(NotificationEmitSummary::default());
        };
        let policies = self
            .policies
            .list_policies(tikeo_storage::NotificationPolicyFilters {
                event_family: Some("job_instance".to_owned()),
                enabled: Some(true),
                ..Default::default()
            })
            .await?;
        let channels = self
            .channels
            .list_channels(NotificationChannelFilters::default())
            .await?;
        let mut summary = NotificationEmitSummary::default();
        for policy in policies.into_iter().filter(|policy| {
            policy_matches_job_event(policy, &job.id, &job.namespace, &job.app, &event)
        }) {
            summary.matched_policies += 1;
            let severity = if policy.severity.trim().is_empty() {
                event.default_severity().to_owned()
            } else {
                policy.severity.clone()
            };
            let mut subject = format!("Tikeo job {}: {}", job.name, event.filter_status());
            let mut body = reason.map_or_else(
                || {
                    format!(
                        "Job {} instance {} emitted {}",
                        job.name,
                        instance.id,
                        event.event_type()
                    )
                },
                |reason| {
                    format!(
                        "Job {} instance {} emitted {}: {reason}",
                        job.name,
                        instance.id,
                        event.event_type()
                    )
                },
            );
            let dedupe_key = format!("{}:{}:{}", policy.id, instance.id, event.event_type());
            let logs_url = format!("/instances/{}/logs", instance.id);
            let mut payload = serde_json::json!({
                "eventType": event.event_type(),
                "jobId": job.id,
                "jobName": job.name,
                "resourceType": "job",
                "resourceId": job.id,
                "namespace": job.namespace,
                "app": job.app,
                "instanceId": instance.id,
                "status": instance.status.to_string(),
                "triggerType": instance.trigger_type.to_string(),
                "executionMode": instance.execution_mode.to_string(),
                "startedAt": instance.created_at,
                "finishedAt": instance.updated_at,
                "workerId": instance.result.as_ref().map(|result| result.worker_id.clone()),
                "operatorType": "system",
                "operatorName": "tikeo",
                "logsUrl": logs_url,
                "reason": reason,
                "severity": severity,
                "policyId": policy.id,
                "dedupeKey": dedupe_key,
                "job": {
                    "id": job.id,
                    "name": job.name,
                    "namespace": job.namespace,
                    "app": job.app,
                    "executionMode": instance.execution_mode.to_string()
                },
                "instance": {
                    "id": instance.id,
                    "status": instance.status.to_string(),
                    "triggerType": instance.trigger_type.to_string(),
                    "executionMode": instance.execution_mode.to_string(),
                    "startedAt": instance.created_at,
                    "finishedAt": instance.updated_at,
                    "workerId": instance.result.as_ref().map(|result| result.worker_id.clone())
                },
                "operator": {"type": "system", "name": "tikeo"},
                "logs": {"url": format!("/instances/{}/logs", instance.id), "excerpt": serde_json::Value::Null}
            });
            if let Some(template) = load_policy_template(&self.templates, &policy).await? {
                apply_message_template(
                    &mut subject,
                    &mut body,
                    &mut payload,
                    &template,
                    &policy.id,
                    &dedupe_key,
                );
            }
            let payload_json = payload.to_string();
            let (message, created_message) = if let Some(message) = self
                .messages
                .latest_message_by_dedupe_key(&dedupe_key)
                .await?
                .filter(|message| !dedupe_window_elapsed(message, policy.dedupe_seconds))
            {
                (message, false)
            } else {
                let message = self
                    .messages
                    .create_message(CreateNotificationMessage {
                        source_type: "job_instance".to_owned(),
                        source_id: instance.id.clone(),
                        policy_id: policy.id.clone(),
                        event_type: event.event_type().to_owned(),
                        resource_type: "job".to_owned(),
                        resource_id: job.id.clone(),
                        severity,
                        subject,
                        body,
                        payload_json,
                        dedupe_key,
                        trace_id: None,
                        status: "pending".to_owned(),
                    })
                    .await?;
                summary.messages_created += 1;
                (message, true)
            };
            if !created_message {
                continue;
            }
            for channel_id in extract_channel_refs(&policy.channel_refs_json) {
                if let Some(channel) = channels.iter().find(|channel| channel.id == channel_id) {
                    if !channel.enabled {
                        continue;
                    }
                    let _attempt = self
                        .attempts
                        .record_attempt(RecordNotificationDeliveryAttempt {
                            message_id: message.id.clone(),
                            policy_id: policy.id.clone(),
                            channel_id: channel.id.clone(),
                            provider: channel.provider.clone(),
                            target_redacted: channel.target_redacted.clone(),
                            attempt: 0,
                            delivered: false,
                            status_code: None,
                            error: None,
                            retry_state: "retry_pending".to_owned(),
                            next_retry_at: None,
                        })
                        .await?;
                    summary.delivery_attempts_created += 1;
                }
            }
        }
        Ok(summary)
    }
}

/// Render a reusable notification template body against a sample payload without provider delivery.
#[must_use]
pub fn render_notification_template_preview(
    template: &serde_json::Value,
    sample: &serde_json::Value,
) -> serde_json::Value {
    render_notification_template_preview_with_overlay(template, sample, None, None)
}

/// Validate that a reusable notification template only uses supported inert replacement tokens.
///
/// # Errors
///
/// Returns a string describing unsupported or malformed template delimiters.
pub fn validate_notification_template_tokens(template: &serde_json::Value) -> Result<(), String> {
    validate_template_tokens(template)
}

fn render_notification_template_preview_with_overlay(
    template: &serde_json::Value,
    sample: &serde_json::Value,
    subject_overlay: Option<&str>,
    body_overlay: Option<&str>,
) -> serde_json::Value {
    let message = sample_notification_message(sample, subject_overlay, body_overlay);
    let mut rendered = template.clone();
    render_template_value(&mut rendered, &message);
    rendered
}

fn sample_notification_message(
    sample: &serde_json::Value,
    subject_overlay: Option<&str>,
    body_overlay: Option<&str>,
) -> NotificationMessageSummary {
    let payload = if sample.is_object() {
        sample.clone()
    } else {
        serde_json::json!({})
    };
    let string = |keys: &[&str], default: &str| {
        keys.iter()
            .find_map(|key| payload.get(*key).and_then(serde_json::Value::as_str))
            .unwrap_or(default)
            .to_owned()
    };
    NotificationMessageSummary {
        id: string(
            &["messageId", "message_id", "id"],
            "notification-message-preview",
        ),
        source_type: string(&["sourceType", "source_type"], "preview"),
        source_id: string(&["sourceId", "source_id"], "preview-source"),
        policy_id: string(&["policyId", "policy_id"], "notification-policy-preview"),
        event_type: string(&["eventType", "event_type"], "job_instance.failed"),
        resource_type: string(&["resourceType", "resource_type"], "job"),
        resource_id: string(&["resourceId", "resource_id"], "job-preview"),
        severity: string(&["severity"], "critical"),
        subject: subject_overlay.map_or_else(
            || string(&["subject", "title"], "Preview notification"),
            ToOwned::to_owned,
        ),
        body: body_overlay.map_or_else(
            || {
                string(
                    &["body", "message", "content"],
                    "This is a template preview.",
                )
            },
            ToOwned::to_owned,
        ),
        payload_json: payload.to_string(),
        dedupe_key: string(&["dedupeKey", "dedupe_key"], "preview-dedupe-key"),
        trace_id: payload
            .get("traceId")
            .or_else(|| payload.get("trace_id"))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        status: string(&["status"], "pending"),
        created_at: string(
            &["triggeredAt", "createdAt", "created_at"],
            "2026-06-11T00:00:00Z",
        ),
        updated_at: string(&["updatedAt", "updated_at"], "2026-06-11T00:00:00Z"),
    }
}

/// Best-effort wrapper for runtime paths that must not fail the job lifecycle because a notification ledger write failed.
pub async fn emit_job_instance_event_best_effort(
    center: &NotificationCenter,
    instance: &JobInstanceSummary,
    event: JobNotificationEvent,
    reason: Option<&str>,
) {
    if let Err(error) = center
        .emit_job_instance_event(instance, event, reason)
        .await
    {
        warn!(%error, instance_id = %instance.id, "failed to materialize notification event");
    }
}

/// Best-effort wrapper for alerting paths that must preserve legacy alert behavior even if
/// Notification Center ledger materialization fails.
pub async fn emit_alert_event_best_effort(
    center: &NotificationCenter,
    event: &tikeo_storage::AlertEventSummary,
) {
    if let Err(error) = center.emit_alert_event(event).await {
        warn!(%error, alert_event_id = %event.id, "failed to materialize alert notification event");
    }
}

async fn load_policy_template(
    templates: &NotificationTemplateRepository,
    policy: &NotificationPolicySummary,
) -> Result<Option<NotificationTemplateSummary>, tikeo_storage::DbErr> {
    let Some(template_ref) = policy
        .template_ref
        .as_deref()
        .filter(|item| !item.trim().is_empty())
    else {
        return Ok(None);
    };
    templates
        .get_template(template_ref)
        .await
        .map(|template| template.filter(|item| item.enabled))
}

fn apply_message_template(
    subject: &mut String,
    body: &mut String,
    payload: &mut serde_json::Value,
    template: &NotificationTemplateSummary,
    policy_id: &str,
    dedupe_key: &str,
) {
    let body_json = serde_json::from_str::<serde_json::Value>(&template.body_json)
        .unwrap_or_else(|_| serde_json::json!({}));
    let resource_type = payload
        .get("resourceType")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String("job".to_owned()));
    let resource_id = payload
        .get("resourceId")
        .or_else(|| payload.get("jobId"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let mut sample = payload.clone();
    if !sample.is_object() {
        sample = serde_json::json!({});
    }
    if let Some(object) = sample.as_object_mut() {
        for (key, value) in [
            (
                "subject",
                serde_json::Value::String(subject.as_str().to_owned()),
            ),
            ("body", serde_json::Value::String(body.as_str().to_owned())),
            (
                "eventType",
                payload
                    .get("eventType")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            ),
            ("resourceType", resource_type),
            ("resourceId", resource_id),
            (
                "severity",
                payload
                    .get("severity")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            ),
            ("messageId", serde_json::Value::String("pending".to_owned())),
            ("policyId", serde_json::Value::String(policy_id.to_owned())),
            (
                "dedupeKey",
                serde_json::Value::String(dedupe_key.to_owned()),
            ),
            (
                "triggeredAt",
                payload
                    .get("createdAt")
                    .or_else(|| payload.get("startedAt"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            ),
            (
                "templateRef",
                serde_json::Value::String(template.id.clone()),
            ),
            (
                "templateKey",
                serde_json::Value::String(template.template_key.clone()),
            ),
        ] {
            object.insert(key.to_owned(), value);
        }
    }

    let preview = render_notification_template_preview_with_overlay(
        &body_json,
        &sample,
        Some(subject),
        Some(body),
    );
    if let Some(rendered_subject) = template_string_field(&preview, &["subject", "title"]) {
        *subject = rendered_subject;
    }
    if let Some(rendered_body) = template_string_field(&preview, &["body", "text", "content"]) {
        *body = rendered_body;
    }

    let mut rendered = render_notification_template_preview_with_overlay(
        &body_json,
        &sample,
        Some(subject),
        Some(body),
    );
    if let Some(object) = rendered.as_object_mut() {
        object
            .entry("messageType")
            .or_insert_with(|| serde_json::Value::String(template.message_type.clone()));
    }
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "templateRef".to_owned(),
            serde_json::Value::String(template.id.clone()),
        );
        object.insert(
            "templateKey".to_owned(),
            serde_json::Value::String(template.template_key.clone()),
        );
        object.insert("template".to_owned(), rendered);
    }
}

fn template_string_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn policy_matches_job_event(
    policy: &NotificationPolicySummary,
    job_id: &str,
    namespace: &str,
    app: &str,
    event: &JobNotificationEvent,
) -> bool {
    match policy.owner_type.as_str() {
        "job" if policy.owner_id.as_deref() != Some(job_id) => return false,
        "app" if !app_owner_matches(policy.owner_id.as_deref(), namespace, app) => return false,
        "namespace" if policy.owner_id.as_deref() != Some(namespace) => return false,
        "global" | "job" | "app" | "namespace" => {}
        _ => return false,
    }
    filter_matches(
        &policy.event_filter_json,
        event.filter_status(),
        event.event_type(),
    )
}

pub(crate) use delivery::{
    deliver_notification_channel_once, process_due_notification_delivery_attempts,
    run_delivery_loop,
};

fn app_owner_matches(owner_id: Option<&str>, namespace: &str, app: &str) -> bool {
    owner_id.is_some_and(|owner_id| owner_id == app || owner_id == format!("{namespace}/{app}"))
}

fn filter_matches(raw: &str, status: &str, event_type: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    let status_match = value
        .get("statuses")
        .and_then(serde_json::Value::as_array)
        .is_none_or(|items| items.iter().any(|item| item.as_str() == Some(status)));
    let event_match = value
        .get("eventTypes")
        .or_else(|| value.get("event_types"))
        .and_then(serde_json::Value::as_array)
        .is_none_or(|items| items.iter().any(|item| item.as_str() == Some(event_type)));
    status_match && event_match
}

fn extract_channel_refs(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_str().map(ToOwned::to_owned).or_else(|| {
                    item.get("channelId")
                        .or_else(|| item.get("channel_id"))
                        .or_else(|| item.get("id"))
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                })
            })
            .collect(),
        serde_json::Value::String(item) => vec![item],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
