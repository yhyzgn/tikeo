//! Alert rules and notification channels.

mod email;
mod retry;

use email::deliver_email_channel;
pub use retry::{
    AlertRetryPolicy, AlertRetryProcessSummary, process_due_alert_delivery_retries, run_retry_loop,
};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, sync::Arc, time::Duration};
use tikeo_storage::PluginAlertChannelTypeSummary;
use tracing::{info, warn};
use url::Url;

/// Alert severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Critical.
    Critical,
}

/// Alert rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Alert severity.
    pub severity: Severity,
    /// Condition that triggers the alert.
    pub condition: AlertCondition,
    /// Notification channels.
    pub channels: Vec<NotificationChannel>,
}

/// Conditions that can trigger alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    /// Job failure count exceeds threshold within a time window.
    JobFailureThreshold {
        /// Maximum failures before alerting.
        threshold: u32,
    },
    /// Worker offline (no heartbeat within grace period).
    WorkerOffline,
    /// Tikeo tick delay exceeds threshold in seconds.
    ScheduleDelay {
        /// Maximum delay in seconds before alerting.
        threshold_seconds: u64,
    },
    /// Script execution governance failures exceed a threshold.
    ScriptGovernanceFailure {
        /// Stable governance failure class.
        failure_class: String,
        /// Maximum failures before alerting.
        threshold: u32,
    },
}

/// Notification channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationChannel {
    /// Generic webhook notification (HTTP POST).
    Webhook {
        /// Target URL.
        url: String,
    },
    /// Slack incoming webhook notification.
    Slack {
        /// Slack incoming webhook URL.
        url: String,
    },
    /// `DingTalk` robot webhook notification.
    #[serde(rename = "dingtalk", alias = "ding_talk")]
    DingTalk {
        /// `DingTalk` webhook URL.
        url: String,
    },
    /// Feishu/Lark robot webhook notification.
    Feishu {
        /// Feishu webhook URL.
        url: String,
    },
    /// WeCom/WeChat Work robot webhook notification.
    #[serde(rename = "wechat_work", alias = "wecom")]
    WechatWork {
        /// `WeChat` Work webhook URL.
        url: String,
    },
    /// `PagerDuty` Events API v2 notification.
    #[serde(rename = "pagerduty", alias = "pager_duty")]
    PagerDuty {
        /// Optional Events API URL, primarily for local smoke tests.
        url: Option<String>,
        /// `PagerDuty` routing/integration key.
        routing_key: String,
    },
    /// Plugin-defined webhook-compatible notification channel.
    #[serde(skip)]
    PluginWebhook {
        /// Plugin channel type.
        channel_type: String,
        /// Target webhook URL.
        url: String,
        /// Plugin-provided payload template metadata.
        template: serde_json::Value,
    },
    /// Email channel delivered through a configured SMTP endpoint.
    Email {
        /// Recipient addresses.
        #[serde(
            default,
            alias = "to",
            deserialize_with = "email::deserialize_recipients"
        )]
        recipients: Vec<String>,
        /// SMTP endpoint URL. Plain `smtp://` is allowed only for explicit local loopback smoke tests.
        #[serde(default, alias = "url")]
        smtp_url: Option<String>,
        /// Secret/env reference that contains the SMTP endpoint URL.
        #[serde(default)]
        smtp_url_secret_ref: Option<String>,
        /// Envelope sender address.
        #[serde(default)]
        from: Option<String>,
        /// SMTP username or user secret/env reference.
        #[serde(default)]
        username: Option<String>,
        /// SMTP password secret/env reference.
        #[serde(default)]
        password_secret_ref: Option<String>,
    },
}

/// Build notification channels from persisted JSON and enabled plugin channel declarations.
#[must_use]
pub fn notification_channels_from_json(
    channels_json: &str,
    plugins: &[PluginAlertChannelTypeSummary],
) -> Vec<NotificationChannel> {
    let values = serde_json::from_str::<Vec<serde_json::Value>>(channels_json).unwrap_or_default();
    values
        .into_iter()
        .filter_map(|value| notification_channel_from_value(&value, plugins))
        .collect()
}

