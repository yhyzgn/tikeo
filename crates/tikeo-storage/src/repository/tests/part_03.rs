use crate::repository::{
    CreateNotificationChannel, CreateNotificationPolicy, CreateNotificationTemplate,
    NotificationChannelFilters, NotificationChannelRepository, NotificationPolicyRepository,
    NotificationTemplateFilters, NotificationTemplateRepository, UpdateNotificationChannel,
    UpdateNotificationTemplate,
};
use std::collections::{BTreeMap, BTreeSet};

fn notification_channel_example_suffix(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = true;
    for item in value.chars() {
        if item.is_ascii_uppercase() {
            if !previous_was_separator {
                normalized.push('_');
            }
            normalized.push(item);
            previous_was_separator = false;
        } else if item.is_ascii_alphanumeric() {
            normalized.push(item.to_ascii_uppercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }
    let trimmed = normalized.trim_matches('_').to_owned();
    if trimmed.is_empty() {
        "CUSTOM".to_owned()
    } else {
        trimmed
    }
}

fn notification_secret_ref_values(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(item) => vec![item.clone()],
        serde_json::Value::Array(items) => items
            .iter()
            .flat_map(notification_secret_ref_values)
            .collect(),
        serde_json::Value::Object(items) => items
            .values()
            .flat_map(notification_secret_ref_values)
            .collect(),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            Vec::new()
        }
    }
}

