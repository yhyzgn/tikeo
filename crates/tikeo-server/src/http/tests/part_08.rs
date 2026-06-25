    #[tokio::test]
    async fn workflow_notification_node_materializes_notification_center_message_and_attempt() {
        let app = router().await;
        let channel_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Workflow webhook","provider":"webhook","enabled":true,"config":{"url":"https://hooks.example.com/services/workflow-token","messageType":"json","template":{"body":{"text":"{{subject}}"}}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel create should respond: {error}"));
        assert!(channel_created.status().is_success());
        let body = axum::body::to_bytes(channel_created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let channel_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let channel_id = channel_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("channel id should exist"));

        let template_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"workflow.node.notice","name":"Workflow node notice","provider":"webhook","messageType":"json","enabled":true,"body":{"body":{"text":"Workflow {{resourceId}} notification","node":"{{resourceId}}","event":"{{eventType}}"}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification template create should respond: {error}"));
        assert!(template_created.status().is_success());
        let body = axum::body::to_bytes(template_created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("template body should collect: {error}"));
        let template_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("template body should be JSON: {error}"));
        let template_id = template_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("template id should exist"));

        let workflow_body = serde_json::json!({
            "name": "workflow-notify",
            "definition": {
                "nodes": [
                    {
                        "key": "notify",
                        "name": "Notify",
                        "kind": "notification",
                        "config": {
                            "channelRefs": [{"channelId": channel_id}],
                            "templateRef": template_id,
                            "subject": "Workflow notification requested",
                            "body": "A workflow notification node was materialized",
                            "severity": "warning"
                        }
                    }
                ],
                "edges": []
            }
        })
        .to_string();
        let workflow = post_json(app.clone(), "/api/v1/workflows", &workflow_body).await;
        assert_eq!(workflow["code"], 0);
        let workflow_id = workflow["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow id should exist"));

        let run = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/run"),
            r#"{"triggerType":"api"}"#,
        )
        .await;
        assert_eq!(run["code"], 0);
        assert!(run["data"]["id"].as_str().is_some_and(|value| !value.is_empty()));

        let materialized = post_json(
            app.clone(),
            "/api/v1/workflow-instances/materialize-next",
            "{}",
        )
        .await;
        assert_eq!(materialized["code"], 0);
        assert_eq!(materialized["data"]["node"]["nodeKey"], "notify");
        assert_eq!(materialized["data"]["node"]["status"], "succeeded");

        let workflow_node_instance_id = materialized["data"]["node"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow node instance id should exist"));

        let messages = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/notification-messages?source_id={workflow_node_instance_id}&event_type=workflow_node.notification_requested"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification messages should respond: {error}"));
        assert!(messages.status().is_success());
        let body = axum::body::to_bytes(messages.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("messages body should collect: {error}"));
        let messages_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("messages body should be JSON: {error}"));
        assert_eq!(messages_json["code"], 0);
        let messages = messages_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("messages should be an array"));
        assert_eq!(messages.len(), 1, "expected one workflow notification message: {messages_json}");
        let message = &messages[0];
        assert_eq!(message["sourceType"], "workflow_node_instance");
        assert_eq!(message["sourceId"], workflow_node_instance_id);
        assert_eq!(message["eventType"], "workflow_node.notification_requested");
        assert_eq!(message["resourceType"], "workflow_node");
        assert_eq!(message["resourceId"], "notify");
        assert_eq!(message["severity"], "warning");
        assert_eq!(message["subject"], "Workflow notification requested");
        assert_eq!(message["body"], "A workflow notification node was materialized");
        let payload = message["payloadJson"]
            .as_str()
            .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
            .unwrap_or_else(|| panic!("payloadJson should be JSON: {}", message["payloadJson"]));
        assert_eq!(payload["workflowNodeInstanceId"], workflow_node_instance_id);
        assert_eq!(payload["templateRef"], template_id);
        assert_eq!(payload["templateKey"], "workflow.node.notice");
        let policy_id = message["policyId"]
            .as_str()
            .unwrap_or_else(|| panic!("message policy id should exist"));

        let attempts = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/notification-delivery-attempts?policy_id={policy_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("delivery attempts should respond: {error}"));
        assert!(attempts.status().is_success());
        let body = axum::body::to_bytes(attempts.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("attempts body should collect: {error}"));
        let attempts_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("attempts body should be JSON: {error}"));
        let attempts = attempts_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("attempts should be an array"));
        assert_eq!(attempts.len(), 1, "expected one delivery attempt: {attempts_json}");
        assert_eq!(attempts[0]["channelId"], channel_id);
        assert_eq!(attempts[0]["retryState"], "retry_pending");
    }

    #[tokio::test]
    async fn job_notification_binding_api_compiles_to_job_owned_policy_and_preview() {
        let app = router().await;
        let _scope = post_json(
            app.clone(),
            "/api/v1/apps",
            r#"{"namespace":"default","name":"ops"}"#,
        )
        .await;
        let job = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"ops","name":"nightly-import","scheduleType":"api","executionMode":"single"}"#,
        )
        .await;
        assert_eq!(job["code"], 0);
        let job_id = job["data"]["id"].as_str().unwrap_or_else(|| panic!("job id should exist"));
        let channel = post_json(
            app.clone(),
            "/api/v1/notification-channels",
            r#"{"scopeType":"global","name":"Ops webhook","provider":"webhook","enabled":true,"config":{"url":"https://hooks.example.com/services/job-token","messageType":"json","template":{"body":{"text":"{{jobName}} {{instanceId}} {{logsUrl}}"}}}}"#,
        )
        .await;
        assert_eq!(channel["code"], 0);
        let channel_id = channel["data"]["id"].as_str().unwrap_or_else(|| panic!("channel id should exist"));
        let template = post_json(
            app.clone(),
            "/api/v1/notification-templates",
            r#"{"templateKey":"job.failure.card","name":"Job failure card","provider":"webhook","messageType":"json","enabled":true,"body":{"body":{"job":"{{jobName}}","instance":"{{instanceId}}","logs":"{{logsUrl}}","event":"{{eventType}}"}}}"#,
        )
        .await;
        assert_eq!(template["code"], 0);
        let template_id = template["data"]["id"].as_str().unwrap_or_else(|| panic!("template id should exist"));

        let request = serde_json::json!({
            "name": "Nightly import failure notifications",
            "trigger": "failure",
            "channelIds": [channel_id],
            "templateRef": template_id,
            "includeLogLink": true,
            "includeLogExcerpt": true,
            "logExcerptLines": 40
        })
        .to_string();
        let validate = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings:validate"),
            &request,
        )
        .await;
        assert_eq!(validate["code"], 0);
        assert_eq!(validate["data"]["valid"], true);
        assert!(validate["data"]["eventTypes"].as_array().unwrap_or_else(|| panic!("eventTypes should be array")).iter().any(|item| item.as_str() == Some("job_instance.failed")));

        let preview = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings:preview"),
            &request,
        )
        .await;
        assert_eq!(preview["code"], 0);
        assert_eq!(preview["data"]["sampleContext"]["jobName"], "nightly-import");
        assert_eq!(preview["data"]["sampleContext"]["consoleUrl"], "/public/instances/preview-instance/console");
        assert_eq!(preview["data"]["renderedTemplate"]["body"]["logs"], "/public/instances/preview-instance/console");

        let created = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings"),
            &request,
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["jobId"], job_id);
        assert_eq!(created["data"]["trigger"], "failure");
        assert_eq!(created["data"]["templateRef"], template_id);
        assert_eq!(created["data"]["policy"]["ownerType"], "job");
        assert_eq!(created["data"]["policy"]["eventFamily"], "job_instance");
        let filter: Value = serde_json::from_str(created["data"]["policy"]["eventFilterJson"].as_str().unwrap_or("{}"))
            .unwrap_or_else(|error| panic!("event filter should be JSON: {error}"));
        assert_eq!(filter["includeLogLink"], true);
        assert_eq!(filter["includeLogExcerpt"], true);

        let listed = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", format!("/api/v1/jobs/{job_id}/notification-bindings")).await)
            .await
            .unwrap_or_else(|error| panic!("binding list should respond: {error}"));
        assert!(listed.status().is_success());
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("binding list body should collect: {error}"));
        let listed_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("binding list should be JSON: {error}"));
        assert_eq!(listed_json["code"], 0);
        assert_eq!(listed_json["data"].as_array().unwrap_or_else(|| panic!("bindings should be array")).len(), 1);
    }


    #[tokio::test]
    async fn job_notification_binding_accepts_running_status_without_localized_event_leak() {
        let app = router().await;
        let _scope = post_json(
            app.clone(),
            "/api/v1/apps",
            r#"{"namespace":"default","name":"ops"}"#,
        )
        .await;
        let job = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"ops","name":"running-job","scheduleType":"api","executionMode":"single"}"#,
        )
        .await;
        assert_eq!(job["code"], 0);
        let job_id = job["data"]["id"].as_str().unwrap_or_else(|| panic!("job id should exist"));
        let channel = post_json(
            app.clone(),
            "/api/v1/notification-channels",
            r#"{"scopeType":"global","name":"Ops webhook","provider":"webhook","enabled":true,"config":{"url":"https://hooks.example.com/services/running-token","messageType":"json","template":{"body":{"text":"{{jobName}} {{eventType}}"}}}}"#,
        )
        .await;
        assert_eq!(channel["code"], 0);
        let channel_id = channel["data"]["id"].as_str().unwrap_or_else(|| panic!("channel id should exist"));

        let request = serde_json::json!({
            "name": "Running notifications",
            "trigger": "advanced",
            "eventTypes": ["job_instance.running"],
            "channelIds": [channel_id]
        })
        .to_string();
        let validate = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings:validate"),
            &request,
        )
        .await;
        assert_eq!(validate["code"], 0);
        assert_eq!(validate["data"]["valid"], true);
        assert_eq!(validate["data"]["eventTypes"][0], "job_instance.running");

        let localized_request = serde_json::json!({
            "name": "Localized running notifications",
            "trigger": "advanced",
            "eventTypes": ["job_instance.运行中"],
            "channelIds": [channel_id]
        })
        .to_string();
        let localized_validate = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings:validate"),
            &localized_request,
        )
        .await;
        assert_eq!(localized_validate["code"], 0);
        assert_eq!(localized_validate["data"]["valid"], true);
        assert_eq!(localized_validate["data"]["eventTypes"][0], "job_instance.running");

        let created = post_json(
            app,
            &format!("/api/v1/jobs/{job_id}/notification-bindings"),
            &request,
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["eventTypes"][0], "job_instance.running");
        let filter: Value = serde_json::from_str(created["data"]["policy"]["eventFilterJson"].as_str().unwrap_or("{}"))
            .unwrap_or_else(|error| panic!("event filter should be JSON: {error}"));
        assert_eq!(filter["eventTypes"][0], "job_instance.running");
        assert_eq!(filter["statuses"][0], "running");
    }

    #[tokio::test]
    async fn notification_message_trace_includes_job_instance_attempts_and_redacted_logs() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let logs = JobInstanceLogRepository::new(db.clone());
        let attempts_repo = JobInstanceAttemptRepository::new(db.clone());
        let users = UserRepository::new(db.clone());
        let scripts = ScriptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let channels = tikeo_storage::NotificationChannelRepository::new(db.clone());
        let policies = tikeo_storage::NotificationPolicyRepository::new(db.clone());
        let messages = tikeo_storage::NotificationMessageRepository::new(db.clone());
        let delivery_attempts = tikeo_storage::NotificationDeliveryAttemptRepository::new(db.clone());

        let job = jobs
            .create_job(CreateJob {
                created_by: Some("test".to_owned()),
                namespace: "default".to_owned(),
                app: "ops".to_owned(),
                name: "traceable-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "skip".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
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
            .unwrap_or_else(|| panic!("job should exist"));
        for (sequence, message) in [
            (1, "starting job".to_owned()),
            (
                2,
                "password=plain token:abc secret=hidden routingKey:rk signingKey=sig".to_owned(),
            ),
        ] {
            logs.append(AppendJobInstanceLog {
                instance_id: instance.id.clone(),
                worker_id: "worker-1".to_owned(),
                level: "info".to_owned(),
                message,
                sequence,
            })
            .await
            .unwrap_or_else(|error| panic!("log should append: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        }
        let channel = channels
            .create_channel(tikeo_storage::CreateNotificationChannel {
                scope_type: "global".to_owned(),
                namespace: None,
                app: None,
                worker_pool: None,
                name: "Trace webhook".to_owned(),
                provider: "webhook".to_owned(),
                enabled: true,
                config_json: serde_json::json!({"url":"https://hooks.example.com/trace","messageType":"json"}).to_string(),
                secret_refs_json: "{}".to_owned(),
                safety_policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("channel should create: {error}"));
        let policy = policies
            .create_policy(tikeo_storage::CreateNotificationPolicy {
                owner_type: "job".to_owned(),
                owner_id: Some(job.id.clone()),
                name: "Trace policy".to_owned(),
                event_family: "job_instance".to_owned(),
                event_filter_json: serde_json::json!({"eventTypes":["job_instance.failed"]}).to_string(),
                channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
                template_ref: None,
                severity: "critical".to_owned(),
                enabled: true,
                dedupe_seconds: 300,
            })
            .await
            .unwrap_or_else(|error| panic!("policy should create: {error}"));
        let message = messages
            .create_message(tikeo_storage::CreateNotificationMessage {
                source_type: "job_instance".to_owned(),
                source_id: instance.id.clone(),
                policy_id: policy.id.clone(),
                event_type: "job_instance.failed".to_owned(),
                resource_type: "job".to_owned(),
                resource_id: job.id.clone(),
                severity: "critical".to_owned(),
                subject: "job failed".to_owned(),
                body: "exit 2".to_owned(),
                payload_json: serde_json::json!({"instanceId": instance.id, "jobId": job.id}).to_string(),
                dedupe_key: "trace-dedupe".to_owned(),
                trace_id: Some("trace-1".to_owned()),
                status: "pending".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("message should create: {error}"));
        delivery_attempts
            .record_attempt(tikeo_storage::RecordNotificationDeliveryAttempt {
                message_id: message.id.clone(),
                policy_id: policy.id.clone(),
                channel_id: "channel-trace".to_owned(),
                provider: "webhook".to_owned(),
                target_redacted: "https://hooks.example.com/...".to_owned(),
                attempt: 0,
                delivered: false,
                status_code: Some(500),
                error: Some("temporary failure".to_owned()),
                retry_state: "retry_pending".to_owned(),
                next_retry_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("attempt should record: {error}"));

        let app = router_with_state(app_state!(
            jobs,
            instances,
            logs,
            attempts_repo,
            users,
            scripts,
            workflows,
            audit,
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));
        let response = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/notification-messages/{}/trace", message.id),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("trace should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("trace body should collect: {error}"));
        let trace: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("trace body should be JSON: {error}"));
        assert_eq!(trace["code"], 0);
        assert_eq!(trace["data"]["message"]["id"], message.id);
        assert_eq!(trace["data"]["policy"]["id"], policy.id);
        assert_eq!(trace["data"]["attempts"].as_array().unwrap_or_else(|| panic!("attempts should be array")).len(), 1);
        assert_eq!(trace["data"]["job"]["name"], "traceable-job");
        assert_eq!(trace["data"]["instance"]["id"], instance.id);
        assert_eq!(trace["data"]["logs"]["url"], format!("/instances/{}/logs", instance.id));
        let public_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/public/job-instances/{}/trace", instance.id))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("public request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("public notification trace should respond without auth: {error}"));
        let public_status = public_response.status();
        assert!(public_status.is_success(), "public trace status should be success: {public_status}");
        let public_body = axum::body::to_bytes(public_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("public trace body should collect: {error}"));
        let public_trace: Value = serde_json::from_slice(&public_body)
            .unwrap_or_else(|error| panic!("public trace body should be JSON: {error}"));
        assert_eq!(public_trace["code"], 0);
        assert_eq!(public_trace["data"]["instance"]["id"], instance.id);
        assert_eq!(public_trace["data"]["logs"]["url"], format!("/public/instances/{}/console", instance.id));
        let excerpt = trace["data"]["logs"]["excerpt"]
            .as_array()
            .unwrap_or_else(|| panic!("log excerpt should be array"));
        assert_eq!(excerpt.len(), 2);
        let redacted = excerpt[1]["message"].as_str().unwrap_or_default();
        assert!(redacted.contains("password=***"), "password should redact: {redacted}");
        assert!(redacted.contains("token:***"), "token should redact: {redacted}");
        assert!(redacted.contains("secret=***"), "secret should redact: {redacted}");
        assert!(!redacted.contains("plain"));
        assert!(!redacted.contains("abc"));
        assert!(!redacted.contains("hidden"));
    }

    #[tokio::test]
    async fn job_notification_binding_validation_rejects_empty_advanced_events_and_provider_mismatch() {
        let app = router().await;
        let scope_response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/apps",
                    r#"{"namespace":"default","name":"ops"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("scope create should respond: {error}"));
        let status = scope_response.status();
        let body = axum::body::to_bytes(scope_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("scope body should collect: {error}"));
        let scope: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("scope body should be JSON: {error}"));
        assert!(status.is_success(), "scope create failed with {status}: {scope}");
        assert_eq!(scope["code"], 0);
        let job_response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"ops","name":"validation-job","scheduleType":"api","executionMode":"single"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("job create should respond: {error}"));
        let status = job_response.status();
        let body = axum::body::to_bytes(job_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("job body should collect: {error}"));
        let job: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("job body should be JSON: {error}"));
        assert!(status.is_success(), "job create failed with {status}: {job}");
        assert_eq!(job["code"], 0);
        let job_id = job["data"]["id"].as_str().unwrap_or_else(|| panic!("job id should exist"));
        let channel_response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Slack","provider":"slack","enabled":true,"config":{"messageType":"text","text":"{{subject}}"},"secretRefs":{"webhookUrl":"https://hooks.slack.com/services/test"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("channel create should respond: {error}"));
        let status = channel_response.status();
        let body = axum::body::to_bytes(channel_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("channel body should collect: {error}"));
        let channel: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("channel body should be JSON: {error}"));
        assert!(status.is_success(), "channel create failed with {status}: {channel}");
        assert_eq!(channel["code"], 0);
        let channel_id = channel["data"]["id"].as_str().unwrap_or_else(|| panic!("channel id should exist"));
        let template = post_json(
            app.clone(),
            "/api/v1/notification-templates",
            r#"{"templateKey":"webhook.only","name":"Webhook only","provider":"webhook","messageType":"json","enabled":true,"body":{"body":{"text":"{{subject}}"}}}"#,
        )
        .await;
        assert_eq!(template["code"], 0);
        let template_id = template["data"]["id"].as_str().unwrap_or_else(|| panic!("template id should exist"));

        let empty_advanced = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings:validate"),
            &serde_json::json!({
                "name": "Empty advanced",
                "trigger": "advanced",
                "eventTypes": [],
                "channelIds": [channel_id]
            })
            .to_string(),
        )
        .await;
        assert_eq!(empty_advanced["code"], 0);
        assert_eq!(empty_advanced["data"]["valid"], false);
        assert!(empty_advanced["data"]["issues"].as_array().unwrap_or_else(|| panic!("issues should be array")).iter().any(|issue| issue.as_str().unwrap_or_default().contains("event type")));

        let mismatch_request = serde_json::json!({
            "name": "Provider mismatch",
            "trigger": "failure",
            "channelIds": [channel_id],
            "templateRef": template_id
        })
        .to_string();
        let validate = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/notification-bindings:validate"),
            &mismatch_request,
        )
        .await;
        assert_eq!(validate["code"], 0);
        assert_eq!(validate["data"]["valid"], false);
        assert!(validate["data"]["issues"].as_array().unwrap_or_else(|| panic!("issues should be array")).iter().any(|issue| issue.as_str().unwrap_or_default().contains("does not match")));

        let create = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/jobs/{job_id}/notification-bindings"),
                    &mismatch_request,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("mismatched binding create should respond: {error}"));
        assert!(create.status().is_client_error());
    }