fn notification_channel_from_value(
    value: &serde_json::Value,
    plugins: &[PluginAlertChannelTypeSummary],
) -> Option<NotificationChannel> {
    if let Ok(channel) = serde_json::from_value::<NotificationChannel>(value.clone()) {
        return Some(channel);
    }
    let channel_type = value.get("type").and_then(serde_json::Value::as_str)?;
    let plugin = plugins
        .iter()
        .find(|plugin| plugin.r#type == channel_type)?;
    if plugin.target_kind != "webhook" {
        return None;
    }
    value
        .get("url")
        .or_else(|| value.get("webhook_url"))
        .and_then(serde_json::Value::as_str)
        .filter(|url| !url.trim().is_empty())
        .map(|url| NotificationChannel::PluginWebhook {
            channel_type: channel_type.to_owned(),
            url: url.to_owned(),
            template: plugin.template.clone(),
        })
}

/// Payload sent to notification channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    /// Rule that triggered.
    pub rule_name: String,
    /// Alert severity.
    pub severity: Severity,
    /// Human-readable description.
    pub message: String,
    /// Resource type (job, worker, etc.).
    pub resource_type: String,
    /// Resource identifier.
    pub resource_id: String,
    /// RFC3339 timestamp.
    pub triggered_at: String,
}

/// Alert delivery safety policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertDeliveryPolicy {
    /// Allow HTTP delivery to loopback addresses for local smoke tests.
    pub allow_insecure_loopback: bool,
}

impl AlertDeliveryPolicy {
    /// Production-safe default: HTTPS public webhooks only.
    #[must_use]
    pub const fn production() -> Self {
        Self {
            allow_insecure_loopback: false,
        }
    }
}

/// One notification channel delivery result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDeliveryResult {
    /// Channel provider.
    pub provider: String,
    /// Redacted target or provider label.
    pub target: String,
    /// Whether the provider accepted the delivery.
    pub delivered: bool,
    /// HTTP status when available.
    pub status: Option<u16>,
    /// Error or rejection reason when delivery did not succeed.
    pub error: Option<String>,
}

/// Redacted provider identity for matching persisted retry attempts back to rule channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationChannelIdentity {
    /// Provider name.
    pub provider: String,
    /// Redacted delivery target.
    pub target: String,
}

/// Return the retry identity for a notification channel without sending it.
#[must_use]
pub fn notification_channel_identity(channel: &NotificationChannel) -> NotificationChannelIdentity {
    match channel {
        NotificationChannel::Webhook { url } => NotificationChannelIdentity {
            provider: "webhook".to_owned(),
            target: redact_url(url),
        },
        NotificationChannel::Slack { url } => NotificationChannelIdentity {
            provider: "slack".to_owned(),
            target: redact_url(url),
        },
        NotificationChannel::DingTalk { url } => NotificationChannelIdentity {
            provider: "dingtalk".to_owned(),
            target: redact_url(url),
        },
        NotificationChannel::Feishu { url } => NotificationChannelIdentity {
            provider: "feishu".to_owned(),
            target: redact_url(url),
        },
        NotificationChannel::WechatWork { url } => NotificationChannelIdentity {
            provider: "wechat_work".to_owned(),
            target: redact_url(url),
        },
        NotificationChannel::PagerDuty { url, .. } => NotificationChannelIdentity {
            provider: "pagerduty".to_owned(),
            target: redact_url(
                url.as_deref()
                    .unwrap_or("https://events.pagerduty.com/v2/enqueue"),
            ),
        },
        NotificationChannel::PluginWebhook {
            channel_type, url, ..
        } => NotificationChannelIdentity {
            provider: channel_type.clone(),
            target: redact_url(url),
        },
        NotificationChannel::Email { recipients, .. } => NotificationChannelIdentity {
            provider: "email".to_owned(),
            target: recipients.join(","),
        },
    }
}

/// Alert dispatcher that evaluates rules and sends notifications.
#[derive(Debug, Clone)]
pub struct AlertDispatcher {
    rules: Arc<Vec<AlertRule>>,
    http: reqwest::Client,
    policy: AlertDeliveryPolicy,
}

