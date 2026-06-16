use crate::repository::{
    CreateNotificationChannel, CreateNotificationPolicy, CreateNotificationTemplate,
    NotificationChannelFilters, NotificationChannelRepository, NotificationPolicyRepository,
    NotificationTemplateFilters, NotificationTemplateRepository, UpdateNotificationChannel,
    UpdateNotificationTemplate,
};

#[tokio::test]
async fn notification_channel_examples_are_not_seeded_by_schema_migrations() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let channels = NotificationChannelRepository::new(db);

    let listed = channels
        .list_channels(NotificationChannelFilters::default())
        .await
        .unwrap_or_else(|error| panic!("channels should list: {error}"));

    assert!(
        listed
            .iter()
            .all(|item| !item.id.starts_with("notification-channel-example-")),
        "schema migrations must not seed editable notification channel examples into runtime databases: {listed:?}"
    );
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

#[tokio::test]
async fn worker_dispatch_outbox_repository_claims_and_marks_delivery() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let repository = crate::repository::WorkerDispatchOutboxRepository::new(db);

    let created = repository
        .create(crate::repository::CreateWorkerDispatchOutbox {
            instance_id: "inst-1".to_owned(),
            attempt_id: "attempt-1".to_owned(),
            worker_id: "worker-1".to_owned(),
            logical_instance_id: "default/app/local/worker-1".to_owned(),
            gateway_node_id: "gateway-a".to_owned(),
            gateway_generation: 3,
            assignment_token: "asg-1".to_owned(),
            dispatch_payload: r#"{"instanceId":"inst-1"}"#.to_owned(),
            shard_id: 12,
            owner_node_id: "owner-a".to_owned(),
            owner_epoch: 7,
            owner_fencing_token: "fence-7".to_owned(),
            next_delivery_at: None,
        })
        .await
        .unwrap_or_else(|error| panic!("outbox row should create: {error}"));

    assert_eq!(created.status, "queued");
    assert_eq!(created.gateway_node_id, "gateway-a");
    assert_eq!(created.delivery_attempts, 0);

    let claimed = repository
        .claim_next_for_gateway("gateway-a", 10)
        .await
        .unwrap_or_else(|error| panic!("outbox row should claim: {error}"))
        .unwrap_or_else(|| panic!("queued outbox row should be claimable"));

    assert_eq!(claimed.id, created.id);
    assert_eq!(claimed.status, "delivering");
    assert_eq!(claimed.delivery_attempts, 1);

    let delivered = repository
        .mark_delivered(&claimed.id, 30)
        .await
        .unwrap_or_else(|error| panic!("outbox row should mark delivered: {error}"))
        .unwrap_or_else(|| panic!("delivered outbox row should exist"));

    assert_eq!(delivered.status, "delivered");
    assert!(delivered.visibility_deadline.is_some());
}


#[tokio::test]
async fn worker_dispatch_outbox_repository_requeues_and_completes_rows() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let repository = crate::repository::WorkerDispatchOutboxRepository::new(db);
    let created = repository
        .create(crate::repository::CreateWorkerDispatchOutbox {
            instance_id: "inst-2".to_owned(),
            attempt_id: "attempt-2".to_owned(),
            worker_id: "worker-old".to_owned(),
            logical_instance_id: "default/app/local/worker-2".to_owned(),
            gateway_node_id: "gateway-old".to_owned(),
            gateway_generation: 1,
            assignment_token: "asg-2".to_owned(),
            dispatch_payload: r#"{"instanceId":"inst-2"}"#.to_owned(),
            shard_id: 22,
            owner_node_id: "owner-a".to_owned(),
            owner_epoch: 8,
            owner_fencing_token: "fence-8".to_owned(),
            next_delivery_at: None,
        })
        .await
        .unwrap_or_else(|error| panic!("outbox row should create: {error}"));

    let rerouted = repository
        .reroute(
            &created.id,
            "gateway-new",
            "worker-new",
            2,
        )
        .await
        .unwrap_or_else(|error| panic!("outbox row should reroute: {error}"))
        .unwrap_or_else(|| panic!("rerouted outbox row should exist"));

    assert_eq!(rerouted.status, "queued");
    assert_eq!(rerouted.gateway_node_id, "gateway-new");
    assert_eq!(rerouted.worker_id, "worker-new");
    assert_eq!(rerouted.gateway_generation, 2);

    let claimed = repository
        .claim_next_for_gateway("gateway-new", 10)
        .await
        .unwrap_or_else(|error| panic!("outbox row should claim: {error}"))
        .unwrap_or_else(|| panic!("rerouted outbox should be claimable"));
    let requeued = repository
        .mark_delivery_failed(&claimed.id, "stream missing", 5)
        .await
        .unwrap_or_else(|error| panic!("outbox row should requeue after failure: {error}"))
        .unwrap_or_else(|| panic!("failed outbox row should exist"));
    assert_eq!(requeued.status, "queued");
    assert_eq!(requeued.last_error.as_deref(), Some("stream missing"));

    let completed = repository
        .mark_completed_by_assignment("inst-2", "worker-new", "asg-2")
        .await
        .unwrap_or_else(|error| panic!("outbox row should complete: {error}"));
    assert!(completed);
    let loaded = repository
        .get(&created.id)
        .await
        .unwrap_or_else(|error| panic!("outbox row should load: {error}"))
        .unwrap_or_else(|| panic!("outbox row should exist"));
    assert_eq!(loaded.status, "completed");
}