#[tokio::test]
async fn notification_channel_examples_are_seeded_as_normal_disabled_channels() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db);

    let listed = channels
        .list_channels(NotificationChannelFilters::default())
        .await
        .unwrap_or_else(|error| panic!("channels should list: {error}"));

    for (provider, message_type) in [
        ("webhook", "json"),
        ("slack", "text"),
        ("slack", "blockKit"),
        ("slack", "attachments"),
        ("dingtalk", "text"),
        ("dingtalk", "markdown"),
        ("dingtalk", "link"),
        ("dingtalk", "actionCard"),
        ("dingtalk", "feedCard"),
        ("feishu", "text"),
        ("feishu", "post"),
        ("feishu", "image"),
        ("feishu", "share_chat"),
        ("feishu", "interactive"),
        ("wechat_work", "text"),
        ("wechat_work", "markdown"),
        ("wechat_work", "markdown_v2"),
        ("wechat_work", "image"),
        ("wechat_work", "news"),
        ("wechat_work", "file"),
        ("wechat_work", "voice"),
        ("wechat_work", "template_card"),
        ("pagerduty", "trigger"),
        ("pagerduty", "acknowledge"),
        ("pagerduty", "resolve"),
        ("email", "plain"),
        ("email", "html"),
    ] {
        let matches = listed
            .iter()
            .filter(|item| item.provider == provider && item.name.contains(message_type))
            .collect::<Vec<_>>();
        assert!(
            (1..=2).contains(&matches.len()),
            "seeded channel examples should include 1-2 normal rows for {provider}/{message_type}, got {}",
            matches.len()
        );
        let channel = listed
            .iter()
            .find(|item| item.provider == provider && item.name.contains(message_type))
            .unwrap_or_else(|| panic!("seeded channel example should exist for {provider}/{message_type}"));
        assert!(!channel.enabled, "seeded example channels must be disabled until configured");
        assert_eq!(channel.scope_type, "global");
        assert!(channel.config_json.contains(message_type));
        assert!(channel.secret_configured, "{provider}/{message_type} should carry channel-private credentials");
        assert!(channel.target_configured, "{provider}/{message_type} should show a configured redacted target");
        let secret_refs: serde_json::Value = serde_json::from_str(&channel.secret_refs_json)
            .unwrap_or_else(|error| panic!("{provider}/{message_type} secretRefs should parse: {error}"));
        let secret_ref_values = notification_secret_ref_values(&secret_refs);
        assert!(
            !secret_ref_values.is_empty(),
            "{provider}/{message_type} should include concrete private credential values"
        );
        let message_type_suffix = notification_channel_example_suffix(message_type);
        assert!(
            !secret_ref_values.iter().any(|value| value.starts_with("env:TIKEO_NOTIFICATION_CHANNEL_")),
            "{provider}/{message_type} seed secretRefs should demonstrate direct private values"
        );
        match provider {
            "slack" | "dingtalk" | "feishu" | "wechat_work" | "webhook" => assert!(
                secret_refs["url"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("https://")),
                "{provider}/{message_type} should include a direct webhook URL"
            ),
            "pagerduty" => assert!(
                secret_refs["routingKey"]
                    .as_str()
                    .is_some_and(|value| value.contains("PAGERDUTY") && value.contains(&message_type_suffix)),
                "{provider}/{message_type} should include a direct routing key placeholder"
            ),
            "email" => {
                assert_eq!(
                    secret_refs["smtpUrl"].as_str(),
                    Some("smtp+starttls://smtp.example.com:587")
                );
                assert!(
                    secret_refs["password"]
                        .as_str()
                        .is_some_and(|value| value.contains("SMTP") && value.contains(&message_type_suffix)),
                    "email/{message_type} should include a direct SMTP password placeholder"
                );
            }
            _ => {}
        }
        if matches!(provider, "dingtalk" | "feishu") {
            assert!(
                secret_refs["signingKey"]
                    .as_str()
                    .is_some_and(|value| value.contains("SEC_") && value.contains(&message_type_suffix)),
                "{provider}/{message_type} should include a direct signing secret placeholder"
            );
        }
        for global_ref in [
            "env:TIKEO_NOTIFICATION_WEBHOOK_URL",
            "env:TIKEO_NOTIFICATION_AUTHORIZATION",
            "env:SLACK_WEBHOOK_URL",
            "env:DINGTALK_WEBHOOK_URL",
            "env:DINGTALK_SIGNING_SECRET",
            "env:FEISHU_WEBHOOK_URL",
            "env:FEISHU_BOT_SECRET",
            "env:WECOM_WEBHOOK_URL",
            "env:PAGERDUTY_ROUTING_KEY",
            "env:TIKEO_SMTP_URL",
            "env:TIKEO_SMTP_PASSWORD",
        ] {
            assert!(
                !secret_ref_values.iter().any(|value| value == global_ref),
                "{provider}/{message_type} should not reuse global secret ref {global_ref}"
            );
        }
        assert!(!channel.config_json.contains("hooks.slack.com/services/"));
        assert!(!channel.config_json.contains("top-secret"));
        assert!(!channel.config_json.contains("xoxb-"));
    }

    let mut provider_targets = BTreeMap::<String, BTreeSet<String>>::new();
    for channel in listed {
        let secret_refs: serde_json::Value = serde_json::from_str(&channel.secret_refs_json)
            .unwrap_or_else(|error| panic!("{} secretRefs should parse: {error}", channel.name));
        let target_ref = secret_refs
            .get("url")
            .or_else(|| secret_refs.get("routingKey"))
            .or_else(|| secret_refs.get("smtpUrl"))
            .and_then(serde_json::Value::as_str);
        if let Some(target_ref) = target_ref {
            provider_targets
                .entry(channel.provider.clone())
                .or_default()
                .insert(target_ref.to_owned());
        }
    }
    for (provider, expected_count) in [
        ("slack", 3),
        ("dingtalk", 5),
        ("feishu", 5),
        ("wechat_work", 8),
        ("pagerduty", 3),
        ("email", 1),
    ] {
        assert_eq!(
            provider_targets.get(provider).map(BTreeSet::len),
            Some(expected_count),
            "{provider} example rows should not share one global target credential"
        );
    }
}

