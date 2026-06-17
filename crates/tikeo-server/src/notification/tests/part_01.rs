#[tokio::test]
async fn due_delivery_attempts_post_to_webhook_and_update_message_status() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let received = std::sync::Arc::new(tokio::sync::Mutex::new(None::<serde_json::Value>));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
    let url = format!(
        "http://{}/notify/top-secret-token",
        listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener addr should read: {error}"))
    );
    let received_for_route = received.clone();
    let app = axum::Router::new().route(
        "/notify/top-secret-token",
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
            canary_policy: None,
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
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "ops".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({
                "url": url,
                "messageType": "json",
                "template": {
                    "body": {
                        "eventType": "{{eventType}}",
                        "subject": "{{subject}}",
                        "summary": "Channel template rendered {{body}}"
                    }
                }
            })
            .to_string(),
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
            template_ref: None,
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
        tikeo_storage::NotificationTemplateRepository::new(channels.db()),
        jobs,
    );
    center
        .emit_job_instance_event(&instance, JobNotificationEvent::Failed, Some("exit 2"))
        .await
        .unwrap_or_else(|error| panic!("notification should emit: {error}"));
    let delivered = process_due_notification_delivery_attempts(
        &channels,
        &messages,
        &attempts,
        50,
        NotificationDeliveryPolicy {
            max_attempts: 3,
            backoff_seconds: 300,
        },
    )
    .await
    .unwrap_or_else(|error| panic!("delivery processor should run: {error}"));

    assert_eq!(delivered.scanned, 1);
    assert_eq!(delivered.delivered, 1);
    let stored_payload = received
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| panic!("webhook should receive payload"));
    assert_eq!(stored_payload["eventType"], "job_instance.failed");
    assert_eq!(
        stored_payload["subject"],
        "Tikeo job billing-nightly: failed"
    );
    assert!(
        stored_payload["summary"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Channel template rendered Job billing-nightly instance "),
        "channel-level webhook template should render when no policy template is linked: {stored_payload}"
    );
    assert!(!stored_payload.to_string().contains("top-secret-token"));
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline[0].status, "delivered");
    let attempts_list = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert!(attempts_list.iter().any(|attempt| attempt.attempt == 0
        && !attempt.delivered
        && attempt.retry_state == "retry_consumed"));
    assert!(attempts_list.iter().any(|attempt| attempt.attempt == 1
        && attempt.delivered
        && attempt.retry_state == "delivered"));
    assert!(attempts_list.iter().any(|attempt| attempt.delivered
        && attempt.retry_state == "delivered"
        && !attempt.target_redacted.contains("top-secret-token")));
    server.abort();
}

#[tokio::test]
async fn webhook_delivery_injects_authorization_from_secret_refs_without_leaking_it() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let received_headers =
        std::sync::Arc::new(tokio::sync::Mutex::new((None::<String>, None::<String>)));
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
                *received.lock().await = (
                    headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .map(ToOwned::to_owned),
                    headers
                        .get("x-tikeo-secret-header")
                        .and_then(|value| value.to_str().ok())
                        .map(ToOwned::to_owned),
                );
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
            canary_policy: None,
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
    let expected_authorization = std::env::var("PATH")
        .unwrap_or_else(|error| panic!("PATH should be available for secret-ref test: {error}"));
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
            secret_refs_json: serde_json::json!({
                "authorization": "env:PATH",
                "headers": {"x-tikeo-secret-header": "env:PATH"}
            })
            .to_string(),
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
            template_ref: None,
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
        tikeo_storage::NotificationTemplateRepository::new(channels.db()),
        jobs,
    );
    center
        .emit_job_instance_event(&instance, JobNotificationEvent::Failed, Some("exit 2"))
        .await
        .unwrap_or_else(|error| panic!("notification should emit: {error}"));
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
    let received_headers = received_headers.lock().await.clone();
    assert_eq!(
        received_headers.0.as_deref(),
        Some(expected_authorization.as_str())
    );
    assert_eq!(
        received_headers.1.as_deref(),
        Some(expected_authorization.as_str())
    );
    let attempts_list = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert!(!attempts_list.iter().any(|attempt| {
        attempt
            .error
            .as_deref()
            .unwrap_or_default()
            .contains(&expected_authorization)
    }));
    server.abort();
}

#[test]
fn email_channel_uses_direct_secret_refs_without_env_lookup() {
    let channel = NotificationChannelDeliveryConfig {
        id: "notification-channel-email".to_owned(),
        provider: "email".to_owned(),
        enabled: true,
        config_json: serde_json::json!({
            "to": ["ops@example.com"],
            "username": "tikeo"
        })
        .to_string(),
        secret_refs_json: serde_json::json!({
            "smtpUrl": "smtp+starttls://smtp.example.com:587",
            "password": "direct-smtp-password"
        })
        .to_string(),
        target_redacted: "email".to_owned(),
        safety_policy_json: None,
    };

    let resolved = notification_channel_from_delivery_config(&channel)
        .unwrap_or_else(|| panic!("email channel should resolve from direct secretRefs values"));
    match resolved {
        NotificationChannel::Email {
            smtp_url,
            password,
            password_secret_ref,
            ..
        } => {
            assert_eq!(smtp_url.as_deref(), Some("smtp+starttls://smtp.example.com:587"));
            assert_eq!(password.as_deref(), Some("direct-smtp-password"));
            assert_eq!(password_secret_ref, None);
        }
        other => panic!("expected email channel, got {other:?}"),
    }
}

