//! Script execution governance audit materialization helpers.

use crate::alert::{AlertDispatcher, AlertPayload, NotificationChannel, Severity};
use tikee_storage::{AlertRepository, AuditLogRepository, CreateAuditLog};

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
) -> Result<(), tikee_storage::DbErr> {
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
    for event in events.into_iter().filter(|event| event.status == "firing") {
        let Some(rule) = alerts.get_rule(&event.rule_id).await? else {
            continue;
        };
        let Ok(channels) = serde_json::from_str::<Vec<NotificationChannel>>(&rule.channels_json)
        else {
            continue;
        };
        let payload = AlertPayload {
            rule_name: event.rule_name,
            severity: severity_from_str(&event.severity),
            message: event.message.unwrap_or_else(|| message.to_owned()),
            resource_type: event.resource_type,
            resource_id: event.resource_id,
            triggered_at: event.created_at,
        };
        let _results = AlertDispatcher::noop()
            .deliver_payload(&channels, &payload)
            .await;
    }
    Ok(())
}

/// Build the canonical governance event payload shared by instance logs and audit rows.
#[must_use]
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
