#[tokio::test]
async fn alert_event_policy_materializes_notification_message_and_attempts() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let jobs = JobRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let templates = NotificationTemplateRepository::new(db);

    let rule = alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: "Script runtime failures".to_owned(),
            severity: "critical".to_owned(),
            condition_json: serde_json::json!({
                "type": "script_governance_failure",
                "failure_class": "script_runtime_unavailable",
                "threshold": 1
            })
            .to_string(),
            channels_json: "[]".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
            silenced_until: None,
        })
        .await
        .unwrap_or_else(|error| panic!("alert rule should create: {error}"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "global".to_owned(),
            namespace: None,
            app: None,
            worker_pool: None,
            name: "ops-alerts".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json:
                serde_json::json!({"url":"https://hooks.example.com/services/top-secret-token"})
                    .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "alert_rule".to_owned(),
            owner_id: Some(rule.id.clone()),
            name: "alert firing to ops".to_owned(),
            event_family: "alert".to_owned(),
            event_filter_json: serde_json::json!({
                "eventTypes": ["alert.firing"],
                "statuses": ["firing"],
                "severity": ["critical"]
            })
            .to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: String::new(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));
    let events = alerts
        .record_script_governance_failure(
            "instance-top-secret-token",
            "script_runtime_unavailable",
            "runtime image unavailable",
        )
        .await
        .unwrap_or_else(|error| panic!("alert event should create: {error}"));
    let event = events
        .into_iter()
        .find(|event| event.status == "firing")
        .unwrap_or_else(|| panic!("firing alert event should exist"));

    let center = NotificationCenter::new(
        channels.clone(),
        policies,
        messages.clone(),
        attempts.clone(),
        templates,
        jobs,
    );
    let emitted = center
        .emit_alert_event(&event)
        .await
        .unwrap_or_else(|error| panic!("alert notification should emit: {error}"));

    assert_eq!(emitted.matched_policies, 1);
    assert_eq!(emitted.messages_created, 1);
    assert_eq!(emitted.delivery_attempts_created, 1);
    let deduped = center
        .emit_alert_event(&event)
        .await
        .unwrap_or_else(|error| panic!("duplicate alert notification should dedupe: {error}"));
    assert_eq!(deduped.matched_policies, 1);
    assert_eq!(deduped.messages_created, 0);
    assert_eq!(deduped.delivery_attempts_created, 0);

    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("alert_event".to_owned()),
            source_id: Some(event.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].event_type, "alert.firing");
    assert_eq!(timeline[0].resource_type, "script_execution_governance");
    assert_eq!(timeline[0].resource_id, "instance-top-secret-token");
    assert_eq!(timeline[0].severity, "critical");
    assert!(timeline[0].subject.contains("Script runtime failures"));
    let payload: serde_json::Value = serde_json::from_str(&timeline[0].payload_json)
        .unwrap_or_else(|error| panic!("payload should parse: {error}"));
    assert_eq!(payload["eventType"], "alert.firing");
    assert_eq!(payload["legacyEventType"], "script_governance_failure");
    assert_eq!(payload["status"], "firing");
    assert!(!timeline[0].payload_json.contains("hooks.example.com/services/top-secret-token"));

    let delivery = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].retry_state, "retry_pending");
    assert_eq!(delivery[0].target_redacted, "https://hooks.example.com/...");
}



#[tokio::test]
async fn alert_rule_create_backfills_notification_policy_for_legacy_channels() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db);
    let rule = alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: "Create bridge".to_owned(),
            severity: "critical".to_owned(),
            condition_json: serde_json::json!({
                "type": "script_governance_failure",
                "failure_class": "script_runtime_unavailable",
                "threshold": 1
            })
            .to_string(),
            channels_json: serde_json::json!([
                {"type":"webhook","url":"https://legacy.example.com/create-token"}
            ])
            .to_string(),
            enabled: true,
            dedupe_seconds: 180,
            silenced_until: None,
        })
        .await
        .unwrap_or_else(|error| panic!("rule should create: {error}"));

    let first = crate::notification::ensure_alert_rule_notification_policy_from_channels(
        &rule,
        &channels,
        &policies,
        &[],
    )
    .await
    .unwrap_or_else(|error| panic!("legacy policy should backfill: {error}"));
    assert!(first.is_some());
    let second = crate::notification::ensure_alert_rule_notification_policy_from_channels(
        &rule,
        &channels,
        &policies,
        &[],
    )
    .await
    .unwrap_or_else(|error| panic!("legacy policy backfill should be idempotent: {error}"));
    assert_eq!(first.as_ref().map(|policy| &policy.id), second.as_ref().map(|policy| &policy.id));
    let items = policies
        .list_policies(tikeo_storage::NotificationPolicyFilters {
            owner_type: Some("alert_rule".to_owned()),
            owner_id: Some(rule.id.clone()),
            event_family: Some("alert".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("policies should list: {error}"));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].dedupe_seconds, 180);
    assert_eq!(items[0].severity, "critical");
    assert!(items[0].event_filter_json.contains("alert_rules.channels_json"));
    let migrated_channels = channels
        .list_channels(tikeo_storage::NotificationChannelFilters::default())
        .await
        .unwrap_or_else(|error| panic!("channels should list: {error}"));
    let migrated_channel = migrated_channels
        .iter()
        .find(|channel| channel.target_redacted == "https://legacy.example.com/...")
        .unwrap_or_else(|| panic!("legacy alert channel should be listed alongside seeded examples"));
    assert!(!migrated_channel.config_json.contains("create-token"));
}