#[test]
fn email_channel_accepts_metadata_secret_refs_password_alias() {
    let channel = NotificationChannelDeliveryConfig {
        id: "notification-channel-email".to_owned(),
        provider: "email".to_owned(),
        enabled: true,
        config_json: serde_json::json!({
            "to": ["ops@example.com"],
            "smtpUrl": "smtp+starttls://smtp.example.com:587",
            "username": "tikeo"
        })
        .to_string(),
        secret_refs_json: serde_json::json!({
            "password": "env:TIKEO_SMTP_PASSWORD"
        })
        .to_string(),
        target_redacted: "email".to_owned(),
        safety_policy_json: None,
    };

    let resolved = notification_channel_from_delivery_config(&channel)
        .unwrap_or_else(|| panic!("email channel should resolve from metadata-shaped secretRefs"));
    match resolved {
        NotificationChannel::Email {
            password_secret_ref,
            ..
        } => {
            assert_eq!(
                password_secret_ref.as_deref(),
                Some("env:TIKEO_SMTP_PASSWORD")
            );
        }
        other => panic!("expected email channel, got {other:?}"),
    }
}

#[test]
fn email_channel_prefers_split_smtp_fields_when_legacy_smtp_url_is_bare_host() {
    let channel = NotificationChannelDeliveryConfig {
        id: "notification-channel-email".to_owned(),
        provider: "email".to_owned(),
        enabled: true,
        config_json: serde_json::json!({
            "to": ["ops@example.com"],
            "host": "smtp.feishu.cn",
            "port": "465",
            "ssl": true,
            "starttls": false,
            "username": "alerts@example.com"
        })
        .to_string(),
        secret_refs_json: serde_json::json!({
            "smtpUrl": "smtp.feishu.cn",
            "password": "direct-smtp-password"
        })
        .to_string(),
        target_redacted: "ops@example.com".to_owned(),
        safety_policy_json: None,
    };

    let resolved = notification_channel_from_delivery_config(&channel)
        .unwrap_or_else(|| panic!("email channel should resolve from split SMTP config"));
    match resolved {
        NotificationChannel::Email { smtp_url, password, .. } => {
            assert_eq!(smtp_url.as_deref(), Some("smtps://smtp.feishu.cn:465"));
            assert_eq!(password.as_deref(), Some("direct-smtp-password"));
        }
        other => panic!("expected email channel, got {other:?}"),
    }
}


#[tokio::test]
async fn running_job_instance_event_materializes_message_and_delivery_attempts() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "billing-running".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("demo.running".to_owned()),
            processor_type: None,
            script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
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
        .update_status(&instance.id, InstanceStatus::Running)
        .await
        .unwrap_or_else(|error| panic!("status should update: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "ops".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"url":"https://hooks.example.com/services/running-token"})
                .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some(job.id.clone()),
            name: "job running".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["running"],"eventTypes":["job_instance.running"]}).to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: "info".to_owned(),
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
        tikeo_storage::NotificationTemplateRepository::new(channels.db()),
        jobs,
    );
    let emitted = center
        .emit_job_instance_event(
            &instance,
            JobNotificationEvent::Running,
            Some("dispatched to worker worker-1"),
        )
        .await
        .unwrap_or_else(|error| panic!("running notification should emit: {error}"));

    assert_eq!(emitted.matched_policies, 1);
    assert_eq!(emitted.messages_created, 1);
    assert_eq!(emitted.delivery_attempts_created, 1);
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline[0].event_type, "job_instance.running");
    assert_eq!(timeline[0].severity, "info");
    let payload: serde_json::Value = serde_json::from_str(&timeline[0].payload_json)
        .unwrap_or_else(|error| panic!("payload should parse: {error}"));
    assert_eq!(payload["status"], "running");
    assert_eq!(payload["eventType"], "job_instance.running");
    let delivery = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert_eq!(delivery[0].retry_state, "retry_pending");
}

#[tokio::test]
async fn job_instance_event_materializes_message_and_delivery_attempts() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let attempts = NotificationDeliveryAttemptRepository::new(db.clone());
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
            canary_policy: None,
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
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "ops".to_owned(),
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
            owner_type: "job".to_owned(),
            owner_id: Some(job.id.clone()),
            name: "job failures".to_owned(),
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

    let center = NotificationCenter::new(
        channels.clone(),
        policies,
        messages.clone(),
        attempts.clone(),
        tikeo_storage::NotificationTemplateRepository::new(channels.db()),
        jobs,
    );
    let emitted = center
        .emit_job_instance_event(&instance, JobNotificationEvent::Failed, Some("exit 2"))
        .await
        .unwrap_or_else(|error| panic!("notification should emit: {error}"));

    assert_eq!(emitted.matched_policies, 1);
    assert_eq!(emitted.messages_created, 1);
    assert_eq!(emitted.delivery_attempts_created, 1);
    let deduped = center
        .emit_job_instance_event(&instance, JobNotificationEvent::Failed, Some("exit 2"))
        .await
        .unwrap_or_else(|error| panic!("duplicate notification should dedupe: {error}"));
    assert_eq!(deduped.matched_policies, 1);
    assert_eq!(deduped.messages_created, 0);
    assert_eq!(deduped.delivery_attempts_created, 0);
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert_eq!(timeline[0].event_type, "job_instance.failed");
    assert!(!timeline[0].payload_json.contains("top-secret-token"));
    let delivery = attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("attempts should list: {error}"));
    assert_eq!(delivery[0].retry_state, "retry_pending");
    assert_eq!(delivery[0].target_redacted, "https://hooks.example.com/...");
}