#[tokio::test]
async fn notification_channels_are_reusable_redacted_and_policy_referenced() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db);

    let created = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "Billing Feishu".to_owned(),
            provider: "feishu".to_owned(),
            enabled: true,
            config_json: serde_json::json!({
                "url": "https://open.feishu.cn/open-apis/bot/v2/hook/super-secret-token",
                "mentionAll": true
            })
            .to_string(),
            secret_refs_json: serde_json::json!({"signingKey":"env:FEISHU_BOT_SECRET"}).to_string(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("notification channel should create: {error}"));

    assert_eq!(created.provider, "feishu");
    assert_eq!(created.target_redacted, "https://open.feishu.cn/...");
    assert!(!created.config_json.contains("super-secret-token"));
    assert!(created.secret_configured);

    let policy = policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some("job-billing-nightly".to_owned()),
            name: "Billing failure notifications".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["failed","retry_exhausted"]}).to_string(),
            channel_refs_json: serde_json::json!([{"channelId": created.id}]).to_string(),
            template_ref: None,
            severity: "critical".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("notification policy should create: {error}"));

    assert_eq!(policy.owner_type, "job");
    assert!(policy.channel_refs_json.contains("notification-channel"));

    let listed = channels
        .list_channels(NotificationChannelFilters::default())
        .await
        .unwrap_or_else(|error| panic!("channels should list: {error}"));
    let listed_created = listed
        .iter()
        .find(|item| item.id == created.id)
        .unwrap_or_else(|| panic!("created channel should be listed alongside seeded examples"));
    assert_eq!(listed_created.target_redacted, "https://open.feishu.cn/...");
    assert!(!listed_created.config_json.contains("super-secret-token"));

    let delete_result = channels
        .delete_channel(&listed_created.id)
        .await
        .unwrap_or_else(|error| panic!("delete should return a governed result: {error}"));
    assert!(!delete_result.deleted);
    assert_eq!(delete_result.referenced_by_policies, 1);
}

#[tokio::test]
async fn notification_channel_provider_update_recomputes_redacted_target_without_config_patch() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db);
    let created = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "PagerDuty".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"routingKey":"pd-secret-routing-key"}).to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    assert_eq!(created.target_redacted, "unconfigured");

    let updated = channels
        .update_channel(
            &created.id,
            UpdateNotificationChannel {
                provider: Some("pagerduty".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap_or_else(|error| panic!("channel should update: {error}"))
        .unwrap_or_else(|| panic!("channel should exist"));

    assert_eq!(updated.provider, "pagerduty");
    assert_eq!(updated.target_redacted, "pagerduty:***redacted***");
}

#[tokio::test]
async fn notification_channel_redacts_camel_case_url_and_smtp_keys() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db);

    let webhook = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "Camel webhook".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({
                "webhookUrl": "https://hooks.example.com/services/camel-secret-token",
                "headers": {
                    "Authorization": "Bearer raw-secret-token",
                    "X-API-Key": "raw-api-key-secret"
                }
            })
            .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("webhook channel should create: {error}"));

    assert_eq!(webhook.target_redacted, "https://hooks.example.com/...");
    assert!(!webhook.config_json.contains("camel-secret-token"));
    assert!(!webhook.config_json.contains("raw-secret-token"));
    assert!(!webhook.config_json.contains("raw-api-key-secret"));

    let email = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "Camel email".to_owned(),
            provider: "email".to_owned(),
            enabled: true,
            config_json: serde_json::json!({
                "smtpUrl": "smtps://smtp-user:smtp-secret-password@smtp.example.com:465",
                "to": ["ops@example.com"]
            })
            .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("email channel should create: {error}"));

    assert_eq!(email.target_redacted, "ops@example.com");
    assert!(!email.config_json.contains("smtp-secret-password"));
    assert!(email.secret_configured);
}