#[tokio::test]
async fn worker_dispatch_outbox_repository_summarizes_status_counts_and_oldest_queued_age() {
    let db = crate::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let repository = crate::repository::WorkerDispatchOutboxRepository::new(db);
    repository
        .create(crate::repository::CreateWorkerDispatchOutbox {
            instance_id: "inst-summary-1".to_owned(),
            attempt_id: "attempt-summary-1".to_owned(),
            worker_id: "worker-summary".to_owned(),
            logical_instance_id: "logical-summary".to_owned(),
            gateway_node_id: "gateway-summary".to_owned(),
            gateway_generation: 1,
            assignment_token: "asg-summary-1".to_owned(),
            dispatch_payload: "payload".to_owned(),
            shard_id: 0,
            owner_node_id: "owner".to_owned(),
            owner_epoch: 0,
            owner_fencing_token: "fence".to_owned(),
            next_delivery_at: None,
        })
        .await
        .unwrap_or_else(|error| panic!("outbox row should create: {error}"));
    let second = repository
        .create(crate::repository::CreateWorkerDispatchOutbox {
            instance_id: "inst-summary-2".to_owned(),
            attempt_id: "attempt-summary-2".to_owned(),
            worker_id: "worker-summary".to_owned(),
            logical_instance_id: "logical-summary".to_owned(),
            gateway_node_id: "gateway-summary".to_owned(),
            gateway_generation: 1,
            assignment_token: "asg-summary-2".to_owned(),
            dispatch_payload: "payload".to_owned(),
            shard_id: 0,
            owner_node_id: "owner".to_owned(),
            owner_epoch: 0,
            owner_fencing_token: "fence".to_owned(),
            next_delivery_at: None,
        })
        .await
        .unwrap_or_else(|error| panic!("outbox row should create: {error}"));
    let claimed = repository
        .claim_next_for_gateway("gateway-summary", 10)
        .await
        .unwrap_or_else(|error| panic!("outbox row should claim: {error}"))
        .unwrap_or_else(|| panic!("outbox row should be claimable"));
    repository
        .mark_delivered(&claimed.id, 30)
        .await
        .unwrap_or_else(|error| panic!("outbox row should deliver: {error}"));
    repository
        .mark_completed_by_assignment("inst-summary-2", "worker-summary", "asg-summary-2")
        .await
        .unwrap_or_else(|error| panic!("outbox row should complete: {error}"));

    let summary = repository
        .summary()
        .await
        .unwrap_or_else(|error| panic!("summary should load: {error}"));

    assert_eq!(summary.total, 2);
    assert_eq!(summary.by_status.get("delivered"), Some(&1));
    assert_eq!(summary.by_status.get("completed"), Some(&1));
    assert!(summary.oldest_queued_age_seconds <= 1);
    assert_eq!(second.instance_id, "inst-summary-2");
}