impl AlertDispatcher {
    /// Create a dispatcher with the given rules.
    #[must_use]
    pub fn new(rules: Vec<AlertRule>) -> Self {
        Self::new_with_policy(rules, AlertDeliveryPolicy::production())
    }

    /// Create a dispatcher with explicit delivery policy.
    #[must_use]
    pub fn new_with_policy(rules: Vec<AlertRule>, policy: AlertDeliveryPolicy) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|error| {
                warn!(%error, "failed to build alert HTTP client; using default client");
                reqwest::Client::new()
            });
        Self {
            rules: Arc::new(rules),
            http,
            policy,
        }
    }

    /// Create a dispatcher with no rules (no-op).
    #[must_use]
    pub fn noop() -> Self {
        Self::new(Vec::new())
    }

    /// Fire a job failure alert.
    pub async fn fire_job_failure(
        &self,
        job_id: &str,
        failure_count: u32,
    ) -> Vec<AlertDeliveryResult> {
        let mut results = Vec::new();
        for rule in self.rules.iter() {
            let AlertCondition::JobFailureThreshold { threshold } = &rule.condition else {
                continue;
            };
            if failure_count < *threshold {
                continue;
            }
            let payload = AlertPayload {
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                message: format!(
                    "job {job_id} has failed {failure_count} times (threshold={threshold})"
                ),
                resource_type: "job".to_owned(),
                resource_id: job_id.to_owned(),
                triggered_at: now_rfc3339(),
            };
            results.extend(self.dispatch(&rule.channels, &payload).await);
        }
        results
    }

    /// Fire a script governance failure alert.
    pub async fn fire_script_governance_failure(
        &self,
        instance_id: &str,
        failure_class: &str,
        failure_count: u32,
    ) -> Vec<AlertDeliveryResult> {
        let mut results = Vec::new();
        for rule in self.rules.iter() {
            let AlertCondition::ScriptGovernanceFailure {
                failure_class: expected,
                threshold,
            } = &rule.condition
            else {
                continue;
            };
            if expected != failure_class || failure_count < *threshold {
                continue;
            }
            let payload = AlertPayload {
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                message: format!(
                    "script governance failure {failure_class} occurred {failure_count} times (threshold={threshold})"
                ),
                resource_type: "script_execution_governance".to_owned(),
                resource_id: instance_id.to_owned(),
                triggered_at: now_rfc3339(),
            };
            results.extend(self.dispatch(&rule.channels, &payload).await);
        }
        results
    }

    /// Fire a worker offline alert.
    pub async fn fire_worker_offline(&self, worker_id: &str) -> Vec<AlertDeliveryResult> {
        let mut results = Vec::new();
        for rule in self.rules.iter() {
            if !matches!(rule.condition, AlertCondition::WorkerOffline) {
                continue;
            }
            let payload = AlertPayload {
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                message: format!("worker {worker_id} is offline"),
                resource_type: "worker".to_owned(),
                resource_id: worker_id.to_owned(),
                triggered_at: now_rfc3339(),
            };
            results.extend(self.dispatch(&rule.channels, &payload).await);
        }
        results
    }

    async fn dispatch(
        &self,
        channels: &[NotificationChannel],
        payload: &AlertPayload,
    ) -> Vec<AlertDeliveryResult> {
        self.deliver_payload(channels, payload).await
    }

    /// Deliver an already materialized alert payload to provider channels.
    pub async fn deliver_payload(
        &self,
        channels: &[NotificationChannel],
        payload: &AlertPayload,
    ) -> Vec<AlertDeliveryResult> {
        let mut results = Vec::new();
        for channel in channels {
            match channel {
                NotificationChannel::Webhook { url } => {
                    let body =
                        serde_json::to_value(payload).unwrap_or_else(|_| serde_json::json!({}));
                    results.push(self.post_json("webhook", url, &body).await);
                }
                NotificationChannel::Slack { url } => {
                    let body = serde_json::json!({ "text": alert_text(payload) });
                    results.push(self.post_json("slack", url, &body).await);
                }
                NotificationChannel::DingTalk { url } => {
                    let body = serde_json::json!({
                        "msgtype": "text",
                        "text": { "content": alert_text(payload) },
                    });
                    results.push(self.post_json("dingtalk", url, &body).await);
                }
                NotificationChannel::Feishu { url } => {
                    let body = serde_json::json!({
                        "msg_type": "text",
                        "content": { "text": alert_text(payload) },
                    });
                    results.push(self.post_json("feishu", url, &body).await);
                }
                NotificationChannel::WechatWork { url } => {
                    let body = serde_json::json!({
                        "msgtype": "text",
                        "text": { "content": alert_text(payload) },
                    });
                    results.push(self.post_json("wechat_work", url, &body).await);
                }
                NotificationChannel::PagerDuty { url, routing_key } => {
                    let target = url
                        .as_deref()
                        .unwrap_or("https://events.pagerduty.com/v2/enqueue");
                    let body = serde_json::json!({
                        "routing_key": routing_key,
                        "event_action": "trigger",
                        "dedup_key": format!("{}:{}", payload.resource_type, payload.resource_id),
                        "payload": {
                            "summary": payload.message,
                            "source": "tikeo",
                            "severity": pagerduty_severity(&payload.severity),
                            "component": payload.resource_type,
                            "custom_details": payload,
                        },
                    });
                    results.push(self.post_json("pagerduty", target, &body).await);
                }
                NotificationChannel::PluginWebhook {
                    channel_type,
                    url,
                    template,
                } => {
                    let body = plugin_webhook_body(template, payload);
                    let headers = plugin_webhook_headers(template, payload);
                    results.push(
                        self.post_json_with_headers(channel_type, url, &body, &headers)
                            .await,
                    );
                }
                NotificationChannel::Email {
                    recipients,
                    smtp_url,
                    smtp_url_secret_ref,
                    from,
                    username,
                    password_secret_ref,
                } => {
                    let smtp_url = smtp_url
                        .clone()
                        .or_else(|| resolve_secret_ref(smtp_url_secret_ref.as_deref()));
                    let password = resolve_secret_ref(password_secret_ref.as_deref());
                    results.push(
                        self.deliver_email(
                            recipients,
                            smtp_url.as_deref(),
                            from.as_deref(),
                            username.as_deref(),
                            password.as_deref(),
                            payload,
                        )
                        .await,
                    );
                }
            }
        }
        results
    }

    async fn deliver_email(
        &self,
        recipients: &[String],
        smtp_url: Option<&str>,
        from: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        payload: &AlertPayload,
    ) -> AlertDeliveryResult {
        let Some(smtp_url) = smtp_url else {
            return email_failure(recipients, "smtp_url is required for email delivery");
        };
        if recipients.is_empty() {
            return email_failure(recipients, "at least one email recipient is required");
        }
        let from = from.unwrap_or("tikeo@localhost");
        deliver_email_channel(
            smtp_url,
            from,
            recipients,
            username,
            password,
            payload,
            self.policy,
        )
        .await
    }

    async fn post_json(
        &self,
        provider: &str,
        url: &str,
        body: &serde_json::Value,
    ) -> AlertDeliveryResult {
        self.post_json_with_headers(provider, url, body, &[]).await
    }

    async fn post_json_with_headers(
        &self,
        provider: &str,
        url: &str,
        body: &serde_json::Value,
        headers: &[(String, String)],
    ) -> AlertDeliveryResult {
        if let Err(error) = validate_webhook_url(url, self.policy) {
            warn!(provider, target = %redact_url(url), %error, "alert provider rejected by safety policy");
            return AlertDeliveryResult {
                provider: provider.to_owned(),
                target: redact_url(url),
                delivered: false,
                status: None,
                error: Some(error.to_owned()),
            };
        }
        let mut request = self.http.post(url).json(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        match request.send().await {
            Ok(resp) => {
                let status = resp.status();
                let delivered = status.is_success();
                info!(provider, target = %redact_url(url), status = status.as_u16(), "alert provider delivered");
                AlertDeliveryResult {
                    provider: provider.to_owned(),
                    target: redact_url(url),
                    delivered,
                    status: Some(status.as_u16()),
                    error: if delivered {
                        None
                    } else {
                        Some(format!("{provider} returned HTTP {status}"))
                    },
                }
            }
            Err(error) => {
                warn!(provider, target = %redact_url(url), %error, "alert provider delivery failed");
                AlertDeliveryResult {
                    provider: provider.to_owned(),
                    target: redact_url(url),
                    delivered: false,
                    status: None,
                    error: Some(error.to_string()),
                }
            }
        }
    }
}

