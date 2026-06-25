//! Notification Center provider delivery and retry processing.

use std::{sync::Arc, time::Duration};

use tikeo_storage::{
    NotificationChannelDeliveryConfig, NotificationChannelRepository,
    NotificationDeliveryAttemptRepository, NotificationDeliveryAttemptSummary,
    NotificationMessageRepository, NotificationMessageSummary, RecordNotificationDeliveryAttempt,
};
use tokio::{sync::Notify, time as tokio_time};
use tracing::{info, warn};

use super::provider_templates::{
    dingtalk_payload, email_alert_payload_from_message, feishu_payload,
    missing_required_template_reason, pagerduty_payload, slack_payload, webhook_payload,
    wechat_work_payload,
};
use super::signing::{add_feishu_signature, signed_dingtalk_url};
use super::{NotificationDeliveryPolicy, NotificationDeliveryProcessSummary};
use crate::{
    alert::{
        self, AlertDeliveryPolicy, AlertDispatcher, AlertPayload, NotificationChannel, Severity,
    },
    cluster::SharedClusterCoordinator,
};

/// In-process signal used to wake the Notification Center delivery worker as soon as
/// new due attempts are materialized. Periodic scans still remain the cross-process
/// and crash-recovery fallback.
#[derive(Debug, Clone, Default)]
pub struct NotificationDeliveryTrigger {
    notify: Arc<Notify>,
}

impl NotificationDeliveryTrigger {
    /// Create a new shared delivery trigger.
    #[must_use]
    /// New.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wake one delivery worker iteration. Multiple rapid kicks intentionally coalesce.
    pub fn kick(&self) {
        self.notify.notify_one();
    }

