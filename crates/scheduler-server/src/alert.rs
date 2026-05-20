//! Alert rules and notification channels.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

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
    /// Scheduler tick delay exceeds threshold in seconds.
    ScheduleDelay {
        /// Maximum delay in seconds before alerting.
        threshold_seconds: u64,
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

/// Alert dispatcher that evaluates rules and sends notifications.
#[derive(Debug, Clone)]
pub struct AlertDispatcher {
    rules: Arc<Vec<AlertRule>>,
    http: reqwest::Client,
}

impl AlertDispatcher {
    /// Create a dispatcher with the given rules.
    #[must_use]
    pub fn new(rules: Vec<AlertRule>) -> Self {
        Self {
            rules: Arc::new(rules),
            http: reqwest::Client::new(),
        }
    }

    /// Create a dispatcher with no rules (no-op).
    #[must_use]
    pub fn noop() -> Self {
        Self::new(Vec::new())
    }

    /// Fire a job failure alert.
    pub async fn fire_job_failure(&self, job_id: &str, failure_count: u32) {
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
            self.dispatch(&rule.channels, &payload).await;
        }
    }

    /// Fire a worker offline alert.
    pub async fn fire_worker_offline(&self, worker_id: &str) {
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
            self.dispatch(&rule.channels, &payload).await;
        }
    }

    async fn dispatch(&self, channels: &[NotificationChannel], payload: &AlertPayload) {
        for channel in channels {
            match channel {
                NotificationChannel::Webhook { url } => {
                    match self.http.post(url).json(payload).send().await {
                        Ok(resp) => {
                            info!(
                                url,
                                status = resp.status().as_u16(),
                                "alert webhook delivered"
                            );
                        }
                        Err(e) => {
                            warn!(url, error = %e, "alert webhook delivery failed");
                        }
                    }
                }
                NotificationChannel::Email { .. } => {
                    warn!("email notification channel not yet implemented");
                }
            }
        }
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
    }
}
