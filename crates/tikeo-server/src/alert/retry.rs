//! Alert delivery retry processor.

use super::{AlertDeliveryPolicy, AlertDispatcher};
use crate::cluster::SharedClusterCoordinator;
use std::time::Duration;
use tikeo_storage::{
    AlertDeliveryAttemptSummary, AlertEventSummary, AlertRepository, AlertRuleSummary,
    RecordAlertDeliveryAttempt,
};
use tokio::time as tokio_time;
use tracing::{debug, info, warn};

/// Retry policy for alert delivery attempts.
#[derive(Debug, Clone, Copy)]
pub struct AlertRetryPolicy {
    /// Maximum attempts before an item moves to dead-letter state.
    pub max_attempts: i32,
    /// Backoff in seconds before another retry may run.
    pub backoff_seconds: i64,
}

impl Default for AlertRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_seconds: 300,
        }
    }
}

/// Retry processing result.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AlertRetryProcessSummary {
    /// Due attempts inspected.
    pub scanned: u64,
    /// New retry attempt records appended.
    pub retried: u64,
    /// Attempts that moved to dead-letter state.
    pub dead_lettered: u64,
    /// Due attempts skipped because their source event/rule/channel was unavailable.
    pub skipped: u64,
}

/// Process due alert delivery attempts using production-safe delivery policy.
///
/// # Errors
/// Returns storage errors from loading or updating delivery attempts.
pub async fn process_due_alert_delivery_retries(
    alerts: &AlertRepository,
    limit: u64,
    policy: AlertRetryPolicy,
) -> Result<AlertRetryProcessSummary, tikeo_storage::DbErr> {
    process_due_alert_delivery_retries_with_delivery_policy(
        alerts,
        limit,
        policy,
        AlertDeliveryPolicy::production(),
    )
    .await
}

/// Process due alert delivery attempts with an explicit delivery policy.
///
/// # Errors
/// Returns storage errors from loading rules/events or writing retry attempt state.
pub async fn retry_once_if_owner(
    alerts: &AlertRepository,
    cluster: &SharedClusterCoordinator,
    limit: u64,
    policy: AlertRetryPolicy,
) -> Result<AlertRetryProcessSummary, tikeo_storage::DbErr> {
    let status = cluster.status().await;
    if !status.can_schedule {
        debug!(role = status.role.as_str(), node_id = %status.node_id, "skip alert retry processing without cluster ownership");
        return Ok(AlertRetryProcessSummary::default());
    }
    process_due_alert_delivery_retries(alerts, limit, policy).await
}

/// Run the alert retry worker forever.
pub async fn run_retry_loop(
    alerts: AlertRepository,
    cluster: SharedClusterCoordinator,
    interval: Duration,
    limit: u64,
    policy: AlertRetryPolicy,
) {
    let mut ticker = tokio_time::interval(interval.max(Duration::from_secs(1)));
    info!(
        interval_seconds = interval.as_secs(),
        limit,
        max_attempts = policy.max_attempts,
        "alert retry worker started"
    );
    loop {
        ticker.tick().await;
        match retry_once_if_owner(&alerts, &cluster, limit, policy).await {
            Ok(summary) if summary.scanned > 0 => {
                info!(
                    scanned = summary.scanned,
                    retried = summary.retried,
                    dead_lettered = summary.dead_lettered,
                    skipped = summary.skipped,
                    "alert retry iteration completed"
                );
            }
            Ok(_) => {}
            Err(error) => warn!(%error, "alert retry iteration failed"),
        }
    }
}

/// Process due alert delivery attempts with an explicit delivery policy.
///
/// # Errors
/// Returns storage errors from loading rules/events or writing retry attempt state.
pub async fn process_due_alert_delivery_retries_with_delivery_policy(
    alerts: &AlertRepository,
    limit: u64,
    policy: AlertRetryPolicy,
    delivery_policy: AlertDeliveryPolicy,
) -> Result<AlertRetryProcessSummary, tikeo_storage::DbErr> {
    let due = alerts.list_due_delivery_attempts(limit).await?;
    let mut summary = AlertRetryProcessSummary::default();
    for attempt in due {
        summary.scanned = summary.scanned.saturating_add(1);
        process_due_attempt(alerts, &attempt, policy, delivery_policy, &mut summary).await?;
    }
    Ok(summary)
}

