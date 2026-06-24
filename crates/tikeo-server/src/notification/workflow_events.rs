//! Workflow notification node materialization into the generic Notification Center.

use tikeo_storage::{
    CreateNotificationMessage, CreateNotificationPolicy, MaterializeWorkflowNodeResult,
    NotificationChannelFilters, NotificationPolicyFilters, NotificationPolicySummary,
    RecordNotificationDeliveryAttempt, UpdateNotificationPolicy, WorkflowNodeSpec,
    WorkflowRepository, WorkflowSummary,
};
use tracing::warn;

use super::{
    NotificationCenter, NotificationEmitSummary, apply_message_template, dedupe_window_elapsed,
    extract_channel_refs, filter_matches, load_policy_template,
};

const WORKFLOW_NOTIFICATION_EVENT_TYPE: &str = "workflow_node.notification_requested";
const WORKFLOW_NOTIFICATION_FILTER_STATUS: &str = "notification_requested";
const WORKFLOW_INLINE_POLICY_MARKER: &str = "workflow_node.config";

/// Materialize a just-completed workflow `notification` node into Notification Center messages.
///
/// # Errors
///
/// Returns storage errors from workflow/notification repositories.
pub async fn emit_workflow_notification_node_requested(
    center: &NotificationCenter,
    workflows: &WorkflowRepository,
    materialized: &MaterializeWorkflowNodeResult,
) -> Result<NotificationEmitSummary, tikeo_storage::DbErr> {
    let Some(workflow) = workflows
        .get_workflow(&materialized.instance.workflow_id)
        .await?
    else {
        return Ok(NotificationEmitSummary::default());
    };
    let Some(node_spec) = workflow
        .definition
        .nodes
        .iter()
        .find(|node| node.key == materialized.node.node_key)
    else {
        return Ok(NotificationEmitSummary::default());
    };
    if node_spec.kind.as_deref().unwrap_or("job") != "notification" {
        return Ok(NotificationEmitSummary::default());
    }
    center
        .emit_workflow_notification_node(&workflow, node_spec, materialized)
        .await
}

/// Best-effort workflow notification-node materializer for runtime paths where delivery must not
/// unexpectedly block or fail workflow progression.
pub async fn emit_workflow_notification_node_requested_best_effort(
    center: &NotificationCenter,
    workflows: &WorkflowRepository,
    materialized: &MaterializeWorkflowNodeResult,
) {
    if let Err(error) =
        emit_workflow_notification_node_requested(center, workflows, materialized).await
    {
        warn!(
            %error,
            workflow_instance_id = %materialized.instance.id,
            workflow_node_instance_id = %materialized.node.id,
            node_key = %materialized.node.node_key,
            "failed to materialize workflow notification node event"
        );
    }
}

