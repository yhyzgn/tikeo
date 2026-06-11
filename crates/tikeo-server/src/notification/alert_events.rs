//! Alert event materialization into the generic Notification Center.

use tikeo_storage::{
    AlertEventSummary, AlertRuleSummary, CreateNotificationChannel, CreateNotificationMessage,
    CreateNotificationPolicy, NotificationChannelFilters, NotificationChannelRepository,
    NotificationPolicyFilters, NotificationPolicyRepository, NotificationPolicySummary,
    PluginAlertChannelTypeSummary, RecordNotificationDeliveryAttempt,
};

use super::{
    NotificationCenter, NotificationEmitSummary, apply_message_template, dedupe_window_elapsed,
    extract_channel_refs, filter_matches, load_policy_template,
};
use crate::alert::{
    NotificationChannel, notification_channel_identity, notification_channels_from_json,
};

const ALERT_LEGACY_MIGRATION_MARKER: &str = "alert_rules.channels_json";

/// Summary of a legacy alert-rule channel migration pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertRuleNotificationBackfillSummary {
    /// Enabled and disabled rules inspected.
    pub rules_seen: u64,
    /// Rules that had no parseable legacy outbound channels.
    pub rules_without_channels: u64,
    /// New Notification Center channels inserted.
    pub channels_created: u64,
    /// New `owner_type=alert_rule` policies inserted.
    pub policies_created: u64,
    /// Rules that already had a previous legacy migration policy.
    pub already_backfilled: u64,
}