    async fn notified(&self) {
        self.notify.notified().await;
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

/// Runtime dependencies and policy for the generic notification delivery worker.
#[derive(Debug, Clone)]
pub struct NotificationDeliveryLoopContext {
    channels: NotificationChannelRepository,
    messages: NotificationMessageRepository,
    attempts: NotificationDeliveryAttemptRepository,
    cluster: SharedClusterCoordinator,
    interval: Duration,
    limit: u64,
    policy: NotificationDeliveryPolicy,
    trigger: Option<NotificationDeliveryTrigger>,
}

impl NotificationDeliveryLoopContext {
    /// Create a notification delivery loop context.
    #[must_use]
    /// New.
    pub fn new(
        repositories: NotificationDeliveryRepositories,
        cluster: SharedClusterCoordinator,
        interval: Duration,
        limit: u64,
        policy: NotificationDeliveryPolicy,
        trigger: Option<NotificationDeliveryTrigger>,
    ) -> Self {
        Self {
            channels: repositories.channels,
            messages: repositories.messages,
            attempts: repositories.attempts,
            cluster,
            interval,
            limit,
            policy,
            trigger,
        }
    }
}

/// Storage repositories used by the generic notification delivery worker.
#[derive(Debug, Clone)]
pub struct NotificationDeliveryRepositories {
    channels: NotificationChannelRepository,
    messages: NotificationMessageRepository,
    attempts: NotificationDeliveryAttemptRepository,
}

impl NotificationDeliveryRepositories {
    /// Create repository bundle for notification delivery.
    #[must_use]
    /// New.
    pub const fn new(
        channels: NotificationChannelRepository,
        messages: NotificationMessageRepository,
        attempts: NotificationDeliveryAttemptRepository,
    ) -> Self {
        Self {
            channels,
            messages,
            attempts,
        }
    }
}

/// Run the generic notification delivery worker forever.
pub async fn run_delivery_loop(context: NotificationDeliveryLoopContext) {
    let mut ticker = tokio_time::interval(context.interval.max(Duration::from_secs(1)));
    info!(
        interval_seconds = context.interval.as_secs(),
        limit = context.limit,
        max_attempts = context.policy.max_attempts,
        immediate_wakeup = context.trigger.is_some(),
        "notification delivery worker started"
    );
    loop {
        wait_for_delivery_iteration(&mut ticker, context.trigger.as_ref()).await;
        match retry_once_if_owner(
            &context.channels,
            &context.messages,
            &context.attempts,
            &context.cluster,
            context.limit,
            context.policy,
        )
        .await
        {
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

async fn wait_for_delivery_iteration(
    ticker: &mut tokio_time::Interval,
    trigger: Option<&NotificationDeliveryTrigger>,
) {
    if let Some(trigger) = trigger {
        tokio::select! {
            _ = ticker.tick() => {}
            () = trigger.notified() => {}
        }
    } else {
        ticker.tick().await;
    }
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

/// Dedupe window elapsed.
pub(super) fn dedupe_window_elapsed(
    message: &NotificationMessageSummary,
    dedupe_seconds: i64,
) -> bool {
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
pub struct NotificationProviderDeliveryResult {
    pub(crate) provider: String,
    /// Target redacted value.
    pub(crate) target_redacted: String,
    pub(crate) delivered: bool,
    /// Status code value.
    pub(crate) status_code: Option<u16>,
    pub(crate) error: Option<String>,
    /// Rendered payload value.
    pub(crate) rendered_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub(super) struct NotificationProviderClient {
    http: reqwest::Client,
    policy: AlertDeliveryPolicy,
}

impl NotificationProviderClient {
    /// New.
    pub(super) fn new(policy: AlertDeliveryPolicy) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|error| {
                warn!(%error, "failed to build notification HTTP client; using default client");
                reqwest::Client::new()
            });
        Self { http, policy }
    }

    /// Deliver.
    pub(super) async fn deliver(
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
                rendered_payload: None,
            };
        };
        if let Some(error) = missing_required_template_reason(&channel.provider, message, &config) {
            return NotificationProviderDeliveryResult {
                provider: channel.provider.clone(),
                target_redacted: channel.target_redacted.clone(),
                delivered: false,
                status_code: None,
                error: Some(error),
                rendered_payload: None,
            };
        }
        match notification_channel {
            NotificationChannel::Webhook { url } => {
                let headers = webhook_headers(&config, &secrets);
                self.post_json(
                    "webhook",
                    channel,
                    &url,
                    &webhook_payload(message, &config),
                    &headers,
                )
                .await
            }
            NotificationChannel::Slack { url } => {
                let body = slack_payload(message, &config);
                self.post_json("slack", channel, &url, &body, &[]).await
            }
            NotificationChannel::DingTalk { url } => {
                let url = if let Some(secret) = optional_signing_secret(&config, &secrets) {
                    signed_dingtalk_url(&url, &secret)
                } else {
                    url
                };
                let body = dingtalk_payload(message, &config);
                self.post_json("dingtalk", channel, &url, &body, &[]).await
            }
            NotificationChannel::Feishu { url } => {
                let mut body = feishu_payload(message, &config);
                if let Some(secret) = optional_signing_secret(&config, &secrets) {
                    add_feishu_signature(&mut body, &secret);
                }
                self.post_json("feishu", channel, &url, &body, &[]).await
            }
            NotificationChannel::WechatWork { url } => {
                let body = wechat_work_payload(message, &config);
                self.post_json("wechat_work", channel, &url, &body, &[])
                    .await
            }
            NotificationChannel::PagerDuty { url, routing_key } => {
                let target = url
                    .as_deref()
                    .unwrap_or("https://events.pagerduty.com/v2/enqueue");
                let body = pagerduty_payload(message, &routing_key, &config);
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
                    &webhook_payload(message, &config),
                    &headers,
                )
                .await
            }
            email @ NotificationChannel::Email { .. } => {
                let policy = effective_delivery_policy(self.policy, channel);
                let mut results = AlertDispatcher::new_with_policy(Vec::new(), policy)
                    .deliver_payload(
                        &[email],
                        &email_alert_payload_from_message(message, &config),
                    )
                    .await;
                results.pop().map_or_else(
                    || NotificationProviderDeliveryResult {
                        provider: "email".to_owned(),
                        target_redacted: channel.target_redacted.clone(),
                        delivered: false,
                        status_code: None,
                        error: Some("email provider returned no delivery result".to_owned()),
                        rendered_payload: None,
                    },
                    |result| NotificationProviderDeliveryResult {
                        provider: result.provider,
                        target_redacted: result.target,
                        delivered: result.delivered,
                        status_code: result.status,
                        error: result.error,
                        rendered_payload: Some(serde_json::json!({
                            "subject": email_alert_payload_from_message(message, &config).rule_name,
                            "body": email_alert_payload_from_message(message, &config).message,
                        })),
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
                rendered_payload: Some(body.clone()),
            };
        }
        let mut request = self.http.post(url).json(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let response_body = response.text().await.unwrap_or_default();
                let provider_error = provider_response_error(provider, &response_body);
                let delivered = status.is_success() && provider_error.is_none();
                NotificationProviderDeliveryResult {
                    provider: provider.to_owned(),
                    target_redacted,
                    delivered,
                    status_code: Some(status.as_u16()),
                    error: if delivered {
                        None
                    } else if let Some(error) = provider_error {
                        Some(error)
                    } else {
                        Some(format!("{provider} returned HTTP {status}"))
                    },
                    rendered_payload: Some(body.clone()),
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
                    rendered_payload: Some(body.clone()),
                }
            }
        }
    }
}

fn provider_response_error(provider: &str, response_body: &str) -> Option<String> {
    if response_body.trim().is_empty() {
        return None;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(response_body) else {
        return None;
    };
    match provider {
        "feishu" => {
            provider_numeric_code_error(provider, &value, &["code"], &[0], &["msg", "message"])
        }
        "dingtalk" | "wechat_work" => provider_numeric_code_error(
            provider,
            &value,
            &["errcode"],
            &[0],
            &["errmsg", "message"],
        ),
        "slack" => {
            if value.get("ok").and_then(serde_json::Value::as_bool) == Some(false) {
                Some(format!(
                    "slack returned error: {}",
                    value
                        .get("error")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                ))
            } else {
                None
            }
        }
        "pagerduty" => {
            if value.get("status").and_then(serde_json::Value::as_str) == Some("success") {
                None
            } else {
                value
                    .get("message")
                    .or_else(|| value.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .map(|message| format!("pagerduty returned error: {message}"))
            }
        }
        _ => None,
    }
}

fn provider_numeric_code_error(
    provider: &str,
    value: &serde_json::Value,
    code_keys: &[&str],
    success_codes: &[i64],
    message_keys: &[&str],
) -> Option<String> {
    let code = code_keys
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(|item| item.as_i64().or_else(|| item.as_str()?.parse::<i64>().ok()));
    if code.is_some_and(|item| success_codes.contains(&item)) {
        return None;
    }
    let message = message_keys
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
        .unwrap_or("provider returned an application-level error");
    code.map(|item| format!("{provider} returned code {item}: {message}"))
}

/// Deliver one notification message through one already-loaded channel using the same provider
/// adapters as the retry worker.
#[must_use]
/// Deliver notification channel once.
pub async fn deliver_notification_channel_once(
    channel: &NotificationChannelDeliveryConfig,
    message: &NotificationMessageSummary,
    delivery_policy: AlertDeliveryPolicy,
) -> NotificationProviderDeliveryResult {
    NotificationProviderClient::new(delivery_policy)
        .deliver(channel, message)
        .await
}

/// Notification channel from delivery config.
pub(super) fn notification_channel_from_delivery_config(
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
            smtp_url: optional_smtp_url(&config, &secrets)
                .or_else(|| smtp_url_from_config(&config)),
            smtp_url_secret_ref: optional_string(
                &config,
                &["smtpUrlSecretRef", "smtp_url_secret_ref"],
            )
            .or_else(|| secret_ref_string(&secrets, &["smtpUrlSecretRef", "smtp_url_secret_ref"])),
            from: optional_string(&config, &["from"]),
            username: smtp_auth_enabled(&config)
                .then(|| optional_string(&config, &["username"]))
                .flatten(),
            password: smtp_auth_enabled(&config)
                .then(|| {
                    optional_secret(
                        &secrets,
                        &["password", "passwordSecretRef", "password_secret_ref"],
                    )
                })
                .flatten(),
            password_secret_ref: optional_string(
                &config,
                &["passwordSecretRef", "password_secret_ref"],
            )
            .or_else(|| secret_ref_string(&secrets, &["passwordSecretRef", "password_secret_ref"]))
            .or_else(|| {
                secret_ref_string(&secrets, &["password"])
                    .filter(|reference| is_env_secret_ref(reference))
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

fn smtp_url_from_config(config: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let host = optional_string(config, &["host", "smtpHost", "smtp_host"])?;
    let port = optional_u16(config, &["port", "smtpPort", "smtp_port"]).unwrap_or_else(|| {
        if optional_bool(config, &["ssl"]).unwrap_or(false) {
            465
        } else if optional_bool(config, &["starttls", "startTls", "start_tls"]).unwrap_or(false) {
            587
        } else {
            25
        }
    });
    let scheme = if optional_bool(config, &["ssl"]).unwrap_or(false) {
        "smtps"
    } else if optional_bool(config, &["starttls", "startTls", "start_tls"]).unwrap_or(false) {
        "smtp+starttls"
    } else {
        "smtp"
    };
    Some(format!("{scheme}://{host}:{port}"))
}

fn optional_smtp_url(
    config: &serde_json::Map<String, serde_json::Value>,
    secrets: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    optional_string(config, &["smtpUrl", "smtp_url", "url"])
        .or_else(|| optional_secret(secrets, &["smtpUrl", "smtp_url", "url"]))
        .filter(|value| smtp_url_has_scheme(value))
}

fn smtp_url_has_scheme(value: &str) -> bool {
    value.contains("://")
}

fn smtp_auth_enabled(config: &serde_json::Map<String, serde_json::Value>) -> bool {
    optional_bool(config, &["auth", "smtpAuth", "smtp_auth"])
        .unwrap_or_else(|| optional_string(config, &["username"]).is_some())
}

/// Alert payload from message.
pub(super) fn alert_payload_from_message(message: &NotificationMessageSummary) -> AlertPayload {
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

/// Parse json object.
pub(super) fn parse_json_object(raw: &str) -> serde_json::Map<String, serde_json::Value> {
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

/// Optional string.
pub(super) fn optional_string(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| map.get(*key).and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_bool(map: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| {
        map.get(*key).and_then(|value| match value {
            serde_json::Value::Bool(item) => Some(*item),
            serde_json::Value::String(item) => match item.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            _ => None,
        })
    })
}

fn optional_u16(map: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<u16> {
    keys.iter().find_map(|key| {
        map.get(*key).and_then(|value| match value {
            serde_json::Value::Number(item) => {
                item.as_u64().and_then(|value| u16::try_from(value).ok())
            }
            serde_json::Value::String(item) => item.trim().parse::<u16>().ok(),
            _ => None,
        })
    })
}

fn optional_secret(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    secret_ref_string(map, keys)
        .and_then(|reference| resolve_channel_secret_value(Some(&reference)))
}

fn optional_signing_secret(
    config: &serde_json::Map<String, serde_json::Value>,
    secrets: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    optional_string(
        config,
        &[
            "signingKey",
            "signing_key",
            "secret",
            "secretKey",
            "secret_key",
        ],
    )
    .or_else(|| {
        optional_secret(
            secrets,
            &[
                "signingKey",
                "signing_key",
                "secret",
                "secretKey",
                "secret_key",
            ],
        )
    })
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

fn resolve_channel_secret_value(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(env_name) = value.strip_prefix("env:") {
        return std::env::var(env_name.trim())
            .ok()
            .filter(|resolved| !resolved.trim().is_empty());
    }
    Some(value.to_owned())
}

fn is_env_secret_ref(value: &str) -> bool {
    value.trim().starts_with("env:")
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

/// Notification payload.
pub(super) fn notification_payload(message: &NotificationMessageSummary) -> serde_json::Value {
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
        map.insert(
            "triggeredAt".to_owned(),
            serde_json::Value::String(message.created_at.clone()),
        );
        map.insert(
            "createdAt".to_owned(),
            serde_json::Value::String(message.created_at.clone()),
        );
    }
    payload
}

/// Notification text.
pub(super) fn notification_text(message: &NotificationMessageSummary) -> String {
    format!(
        "[tikeo/{}] {}: {} ({}/{})",
        message.severity,
        message.event_type,
        message.subject,
        message.resource_type,
        message.resource_id
    )
}

/// Pagerduty severity.
pub(super) fn pagerduty_severity(severity: &str) -> &'static str {
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
                serde_json::Value::String(reference) => {
                    resolve_channel_secret_value(Some(reference))
                }
                serde_json::Value::Object(object) => object
                    .get("ref")
                    .or_else(|| object.get("secretRef"))
                    .or_else(|| object.get("secret_ref"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|reference| resolve_channel_secret_value(Some(reference))),
                _ => None,
            };
            if let Some(resolved) = resolved {
                headers.push((name.clone(), resolved));
            }
        }
    }
    headers
}