impl NotificationCenter {
    async fn emit_workflow_notification_node(
        &self,
        workflow: &WorkflowSummary,
        node_spec: &WorkflowNodeSpec,
        materialized: &MaterializeWorkflowNodeResult,
    ) -> Result<NotificationEmitSummary, tikeo_storage::DbErr> {
        let inline_policy = self
            .ensure_inline_workflow_node_policy(workflow, node_spec)
            .await?;
        let channels = self
            .channels
            .list_channels(NotificationChannelFilters::default())
            .await?;
        let mut policies = self
            .policies
            .list_policies(NotificationPolicyFilters {
                event_family: Some("workflow".to_owned()),
                enabled: Some(true),
                ..Default::default()
            })
            .await?
            .into_iter()
            .filter(|policy| workflow_policy_matches(policy, workflow, node_spec, materialized))
            .collect::<Vec<_>>();
        if let Some(policy) = inline_policy
            && policy.enabled
            && !policies.iter().any(|item| item.id == policy.id)
            && workflow_policy_matches(&policy, workflow, node_spec, materialized)
        {
            policies.push(policy);
        }

        let mut summary = NotificationEmitSummary::default();
        for policy in policies {
            summary.matched_policies = summary.matched_policies.saturating_add(1);
            let severity = workflow_notification_severity(node_spec, &policy);
            let mut subject = workflow_notification_subject(workflow, node_spec);
            let mut body = workflow_notification_body(workflow, node_spec, materialized);
            let dedupe_key = format!(
                "{}:{}:{}",
                policy.id, materialized.node.id, WORKFLOW_NOTIFICATION_EVENT_TYPE
            );
            let mut payload = workflow_notification_payload(
                workflow,
                node_spec,
                materialized,
                &policy,
                &severity,
                &dedupe_key,
            );
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
                        source_type: "workflow_node_instance".to_owned(),
                        source_id: materialized.node.id.clone(),
                        policy_id: policy.id.clone(),
                        event_type: WORKFLOW_NOTIFICATION_EVENT_TYPE.to_owned(),
                        resource_type: "workflow_node".to_owned(),
                        resource_id: materialized.node.node_key.clone(),
                        severity,
                        subject,
                        body,
                        payload_json: payload.to_string(),
                        dedupe_key,
                        trace_id: None,
                        status: "pending".to_owned(),
                    })
                    .await?;
                summary.messages_created = summary.messages_created.saturating_add(1);
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
                    self.attempts
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
                    summary.delivery_attempts_created =
                        summary.delivery_attempts_created.saturating_add(1);
                }
            }
        }
        self.kick_delivery_if_attempts_created(&summary);
        Ok(summary)
    }

    async fn ensure_inline_workflow_node_policy(
        &self,
        workflow: &WorkflowSummary,
        node_spec: &WorkflowNodeSpec,
    ) -> Result<Option<NotificationPolicySummary>, tikeo_storage::DbErr> {
        let Some(channel_refs) = workflow_notification_channel_refs(node_spec) else {
            return Ok(None);
        };
        self.validate_inline_workflow_node_refs(node_spec, &channel_refs)
            .await?;
        let owner_id = workflow_node_owner_id(workflow, node_spec);
        let event_filter_json = serde_json::json!({
            "eventTypes": [WORKFLOW_NOTIFICATION_EVENT_TYPE],
            "statuses": [WORKFLOW_NOTIFICATION_FILTER_STATUS],
            "workflowIds": [workflow.id],
            "nodeKeys": [node_spec.key],
            "compiledFrom": WORKFLOW_INLINE_POLICY_MARKER,
        })
        .to_string();
        let existing = self
            .policies
            .list_policies(NotificationPolicyFilters {
                owner_type: Some("workflow_node".to_owned()),
                owner_id: Some(owner_id.clone()),
                event_family: Some("workflow".to_owned()),
                ..Default::default()
            })
            .await?
            .into_iter()
            .find(is_inline_workflow_policy);
        if let Some(existing) = existing {
            return self
                .policies
                .update_policy(
                    &existing.id,
                    UpdateNotificationPolicy {
                        name: Some(format!(
                            "Workflow notification node: {} / {}",
                            workflow.name, node_spec.key
                        )),
                        event_filter_json: Some(event_filter_json),
                        channel_refs_json: Some(channel_refs.to_string()),
                        template_ref: Some(workflow_notification_template_ref(node_spec)),
                        severity: Some(
                            workflow_notification_config_string(node_spec, &["severity"])
                                .unwrap_or("info")
                                .to_owned(),
                        ),
                        enabled: Some(
                            workflow_notification_config_bool(node_spec, "enabled").unwrap_or(true),
                        ),
                        dedupe_seconds: Some(workflow_notification_dedupe_seconds(node_spec)),
                        ..Default::default()
                    },
                )
                .await;
        }
        self.policies
            .create_policy(CreateNotificationPolicy {
                owner_type: "workflow_node".to_owned(),
                owner_id: Some(owner_id),
                name: format!(
                    "Workflow notification node: {} / {}",
                    workflow.name, node_spec.key
                ),
                event_family: "workflow".to_owned(),
                event_filter_json,
                channel_refs_json: channel_refs.to_string(),
                template_ref: workflow_notification_template_ref(node_spec),
                severity: workflow_notification_config_string(node_spec, &["severity"])
                    .unwrap_or("info")
                    .to_owned(),
                enabled: workflow_notification_config_bool(node_spec, "enabled").unwrap_or(true),
                dedupe_seconds: workflow_notification_dedupe_seconds(node_spec),
            })
            .await
            .map(Some)
    }

    async fn validate_inline_workflow_node_refs(
        &self,
        node_spec: &WorkflowNodeSpec,
        channel_refs: &serde_json::Value,
    ) -> Result<(), tikeo_storage::DbErr> {
        let channel_ids = extract_channel_refs(&channel_refs.to_string());
        if channel_ids.is_empty() {
            return Err(tikeo_storage::DbErr::Custom(format!(
                "notification node {} requires registered channel refs",
                node_spec.key
            )));
        }
        let channels = self
            .channels
            .list_channels(NotificationChannelFilters::default())
            .await?;
        let mut providers = Vec::new();
        for channel_id in &channel_ids {
            match channels.iter().find(|channel| channel.id == *channel_id) {
                Some(channel) if !channel.enabled => {
                    return Err(tikeo_storage::DbErr::Custom(format!(
                        "notification node {} channel is disabled: {channel_id}",
                        node_spec.key
                    )));
                }
                Some(channel) => providers.push(channel.provider.clone()),
                None => {
                    return Err(tikeo_storage::DbErr::Custom(format!(
                        "notification node {} channel does not exist: {channel_id}",
                        node_spec.key
                    )));
                }
            }
        }
        let Some(template_ref) = workflow_notification_template_ref(node_spec) else {
            return Ok(());
        };
        let Some(template) = self.templates.get_template(&template_ref).await? else {
            return Err(tikeo_storage::DbErr::Custom(format!(
                "notification node {} template does not exist: {template_ref}",
                node_spec.key
            )));
        };
        if !template.enabled {
            return Err(tikeo_storage::DbErr::Custom(format!(
                "notification node {} template is disabled: {template_ref}",
                node_spec.key
            )));
        }
        let mut mismatched = providers
            .into_iter()
            .filter(|provider| provider != &template.provider)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !mismatched.is_empty() {
            mismatched.sort();
            return Err(tikeo_storage::DbErr::Custom(format!(
                "notification node {} template provider {} does not match channel provider(s): {}",
                node_spec.key,
                template.provider,
                mismatched.join(", ")
            )));
        }
        Ok(())
    }
}