fn plugin_webhook_body(template: &serde_json::Value, payload: &AlertPayload) -> serde_json::Value {
    let mut body = template.get("body").cloned().unwrap_or_else(|| {
        serde_json::json!({
            "text": alert_text(payload),
        })
    });
    replace_template_tokens(&mut body, payload);
    body
}

fn plugin_webhook_headers(
    template: &serde_json::Value,
    payload: &AlertPayload,
) -> Vec<(String, String)> {
    let mut headers = template
        .get("headers")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    replace_template_tokens(&mut headers, payload);
    headers
        .as_object()
        .map(|items| {
            items
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
fn plugin_webhook_example_payload() -> AlertPayload {
    AlertPayload {
        rule_name: "Plugin alert".to_owned(),
        severity: Severity::Warning,
        message: "custom plugin alert message".to_owned(),
        resource_type: "job".to_owned(),
        resource_id: "job_plugin".to_owned(),
        triggered_at: "2026-05-28T00:00:00Z".to_owned(),
    }
}

fn replace_template_tokens(value: &mut serde_json::Value, payload: &AlertPayload) {
    match value {
        serde_json::Value::String(item) => {
            *item = item
                .replace("{{message}}", &payload.message)
                .replace("{{resource_id}}", &payload.resource_id)
                .replace("{{resource_type}}", &payload.resource_type)
                .replace("{{severity}}", pagerduty_severity(&payload.severity));
        }
        serde_json::Value::Array(items) => {
            for item in items {
                replace_template_tokens(item, payload);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                replace_template_tokens(item, payload);
            }
        }
        _ => {}
    }
}

fn email_failure(recipients: &[String], error: &str) -> AlertDeliveryResult {
    AlertDeliveryResult {
        provider: "email".to_owned(),
        target: if recipients.is_empty() {
            "email".to_owned()
        } else {
            recipients.join(",")
        },
        delivered: false,
        status: None,
        error: Some(error.to_owned()),
    }
}

fn resolve_secret_ref(reference: Option<&str>) -> Option<String> {
    let reference = reference?.trim();
    let key = reference.strip_prefix("env:").unwrap_or(reference);
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn alert_text(payload: &AlertPayload) -> String {
    format!(
        "[tikeo/{:?}] {}: {} ({}/{})",
        payload.severity,
        payload.rule_name,
        payload.message,
        payload.resource_type,
        payload.resource_id
    )
}

const fn pagerduty_severity(severity: &Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn validate_webhook_url(value: &str, policy: AlertDeliveryPolicy) -> Result<(), &'static str> {
    let parsed = Url::parse(value).map_err(|_| "invalid url")?;
    let Some(host) = parsed.host_str() else {
        return Err("webhook url must include host");
    };
    let host_lower = host.to_ascii_lowercase();
    if parsed.scheme() == "http" && policy.allow_insecure_loopback && is_loopback_host(&host_lower)
    {
        return Ok(());
    }
    if parsed.scheme() != "https" {
        return Err("webhook url must use https");
    }
    if matches!(
        host_lower.as_str(),
        "localhost" | "metadata.google.internal"
    ) {
        return Err("webhook host is not allowed");
    }
    if let Ok(ip) = host.parse::<IpAddr>()
        && !is_public_ip(ip)
    {
        return Err("webhook ip must be public");
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn redact_url(value: &str) -> String {
    Url::parse(value).map_or_else(
        |_| "invalid-url".to_owned(),
        |url| {
            let mut redacted = format!(
                "{}://{}",
                url.scheme(),
                url.host_str().unwrap_or("unknown-host")
            );
            if let Some(port) = url.port() {
                redacted.push(':');
                redacted.push_str(&port.to_string());
            }
            if url.path() != "/" && !url.path().is_empty() {
                redacted.push_str("/...");
            }
            redacted
        },
    )
}

const fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified())
        }
        IpAddr::V6(ip) => !(ip.is_loopback() || ip.is_unspecified()),
    }
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_payload_serializes_to_json() {
        let payload = AlertPayload {
            rule_name: "job-fail".to_owned(),
            severity: Severity::Critical,
            message: "job x failed 5 times".to_owned(),
            resource_type: "job".to_owned(),
            resource_id: "job_123".to_owned(),
            triggered_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        let json = serde_json::to_string(&payload).unwrap_or_default();
        assert!(json.contains("\"rule_name\":\"job-fail\""));
        assert!(json.contains("\"severity\":\"critical\""));
    }

    #[tokio::test]
    async fn noop_dispatcher_does_nothing() {
        let dispatcher = AlertDispatcher::noop();
        dispatcher.fire_job_failure("job_x", 10).await;
        dispatcher.fire_worker_offline("worker_x").await;
        dispatcher
            .fire_script_governance_failure("inst_x", "script_runtime_unavailable", 3)
            .await;
    }

    #[test]
    fn script_governance_alert_condition_serializes() {
        let condition = AlertCondition::ScriptGovernanceFailure {
            failure_class: "script_runtime_unavailable".to_owned(),
            threshold: 3,
        };
        let json = serde_json::to_string(&condition).unwrap_or_default();
        assert!(json.contains("script_governance_failure"));
        assert!(json.contains("script_runtime_unavailable"));
    }

    #[tokio::test]
    async fn production_policy_rejects_insecure_loopback_webhook() {
        let dispatcher = AlertDispatcher::new(vec![AlertRule {
            id: "rule-reject-loopback".to_owned(),
            name: "Reject loopback".to_owned(),
            severity: Severity::Critical,
            condition: AlertCondition::ScriptGovernanceFailure {
                failure_class: "script_runtime_unavailable".to_owned(),
                threshold: 1,
            },
            channels: vec![NotificationChannel::Webhook {
                url: "http://127.0.0.1:9/alert?token=secret".to_owned(),
            }],
        }]);

        let results = dispatcher
            .fire_script_governance_failure("inst-loopback", "script_runtime_unavailable", 1)
            .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "webhook");
        assert_eq!(results[0].target, "http://127.0.0.1:9/...");
        assert!(!results[0].delivered);
        assert_eq!(results[0].status, None);
        assert_eq!(
            results[0].error.as_deref(),
            Some("webhook url must use https")
        );
    }

    #[tokio::test]
    async fn email_dispatch_sends_plain_smtp_to_allowed_local_receiver() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("smtp listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("smtp listener should expose addr: {error}"));
        let captured = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
        let captured_server = captured.clone();
        let smtp_server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .unwrap_or_else(|error| panic!("smtp client should connect: {error}"));
            stream
                .write_all(b"220 tikeo-test-smtp ESMTP\r\n")
                .await
                .unwrap_or_else(|error| panic!("smtp greeting should send: {error}"));
            let mut buffer = [0_u8; 4096];
            let mut transcript = String::new();
            loop {
                let read = stream
                    .read(&mut buffer)
                    .await
                    .unwrap_or_else(|error| panic!("smtp command should read: {error}"));
                if read == 0 {
                    break;
                }
                transcript.push_str(&String::from_utf8_lossy(&buffer[..read]));
                if transcript.contains("DATA\r\n") && !transcript.contains("\r\n.\r\n") {
                    stream
                        .write_all(
                            b"250 hello\r\n250 sender ok\r\n250 recipient ok\r\n354 end data\r\n",
                        )
                        .await
                        .unwrap_or_else(|error| panic!("smtp data prompt should send: {error}"));
                } else if transcript.contains("\r\n.\r\n") {
                    stream
                        .write_all(b"250 queued\r\n221 bye\r\n")
                        .await
                        .unwrap_or_else(|error| panic!("smtp completion should send: {error}"));
                    break;
                } else {
                    stream
                        .write_all(b"250 ok\r\n")
                        .await
                        .unwrap_or_else(|error| panic!("smtp response should send: {error}"));
                }
            }
            *captured_server.lock().await = transcript;
        });

        let dispatcher = AlertDispatcher::new_with_policy(
            vec![AlertRule {
                id: "rule-email".to_owned(),
                name: "Email alerts".to_owned(),
                severity: Severity::Warning,
                condition: AlertCondition::ScriptGovernanceFailure {
                    failure_class: "script_runtime_unavailable".to_owned(),
                    threshold: 1,
                },
                channels: vec![NotificationChannel::Email {
                    recipients: vec!["ops@example.com".to_owned()],
                    smtp_url: Some(format!("smtp://{address}")),
                    smtp_url_secret_ref: None,
                    from: Some("tikeo@example.com".to_owned()),
                    username: None,
                    password_secret_ref: None,
                }],
            }],
            AlertDeliveryPolicy {
                allow_insecure_loopback: true,
            },
        );

        let results = dispatcher
            .fire_script_governance_failure("inst-email", "script_runtime_unavailable", 1)
            .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "email");
        assert_eq!(results[0].target, "ops@example.com");
        assert!(
            results[0].delivered,
            "email result should be delivered: {results:?}"
        );
        smtp_server
            .await
            .unwrap_or_else(|error| panic!("smtp server task should complete: {error}"));
        let transcript = captured.lock().await.clone();
        assert!(transcript.contains("MAIL FROM:<tikeo@example.com>"));
        assert!(transcript.contains("RCPT TO:<ops@example.com>"));
        assert!(transcript.contains("Subject: [tikeo/warning] Email alerts"));
    }

    #[tokio::test]
    async fn provider_dispatch_posts_expected_payload_shapes_to_allowed_local_receivers() {
        use axum::{Json, Router, extract::OriginalUri, routing::post};

        type CapturedRequests =
            std::sync::Arc<tokio::sync::Mutex<Vec<(String, serde_json::Value)>>>;

        async fn capture(
            axum::extract::State(captured): axum::extract::State<CapturedRequests>,
            OriginalUri(uri): OriginalUri,
            Json(payload): Json<serde_json::Value>,
        ) {
            captured.lock().await.push((uri.path().to_owned(), payload));
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("test listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("test listener should expose addr: {error}"));
        let captured: CapturedRequests = std::sync::Arc::default();
        let app = Router::new()
            .route("/slack", post(capture))
            .route("/dingtalk", post(capture))
            .route("/feishu", post(capture))
            .route("/wechat", post(capture))
            .route("/pagerduty", post(capture))
            .with_state(captured.clone());
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .unwrap_or_else(|error| panic!("test provider server should run: {error}"));
        });

        let dispatcher = AlertDispatcher::new_with_policy(
            vec![AlertRule {
                id: "rule-provider-webhooks".to_owned(),
                name: "Provider webhooks".to_owned(),
                severity: Severity::Critical,
                condition: AlertCondition::ScriptGovernanceFailure {
                    failure_class: "script_runtime_unavailable".to_owned(),
                    threshold: 1,
                },
                channels: vec![
                    NotificationChannel::Slack {
                        url: format!("http://{address}/slack"),
                    },
                    NotificationChannel::DingTalk {
                        url: format!("http://{address}/dingtalk"),
                    },
                    NotificationChannel::Feishu {
                        url: format!("http://{address}/feishu"),
                    },
                    NotificationChannel::WechatWork {
                        url: format!("http://{address}/wechat"),
                    },
                    NotificationChannel::PagerDuty {
                        url: Some(format!("http://{address}/pagerduty")),
                        routing_key: "route-123".to_owned(),
                    },
                ],
            }],
            AlertDeliveryPolicy {
                allow_insecure_loopback: true,
            },
        );

        let results = dispatcher
            .fire_script_governance_failure("inst-provider", "script_runtime_unavailable", 1)
            .await;

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|result| result.delivered));
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if captured.lock().await.len() == 5 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|error| panic!("provider payloads should arrive: {error}"));
        let captured = captured.lock().await.clone();
        let payload_for = |path: &str| {
            captured
                .iter()
                .find_map(|(captured_path, payload)| (captured_path == path).then_some(payload))
                .unwrap_or_else(|| panic!("missing provider payload for {path}: {captured:?}"))
        };
        assert!(
            payload_for("/slack")["text"]
                .as_str()
                .is_some_and(|value| value.contains("script governance failure"))
        );
        assert_eq!(payload_for("/dingtalk")["msgtype"], "text");
        assert_eq!(payload_for("/feishu")["msg_type"], "text");
        assert_eq!(payload_for("/wechat")["msgtype"], "text");
        assert_eq!(payload_for("/pagerduty")["routing_key"], "route-123");
        assert_eq!(payload_for("/pagerduty")["event_action"], "trigger");
    }

    #[tokio::test]
    async fn webhook_dispatch_posts_payload_to_allowed_local_receiver() {
        use axum::{Json, Router, routing::post};
        use tokio::sync::oneshot;

        async fn capture(
            tx: axum::extract::State<
                std::sync::Arc<tokio::sync::Mutex<Option<oneshot::Sender<AlertPayload>>>>,
            >,
            Json(payload): Json<AlertPayload>,
        ) {
            let sender = tx.lock().await.take();
            if let Some(sender) = sender {
                let _ = sender.send(payload);
            }
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("test listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("test listener should expose addr: {error}"));
        let (tx, rx) = oneshot::channel();
        let app = Router::new()
            .route("/", post(capture))
            .with_state(std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx))));
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .unwrap_or_else(|error| panic!("test webhook server should run: {error}"));
        });

        let dispatcher = AlertDispatcher::new_with_policy(
            vec![AlertRule {
                id: "rule-local-webhook".to_owned(),
                name: "Local webhook".to_owned(),
                severity: Severity::Critical,
                condition: AlertCondition::ScriptGovernanceFailure {
                    failure_class: "script_runtime_unavailable".to_owned(),
                    threshold: 1,
                },
                channels: vec![NotificationChannel::Webhook {
                    url: format!("http://{address}/"),
                }],
            }],
            AlertDeliveryPolicy {
                allow_insecure_loopback: true,
            },
        );

        let results = dispatcher
            .fire_script_governance_failure("inst-webhook", "script_runtime_unavailable", 1)
            .await;
        assert_eq!(results.len(), 1);
        assert!(results[0].delivered);
        let received = tokio::time::timeout(std::time::Duration::from_secs(2), rx)
            .await
            .unwrap_or_else(|error| panic!("webhook payload should arrive: {error}"))
            .unwrap_or_else(|_| panic!("webhook receiver should not drop"));
        assert_eq!(received.rule_name, "Local webhook");
        assert_eq!(received.resource_id, "inst-webhook");
    }

    #[test]
    fn plugin_webhook_template_replaces_payload_tokens() {
        let payload = plugin_webhook_example_payload();
        let template = serde_json::json!({
            "headers": {
                "X-Tikeo-Resource": "{{resource_type}}/{{resource_id}}",
                "X-Tikeo-Severity": "{{severity}}"
            },
            "body": {
                "text": "{{message}}",
                "resource": "{{resource_type}}/{{resource_id}}",
                "severity": "{{severity}}",
                "nested": ["{{message}}"]
            }
        });

        let body = plugin_webhook_body(&template, &payload);
        let headers = plugin_webhook_headers(&template, &payload);

        assert_eq!(body["text"], "custom plugin alert message");
        assert_eq!(body["resource"], "job/job_plugin");
        assert_eq!(body["severity"], "warning");
        assert_eq!(body["nested"][0], "custom plugin alert message");
        assert!(headers.contains(&("X-Tikeo-Resource".to_owned(), "job/job_plugin".to_owned())));
        assert!(headers.contains(&("X-Tikeo-Severity".to_owned(), "warning".to_owned())));
    }
}