impl NotificationCenter {
    /// Materialize Notification Center messages and delivery attempts for one alert event.
    ///
    /// Existing alert routes and legacy alert delivery attempts remain compatible; this method
    /// only adds the generic notification ledger path for policies with `event_family="alert"`.
    ///
    /// # Errors
    ///
    /// Returns storage errors from repository operations.
    pub async fn emit_alert_event(
        &self,
        event: &AlertEventSummary,
    ) -> Result<NotificationEmitSummary, tikeo_storage::DbErr> {
        let policies = self
            .policies
            .list_policies(NotificationPolicyFilters {
                event_family: Some("alert".to_owned()),
                enabled: Some(true),
                ..Default::default()
            })
            .await?;
        let channels = self
            .channels
            .list_channels(NotificationChannelFilters::default())
            .await?;
        let mut summary = NotificationEmitSummary::default();
        let event_type = alert_event_type(event);
        for policy in policies
            .into_iter()
            .filter(|policy| policy_matches_alert_event(policy, event, event_type))
        {
            summary.matched_policies += 1;
            let severity = if policy.severity.trim().is_empty() {
                event.severity.clone()
            } else {
                policy.severity.clone()
            };
            let mut subject = format!(
                "Tikeo alert {}: {}",
                event.rule_name,
                alert_status_label(&event.status)
            );
            let mut body = event.message.clone().unwrap_or_else(|| {
                format!(
                    "Alert rule {} emitted {} for {}/{}",
                    event.rule_name, event_type, event.resource_type, event.resource_id
                )
            });
            let dedupe_key = format!("{}:{}:{}", policy.id, event.id, event_type);
            let mut payload = serde_json::json!({
                "eventType": event_type,
                "legacyEventType": event.event_type,
                "alertEventId": event.id,
                "alertRuleId": event.rule_id,
                "alertRuleName": event.rule_name,
                "status": event.status,
                "severity": severity,
                "resourceType": event.resource_type,
                "resourceId": event.resource_id,
                "failureClass": event.failure_class,
                "message": event.message,
                "policyId": policy.id,
                "dedupeKey": dedupe_key,
                "createdAt": event.created_at,
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
                        source_type: "alert_event".to_owned(),
                        source_id: event.id.clone(),
                        policy_id: policy.id.clone(),
                        event_type: event_type.to_owned(),
                        resource_type: event.resource_type.clone(),
                        resource_id: event.resource_id.clone(),
                        severity,
                        subject,
                        body,
                        payload_json: payload.to_string(),
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

/// Ensure one migration policy exists for a legacy alert rule `channels_json` definition.
///
/// This is intentionally additive: explicit Notification Center alert policies remain untouched,
/// while old alert routes keep reading/writing `channels_json` until a documented breaking release.
///
/// # Errors
///
/// Returns storage errors from notification channel or policy repository operations.
pub async fn ensure_alert_rule_notification_policy_from_channels(
    rule: &AlertRuleSummary,
    channels: &NotificationChannelRepository,
    policies: &NotificationPolicyRepository,
    plugins: &[PluginAlertChannelTypeSummary],
) -> Result<Option<NotificationPolicySummary>, tikeo_storage::DbErr> {
    let existing = policies
        .list_policies(NotificationPolicyFilters {
            owner_type: Some("alert_rule".to_owned()),
            owner_id: Some(rule.id.clone()),
            event_family: Some("alert".to_owned()),
            ..Default::default()
        })
        .await?
        .into_iter()
        .find(is_legacy_alert_migration_policy);
    if existing.is_some() {
        return Ok(existing);
    }
    let legacy_channels = notification_channels_from_json(&rule.channels_json, plugins);
    if legacy_channels.is_empty() {
        return Ok(None);
    }
    let mut channel_refs = Vec::new();
    for (index, channel) in legacy_channels.iter().enumerate() {
        let legacy = legacy_channel_config(channel);
        let identity = notification_channel_identity(channel);
        let created = channels
            .create_channel(CreateNotificationChannel {
                scope_type: "global".to_owned(),
                namespace: None,
                app: None,
                worker_pool: None,
                name: format!(
                    "Migrated alert {} {} {}",
                    rule.name,
                    identity.provider,
                    index.saturating_add(1)
                ),
                provider: identity.provider,
                enabled: rule.enabled,
                config_json: legacy.config_json,
                secret_refs_json: legacy.secret_refs_json,
                safety_policy_json: None,
            })
            .await?;
        channel_refs.push(serde_json::json!({
            "channelId": created.id,
            "source": ALERT_LEGACY_MIGRATION_MARKER,
            "legacyTarget": created.target_redacted,
        }));
    }
    let policy = policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "alert_rule".to_owned(),
            owner_id: Some(rule.id.clone()),
            name: format!("Migrated alert delivery: {}", rule.name),
            event_family: "alert".to_owned(),
            event_filter_json: serde_json::json!({
                "eventTypes": ["alert.firing", "alert.recovered"],
                "statuses": ["firing", "recovered"],
                "migratedFrom": ALERT_LEGACY_MIGRATION_MARKER,
            })
            .to_string(),
            channel_refs_json: serde_json::Value::Array(channel_refs).to_string(),
            template_ref: None,
            severity: rule.severity.clone(),
            enabled: rule.enabled,
            dedupe_seconds: rule.dedupe_seconds,
        })
        .await?;
    Ok(Some(policy))
}

/// Backfill Notification Center policies for all existing alert rules that still use
/// `alert_rules.channels_json`.
///
/// # Errors
///
/// Returns storage errors from alert rule, notification channel, or policy repository operations.
pub async fn backfill_alert_rule_notification_policies(
    alerts: &tikeo_storage::AlertRepository,
    channels: &NotificationChannelRepository,
    policies: &NotificationPolicyRepository,
    plugins: &[PluginAlertChannelTypeSummary],
) -> Result<AlertRuleNotificationBackfillSummary, tikeo_storage::DbErr> {
    let mut summary = AlertRuleNotificationBackfillSummary::default();
    for rule in alerts.list_rules().await? {
        summary.rules_seen = summary.rules_seen.saturating_add(1);
        let before_policies = legacy_migration_policies(policies, &rule).await?;
        if !before_policies.is_empty() {
            summary.already_backfilled = summary.already_backfilled.saturating_add(1);
            continue;
        }
        let legacy_channels = notification_channels_from_json(&rule.channels_json, plugins);
        if legacy_channels.is_empty() {
            summary.rules_without_channels = summary.rules_without_channels.saturating_add(1);
            continue;
        }
        if ensure_alert_rule_notification_policy_from_channels(&rule, channels, policies, plugins)
            .await?
            .is_some()
        {
            summary.policies_created = summary.policies_created.saturating_add(1);
            summary.channels_created = summary
                .channels_created
                .saturating_add(u64::try_from(legacy_channels.len()).unwrap_or(u64::MAX));
        }
    }
    Ok(summary)
}

fn policy_matches_alert_event(
    policy: &tikeo_storage::NotificationPolicySummary,
    event: &AlertEventSummary,
    event_type: &str,
) -> bool {
    match policy.owner_type.as_str() {
        "alert_rule" if policy.owner_id.as_deref() != Some(event.rule_id.as_str()) => {
            return false;
        }
        "global" | "alert_rule" => {}
        _ => return false,
    }
    if !filter_matches(&policy.event_filter_json, &event.status, event_type) {
        return false;
    }
    alert_severity_matches(&policy.event_filter_json, &event.severity)
}

fn alert_event_type(event: &AlertEventSummary) -> &'static str {
    match event.status.as_str() {
        "recovered" => "alert.recovered",
        "suppressed" => "alert.suppressed",
        "silenced" => "alert.silenced",
        _ => "alert.firing",
    }
}

fn alert_status_label(status: &str) -> &'static str {
    match status {
        "recovered" => "recovered",
        "suppressed" => "suppressed",
        "silenced" => "silenced",
        _ => "firing",
    }
}

fn alert_severity_matches(raw: &str, severity: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    value
        .get("severity")
        .or_else(|| value.get("severities"))
        .and_then(serde_json::Value::as_array)
        .is_none_or(|items| items.iter().any(|item| item.as_str() == Some(severity)))
}

fn is_legacy_alert_migration_policy(policy: &NotificationPolicySummary) -> bool {
    serde_json::from_str::<serde_json::Value>(&policy.event_filter_json)
        .ok()
        .and_then(|value| {
            value
                .get("migratedFrom")
                .or_else(|| value.get("migrated_from"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        == Some(ALERT_LEGACY_MIGRATION_MARKER)
}

async fn legacy_migration_policies(
    policies: &NotificationPolicyRepository,
    rule: &AlertRuleSummary,
) -> Result<Vec<NotificationPolicySummary>, tikeo_storage::DbErr> {
    policies
        .list_policies(NotificationPolicyFilters {
            owner_type: Some("alert_rule".to_owned()),
            owner_id: Some(rule.id.clone()),
            event_family: Some("alert".to_owned()),
            ..Default::default()
        })
        .await
        .map(|policies| {
            policies
                .into_iter()
                .filter(is_legacy_alert_migration_policy)
                .collect()
        })
}

#[derive(Debug, Clone)]
struct LegacyChannelConfig {
    config_json: String,
    secret_refs_json: String,
}

fn legacy_channel_config(channel: &NotificationChannel) -> LegacyChannelConfig {
    let config = match channel {
        NotificationChannel::Webhook { url }
        | NotificationChannel::Slack { url }
        | NotificationChannel::DingTalk { url }
        | NotificationChannel::Feishu { url }
        | NotificationChannel::WechatWork { url } => serde_json::json!({ "url": url }),
        NotificationChannel::PagerDuty { url, routing_key } => {
            let mut value = serde_json::json!({ "routingKey": routing_key });
            if let (Some(url), Some(object)) = (url, value.as_object_mut()) {
                object.insert("url".to_owned(), serde_json::Value::String(url.clone()));
            }
            value
        }
        NotificationChannel::PluginWebhook { url, template, .. } => {
            serde_json::json!({ "url": url, "template": template })
        }
        NotificationChannel::Email {
            recipients,
            smtp_url,
            smtp_url_secret_ref,
            from,
            username,
            password_secret_ref,
        } => serde_json::json!({
            "recipients": recipients,
            "smtpUrl": smtp_url,
            "smtpUrlSecretRef": smtp_url_secret_ref,
            "from": from,
            "username": username,
            "passwordSecretRef": password_secret_ref,
        }),
    };
    LegacyChannelConfig {
        config_json: config.to_string(),
        secret_refs_json: "{}".to_owned(),
    }
}