fn workflow_policy_matches(
    policy: &NotificationPolicySummary,
    workflow: &WorkflowSummary,
    node_spec: &WorkflowNodeSpec,
    materialized: &MaterializeWorkflowNodeResult,
) -> bool {
    let owner_matches = match policy.owner_type.as_str() {
        "global" => true,
        "workflow" => policy.owner_id.as_deref() == Some(workflow.id.as_str()),
        "workflow_node" => {
            workflow_node_policy_owner_matches(policy, workflow, node_spec, materialized)
        }
        _ => false,
    };
    if !owner_matches {
        return false;
    }
    filter_matches(
        &policy.event_filter_json,
        WORKFLOW_NOTIFICATION_FILTER_STATUS,
        WORKFLOW_NOTIFICATION_EVENT_TYPE,
    ) && workflow_filter_matches(&policy.event_filter_json, workflow, node_spec)
}

fn workflow_node_policy_owner_matches(
    policy: &NotificationPolicySummary,
    workflow: &WorkflowSummary,
    node_spec: &WorkflowNodeSpec,
    materialized: &MaterializeWorkflowNodeResult,
) -> bool {
    policy.owner_id.as_deref().is_some_and(|owner_id| {
        owner_id == workflow_node_owner_id(workflow, node_spec)
            || owner_id == materialized.node.id
            || owner_id == node_spec.key
    })
}

fn workflow_filter_matches(
    raw: &str,
    workflow: &WorkflowSummary,
    node_spec: &WorkflowNodeSpec,
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    json_string_array_matches(&value, &["workflowIds", "workflow_ids"], &workflow.id)
        && json_string_array_matches(&value, &["nodeKeys", "node_keys"], &node_spec.key)
}

fn json_string_array_matches(value: &serde_json::Value, keys: &[&str], candidate: &str) -> bool {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_array))
        .is_none_or(|items| items.iter().any(|item| item.as_str() == Some(candidate)))
}

fn workflow_node_owner_id(workflow: &WorkflowSummary, node_spec: &WorkflowNodeSpec) -> String {
    format!("{}:{}", workflow.id, node_spec.key)
}

fn workflow_notification_channel_refs(node_spec: &WorkflowNodeSpec) -> Option<serde_json::Value> {
    let config = node_spec.config.as_ref()?;
    config
        .get("channelRefs")
        .or_else(|| config.get("channel_refs"))
        .filter(|value| !extract_channel_refs(&value.to_string()).is_empty())
        .cloned()
}