#[tokio::test]
async fn alert_rule_legacy_backfill_migrates_all_existing_rules_idempotently() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db);
    let first = alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: "First legacy".to_owned(),
            severity: "warning".to_owned(),
            condition_json: serde_json::json!({"type":"script_governance_failure","failure_class":"a","threshold":1}).to_string(),
            channels_json: serde_json::json!([{"type":"webhook","url":"https://first.example.com/hook"}]).to_string(),
            enabled: true,
            dedupe_seconds: 60,
            silenced_until: None,
        })
        .await
        .unwrap_or_else(|error| panic!("first rule should create: {error}"));
    let second = alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: "Second legacy".to_owned(),
            severity: "critical".to_owned(),
            condition_json: serde_json::json!({"type":"script_governance_failure","failure_class":"b","threshold":1}).to_string(),
            channels_json: serde_json::json!([{"type":"slack","url":"https://hooks.slack.example.com/services/token"}]).to_string(),
            enabled: true,
            dedupe_seconds: 120,
            silenced_until: None,
        })
        .await
        .unwrap_or_else(|error| panic!("second rule should create: {error}"));

    let first_run = crate::notification::backfill_alert_rule_notification_policies(
        &alerts,
        &channels,
        &policies,
        &[],
    )
    .await
    .unwrap_or_else(|error| panic!("backfill should run: {error}"));
    assert_eq!(first_run.rules_seen, 2);
    assert_eq!(first_run.policies_created, 2);
    assert_eq!(first_run.channels_created, 2);
    let second_run = crate::notification::backfill_alert_rule_notification_policies(
        &alerts,
        &channels,
        &policies,
        &[],
    )
    .await
    .unwrap_or_else(|error| panic!("backfill should be idempotent: {error}"));
    assert_eq!(second_run.rules_seen, 2);
    assert_eq!(second_run.policies_created, 0);
    assert_eq!(second_run.channels_created, 0);

    let first_policies = policies
        .list_policies(tikeo_storage::NotificationPolicyFilters {
            owner_type: Some("alert_rule".to_owned()),
            owner_id: Some(first.id),
            event_family: Some("alert".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("policies should list: {error}"));
    assert_eq!(first_policies.len(), 1);
    assert_eq!(first_policies[0].dedupe_seconds, 60);
    let second_policies = policies
        .list_policies(tikeo_storage::NotificationPolicyFilters {
            owner_type: Some("alert_rule".to_owned()),
            owner_id: Some(second.id),
            event_family: Some("alert".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("policies should list: {error}"));
    assert_eq!(second_policies.len(), 1);
    assert_eq!(second_policies[0].severity, "critical");
}

#[tokio::test]
async fn alert_recovery_event_policy_materializes_notification_message() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let jobs = JobRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let templates = NotificationTemplateRepository::new(db);

    let rule = alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: "Recovery bridge".to_owned(),
            severity: "warning".to_owned(),
            condition_json: serde_json::json!({
                "type": "script_governance_failure",
                "failure_class": "script_runtime_unavailable",
                "threshold": 1
            })
            .to_string(),
            channels_json: "[]".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
            silenced_until: None,
        })
        .await
        .unwrap_or_else(|error| panic!("rule should create: {error}"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "global".to_owned(),
            namespace: None,
            app: None,
            worker_pool: None,
            name: "ops-recovery".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"url":"https://hooks.example.com/recovery-token"}).to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "alert_rule".to_owned(),
            owner_id: Some(rule.id.clone()),
            name: "alert recovery to ops".to_owned(),
            event_family: "alert".to_owned(),
            event_filter_json: serde_json::json!({
                "eventTypes": ["alert.recovered"],
                "statuses": ["recovered"]
            })
            .to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: "info".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));
    let firing = alerts
        .record_script_governance_failure("inst-recovery", "script_runtime_unavailable", "runtime missing")
        .await
        .unwrap_or_else(|error| panic!("firing should create: {error}"))
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("firing event should exist"));
    let recovered = alerts
        .record_script_governance_recovery(&firing.id)
        .await
        .unwrap_or_else(|error| panic!("recovery should create: {error}"))
        .unwrap_or_else(|| panic!("recovery event should be returned"));
    assert_eq!(recovered.status, "recovered");

    let center = NotificationCenter::new(
        channels.clone(),
        policies,
        messages.clone(),
        attempts.clone(),
        templates,
        jobs,
    );
    let emitted = center
        .emit_alert_event(&recovered)
        .await
        .unwrap_or_else(|error| panic!("recovery notification should emit: {error}"));
    assert_eq!(emitted.matched_policies, 1);
    assert_eq!(emitted.messages_created, 1);
    assert_eq!(emitted.delivery_attempts_created, 1);
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("alert_event".to_owned()),
            source_id: Some(recovered.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].event_type, "alert.recovered");
    assert_eq!(timeline[0].severity, "info");
    assert!(!timeline[0].payload_json.contains("recovery-token"));
    let delivery = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].target_redacted, "https://hooks.example.com/...");
}

