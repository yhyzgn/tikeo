//! Alert rules and notification channels.

use serde::{Deserialize, Serialize};
use std::{net::IpAddr, sync::Arc, time::Duration};
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
    /// Tikee tick delay exceeds threshold in seconds.
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
    /// Webhook notification (HTTP POST).
    Webhook {
        /// Target URL.
        url: String,
    },
    /// Email channel (reserved for future implementation).
    #[allow(dead_code)]
    Email {
        /// Recipient addresses.
        recipients: Vec<String>,
    },
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
                    if let Err(error) = validate_webhook_url(url, self.policy) {
                        warn!(target = %redact_url(url), %error, "alert webhook rejected by safety policy");
                        results.push(AlertDeliveryResult {
                            provider: "webhook".to_owned(),
                            target: redact_url(url),
                            delivered: false,
                            status: None,
                            error: Some(error.to_owned()),
                        });
                        continue;
                    }
                    match self.http.post(url).json(payload).send().await {
                        Ok(resp) => {
                            let status = resp.status();
                            let delivered = status.is_success();
                            info!(target = %redact_url(url), status = status.as_u16(), "alert webhook delivered");
                            results.push(AlertDeliveryResult {
                                provider: "webhook".to_owned(),
                                target: redact_url(url),
                                delivered,
                                status: Some(status.as_u16()),
                                error: if delivered {
                                    None
                                } else {
                                    Some(format!("webhook returned HTTP {status}"))
                                },
                            });
                        }
                        Err(e) => {
                            warn!(target = %redact_url(url), error = %e, "alert webhook delivery failed");
                            results.push(AlertDeliveryResult {
                                provider: "webhook".to_owned(),
                                target: redact_url(url),
                                delivered: false,
                                status: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                NotificationChannel::Email { .. } => {
                    warn!("email notification channel not yet implemented");
                    results.push(AlertDeliveryResult {
                        provider: "email".to_owned(),
                        target: "email".to_owned(),
                        delivered: false,
                        status: None,
                        error: Some("email notification channel not yet implemented".to_owned()),
                    });
                }
            }
        }
        results
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
}
