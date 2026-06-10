//! Notification Center message materialization and delivery ledger helpers.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tikeo_core::InstanceStatus;
use tikeo_storage::{
    CreateNotificationMessage, JobInstanceSummary, JobRepository,
    NotificationChannelDeliveryConfig, NotificationChannelFilters, NotificationChannelRepository,
    NotificationDeliveryAttemptRepository, NotificationDeliveryAttemptSummary,
    NotificationMessageRepository, NotificationMessageSummary, NotificationPolicyRepository,
    NotificationPolicySummary, RecordNotificationDeliveryAttempt,
};
use tokio::time as tokio_time;
use tracing::{info, warn};

use crate::{
    alert::{
        self, AlertDeliveryPolicy, AlertDispatcher, AlertPayload, NotificationChannel, Severity,
    },
    cluster::SharedClusterCoordinator,
};

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
        jobs: JobRepository,
    ) -> Self {
        Self {
            channels,
            policies,
            messages,
            attempts,
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
            let subject = format!("Tikeo job {}: {}", job.name, event.filter_status());
            let body = reason.map_or_else(
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
            let payload_json = serde_json::json!({
                "eventType": event.event_type(),
                "jobId": job.id,
                "jobName": job.name,
                "namespace": job.namespace,
                "app": job.app,
                "instanceId": instance.id,
                "status": instance.status.to_string(),
                "reason": reason,
            })
            .to_string();
            let dedupe_key = format!("{}:{}:{}", policy.id, instance.id, event.event_type());
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

/// Process due generic notification delivery attempts using production-safe delivery policy.
///
/// # Errors
/// Returns storage errors from loading/writing delivery context and attempts.
pub async fn process_due_notification_delivery_attempts(
    channels: &NotificationChannelRepository,
    messages: &NotificationMessageRepository,
    attempts: &NotificationDeliveryAttemptRepository,
    limit: u64,
    policy: NotificationDeliveryPolicy,
) -> Result<NotificationDeliveryProcessSummary, tikeo_storage::DbErr> {
    process_due_notification_delivery_attempts_with_delivery_policy(
        channels,
        messages,
        attempts,
        limit,
        policy,
        AlertDeliveryPolicy::production(),
    )
    .await
}

/// Process due generic notification delivery attempts with explicit delivery safety policy.
///
/// # Errors
/// Returns storage errors from loading/writing delivery context and attempts.
pub async fn process_due_notification_delivery_attempts_with_delivery_policy(
    channels: &NotificationChannelRepository,
    messages: &NotificationMessageRepository,
    attempts: &NotificationDeliveryAttemptRepository,
    limit: u64,
    policy: NotificationDeliveryPolicy,
    delivery_policy: AlertDeliveryPolicy,
) -> Result<NotificationDeliveryProcessSummary, tikeo_storage::DbErr> {
    let due = attempts.list_due_attempts(limit).await?;
    let mut summary = NotificationDeliveryProcessSummary::default();
    let client = NotificationProviderClient::new(delivery_policy);
    for attempt in due {
        summary.scanned = summary.scanned.saturating_add(1);
        process_due_attempt(
            channels,
            messages,
            attempts,
            &client,
            &attempt,
            policy,
            &mut summary,
        )
        .await?;
    }
    Ok(summary)
}

/// Process due generic notification attempts only when this node owns scheduling.
///
/// # Errors
/// Returns storage errors from delivery processing.
pub async fn retry_once_if_owner(
    channels: &NotificationChannelRepository,
    messages: &NotificationMessageRepository,
    attempts: &NotificationDeliveryAttemptRepository,
    cluster: &SharedClusterCoordinator,
    limit: u64,
    policy: NotificationDeliveryPolicy,
) -> Result<NotificationDeliveryProcessSummary, tikeo_storage::DbErr> {
    let status = cluster.status().await;
    if !status.can_schedule {
        return Ok(NotificationDeliveryProcessSummary::default());
    }
    process_due_notification_delivery_attempts(channels, messages, attempts, limit, policy).await
}

/// Run the generic notification delivery retry worker forever.
pub async fn run_delivery_loop(
    channels: NotificationChannelRepository,
    messages: NotificationMessageRepository,
    attempts: NotificationDeliveryAttemptRepository,
    cluster: SharedClusterCoordinator,
    interval: Duration,
    limit: u64,
    policy: NotificationDeliveryPolicy,
) {
    let mut ticker = tokio_time::interval(interval.max(Duration::from_secs(1)));
    info!(
        interval_seconds = interval.as_secs(),
        limit,
        max_attempts = policy.max_attempts,
        "notification delivery worker started"
    );
    loop {
        ticker.tick().await;
        match retry_once_if_owner(&channels, &messages, &attempts, &cluster, limit, policy).await {
            Ok(summary) if summary.scanned > 0 => {
                info!(
                    scanned = summary.scanned,
                    delivered = summary.delivered,
                    retried = summary.retried,
                    dead_lettered = summary.dead_lettered,
                    skipped = summary.skipped,
                    "notification delivery iteration completed"
                );
            }
            Ok(_) => {}
            Err(error) => warn!(%error, "notification delivery iteration failed"),
        }
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

async fn process_due_attempt(
    channels: &NotificationChannelRepository,
    messages: &NotificationMessageRepository,
    attempts: &NotificationDeliveryAttemptRepository,
    client: &NotificationProviderClient,
    attempt: &NotificationDeliveryAttemptSummary,
    policy: NotificationDeliveryPolicy,
    summary: &mut NotificationDeliveryProcessSummary,
) -> Result<(), tikeo_storage::DbErr> {
    if attempt.attempt >= policy.max_attempts {
        dead_letter_attempt(
            attempts,
            messages,
            attempt,
            "max retry attempts exhausted",
            summary,
        )
        .await?;
        return Ok(());
    }
    let Some(message) = messages.get_message(&attempt.message_id).await? else {
        dead_letter_attempt(
            attempts,
            messages,
            attempt,
            "source message not found",
            summary,
        )
        .await?;
        return Ok(());
    };
    let Some(channel) = channels
        .get_channel_delivery_config(&attempt.channel_id)
        .await?
    else {
        dead_letter_attempt(
            attempts,
            messages,
            attempt,
            "notification channel not found",
            summary,
        )
        .await?;
        return Ok(());
    };
    if !channel.enabled {
        dead_letter_attempt(
            attempts,
            messages,
            attempt,
            "notification channel is disabled",
            summary,
        )
        .await?;
        return Ok(());
    }
    let result = client.deliver(&channel, &message).await;
    record_delivery_result(messages, attempts, attempt, result, policy, summary).await
}

async fn record_delivery_result(
    messages: &NotificationMessageRepository,
    attempts: &NotificationDeliveryAttemptRepository,
    attempt: &NotificationDeliveryAttemptSummary,
    result: NotificationProviderDeliveryResult,
    policy: NotificationDeliveryPolicy,
    summary: &mut NotificationDeliveryProcessSummary,
) -> Result<(), tikeo_storage::DbErr> {
    let delivered = result.delivered;
    let next_attempt = attempt.attempt.saturating_add(1);
    let exhausted = !delivered && next_attempt >= policy.max_attempts;
    attempts
        .record_attempt(RecordNotificationDeliveryAttempt {
            message_id: attempt.message_id.clone(),
            policy_id: attempt.policy_id.clone(),
            channel_id: attempt.channel_id.clone(),
            provider: result.provider,
            target_redacted: result.target_redacted,
            attempt: next_attempt,
            delivered,
            status_code: result.status_code.map(i32::from),
            error: result.error,
            retry_state: retry_state_for(delivered, exhausted),
            next_retry_at: next_retry_at(delivered, exhausted, policy.backoff_seconds),
        })
        .await?;
    attempts
        .mark_attempt_retry_state(&attempt.id, "retry_consumed", None, None)
        .await?;
    if delivered {
        messages
            .update_message_status(&attempt.message_id, "delivered")
            .await?;
        summary.delivered = summary.delivered.saturating_add(1);
    } else if exhausted {
        messages
            .update_message_status(&attempt.message_id, "dead_letter")
            .await?;
        summary.dead_lettered = summary.dead_lettered.saturating_add(1);
    } else {
        messages
            .update_message_status(&attempt.message_id, "pending")
            .await?;
        summary.retried = summary.retried.saturating_add(1);
    }
    Ok(())
}

async fn dead_letter_attempt(
    attempts: &NotificationDeliveryAttemptRepository,
    messages: &NotificationMessageRepository,
    attempt: &NotificationDeliveryAttemptSummary,
    reason: &str,
    summary: &mut NotificationDeliveryProcessSummary,
) -> Result<(), tikeo_storage::DbErr> {
    attempts
        .mark_attempt_retry_state(&attempt.id, "dead_letter", Some(reason), None)
        .await?;
    messages
        .update_message_status(&attempt.message_id, "dead_letter")
        .await?;
    summary.dead_lettered = summary.dead_lettered.saturating_add(1);
    Ok(())
}

fn retry_state_for(delivered: bool, exhausted: bool) -> String {
    if delivered {
        "delivered".to_owned()
    } else if exhausted {
        "dead_letter".to_owned()
    } else {
        "retry_pending".to_owned()
    }
}

fn next_retry_at(delivered: bool, exhausted: bool, backoff_seconds: i64) -> Option<String> {
    if delivered || exhausted {
        None
    } else {
        Some(rfc3339_after_seconds(backoff_seconds))
    }
}

fn rfc3339_after_seconds(seconds: i64) -> String {
    let seconds = seconds.clamp(1, 86_400);
    (time::OffsetDateTime::now_utc() + time::Duration::seconds(seconds))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn dedupe_window_elapsed(message: &NotificationMessageSummary, dedupe_seconds: i64) -> bool {
    if dedupe_seconds <= 0 {
        return true;
    }
    let Ok(created_at) = time::OffsetDateTime::parse(
        &message.created_at,
        &time::format_description::well_known::Rfc3339,
    ) else {
        return true;
    };
    let elapsed = time::OffsetDateTime::now_utc() - created_at;
    elapsed.whole_seconds() >= dedupe_seconds
}

#[derive(Debug, Clone)]
struct NotificationProviderDeliveryResult {
    provider: String,
    target_redacted: String,
    delivered: bool,
    status_code: Option<u16>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct NotificationProviderClient {
    http: reqwest::Client,
    policy: AlertDeliveryPolicy,
}

impl NotificationProviderClient {
    fn new(policy: AlertDeliveryPolicy) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|error| {
                warn!(%error, "failed to build notification HTTP client; using default client");
                reqwest::Client::new()
            });
        Self { http, policy }
    }

    async fn deliver(
        &self,
        channel: &NotificationChannelDeliveryConfig,
        message: &NotificationMessageSummary,
    ) -> NotificationProviderDeliveryResult {
        let config = parse_json_object(&channel.config_json);
        let secrets = parse_json_object(&channel.secret_refs_json);
        let Some(notification_channel) = notification_channel_from_delivery_config(channel) else {
            return NotificationProviderDeliveryResult {
                provider: channel.provider.clone(),
                target_redacted: channel.target_redacted.clone(),
                delivered: false,
                status_code: None,
                error: Some("notification channel configuration is incomplete".to_owned()),
            };
        };
        match notification_channel {
            NotificationChannel::Webhook { url } => {
                let headers = webhook_headers(&config, &secrets);
                self.post_json(
                    "webhook",
                    channel,
                    &url,
                    &notification_payload(message),
                    &headers,
                )
                .await
            }
            NotificationChannel::Slack { url } => {
                let body = serde_json::json!({ "text": notification_text(message) });
                self.post_json("slack", channel, &url, &body, &[]).await
            }
            NotificationChannel::DingTalk { url } => {
                let body = serde_json::json!({
                    "msgtype": "text",
                    "text": { "content": notification_text(message) },
                });
                self.post_json("dingtalk", channel, &url, &body, &[]).await
            }
            NotificationChannel::Feishu { url } => {
                let body = serde_json::json!({
                    "msg_type": "text",
                    "content": { "text": notification_text(message) },
                });
                self.post_json("feishu", channel, &url, &body, &[]).await
            }
            NotificationChannel::WechatWork { url } => {
                let body = serde_json::json!({
                    "msgtype": "text",
                    "text": { "content": notification_text(message) },
                });
                self.post_json("wechat_work", channel, &url, &body, &[])
                    .await
            }
            NotificationChannel::PagerDuty { url, routing_key } => {
                let target = url
                    .as_deref()
                    .unwrap_or("https://events.pagerduty.com/v2/enqueue");
                let body = serde_json::json!({
                    "routing_key": routing_key,
                    "event_action": "trigger",
                    "dedup_key": message.dedupe_key,
                    "payload": {
                        "summary": message.subject,
                        "source": "tikeo",
                        "severity": pagerduty_severity(&message.severity),
                        "component": message.resource_type,
                        "custom_details": notification_payload(message),
                    },
                });
                self.post_json("pagerduty", channel, target, &body, &[])
                    .await
            }
            NotificationChannel::PluginWebhook {
                channel_type,
                url,
                template: _,
            } => {
                let headers = webhook_headers(&config, &secrets);
                self.post_json(
                    &channel_type,
                    channel,
                    &url,
                    &notification_payload(message),
                    &headers,
                )
                .await
            }
            email @ NotificationChannel::Email { .. } => {
                let policy = effective_delivery_policy(self.policy, channel);
                let mut results = AlertDispatcher::new_with_policy(Vec::new(), policy)
                    .deliver_payload(&[email], &alert_payload_from_message(message))
                    .await;
                results.pop().map_or_else(
                    || NotificationProviderDeliveryResult {
                        provider: "email".to_owned(),
                        target_redacted: channel.target_redacted.clone(),
                        delivered: false,
                        status_code: None,
                        error: Some("email provider returned no delivery result".to_owned()),
                    },
                    |result| NotificationProviderDeliveryResult {
                        provider: result.provider,
                        target_redacted: result.target,
                        delivered: result.delivered,
                        status_code: result.status,
                        error: result.error,
                    },
                )
            }
        }
    }

    async fn post_json(
        &self,
        provider: &str,
        channel: &NotificationChannelDeliveryConfig,
        url: &str,
        body: &serde_json::Value,
        headers: &[(String, String)],
    ) -> NotificationProviderDeliveryResult {
        let target_redacted = alert::redact_url(url);
        if let Err(error) =
            alert::validate_webhook_url(url, effective_delivery_policy(self.policy, channel))
        {
            warn!(provider, target = %target_redacted, %error, "notification provider rejected by safety policy");
            return NotificationProviderDeliveryResult {
                provider: provider.to_owned(),
                target_redacted,
                delivered: false,
                status_code: None,
                error: Some(error.to_owned()),
            };
        }
        let mut request = self.http.post(url).json(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let delivered = status.is_success();
                NotificationProviderDeliveryResult {
                    provider: provider.to_owned(),
                    target_redacted,
                    delivered,
                    status_code: Some(status.as_u16()),
                    error: if delivered {
                        None
                    } else {
                        Some(format!("{provider} returned HTTP {status}"))
                    },
                }
            }
            Err(error) => {
                let _ = error;
                warn!(provider, target = %target_redacted, "notification provider delivery failed");
                NotificationProviderDeliveryResult {
                    provider: provider.to_owned(),
                    target_redacted,
                    delivered: false,
                    status_code: None,
                    error: Some(format!("{provider} request failed")),
                }
            }
        }
    }
}

fn notification_channel_from_delivery_config(
    channel: &NotificationChannelDeliveryConfig,
) -> Option<NotificationChannel> {
    let config = parse_json_object(&channel.config_json);
    let secrets = parse_json_object(&channel.secret_refs_json);
    match channel.provider.as_str() {
        "webhook" => {
            resolved_url(&config, &secrets).map(|url| NotificationChannel::Webhook { url })
        }
        "slack" => resolved_url(&config, &secrets).map(|url| NotificationChannel::Slack { url }),
        "dingtalk" => {
            resolved_url(&config, &secrets).map(|url| NotificationChannel::DingTalk { url })
        }
        "feishu" => resolved_url(&config, &secrets).map(|url| NotificationChannel::Feishu { url }),
        "wechat_work" | "wecom" => {
            resolved_url(&config, &secrets).map(|url| NotificationChannel::WechatWork { url })
        }
        "pagerduty" | "pager_duty" => {
            let url = optional_string(&config, &["url", "webhookUrl", "webhook_url"])
                .or_else(|| optional_secret(&secrets, &["url", "webhookUrl", "webhook_url"]));
            let routing_key = optional_string(
                &config,
                &[
                    "routingKey",
                    "routing_key",
                    "integrationKey",
                    "integration_key",
                ],
            )
            .or_else(|| {
                optional_secret(
                    &secrets,
                    &[
                        "routingKey",
                        "routing_key",
                        "integrationKey",
                        "integration_key",
                    ],
                )
            })?;
            Some(NotificationChannel::PagerDuty { url, routing_key })
        }
        "email" => Some(NotificationChannel::Email {
            recipients: recipients_from_config(&config),
            smtp_url: optional_string(&config, &["smtpUrl", "smtp_url", "url"])
                .or_else(|| optional_secret(&secrets, &["smtpUrl", "smtp_url", "url"])),
            smtp_url_secret_ref: optional_string(
                &config,
                &["smtpUrlSecretRef", "smtp_url_secret_ref"],
            )
            .or_else(|| secret_ref_string(&secrets, &["smtpUrlSecretRef", "smtp_url_secret_ref"])),
            from: optional_string(&config, &["from"]),
            username: optional_string(&config, &["username"]),
            password_secret_ref: optional_string(
                &config,
                &["passwordSecretRef", "password_secret_ref"],
            )
            .or_else(|| {
                secret_ref_string(
                    &secrets,
                    &["password", "passwordSecretRef", "password_secret_ref"],
                )
            }),
        }),
        other => resolved_url(&config, &secrets).map(|url| NotificationChannel::PluginWebhook {
            channel_type: other.to_owned(),
            url,
            template: config
                .get("template")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        }),
    }
}

fn alert_payload_from_message(message: &NotificationMessageSummary) -> AlertPayload {
    AlertPayload {
        rule_name: message.subject.clone(),
        severity: severity_from_str(&message.severity),
        message: message.body.clone(),
        resource_type: message.resource_type.clone(),
        resource_id: message.resource_id.clone(),
        triggered_at: message.created_at.clone(),
    }
}

fn severity_from_str(value: &str) -> Severity {
    match value {
        "critical" => Severity::Critical,
        "warning" => Severity::Warning,
        _ => Severity::Info,
    }
}

fn parse_json_object(raw: &str) -> serde_json::Map<String, serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn resolved_url(
    config: &serde_json::Map<String, serde_json::Value>,
    secrets: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    optional_string(config, &["url", "webhookUrl", "webhook_url"])
        .or_else(|| optional_secret(secrets, &["url", "webhookUrl", "webhook_url"]))
}

fn optional_string(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| map.get(*key).and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_secret(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    secret_ref_string(map, keys).and_then(|reference| alert::resolve_secret_ref(Some(&reference)))
}

fn secret_ref_string(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter().find_map(|key| {
        map.get(*key).and_then(|value| match value {
            serde_json::Value::String(item) => Some(item.clone()),
            serde_json::Value::Object(object) => object
                .get("ref")
                .or_else(|| object.get("secretRef"))
                .or_else(|| object.get("secret_ref"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            _ => None,
        })
    })
}

fn recipients_from_config(map: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    map.get("recipients")
        .or_else(|| map.get("to"))
        .map(|value| match value {
            serde_json::Value::String(item) => vec![item.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default()
}

fn effective_delivery_policy(
    base: AlertDeliveryPolicy,
    channel: &NotificationChannelDeliveryConfig,
) -> AlertDeliveryPolicy {
    let allow_insecure_loopback = serde_json::from_str::<serde_json::Value>(
        channel.safety_policy_json.as_deref().unwrap_or("{}"),
    )
    .ok()
    .and_then(|value| {
        value
            .get("allowInsecureLoopback")
            .or_else(|| value.get("allow_insecure_loopback"))
            .and_then(serde_json::Value::as_bool)
    })
    .unwrap_or(base.allow_insecure_loopback);
    AlertDeliveryPolicy {
        allow_insecure_loopback,
    }
}

fn notification_payload(message: &NotificationMessageSummary) -> serde_json::Value {
    let mut payload = serde_json::from_str::<serde_json::Value>(&message.payload_json)
        .unwrap_or_else(|_| serde_json::json!({}));
    if let serde_json::Value::Object(map) = &mut payload {
        map.insert(
            "eventType".to_owned(),
            serde_json::Value::String(message.event_type.clone()),
        );
        map.insert(
            "messageId".to_owned(),
            serde_json::Value::String(message.id.clone()),
        );
        map.insert(
            "policyId".to_owned(),
            serde_json::Value::String(message.policy_id.clone()),
        );
        map.insert(
            "resourceType".to_owned(),
            serde_json::Value::String(message.resource_type.clone()),
        );
        map.insert(
            "resourceId".to_owned(),
            serde_json::Value::String(message.resource_id.clone()),
        );
        map.insert(
            "severity".to_owned(),
            serde_json::Value::String(message.severity.clone()),
        );
        map.insert(
            "subject".to_owned(),
            serde_json::Value::String(message.subject.clone()),
        );
        map.insert(
            "body".to_owned(),
            serde_json::Value::String(message.body.clone()),
        );
    }
    payload
}

fn notification_text(message: &NotificationMessageSummary) -> String {
    format!(
        "[tikeo/{}] {}: {} ({}/{})",
        message.severity,
        message.event_type,
        message.subject,
        message.resource_type,
        message.resource_id
    )
}

fn pagerduty_severity(severity: &str) -> &'static str {
    match severity {
        "critical" => "critical",
        "warning" => "warning",
        _ => "info",
    }
}

fn webhook_headers(
    config: &serde_json::Map<String, serde_json::Value>,
    secrets: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = config
        .get("headers")
        .and_then(serde_json::Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        && let Some(value) = optional_secret(secrets, &["authorization", "Authorization"])
    {
        headers.push(("Authorization".to_owned(), value));
    }
    if let Some(secret_headers) = secrets
        .get("headers")
        .and_then(serde_json::Value::as_object)
    {
        for (name, value) in secret_headers {
            if headers
                .iter()
                .any(|(existing, _)| existing.eq_ignore_ascii_case(name))
            {
                continue;
            }
            let resolved = match value {
                serde_json::Value::String(reference) => alert::resolve_secret_ref(Some(reference)),
                serde_json::Value::Object(object) => object
                    .get("ref")
                    .or_else(|| object.get("secretRef"))
                    .or_else(|| object.get("secret_ref"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|reference| alert::resolve_secret_ref(Some(reference))),
                _ => None,
            };
            if let Some(resolved) = resolved {
                headers.push((name.clone(), resolved));
            }
        }
    }
    headers
}

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