#[tokio::test]
async fn governance_alert_materialization_backfills_legacy_policy_and_attempt() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());

    let rule = alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: "Governance bridge".to_owned(),
            severity: "warning".to_owned(),
            condition_json: serde_json::json!({
                "type": "script_governance_failure",
                "failure_class": "script_runtime_unavailable",
                "threshold": 1
            })
            .to_string(),
            channels_json: serde_json::json!([
                {"type":"webhook","url":"http://localhost/legacy-token"}
            ])
            .to_string(),
            enabled: true,
            dedupe_seconds: 300,
            silenced_until: None,
        })
        .await
        .unwrap_or_else(|error| panic!("rule should create: {error}"));
    crate::tunnel::governance::materialize_script_governance_audit(
        &AuditLogRepository::new(db),
        "tikeo-dispatcher",
        "inst-bridge",
        "script_runtime_unavailable",
        "runtime unavailable",
    )
    .await
    .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));

    let backfilled_policies = policies
        .list_policies(tikeo_storage::NotificationPolicyFilters {
            owner_type: Some("alert_rule".to_owned()),
            owner_id: Some(rule.id.clone()),
            event_family: Some("alert".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("policies should list: {error}"));
    assert_eq!(backfilled_policies.len(), 1);
    assert!(backfilled_policies[0].event_filter_json.contains("alert_rules.channels_json"));
    let migrated_channels = channels
        .list_channels(tikeo_storage::NotificationChannelFilters::default())
        .await
        .unwrap_or_else(|error| panic!("channels should list: {error}"));
    let migrated_channel = migrated_channels
        .iter()
        .find(|channel| channel.target_redacted == "http://localhost/...")
        .unwrap_or_else(|| panic!("legacy governance channel should be listed alongside seeded examples"));
    assert_eq!(migrated_channel.provider, "webhook");

    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("alert_event".to_owned()),
            event_type: Some("alert.firing".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].resource_id, "inst-bridge");
    assert!(!timeline[0].payload_json.contains("legacy-token"));
    assert_eq!(timeline[0].status, "pending");
    let delivery = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].retry_state, "retry_pending");
    assert_eq!(delivery[0].target_redacted, "http://localhost/...");
}