async fn process_due_attempt(
    alerts: &AlertRepository,
    attempt: &AlertDeliveryAttemptSummary,
    policy: AlertRetryPolicy,
    delivery_policy: AlertDeliveryPolicy,
    summary: &mut AlertRetryProcessSummary,
) -> Result<(), tikeo_storage::DbErr> {
    if attempt.attempt >= policy.max_attempts {
        dead_letter_attempt(alerts, attempt, "max retry attempts exhausted", summary).await?;
        return Ok(());
    }
    let Some((event, rule)) = load_retry_context(alerts, attempt, summary).await? else {
        return Ok(());
    };
    let retry_channels = retry_channels(alerts, attempt, &rule).await?;
    if retry_channels.is_empty() {
        dead_letter_attempt(
            alerts,
            attempt,
            "matching notification channel not found",
            summary,
        )
        .await?;
        return Ok(());
    }
    alerts
        .mark_delivery_attempt_retry_state(&attempt.id, "retry_consumed", None, None)
        .await?;
    let payload = alert_payload_from_event(event);
    let results = AlertDispatcher::new_with_policy(Vec::new(), delivery_policy)
        .deliver_payload(&retry_channels, &payload)
        .await;
    for result in results {
        record_retry_result(alerts, attempt, result, policy, summary).await?;
    }
    Ok(())
}

async fn load_retry_context(
    alerts: &AlertRepository,
    attempt: &AlertDeliveryAttemptSummary,
    summary: &mut AlertRetryProcessSummary,
) -> Result<Option<(AlertEventSummary, AlertRuleSummary)>, tikeo_storage::DbErr> {
    let Some(event) = alerts.get_event(&attempt.event_id).await? else {
        dead_letter_attempt(alerts, attempt, "source alert event not found", summary).await?;
        return Ok(None);
    };
    let Some(rule) = alerts.get_rule(&attempt.rule_id).await? else {
        dead_letter_attempt(alerts, attempt, "source alert rule not found", summary).await?;
        return Ok(None);
    };
    Ok(Some((event, rule)))
}

async fn retry_channels(
    alerts: &AlertRepository,
    attempt: &AlertDeliveryAttemptSummary,
    rule: &AlertRuleSummary,
) -> Result<Vec<super::NotificationChannel>, tikeo_storage::DbErr> {
    let plugin_channels = tikeo_storage::PluginRepository::new(alerts.db())
        .list_plugins()
        .await?
        .into_iter()
        .filter(|plugin| plugin.enabled)
        .flat_map(|plugin| plugin.alert_channel_types)
        .collect::<Vec<_>>();
    let channels = super::notification_channels_from_json(&rule.channels_json, &plugin_channels);
    Ok(channels
        .into_iter()
        .filter(|channel| {
            let identity = super::notification_channel_identity(channel);
            identity.provider == attempt.provider && identity.target == attempt.target
        })
        .collect())
}

fn alert_payload_from_event(event: AlertEventSummary) -> super::AlertPayload {
    super::AlertPayload {
        rule_name: event.rule_name,
        severity: severity_from_str(&event.severity),
        message: event.message.unwrap_or_else(|| event.event_type.clone()),
        resource_type: event.resource_type,
        resource_id: event.resource_id,
        triggered_at: event.created_at,
    }
}

async fn record_retry_result(
    alerts: &AlertRepository,
    attempt: &AlertDeliveryAttemptSummary,
    result: super::AlertDeliveryResult,
    policy: AlertRetryPolicy,
    summary: &mut AlertRetryProcessSummary,
) -> Result<(), tikeo_storage::DbErr> {
    let delivered = result.delivered;
    let next_attempt = attempt.attempt.saturating_add(1);
    let exhausted = !delivered && next_attempt >= policy.max_attempts;
    alerts
        .record_delivery_attempt(RecordAlertDeliveryAttempt {
            event_id: attempt.event_id.clone(),
            rule_id: attempt.rule_id.clone(),
            provider: result.provider,
            target: result.target,
            delivered,
            status_code: result.status.map(i32::from),
            error: result.error,
            attempt: next_attempt,
            retry_state: retry_state_for(delivered, exhausted),
            next_retry_at: next_retry_at(delivered, exhausted, policy.backoff_seconds),
        })
        .await?;
    if delivered || !exhausted {
        summary.retried = summary.retried.saturating_add(1);
    } else {
        summary.dead_lettered = summary.dead_lettered.saturating_add(1);
    }
    Ok(())
}