#[tokio::test]
async fn notification_messages_and_delivery_attempts_are_generic_timeline() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = crate::repository::NotificationMessageRepository::new(db.clone());
    let attempts = crate::repository::NotificationDeliveryAttemptRepository::new(db);

    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "Ops webhook".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"url":"https://hooks.example.com/services/top-secret-token"}).to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    let policy = policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some("job-billing".to_owned()),
            name: "Billing terminal failures".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["failed"]}).to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: "critical".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));

    let message = messages
        .create_message(crate::repository::CreateNotificationMessage {
            source_type: "job_instance".to_owned(),
            source_id: "inst-billing-1".to_owned(),
            policy_id: policy.id.clone(),
            event_type: "job_instance.failed".to_owned(),
            resource_type: "job".to_owned(),
            resource_id: "job-billing".to_owned(),
            severity: "critical".to_owned(),
            subject: "Billing job failed".to_owned(),
            body: "Instance inst-billing-1 failed".to_owned(),
            payload_json: serde_json::json!({"status":"failed"}).to_string(),
            dedupe_key: "job-billing:failed".to_owned(),
            trace_id: Some("trace-notify-1".to_owned()),
            status: "pending".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("message should create: {error}"));
    assert_eq!(message.event_type, "job_instance.failed");
    assert_eq!(message.status, "pending");

    let attempt = attempts
        .record_attempt(crate::repository::RecordNotificationDeliveryAttempt {
            message_id: message.id.clone(),
            policy_id: policy.id.clone(),
            channel_id: channel.id,
            provider: "webhook".to_owned(),
            target_redacted: "https://hooks.example.com/...".to_owned(),
            attempt: 1,
            delivered: false,
            status_code: Some(500),
            error: Some("provider unavailable".to_owned()),
            retry_state: "retry_pending".to_owned(),
            next_retry_at: Some("2030-01-01T00:00:00Z".to_owned()),
        })
        .await
        .unwrap_or_else(|error| panic!("attempt should record: {error}"));
    assert_eq!(attempt.target_redacted, "https://hooks.example.com/...");
    assert_eq!(attempt.retry_state, "retry_pending");

    let timeline = messages
        .list_messages(crate::repository::NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some("inst-billing-1".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].policy_id, policy.id);

    let retry_queue = attempts
        .list_attempts(crate::repository::NotificationDeliveryAttemptFilters {
            retry_state: Some("retry_pending".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert_eq!(retry_queue.len(), 1);
    assert_eq!(retry_queue[0].message_id, message.id);
}

#[tokio::test]
async fn notification_center_menu_permission_is_seeded_for_builtin_roles() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let rbac = crate::repository::RbacRepository::new(db);

    let owner_keys = rbac
        .menu_keys_for_roles(&["owner".to_owned()])
        .await
        .unwrap_or_else(|error| panic!("owner menu keys should list: {error}"));
    let operator_keys = rbac
        .menu_keys_for_roles(&["operator".to_owned()])
        .await
        .unwrap_or_else(|error| panic!("operator menu keys should list: {error}"));
    let viewer_keys = rbac
        .menu_keys_for_roles(&["viewer".to_owned()])
        .await
        .unwrap_or_else(|error| panic!("viewer menu keys should list: {error}"));

    assert!(owner_keys.iter().any(|key| key == "/notifications"));
    assert!(operator_keys.iter().any(|key| key == "/notifications"));
    assert!(viewer_keys.iter().any(|key| key == "/notifications"));
}


#[tokio::test]
async fn notification_templates_are_persisted_filtered_and_mutable() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let templates = NotificationTemplateRepository::new(db);

    let created = templates
        .create_template(CreateNotificationTemplate {
            template_key: "slack.failure".to_owned(),
            name: "Slack failure".to_owned(),
            description: Some("Failure Block Kit template".to_owned()),
            provider: "slack".to_owned(),
            message_type: "blockKit".to_owned(),
            enabled: true,
            body_json: serde_json::json!({
                "messageType":"blockKit",
                "text":"{{subject}}",
                "blocks":[{"type":"section","text":{"type":"mrkdwn","text":"{{body}}"}}]
            })
            .to_string(),
            variables_json: serde_json::json!(["{{subject}}", "{{body}}"]).to_string(),
        })
        .await
        .unwrap_or_else(|error| panic!("template should create: {error}"));

    assert_eq!(created.template_key, "slack.failure");
    assert_eq!(created.provider, "slack");
    assert!(created.body_json.contains("blockKit"));

    let by_key = templates
        .get_template("slack.failure")
        .await
        .unwrap_or_else(|error| panic!("template should load by key: {error}"))
        .unwrap_or_else(|| panic!("template key should resolve"));
    assert_eq!(by_key.id, created.id);

    let filtered = templates
        .list_templates(NotificationTemplateFilters {
            provider: Some("slack".to_owned()),
            message_type: Some("blockKit".to_owned()),
            enabled: Some(true),
        })
        .await
        .unwrap_or_else(|error| panic!("templates should list: {error}"));
    assert_eq!(filtered.len(), 1);

    let updated = templates
        .update_template(
            &created.id,
            UpdateNotificationTemplate {
                name: Some("Slack failure v2".to_owned()),
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap_or_else(|error| panic!("template should update: {error}"))
        .unwrap_or_else(|| panic!("template should exist"));
    assert_eq!(updated.name, "Slack failure v2");
    assert!(!updated.enabled);

    assert!(templates
        .delete_template(&created.id)
        .await
        .unwrap_or_else(|error| panic!("template should delete: {error}")));
}

#[tokio::test]
async fn notification_channel_updates_preserve_unsubmitted_config_and_secret_refs() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db);

    let created = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "global".to_owned(),
            namespace: None,
            app: None,
            worker_pool: None,
            name: "Ops webhook".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({
                "messageType": "json",
                "headers": {"X-Trace": "trace-header"},
                "template": {"body": {"text": "{{subject}}"}}
            })
            .to_string(),
            secret_refs_json: serde_json::json!({
                "url": "env:TIKEO_NOTIFICATION_WEBHOOK_URL",
                "authorization": "env:TIKEO_NOTIFICATION_AUTH"
            })
            .to_string(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));

    assert!(!created.config_json.contains("trace-header"));
    assert!(!created.secret_refs_json.is_empty());

    let renamed = channels
        .update_channel(
            &created.id,
            UpdateNotificationChannel {
                name: Some("Ops webhook renamed".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap_or_else(|error| panic!("metadata-only update should succeed: {error}"))
        .unwrap_or_else(|| panic!("channel should exist"));
    assert_eq!(renamed.name, "Ops webhook renamed");
    assert!(renamed.secret_configured);

    let delivery_after_rename = channels
        .get_channel_delivery_config(&created.id)
        .await
        .unwrap_or_else(|error| panic!("delivery config should load: {error}"))
        .unwrap_or_else(|| panic!("delivery config should exist"));
    assert!(delivery_after_rename.config_json.contains("trace-header"));
    assert!(
        delivery_after_rename
            .secret_refs_json
            .contains("TIKEO_NOTIFICATION_AUTH")
    );

    let config_replaced = channels
        .update_channel(
            &created.id,
            UpdateNotificationChannel {
                config_json: Some(
                    serde_json::json!({"messageType":"json","template":{"body":{"text":"{{body}}"}}})
                        .to_string(),
                ),
                ..Default::default()
            },
        )
        .await
        .unwrap_or_else(|error| panic!("config update should succeed: {error}"))
        .unwrap_or_else(|| panic!("channel should exist"));
    assert!(config_replaced.secret_configured);
    let delivery_after_config = channels
        .get_channel_delivery_config(&created.id)
        .await
        .unwrap_or_else(|error| panic!("delivery config should load: {error}"))
        .unwrap_or_else(|| panic!("delivery config should exist"));
    assert!(
        delivery_after_config
            .secret_refs_json
            .contains("TIKEO_NOTIFICATION_AUTH")
    );

    let secret_replaced = channels
        .update_channel(
            &created.id,
            UpdateNotificationChannel {
                secret_refs_json: Some(
                    serde_json::json!({"url":"env:TIKEO_NEW_WEBHOOK_URL"}).to_string(),
                ),
                ..Default::default()
            },
        )
        .await
        .unwrap_or_else(|error| panic!("secret update should succeed: {error}"))
        .unwrap_or_else(|| panic!("channel should exist"));
    assert!(secret_replaced.config_json.contains("{{body}}"));
    let delivery_after_secret = channels
        .get_channel_delivery_config(&created.id)
        .await
        .unwrap_or_else(|error| panic!("delivery config should load: {error}"))
        .unwrap_or_else(|| panic!("delivery config should exist"));
    assert!(delivery_after_secret.config_json.contains("{{body}}"));
    assert!(
        delivery_after_secret
            .secret_refs_json
            .contains("TIKEO_NEW_WEBHOOK_URL")
    );
}