#[tokio::test]
async fn policy_template_ref_materializes_and_drives_provider_payload() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let templates = NotificationTemplateRepository::new(db.clone());
    let received = std::sync::Arc::new(tokio::sync::Mutex::new(None::<serde_json::Value>));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
    let url = format!(
        "http://{}/notify",
        listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener addr should read: {error}"))
    );
    let received_for_route = received.clone();
    let app = axum::Router::new().route(
        "/notify",
        axum::routing::post(move |axum::Json(payload): axum::Json<serde_json::Value>| {
            let received = received_for_route.clone();
            async move {
                *received.lock().await = Some(payload);
                axum::http::StatusCode::OK
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|error| panic!("webhook server should run: {error}"));
    });

    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "billing-nightly".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("demo.fail".to_owned()),
            processor_type: None,
            script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let instance = instances
        .update_status(&instance.id, InstanceStatus::Failed)
        .await
        .unwrap_or_else(|error| panic!("status should update: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let template = templates
        .create_template(CreateNotificationTemplate {
            template_key: "ops.webhook.failure".to_owned(),
            name: "Ops webhook failure".to_owned(),
            description: Some("Provider-specific webhook template".to_owned()),
            provider: "webhook".to_owned(),
            message_type: "json".to_owned(),
            enabled: true,
            body_json: serde_json::json!({
                "subject": "Templated {{subject}}",
                "text": "Rendered {{body}} / {{eventType}}",
                "body": {
                    "summary": "{{subject}}",
                    "details": "{{body}}",
                    "event": "{{eventType}}",
                    "resource": "{{resourceId}}",
                    "jobId": "{{jobId}}",
                    "instanceId": "{{instanceId}}",
                    "operator": "{{operatorName}}",
                    "logsUrl": "{{logsUrl}}",
                    "templateKey": "{{templateKey}}"
                }
            })
            .to_string(),
            variables_json: serde_json::json!({"severity":"critical"}).to_string(),
        })
        .await
        .unwrap_or_else(|error| panic!("template should create: {error}"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "ops".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"url": url}).to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: Some(
                serde_json::json!({"allowInsecureLoopback": true}).to_string(),
            ),
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some(job.id.clone()),
            name: "job failures".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["failed"]}).to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: Some(template.template_key.clone()),
            severity: "critical".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));

    let center = NotificationCenter::new(
        channels.clone(),
        policies,
        messages.clone(),
        attempts.clone(),
        templates,
        jobs,
    )
    .with_public_console_base_url(Some("https://console.example.com/tikeo"));
    center
        .emit_job_instance_event(&instance, JobNotificationEvent::Failed, Some("exit 2"))
        .await
        .unwrap_or_else(|error| panic!("notification should emit: {error}"));

    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline.len(), 1);
    assert_eq!(
        timeline[0].subject,
        "Templated Tikeo job billing-nightly: failed"
    );
    assert!(
        timeline[0]
            .body
            .starts_with("Rendered Job billing-nightly instance "),
        "body should be rendered from the reusable template: {}",
        timeline[0].body
    );
    assert!(timeline[0].body.ends_with(" / job_instance.failed"));
    let payload: serde_json::Value = serde_json::from_str(&timeline[0].payload_json)
        .unwrap_or_else(|error| panic!("payload should be JSON: {error}"));
    assert_eq!(payload["templateKey"], "ops.webhook.failure");
    assert_eq!(payload["template"]["body"]["event"], "job_instance.failed");
    assert_eq!(payload["template"]["body"]["jobId"], job.id);
    assert_eq!(payload["template"]["body"]["instanceId"], instance.id);
    assert_eq!(payload["template"]["body"]["operator"], "tikeo");
    assert_eq!(
        payload["template"]["body"]["logsUrl"],
        format!("https://console.example.com/tikeo/public/instances/{}/console", instance.id)
    );
    assert_eq!(payload["template"]["body"]["templateKey"], "ops.webhook.failure");

    let delivered = process_due_notification_delivery_attempts(
        &channels,
        &messages,
        &attempts,
        50,
        NotificationDeliveryPolicy::default(),
    )
    .await
    .unwrap_or_else(|error| panic!("delivery processor should run: {error}"));
    assert_eq!(delivered.delivered, 1);
    let stored_payload = received
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| panic!("webhook should receive payload"));
    assert_eq!(stored_payload["summary"], timeline[0].subject);
    assert_eq!(stored_payload["event"], "job_instance.failed");
    assert_ne!(
        stored_payload["summary"],
        "INLINE CHANNEL TEMPLATE SHOULD NOT WIN"
    );
    assert_eq!(stored_payload["details"], timeline[0].body);
    assert_eq!(stored_payload["event"], "job_instance.failed");
    server.abort();
}

#[tokio::test]
async fn rich_provider_delivery_fails_closed_without_required_template() {
    let message = sample_notification_message();
    let client = NotificationProviderClient::new(AlertDeliveryPolicy {
        allow_insecure_loopback: true,
    });

    for (provider, config) in [
        (
            "dingtalk",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "link"}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "actionCard"}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "feedCard"}),
        ),
        (
            "feishu",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "image"}),
        ),
        (
            "feishu",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "share_chat"}),
        ),
        (
            "feishu",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "interactive"}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "image"}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "news"}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "file"}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "voice"}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": "http://127.0.0.1:9/notify", "messageType": "template_card"}),
        ),
    ] {
        let result = client
            .deliver(
                &NotificationChannelDeliveryConfig {
                    id: format!("channel-{provider}"),
                    provider: provider.to_owned(),
                    enabled: true,
                    config_json: config.to_string(),
                    secret_refs_json: "{}".to_owned(),
                    target_redacted: "local".to_owned(),
                    safety_policy_json: Some(
                        serde_json::json!({"allowInsecureLoopback": true}).to_string(),
                    ),
                },
                &message,
            )
            .await;
        assert!(
            !result.delivered,
            "{provider} should fail closed without template"
        );
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|error| error.contains("requires a channel inline template")),
            "{provider} error should explain template requirement: {result:?}"
        );
    }
}