async fn dead_letter_attempt(
    alerts: &AlertRepository,
    attempt: &AlertDeliveryAttemptSummary,
    reason: &str,
    summary: &mut AlertRetryProcessSummary,
) -> Result<(), tikeo_storage::DbErr> {
    alerts
        .mark_delivery_attempt_retry_state(&attempt.id, "dead_letter", Some(reason), None)
        .await?;
    summary.dead_lettered = summary.dead_lettered.saturating_add(1);
    Ok(())
}

fn retry_state_for(delivered: bool, exhausted: bool) -> String {
    if delivered {
        "delivered"
    } else if exhausted {
        "dead_letter"
    } else {
        "retry_pending"
    }
    .to_owned()
}

fn next_retry_at(delivered: bool, exhausted: bool, backoff_seconds: i64) -> Option<String> {
    (!delivered && !exhausted).then(|| retry_after_seconds(backoff_seconds))
}

fn severity_from_str(value: &str) -> super::Severity {
    match value {
        "critical" => super::Severity::Critical,
        "warning" => super::Severity::Warning,
        _ => super::Severity::Info,
    }
}

fn retry_after_seconds(seconds: i64) -> String {
    time::OffsetDateTime::now_utc()
        .saturating_add(time::Duration::seconds(seconds.max(1)))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};

    #[tokio::test]
    async fn retry_once_if_owner_skips_when_cluster_cannot_schedule() {
        let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("storage should initialize: {error}"));
        let alerts = AlertRepository::new(db);
        let summary = retry_once_if_owner(
            &alerts,
            &StaticCoordinator::shared(ClusterStatus {
                mode: ClusterMode::Raft,
                role: ClusterRole::Follower,
                node_id: "node-b".to_owned(),
                nodes: 3,
                can_schedule: false,
                leader_fencing_token: None,
                detail: "test follower".to_owned(),
            }),
            10,
            AlertRetryPolicy::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("retry ownership gate should run: {error}"));

        assert_eq!(summary.scanned, 0);
        assert_eq!(summary.retried, 0);
        assert_eq!(summary.dead_lettered, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[tokio::test]
    async fn retry_processor_delivers_due_attempt_and_marks_previous_consumed() {
        let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("storage should initialize: {error}"));
        let alerts = AlertRepository::new(db);
        let rule = alerts
            .create_rule(tikeo_storage::CreateAlertRule {
                name: "Retry rule".to_owned(),
                severity: "warning".to_owned(),
                condition_json: serde_json::json!({"type":"script_governance_failure","failure_class":"x","threshold":1}).to_string(),
                channels_json: serde_json::json!([{"type":"webhook","url":"http://127.0.0.1:9/retry"}]).to_string(),
                enabled: true,
                dedupe_seconds: 1,
                silenced_until: None,
            })
            .await
            .unwrap_or_else(|error| panic!("rule should create: {error}"));
        let events = alerts
            .record_script_governance_failure("inst-retry", "x", "retry me")
            .await
            .unwrap_or_else(|error| panic!("event should create: {error}"));
        let event = events
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("event should exist"));
        let first = alerts
            .record_delivery_attempt(RecordAlertDeliveryAttempt {
                event_id: event.id.clone(),
                rule_id: rule.id.clone(),
                provider: "webhook".to_owned(),
                target: "http://127.0.0.1:9/...".to_owned(),
                delivered: false,
                status_code: None,
                error: Some("first failure".to_owned()),
                attempt: 1,
                retry_state: "retry_pending".to_owned(),
                next_retry_at: Some("1970-01-01T00:00:00Z".to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("attempt should record: {error}"));

        let summary = process_due_alert_delivery_retries_with_delivery_policy(
            &alerts,
            10,
            AlertRetryPolicy {
                max_attempts: 2,
                backoff_seconds: 1,
            },
            AlertDeliveryPolicy {
                allow_insecure_loopback: true,
            },
        )
        .await
        .unwrap_or_else(|error| panic!("retry should process: {error}"));

        assert_eq!(summary.scanned, 1);
        assert_eq!(summary.dead_lettered, 1);
        let attempts = alerts
            .list_delivery_attempts(tikeo_storage::AlertDeliveryAttemptFilters {
                event_id: Some(event.id),
                ..Default::default()
            })
            .await
            .unwrap_or_else(|error| panic!("attempts should list: {error}"));
        assert_eq!(attempts.len(), 2);
        assert!(
            attempts
                .iter()
                .any(|attempt| attempt.id == first.id && attempt.retry_state == "retry_consumed")
        );
        assert!(
            attempts
                .iter()
                .any(|attempt| attempt.attempt == 2 && attempt.retry_state == "dead_letter")
        );
    }
}
