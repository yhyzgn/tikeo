use crate::repository::{
    CreateNotificationChannel, CreateNotificationPolicy, NotificationChannelRepository,
    NotificationChannelFilters, NotificationPolicyRepository, UpdateNotificationChannel,
};

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
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].target_redacted, "https://open.feishu.cn/...");
    assert!(!listed[0].config_json.contains("super-secret-token"));

    let delete_result = channels
        .delete_channel(&listed[0].id)
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