fn workflow_notification_template_ref(node_spec: &WorkflowNodeSpec) -> Option<String> {
    workflow_notification_config_string(node_spec, &["templateRef", "template_ref"])
        .map(ToOwned::to_owned)
}

fn workflow_notification_subject(
    workflow: &WorkflowSummary,
    node_spec: &WorkflowNodeSpec,
) -> String {
    workflow_notification_config_string(node_spec, &["subject", "title"]).map_or_else(
        || {
            format!(
                "Tikeo workflow {} notification: {}",
                workflow.name, node_spec.key
            )
        },
        ToOwned::to_owned,
    )
}

fn workflow_notification_body(
    workflow: &WorkflowSummary,
    node_spec: &WorkflowNodeSpec,
    materialized: &MaterializeWorkflowNodeResult,
) -> String {
    workflow_notification_config_string(node_spec, &["body", "message", "content"]).map_or_else(
        || {
            format!(
                "Workflow {} instance {} requested notification node {}",
                workflow.name, materialized.instance.id, node_spec.key
            )
        },
        ToOwned::to_owned,
    )
}

fn workflow_notification_severity(
    node_spec: &WorkflowNodeSpec,
    policy: &NotificationPolicySummary,
) -> String {
    if policy.severity.trim().is_empty() {
        workflow_notification_config_string(node_spec, &["severity"])
            .unwrap_or("info")
            .to_owned()
    } else {
        policy.severity.clone()
    }
}

fn workflow_notification_payload(
    workflow: &WorkflowSummary,
    node_spec: &WorkflowNodeSpec,
    materialized: &MaterializeWorkflowNodeResult,
    policy: &NotificationPolicySummary,
    severity: &str,
    dedupe_key: &str,
) -> serde_json::Value {
    serde_json::json!({
        "eventType": WORKFLOW_NOTIFICATION_EVENT_TYPE,
        "workflowId": workflow.id,
        "workflowName": workflow.name,
        "workflowInstanceId": materialized.instance.id,
        "workflowNodeInstanceId": materialized.node.id,
        "nodeKey": materialized.node.node_key,
        "nodeName": node_spec.name,
        "nodeKind": "notification",
        "status": materialized.node.status,
        "resourceType": "workflow_node",
        "resourceId": materialized.node.node_key,
        "severity": severity,
        "policyId": policy.id,
        "dedupeKey": dedupe_key,
        "blocking": workflow_notification_config_bool(node_spec, "blocking").unwrap_or(false),
        "failOnDeliveryFailure": workflow_notification_config_bool(node_spec, "failOnDeliveryFailure")
            .or_else(|| workflow_notification_config_bool(node_spec, "fail_on_delivery_failure"))
            .unwrap_or(false),
        "templateRef": workflow_notification_template_ref(node_spec),
        "createdAt": materialized.node.updated_at,
    })
}

fn workflow_notification_config_string<'a>(
    node_spec: &'a WorkflowNodeSpec,
    keys: &[&str],
) -> Option<&'a str> {
    let config = node_spec.config.as_ref()?;
    keys.iter()
        .find_map(|key| config.get(*key).and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn workflow_notification_config_bool(node_spec: &WorkflowNodeSpec, key: &str) -> Option<bool> {
    node_spec.config.as_ref()?.get(key).and_then(|value| {
        value.as_bool().or_else(|| {
            value
                .as_str()
                .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
                    "true" | "yes" | "1" => Some(true),
                    "false" | "no" | "0" => Some(false),
                    _ => None,
                })
        })
    })
}

fn workflow_notification_dedupe_seconds(node_spec: &WorkflowNodeSpec) -> i64 {
    node_spec
        .config
        .as_ref()
        .and_then(|config| {
            config
                .get("dedupeSeconds")
                .or_else(|| config.get("dedupe_seconds"))
        })
        .and_then(|value| {
            value.as_i64().or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<i64>().ok())
            })
        })
        .unwrap_or(0)
        .max(0)
}

fn is_inline_workflow_policy(policy: &NotificationPolicySummary) -> bool {
    serde_json::from_str::<serde_json::Value>(&policy.event_filter_json)
        .ok()
        .and_then(|value| {
            value
                .get("compiledFrom")
                .or_else(|| value.get("compiled_from"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        == Some(WORKFLOW_INLINE_POLICY_MARKER)
}
