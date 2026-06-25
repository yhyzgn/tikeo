//! Script execution governance audit materialization helpers.

use crate::{
    alert::{AlertDispatcher, AlertPayload, Severity, notification_channels_from_json},
    notification::{
        NotificationCenter, emit_alert_event_best_effort,
        ensure_alert_rule_notification_policy_from_channels,
    },
};
use tikeo_storage::{
    AlertRepository, AuditLogRepository, CreateAuditLog, JobRepository,
    NotificationChannelRepository, NotificationDeliveryAttemptRepository,
    NotificationMessageRepository, NotificationPolicyRepository, NotificationTemplateRepository,
    RecordAlertDeliveryAttempt,
};

const GOVERNANCE_EVENT: &str = "script_execution_governance";
const GOVERNANCE_ACTION: &str = "script_governance_failure";

/// Persist a script execution governance failure into the durable audit stream.
///
/// The relation to job instances intentionally remains soft: `resource_id` stores
/// the instance id without introducing a database foreign key.
///
/// # Errors
///
/// Returns an error when the audit repository cannot append the row.
pub async fn materialize_script_governance_audit(
    audit: &AuditLogRepository,
    actor: &str,
    instance_id: &str,
    failure_class: &str,
    message: &str,
) -> Result<(), tikeo_storage::DbErr> {
    let payload = serde_json::json!({
        "event": GOVERNANCE_EVENT,
        "failure_class": failure_class,
        "message": message,
    });
    let _ = audit
        .append(CreateAuditLog {
            actor: actor.to_owned(),
            action: GOVERNANCE_ACTION.to_owned(),
            resource_type: GOVERNANCE_EVENT.to_owned(),
            resource_id: instance_id.to_owned(),
            detail: Some(message.to_owned()),
            before: None,
            after: Some(payload.to_string()),
            trace_id: Some(instance_id.to_owned()),
            result: "failed".to_owned(),
            failure_reason: Some(failure_class.to_owned()),
            ip_address: None,
        })
        .await?;
    let alerts = AlertRepository::new(audit.db());
    let events = alerts
        .record_script_governance_failure(instance_id, failure_class, message)
        .await?;
    let notifications = notification_center_from_audit(audit);
    for event in events.into_iter().filter(|event| event.status == "firing") {
        let Some(rule) = alerts.get_rule(&event.rule_id).await? else {
            continue;
        };
        let plugins = tikeo_storage::PluginRepository::new(audit.db())
            .list_plugins()
            .await?
            .into_iter()
            .filter(|plugin| plugin.enabled)
            .flat_map(|plugin| plugin.alert_channel_types)
            .collect::<Vec<_>>();
        let _policy = ensure_alert_rule_notification_policy_from_channels(
            &rule,
            &NotificationChannelRepository::new(audit.db()),
            &NotificationPolicyRepository::new(audit.db()),
            &plugins,
        )
        .await?;
        emit_alert_event_best_effort(&notifications, &event).await;
        let channels = notification_channels_from_json(&rule.channels_json, &plugins);
        let payload = AlertPayload {
            rule_name: event.rule_name,
            severity: severity_from_str(&event.severity),
            message: event.message.unwrap_or_else(|| message.to_owned()),
            resource_type: event.resource_type,
            resource_id: event.resource_id,
            triggered_at: event.created_at,
        };
        let results = AlertDispatcher::noop()
            .deliver_payload(&channels, &payload)
            .await;
        for (index, result) in results.into_iter().enumerate() {
            let delivered = result.delivered;
            let status_code = result.status.map(i32::from);
            let retry_state = if delivered {
                "delivered".to_owned()
            } else {
                "retry_pending".to_owned()
            };
            let _attempt = alerts
                .record_delivery_attempt(RecordAlertDeliveryAttempt {
                    event_id: event.id.clone(),
                    rule_id: event.rule_id.clone(),
                    provider: result.provider,
                    target: result.target,
                    delivered,
                    status_code,
                    error: result.error,
                    attempt: i32::try_from(index.saturating_add(1)).unwrap_or(i32::MAX),
                    retry_state,
                    next_retry_at: if delivered {
                        None
                    } else {
                        Some(retry_after_seconds(60))
                    },
                })
                .await?;
        }
    }
    Ok(())
}

fn notification_center_from_audit(audit: &AuditLogRepository) -> NotificationCenter {
    let db = audit.db();
    NotificationCenter::new(
        NotificationChannelRepository::new(db.clone()),
        NotificationPolicyRepository::new(db.clone()),
        NotificationMessageRepository::new(db.clone()),
        NotificationDeliveryAttemptRepository::new(db.clone()),
        NotificationTemplateRepository::new(db.clone()),
        JobRepository::new(db),
    )
}

/// Build the canonical governance event payload shared by instance logs and audit rows.
#[must_use]
/// Script governance payload.
pub fn script_governance_payload(failure_class: &str, message: &str) -> serde_json::Value {
    serde_json::json!({
        "event": GOVERNANCE_EVENT,
        "failure_class": failure_class,
        "message": message,
    })
}

fn severity_from_str(value: &str) -> Severity {
    match value {
        "critical" => Severity::Critical,
        "warning" => Severity::Warning,
        _ => Severity::Info,
    }
}

fn retry_after_seconds(seconds: i64) -> String {
    time::OffsetDateTime::now_utc()
        .saturating_add(time::Duration::seconds(seconds.max(1)))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