#[tokio::test]
async fn webhook_delivery_uses_direct_channel_secret_values_without_env_lookup() {
    let received_headers = std::sync::Arc::new(tokio::sync::Mutex::new(None::<(String, String)>));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
    let url = format!(
        "http://{}/notify",
        listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener addr should read: {error}"))
    );
    let received_for_route = received_headers.clone();
    let app = axum::Router::new().route(
        "/notify",
        axum::routing::post(move |headers: axum::http::HeaderMap| {
            let received = received_for_route.clone();
            async move {
                let authorization = headers
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_owned();
                let secret_header = headers
                    .get("x-tikeo-secret-header")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_owned();
                *received.lock().await = Some((authorization.clone(), secret_header.clone()));
                if authorization == "Bearer direct-channel-token" && secret_header == "direct-header-value" {
                    axum::http::StatusCode::OK
                } else {
                    axum::http::StatusCode::UNAUTHORIZED
                }
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|error| panic!("webhook server should run: {error}"));
    });

    let result = deliver_notification_channel_once(
        &NotificationChannelDeliveryConfig {
            id: "channel-direct-webhook".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({
                "messageType": "json",
                "template": {"body": {"subject": "{{subject}}"}}
            })
            .to_string(),
            secret_refs_json: serde_json::json!({
                "url": url,
                "authorization": "Bearer direct-channel-token",
                "headers": {"x-tikeo-secret-header": "direct-header-value"}
            })
            .to_string(),
            target_redacted: "webhook:secret-ref".to_owned(),
            safety_policy_json: Some(
                serde_json::json!({"allowInsecureLoopback": true}).to_string(),
            ),
        },
        &sample_notification_message(),
        AlertDeliveryPolicy {
            allow_insecure_loopback: false,
        },
    )
    .await;

    assert!(result.delivered, "direct channel credentials should deliver: {result:?}");
    assert_eq!(result.status_code, Some(200));
    assert!(
        !result.target_redacted.contains("direct-channel-token"),
        "redacted target must not leak direct credential: {}",
        result.target_redacted
    );
    assert!(!result
        .rendered_payload
        .unwrap_or_default()
        .to_string()
        .contains("direct-channel-token"));
    assert_eq!(
        received_headers.lock().await.clone(),
        Some((
            "Bearer direct-channel-token".to_owned(),
            "direct-header-value".to_owned()
        ))
    );
    server.abort();
}

#[tokio::test]
async fn provider_delivery_renders_configured_message_types_and_templates() {
    let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<serde_json::Value>::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
    let url = format!(
        "http://{}/notify",
        listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener addr should read: {error}"))
    );
    let received_for_route = received.clone();
    let app = axum::Router::new().route(
        "/notify",
        axum::routing::post(move |axum::Json(payload): axum::Json<serde_json::Value>| {
            let received = received_for_route.clone();
            async move {
                received.lock().await.push(payload);
                axum::http::StatusCode::OK
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|error| panic!("webhook server should run: {error}"));
    });

    let message = sample_notification_message();
    let client = NotificationProviderClient::new(AlertDeliveryPolicy {
        allow_insecure_loopback: true,
    });

    for (provider, config) in [
        (
            "slack",
            serde_json::json!({"url": url, "messageType": "blockKit", "template": {"text": "{{subject}}", "blocks": [{"type":"section","text":{"type":"mrkdwn","text":"{{body}} / {{severity}}"}}]}}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": url, "messageType": "markdown", "template": {"title": "{{subject}}", "text": "{{body}} {{eventType}}"}}),
        ),
        (
            "feishu",
            serde_json::json!({"url": url, "messageType": "post", "template": {"post": {"zh_cn": {"title": "{{subject}}", "content": [[{"tag":"text","text":"{{body}}"}]]}}}}),
        ),
        (
            "feishu",
            serde_json::json!({"url": url, "messageType": "image", "template": {"imageKey":"{{resourceId}}-image"}}),
        ),
        (
            "feishu",
            serde_json::json!({"url": url, "messageType": "share_chat", "template": {"shareChatId":"{{resourceId}}-chat"}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "news", "template": {"articles": [{"title":"{{subject}}","description":"{{body}}","url":"https://example.com/{{resourceId}}"}]}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "voice", "template": {"media_id":"{{resourceId}}-voice"}}),
        ),
        (
            "pagerduty",
            serde_json::json!({"url": url, "routingKey": "route-123", "messageType": "resolve", "template": {"summary": "{{subject}}", "dedupKey": "custom-{{messageId}}", "source": "{{resourceType}}", "severity": "error", "timestamp": "{{triggeredAt}}", "component": "{{resourceId}}", "client":"tikeo", "clientUrl":"https://example.com/{{resourceId}}", "links":[{"href":"https://example.com/{{resourceId}}","text":"runbook"}], "images":"[{\"src\":\"https://example.com/{{resourceId}}.png\",\"alt\":\"chart\"}]"}}),
        ),
        (
            "webhook",
            serde_json::json!({"url": url, "template": {"provider":"generic","message":"{{subject}}","dedupe":"{{dedupeKey}}"}}),
        ),
        (
            "ops_bridge",
            serde_json::json!({"url": url, "messageType": "ticket", "template": {"body": {"ticket":"{{subject}}","event":"{{eventType}}"}}}),
        ),
    ] {
        let result = client
            .deliver(
                &NotificationChannelDeliveryConfig {
                    id: format!("channel-{provider}"),
                    provider: provider.to_owned(),
                    enabled: true,
                    config_json: config.to_string(),
                    secret_refs_json: if provider == "pagerduty" {
                        serde_json::json!({"routingKey":"env:PATH"}).to_string()
                    } else {
                        "{}".to_owned()
                    },
                    target_redacted: "local".to_owned(),
                    safety_policy_json: Some(
                        serde_json::json!({"allowInsecureLoopback": true}).to_string(),
                    ),
                },
                &message,
            )
            .await;
        assert!(result.delivered, "{provider} should deliver: {result:?}");
    }

    let payloads = received.lock().await.clone();
    assert_eq!(payloads[0]["text"], "Job failed");
    assert_eq!(
        payloads[0]["blocks"][0]["text"]["text"],
        "Exited 2 / critical"
    );
    assert_eq!(payloads[1]["msgtype"], "markdown");
    assert_eq!(
        payloads[1]["markdown"]["text"],
        "Exited 2 job_instance.failed"
    );
    assert_eq!(payloads[2]["msg_type"], "post");
    assert_eq!(
        payloads[2]["content"]["post"]["zh_cn"]["title"],
        "Job failed"
    );
    assert_eq!(payloads[3]["msg_type"], "image");
    assert_eq!(payloads[3]["content"]["image_key"], "billing-nightly-image");
    assert_eq!(payloads[4]["msg_type"], "share_chat");
    assert_eq!(
        payloads[4]["content"]["share_chat_id"],
        "billing-nightly-chat"
    );
    assert_eq!(payloads[5]["msgtype"], "news");
    assert_eq!(
        payloads[5]["news"]["articles"][0]["description"],
        "Exited 2"
    );
    assert_eq!(payloads[6]["msgtype"], "voice");
    assert_eq!(payloads[6]["voice"]["media_id"], "billing-nightly-voice");
    assert_eq!(payloads[7]["event_action"], "resolve");
    assert_eq!(payloads[7]["dedup_key"], "custom-msg-1");
    assert_eq!(payloads[7]["payload"]["source"], "job");
    assert_eq!(payloads[7]["payload"]["severity"], "error");
    assert_eq!(payloads[7]["payload"]["timestamp"], "2026-06-11T00:00:00Z");
    assert_eq!(payloads[7]["client"], "tikeo");
    assert_eq!(
        payloads[7]["client_url"],
        "https://example.com/billing-nightly"
    );
    assert_eq!(payloads[7]["links"][0]["text"], "runbook");
    assert_eq!(payloads[7]["images"][0]["alt"], "chart");
    assert_eq!(payloads[8]["message"], "Job failed");
    assert_eq!(payloads[8]["dedupe"], "policy-1:instance-1:failed");
    assert_eq!(payloads[9]["ticket"], "Job failed");
    assert_eq!(payloads[9]["event"], "job_instance.failed");
    server.abort();
}

#[tokio::test]
async fn provider_delivery_covers_all_builtin_message_shape_families() {
    let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<serde_json::Value>::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
    let url = format!(
        "http://{}/notify",
        listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener addr should read: {error}"))
    );
    let received_for_route = received.clone();
    let app = axum::Router::new().route(
        "/notify",
        axum::routing::post(move |axum::Json(payload): axum::Json<serde_json::Value>| {
            let received = received_for_route.clone();
            async move {
                received.lock().await.push(payload);
                axum::http::StatusCode::OK
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|error| panic!("webhook server should run: {error}"));
    });

    let message = sample_notification_message();
    let client = NotificationProviderClient::new(AlertDeliveryPolicy {
        allow_insecure_loopback: true,
    });
    let cases = [
        (
            "slack",
            serde_json::json!({"url": url, "messageType": "attachments", "template": {"text": "{{subject}}", "attachments": [{"title":"{{subject}}","text":"{{body}}"}]}}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": url, "messageType": "text", "template": {"content": "{{subject}} {{body}}"}}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": url, "messageType": "link", "template": {"title": "{{subject}}", "text": "{{body}}", "messageUrl": "https://example.com/{{resourceId}}"}}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": url, "messageType": "actionCard", "template": {"title": "{{subject}}", "text": "{{body}}", "singleTitle": "Open", "singleURL": "https://example.com/{{resourceId}}"}}),
        ),
        (
            "dingtalk",
            serde_json::json!({"url": url, "messageType": "feedCard", "template": {"links": [{"title":"{{subject}}","messageURL":"https://example.com/{{resourceId}}"}]}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "text", "mentionedList": ["@all"], "template": {"content": "{{subject}} {{body}}"}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "markdown", "template": {"content": "### {{subject}}\n{{body}}"}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "markdown_v2", "template": {"content": "# {{subject}}\n{{body}}"}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "image", "template": {"base64": "BASE64-{{resourceId}}", "md5": "MD5-{{messageId}}"}}),
        ),
        (
            "wechat_work",
            serde_json::json!({"url": url, "messageType": "file", "template": {"media_id": "{{resourceId}}-file"}}),
        ),
        (
            "pagerduty",
            serde_json::json!({"url": url, "routingKey": "route-123", "messageType": "trigger", "template": {"summary": "{{subject}}", "dedupKey": "{{dedupeKey}}", "customDetails": {"body": "{{body}}"}}}),
        ),
        (
            "pagerduty",
            serde_json::json!({"url": url, "routingKey": "route-123", "messageType": "acknowledge", "template": {"dedupKey": "ack-{{messageId}}"}}),
        ),
        (
            "webhook",
            serde_json::json!({"url": url, "messageType": "json", "template": {"body": "{\"text\":\"{{subject}}\",\"status\":\"{{severity}}\"}"}}),
        ),
    ];

    for (provider, config) in cases {
        let result = client
            .deliver(
                &NotificationChannelDeliveryConfig {
                    id: format!("channel-{provider}"),
                    provider: provider.to_owned(),
                    enabled: true,
                    config_json: config.to_string(),
                    secret_refs_json: "{}".to_owned(),
                    target_redacted: "local".to_owned(),
                    safety_policy_json: Some(
                        serde_json::json!({"allowInsecureLoopback": true}).to_string(),
                    ),
                },
                &message,
            )
            .await;
        assert!(result.delivered, "{provider} should deliver: {result:?}");
    }

    let payloads = received.lock().await.clone();
    assert_eq!(payloads[0]["attachments"][0]["text"], "Exited 2");
    assert_eq!(payloads[1]["text"]["content"], "Job failed Exited 2");
    assert_eq!(
        payloads[2]["link"]["messageUrl"],
        "https://example.com/billing-nightly"
    );
    assert_eq!(
        payloads[3]["actionCard"]["singleURL"],
        "https://example.com/billing-nightly"
    );
    assert_eq!(payloads[4]["feedCard"]["links"][0]["title"], "Job failed");
    assert_eq!(payloads[5]["text"]["content"], "Job failed Exited 2");
    assert_eq!(payloads[5]["mentioned_list"][0], "@all");
    assert_eq!(
        payloads[6]["markdown"]["content"],
        "### Job failed\nExited 2"
    );
    assert_eq!(
        payloads[7]["markdown_v2"]["content"],
        "# Job failed\nExited 2"
    );
    assert_eq!(payloads[8]["image"]["base64"], "BASE64-billing-nightly");
    assert_eq!(payloads[9]["file"]["media_id"], "billing-nightly-file");
    assert_eq!(payloads[10]["event_action"], "trigger");
    assert_eq!(
        payloads[10]["payload"]["custom_details"]["body"],
        "Exited 2"
    );
    assert_eq!(payloads[11]["event_action"], "acknowledge");
    assert_eq!(payloads[11]["dedup_key"], "ack-msg-1");
    assert_eq!(payloads[12]["text"], "Job failed");
    assert_eq!(payloads[12]["status"], "critical");
    server.abort();
}

#[tokio::test]
async fn provider_delivery_accepts_drawer_template_strings_and_signs_office_bots() {
    let received = std::sync::Arc::new(tokio::sync::Mutex::new(
        Vec::<(String, serde_json::Value)>::new(),
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
    let url = format!(
        "http://{}/notify",
        listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener addr should read: {error}"))
    );
    let received_for_route = received.clone();
    let app = axum::Router::new().route(
        "/notify",
        axum::routing::post(
            move |uri: axum::http::Uri, axum::Json(payload): axum::Json<serde_json::Value>| {
                let received = received_for_route.clone();
                async move {
                    received.lock().await.push((uri.to_string(), payload));
                    axum::http::StatusCode::OK
                }
            },
        ),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|error| panic!("webhook server should run: {error}"));
    });

    let message = sample_notification_message();
    let client = NotificationProviderClient::new(AlertDeliveryPolicy {
        allow_insecure_loopback: true,
    });

    for (provider, config, secrets) in [
        (
            "slack",
            serde_json::json!({
                "url": url,
                "messageType": "blockKit",
                "template": {
                    "text": "{{subject}}",
                    "blocks": r#"[{"type":"section","text":{"type":"mrkdwn","text":"{{body}} / {{severity}}"}}]"#
                }
            }),
            serde_json::json!({}),
        ),
        (
            "dingtalk",
            serde_json::json!({
                "url": url,
                "messageType": "markdown",
                "atMobiles": ["13800138000"],
                "isAtAll": false,
                "template": {"title": "{{subject}}", "text": "{{body}} {{eventType}}"}
            }),
            serde_json::json!({"signingKey": "env:PATH"}),
        ),
        (
            "feishu",
            serde_json::json!({
                "url": url,
                "messageType": "interactive",
                "template": {
                    "card": r#"{"header":{"title":{"tag":"plain_text","content":"{{subject}}"}},"elements":[{"tag":"div","text":{"tag":"lark_md","content":"{{body}}"}}]}"#
                }
            }),
            serde_json::json!({"signingKey": "env:PATH"}),
        ),
        (
            "wechat_work",
            serde_json::json!({
                "url": url,
                "messageType": "template_card",
                "template": {
                    "templateCard": r#"{"card_type":"text_notice","main_title":{"title":"{{subject}}","desc":"{{body}}"},"card_action":{"type":1,"url":"https://example.com/{{resourceId}}"}}"#
                }
            }),
            serde_json::json!({}),
        ),
    ] {
        let result = client
            .deliver(
                &NotificationChannelDeliveryConfig {
                    id: format!("channel-{provider}"),
                    provider: provider.to_owned(),
                    enabled: true,
                    config_json: config.to_string(),
                    secret_refs_json: secrets.to_string(),
                    target_redacted: "local".to_owned(),
                    safety_policy_json: Some(
                        serde_json::json!({"allowInsecureLoopback": true}).to_string(),
                    ),
                },
                &message,
            )
            .await;
        assert!(result.delivered, "{provider} should deliver: {result:?}");
    }

    let payloads = received.lock().await.clone();
    assert_eq!(
        payloads[0].1["blocks"][0]["text"]["text"],
        "Exited 2 / critical"
    );
    assert!(payloads[1].0.contains("timestamp="));
    assert!(payloads[1].0.contains("sign="));
    assert_eq!(payloads[1].1["at"]["atMobiles"][0], "13800138000");
    assert!(payloads[2].1.get("timestamp").is_some());
    assert!(payloads[2].1.get("sign").is_some());
    assert_eq!(
        payloads[2].1["card"]["header"]["title"]["content"],
        "Job failed"
    );
    assert_eq!(payloads[3].1["msgtype"], "template_card");
    assert_eq!(
        payloads[3].1["template_card"]["main_title"]["desc"],
        "Exited 2"
    );
    server.abort();
}

#[test]
fn email_template_config_overrides_alert_payload_subject_and_body() {
    let message = sample_notification_message();
    let config = parse_json_object(
        &serde_json::json!({
            "template": {
                "subject": "{{subject}} / {{severity}}",
                "body": "{{body}} / {{eventType}}",
                "html": "<strong>{{body}}</strong>"
            }
        })
        .to_string(),
    );
    let payload = email_alert_payload_from_message(&message, &config);

    assert_eq!(payload.rule_name, "Job failed / critical");
    assert_eq!(payload.message, "Exited 2 / job_instance.failed");
    assert_eq!(payload.resource_type, "job");
    assert_eq!(config["template"]["html"], "<strong>{{body}}</strong>");
}


#[test]
fn feishu_interactive_payload_is_driven_by_rendered_template_data() {
    let message = NotificationMessageSummary {
        payload_json: serde_json::json!({
            "template": {
                "messageType": "interactive",
                "card": {
                    "config": {"wide_screen_mode": true},
                    "header": {
                        "template": "red",
                        "title": {"tag": "plain_text", "content": "{{subject}}"}
                    },
                    "elements": [
                        {"tag": "div", "text": {"tag": "lark_md", "content": "{{body}} / {{reason}}"}},
                        {"tag": "action", "actions": [
                            {"tag": "button", "text": {"tag": "plain_text", "content": "Open"}, "type": "danger", "url": "{{consoleUrl}}"}
                        ]}
                    ]
                }
            },
            "reason": "参数不能为空 should not be empty",
            "consoleUrl": "/public/instances/inst-feishu-card/console"
        })
        .to_string(),
        ..sample_notification_message()
    };
    let config = serde_json::json!({"messageType":"interactive"});
    let payload = feishu_payload(&message, config.as_object().unwrap_or_else(|| panic!("config object")));

    assert_eq!(payload["msg_type"], "interactive");
    assert_eq!(payload["card"]["header"]["template"], "red");
    assert_eq!(payload["card"]["header"]["title"]["content"], "Job failed");
    let rendered = payload.to_string();
    assert!(rendered.contains("Exited 2 / 参数不能为空 should not be empty"));
    assert!(rendered.contains("/public/instances/inst-feishu-card/console"));
    assert!(!rendered.contains("Tikeo Job 任务通知"));
    assert!(!rendered.contains("任务执行失败报警"));
    assert_eq!(payload["card"]["elements"][1]["actions"][0]["type"], "danger");
}

fn sample_notification_message() -> NotificationMessageSummary {
    NotificationMessageSummary {
        id: "msg-1".to_owned(),
        source_type: "job_instance".to_owned(),
        source_id: "instance-1".to_owned(),
        policy_id: "policy-1".to_owned(),
        event_type: "job_instance.failed".to_owned(),
        resource_type: "job".to_owned(),
        resource_id: "billing-nightly".to_owned(),
        severity: "critical".to_owned(),
        subject: "Job failed".to_owned(),
        body: "Exited 2".to_owned(),
        payload_json: serde_json::json!({"eventType":"job_instance.failed"}).to_string(),
        dedupe_key: "policy-1:instance-1:failed".to_owned(),
        trace_id: None,
        status: "pending".to_owned(),
        created_at: "2026-06-11T00:00:00Z".to_owned(),
        updated_at: "2026-06-11T00:00:00Z".to_owned(),
    }
}
