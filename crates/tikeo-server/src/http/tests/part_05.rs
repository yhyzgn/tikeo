    #[tokio::test]
    async fn broadcast_trigger_filters_workers_by_namespace_and_app() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        let worker_a = registry.register(worker("worker-a", "billing"), tx1).await;
        registry
            .register(worker("worker-b", "analytics"), tx2)
            .await;
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"broadcast-filter"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));

        let triggered = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"triggerType":"api","executionMode":"broadcast"}"#,
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("triggered instance should contain id"));
        assert_eq!(triggered["data"]["executionMode"], "broadcast");

        let attempts =
            request_with(app, &format!("/api/v1/instances/{instance_id}/attempts")).await;
        let body = axum::body::to_bytes(attempts.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"]["items"][0]["workerId"], worker_a.worker_id);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_create_validate_run_and_advance_returns_envelopes() {
        let app = router().await;
        let create = post_json(
            app.clone(),
            "/api/v1/workflows",
            r#"{"name":"demo-flow","definition":{"nodes":[{"key":"start","name":"Start","kind":"job","jobId":"job-demo"},{"key":"fanout","name":"Fanout","kind":"map","mapItems":[{"shard":1},{"shard":2}]},{"key":"child","name":"Child","kind":"sub_workflow","childWorkflowId":"wf_child"}],"edges":[{"from":"start","to":"fanout","condition":"on_success"},{"from":"fanout","to":"child","condition":"always"}]}}"#,
        )
        .await;
        assert_eq!(create["code"], 0);
        let workflow_id = create["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow id should exist"));

        let validate = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/validate"),
            "{}",
        )
        .await;
        assert_eq!(validate["data"]["valid"], true);

        let dry_run = post_json(
            app.clone(),
            "/api/v1/workflows/dry-run",
            r#"{"nodes":[{"key":"start","kind":"job","jobId":"job-demo"}],"edges":[]}"#,
        )
        .await;
        assert_eq!(dry_run["data"]["validation"]["valid"], true);
        assert_eq!(dry_run["data"]["startNodes"][0], "start");

        let run = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/run"),
            r#"{"triggerType":"api"}"#,
        )
        .await;
        assert_eq!(run["code"], 0);
        assert_eq!(run["data"]["status"], "pending");
        assert_eq!(run["data"]["nodes"][0]["status"], "queued");
        let instance_id = run["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow instance id should exist"));

        let materialized_job = post_json(
            app.clone(),
            "/api/v1/workflow-instances/materialize-next",
            "{}",
        )
        .await;
        assert_eq!(materialized_job["code"], 0);
        assert_eq!(materialized_job["data"]["node"]["nodeKey"], "start");
        assert!(materialized_job["data"]["node"]["jobInstanceId"].is_string());

        let advanced = post_json(
            app.clone(),
            &format!("/api/v1/workflow-instances/{instance_id}/advance"),
            r#"{"nodeKey":"start","status":"succeeded","message":"ok"}"#,
        )
        .await;
        assert_eq!(advanced["code"], 0);
        assert_eq!(advanced["data"]["queuedNodes"][0], "fanout");
        assert_eq!(advanced["data"]["instance"]["status"], "running");

        let materialized_map = post_json(
            app.clone(),
            "/api/v1/workflow-instances/materialize-next",
            "{}",
        )
        .await;
        assert_eq!(materialized_map["data"]["node"]["nodeKey"], "fanout");
        assert_eq!(
            materialized_map["data"]["shards"].as_array().map(Vec::len),
            Some(2)
        );

        let shards = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/workflow-instances/{instance_id}/shards"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let body = axum::body::to_bytes(shards.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(2));
        let shard_id = json["data"][0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("shard id should exist"));
        let shard_completed = post_json(
            app.clone(),
            &format!("/api/v1/workflow-shards/{shard_id}/complete"),
            r#"{"status":"succeeded","output":{"ok":true},"message":"done"}"#,
        )
        .await;
        assert_eq!(shard_completed["code"], 0);
        assert_eq!(shard_completed["data"]["shard"]["status"], "succeeded");

        let queue = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/dispatch-queue").await)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let body = axum::body::to_bytes(queue.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(json["data"]["items"].as_array().is_some());

        assert_workflow_audit_actions(app.clone()).await;
    }

    async fn assert_workflow_audit_actions(app: axum::Router) {
        let audit = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/audit-logs").await)
            .await
            .unwrap_or_else(|error| panic!("audit logs request should succeed: {error}"));
        let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
        let actions: Vec<_> = json["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("audit items should exist"))
            .iter()
            .filter(|item| {
                item["resource_type"] == "workflow"
                    || item["resource_type"] == "workflow_instance"
                    || item["resource_type"] == "workflow_node_instance"
            })
            .map(|item| item["action"].as_str().unwrap_or_default().to_owned())
            .collect();
        for expected in [
            "create",
            "validate",
            "dry-run",
            "run",
            "advance",
            "materialize",
        ] {
            assert!(
                actions.iter().any(|action| action == expected),
                "missing workflow audit action {expected}; got {actions:?}"
            );
        }
    }



    #[tokio::test]
    async fn workflow_notification_node_requires_channel_refs_or_policy_mode() {
        let app = router().await;
        let invalid = post_json(
            app.clone(),
            "/api/v1/workflows/dry-run",
            r#"{"nodes":[{"key":"notify","kind":"notification","config":{"channel":"webhook","target":"https://hooks.example.com/legacy","template":"legacy raw target"}}],"edges":[]}"#,
        )
        .await;
        assert_eq!(invalid["code"], 0);
        assert_eq!(invalid["data"]["validation"]["valid"], false);
        assert!(invalid["data"]["validation"]["errors"]
            .as_array()
            .unwrap_or_else(|| panic!("validation errors should be an array"))
            .iter()
            .any(|error| error.as_str().is_some_and(|message| {
                message.contains(
                    "notification node notify requires config.channelRefs or config.usePolicies=true",
                )
            })));

        let policy_mode = post_json(
            app,
            "/api/v1/workflows/dry-run",
            r#"{"nodes":[{"key":"notify","kind":"notification","config":{"usePolicies":true}}],"edges":[]}"#,
        )
        .await;
        assert_eq!(policy_mode["code"], 0);
        assert_eq!(policy_mode["data"]["validation"]["valid"], true);
    }



    #[tokio::test]
    async fn workflow_notification_node_validates_registered_channel_and_template_refs() {
        let app = router().await;
        let enabled_channel = post_json(
            app.clone(),
            "/api/v1/notification-channels",
            r#"{"scopeType":"global","name":"Workflow Slack","provider":"slack","enabled":true,"config":{"url":"https://hooks.slack.com/services/T000/B000/enabled","messageType":"text","template":{"text":"{{subject}}"}}}"#,
        )
        .await;
        assert_eq!(enabled_channel["code"], 0);
        let channel_id = enabled_channel["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("enabled channel id should exist"));

        let disabled_channel = post_json(
            app.clone(),
            "/api/v1/notification-channels",
            r#"{"scopeType":"global","name":"Disabled Workflow Slack","provider":"slack","enabled":false,"config":{"url":"https://hooks.slack.com/services/T000/B000/disabled","messageType":"text","template":{"text":"{{subject}}"}}}"#,
        )
        .await;
        assert_eq!(disabled_channel["code"], 0);
        let disabled_channel_id = disabled_channel["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("disabled channel id should exist"));

        let enabled_template = post_json(
            app.clone(),
            "/api/v1/notification-templates",
            r#"{"templateKey":"workflow.slack.enabled","name":"Workflow Slack enabled","provider":"slack","messageType":"text","enabled":true,"body":{"text":"{{subject}}"}}"#,
        )
        .await;
        assert_eq!(enabled_template["code"], 0);

        let disabled_template = post_json(
            app.clone(),
            "/api/v1/notification-templates",
            r#"{"templateKey":"workflow.slack.disabled","name":"Workflow Slack disabled","provider":"slack","messageType":"text","enabled":false,"body":{"text":"{{subject}}"}}"#,
        )
        .await;
        assert_eq!(disabled_template["code"], 0);

        let mismatched_template = post_json(
            app.clone(),
            "/api/v1/notification-templates",
            r#"{"templateKey":"workflow.feishu.mismatch","name":"Workflow Feishu mismatch","provider":"feishu","messageType":"text","enabled":true,"body":{"text":"{{subject}}"}}"#,
        )
        .await;
        assert_eq!(mismatched_template["code"], 0);

        for (body, expected) in [
            (
                r#"{"nodes":[{"key":"notify","kind":"notification","config":{"channelRefs":[{"channelId":"notification-channel-missing"}]}}],"edges":[]}"#.to_owned(),
                "notification node notify channel does not exist: notification-channel-missing",
            ),
            (
                format!(
                    r#"{{"nodes":[{{"key":"notify","kind":"notification","config":{{"channelRefs":[{{"channelId":"{disabled_channel_id}"}}]}}}}],"edges":[]}}"#
                ),
                "notification node notify channel is disabled",
            ),
            (
                format!(
                    r#"{{"nodes":[{{"key":"notify","kind":"notification","config":{{"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"workflow.template.missing"}}}}],"edges":[]}}"#
                ),
                "notification node notify template does not exist: workflow.template.missing",
            ),
            (
                format!(
                    r#"{{"nodes":[{{"key":"notify","kind":"notification","config":{{"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"workflow.slack.disabled"}}}}],"edges":[]}}"#
                ),
                "notification node notify template is disabled: workflow.slack.disabled",
            ),
            (
                format!(
                    r#"{{"nodes":[{{"key":"notify","kind":"notification","config":{{"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"workflow.feishu.mismatch"}}}}],"edges":[]}}"#
                ),
                "notification node notify template provider feishu does not match channel provider(s): slack",
            ),
        ] {
            let dry_run = post_json(app.clone(), "/api/v1/workflows/dry-run", &body).await;
            assert_eq!(dry_run["code"], 0);
            let errors = dry_run["data"]["validation"]["errors"]
                .as_array()
                .unwrap_or_else(|| panic!("validation errors should be an array"));
            assert!(
                errors.iter().any(|error| error.as_str().is_some_and(|message| message.contains(expected))),
                "dry-run should report {expected}; got {errors:?}"
            );

            let create_body = format!(r#"{{"name":"invalid-notification-ref","definition":{body}}}"#);
            let create = app
                .clone()
                .oneshot(
                    admin_json_request_builder(
                        app.clone(),
                        "POST",
                        "/api/v1/workflows",
                        &create_body,
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("workflow create should respond: {error}"));
            assert_eq!(create.status(), axum::http::StatusCode::BAD_REQUEST);
            let create_body_bytes = axum::body::to_bytes(create.into_body(), usize::MAX)
                .await
                .unwrap_or_else(|error| panic!("create body should collect: {error}"));
            let create_json: Value = serde_json::from_slice(&create_body_bytes)
                .unwrap_or_else(|error| panic!("create body should be JSON: {error}"));
            assert!(
                create_json["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected)),
                "create should reject {expected}; got {create_json}"
            );
        }

        let valid_definition = format!(
            r#"{{"nodes":[{{"key":"notify","kind":"notification","config":{{"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"workflow.slack.enabled"}}}}],"edges":[]}}"#
        );
        let created = post_json(
            app.clone(),
            "/api/v1/workflows",
            &format!(r#"{{"name":"valid-notification-ref","definition":{valid_definition}}}"#),
        )
        .await;
        assert_eq!(created["code"], 0);
        let workflow_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow id should exist"));

        let validate = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/validate"),
            "{}",
        )
        .await;
        assert_eq!(validate["code"], 0);
        assert_eq!(validate["data"]["valid"], true);

        let invalid_update_definition = format!(
            r#"{{"nodes":[{{"key":"notify","kind":"notification","config":{{"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"workflow.feishu.mismatch"}}}}],"edges":[]}}"#
        );
        let update = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    &format!("/api/v1/workflows/{workflow_id}"),
                    &format!(
                        r#"{{"name":"invalid-update","definition":{invalid_update_definition}}}"#
                    ),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("workflow update should respond: {error}"));
        assert_eq!(update.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(update.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("update body should collect: {error}"));
        let update_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("update body should be JSON: {error}"));
        assert!(update_json["message"].as_str().is_some_and(|message| {
            message.contains("notification node notify template provider feishu does not match channel provider(s): slack")
        }));
    }


    #[tokio::test]
    async fn workflow_approval_advance_records_audit_log() {
        let app = router().await;
        let create = post_json(
            app.clone(),
            "/api/v1/workflows",
            r#"{"name":"approval-audit","definition":{"nodes":[{"key":"approve","kind":"approval","config":{"approvers":"ops"}},{"key":"done","kind":"end"}],"edges":[{"from":"approve","to":"done","condition":"on_success"}]}}"#,
        )
        .await;
        assert_eq!(create["code"], 0);
        let workflow_id = create["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow id should exist"));
        let run = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/run"),
            r#"{"triggerType":"api"}"#,
        )
        .await;
        let instance_id = run["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow instance id should exist"));
        let materialized = post_json(app.clone(), "/api/v1/workflow-instances/materialize-next", "{}")
            .await;
        assert_eq!(materialized["data"]["node"]["nodeKey"], "approve");
        assert_eq!(materialized["data"]["node"]["status"], "running");

        let advanced = post_json(
            app.clone(),
            &format!("/api/v1/workflow-instances/{instance_id}/advance"),
            r#"{"nodeKey":"approve","status":"succeeded","message":"approved by ops"}"#,
        )
        .await;
        assert_eq!(advanced["code"], 0);
        assert_eq!(advanced["data"]["queuedNodes"][0], "done");

        let audit = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/audit-logs?action=advance&resource_type=workflow_instance&resource_id={instance_id}&page_size=1"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("approval advance audit should respond: {error}"));
        assert!(audit.status().is_success());
        let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
        assert_eq!(json["data"]["items"][0]["action"], "advance");
        assert_eq!(json["data"]["items"][0]["resource_type"], "workflow_instance");
        assert_eq!(json["data"]["items"][0]["resource_id"], instance_id);
        assert!(json["data"]["items"][0]["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("node=approve status=succeeded")));
    }


    #[tokio::test]
    async fn bootstrap_registers_first_admin_once_and_auto_logs_in() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let status = request_with(app.clone(), "/api/v1/auth/bootstrap").await;
        assert_eq!(status.status(), axum::http::StatusCode::OK);
        let status_body = axum::body::to_bytes(status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("bootstrap status body should collect: {error}"));
        let status_json: Value = serde_json::from_slice(&status_body)
            .unwrap_or_else(|error| panic!("bootstrap status should be JSON: {error}"));
        assert_eq!(status_json["data"]["initialized"], false);
        assert_eq!(status_json["data"]["registrationOpen"], true);

        let payload = r#"{"username":"bootstrap_admin","email":"bootstrap.admin@example.com","password":"Tikeo@2026!","confirmPassword":"Tikeo@2026!"}"#;
        let registered = post_json_without_auth(app.clone(), "/api/v1/auth/bootstrap/register", payload).await;
        assert_eq!(registered["data"]["username"], "bootstrap_admin");
        assert_eq!(registered["data"]["roles"][0], "owner");
        assert!(registered["data"]["token"].as_str().is_some_and(|token| !token.is_empty()));

        let closed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/bootstrap/register")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_owned()))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(closed.status(), axum::http::StatusCode::FORBIDDEN);

        let users_response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/users").await)
            .await
            .unwrap_or_else(|error| panic!("users route should respond: {error}"));
        let users_body = axum::body::to_bytes(users_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("users body should collect: {error}"));
        let users_json: Value = serde_json::from_slice(&users_body)
            .unwrap_or_else(|error| panic!("users body should be JSON: {error}"));
        assert_eq!(users_json["data"][0]["bootstrapAdmin"], true);
        assert_eq!(users_json["data"][0]["email"], "bootstrap.admin@example.com");
    }

    #[tokio::test]
    async fn role_management_api_controls_permission_menu_and_ui_action_matrices() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;

        let catalog = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/permissions/catalog")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("catalog should respond: {error}"));
        assert!(catalog.status().is_success());
        let body = axum::body::to_bytes(catalog.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let catalog_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let roles_read_id = catalog_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("catalog should be an array"))
            .iter()
            .find(|item| item["resource"] == "roles" && item["action"] == "read")
            .and_then(|item| item["id"].as_str())
            .unwrap_or_else(|| panic!("roles:read permission should exist"));

        let created = post_json_raw(
            app.clone(),
            "/api/v1/roles",
            &format!(
                r#"{{"name":"tenant-auditor","displayName":"Tenant Auditor","description":"Read role matrix","enabled":true,"permissionIds":["{roles_read_id}"],"menuKeys":["/roles"],"uiActionKeys":[]}}"#
            ),
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["name"], "tenant-auditor");
        assert_eq!(created["data"]["menuKeys"][0], "/roles");

        let roles = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/roles")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("roles should respond: {error}"));
        let body = axum::body::to_bytes(roles.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let roles_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(roles_json["data"].as_array().is_some_and(|items| {
            items.iter().any(|item| item["name"] == "owner" && item["builtin"] == true)
        }));
    }

    #[tokio::test]
    async fn user_management_and_rbac_integration() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        // 1. Get users list (should only contain the bootstrapped admin)
        let response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/users").await)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"][0]["username"], "bootstrap_admin");

        // 2. Create an operator user
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/users",
                    r#"{"username":"test_operator","email":"operator@example.com","password":"Password@123","role":"operator"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let user_id = json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("expected JSON string"))
            .to_owned();

        assert_owner_assignment_is_rejected(app.clone(), &user_id).await;
        assert_assignable_role_change_invalidates_sessions_and_grants_access(app.clone(), &user_id)
            .await;
        delete_user_and_assert_audit(app.clone(), &user_id).await;
    }

    async fn assert_owner_assignment_is_rejected(app: axum::Router, user_id: &str) {
        let create_owner = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/users",
                    r#"{"username":"owner_clone","email":"owner.clone@example.com","password":"Password@123","role":"owner"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("owner create request should respond: {error}"));
        assert_eq!(create_owner.status(), axum::http::StatusCode::BAD_REQUEST);

        let update_owner = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/users/{user_id}"),
                    r#"{"role":"owner"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("owner update request should respond: {error}"));
        assert_eq!(update_owner.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    async fn assert_assignable_role_change_invalidates_sessions_and_grants_access(
        app: axum::Router,
        user_id: &str,
    ) {
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"test_operator","password":"Password@123"}"#,
        )
        .await;
        assert_eq!(login["code"], 0);
        let operator_token = login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("expected JSON string"))
            .to_owned();

        // 4. Verification: Operator is not allowed to create users (Admin only) -> Should return 403 Forbidden
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {operator_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("test operation should succeed: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

        let user_admin_role = create_user_admin_role(app.clone()).await;

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/users/{user_id}"),
                    &format!(r#"{{"role":"{user_admin_role}"}}"#),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let old_token_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {operator_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("old token request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("old token request should respond: {error}"));
        assert_eq!(old_token_response.status(), axum::http::StatusCode::UNAUTHORIZED);

        // 6. Perform a fresh login to fetch new token (the old token was invalidated on role change)
        let login_again = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"test_operator","password":"Password@123"}"#,
        )
        .await;
        assert_eq!(login_again["code"], 0);
        let new_operator_token = login_again["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("expected JSON string"))
            .to_owned();

        // Verify that updated assignable role now HAS access to user list (returns 200 OK)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {new_operator_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("test operation should succeed: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    async fn create_user_admin_role(app: axum::Router) -> String {
        let admin = admin_token(app.clone()).await;
        let catalog = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/permissions/catalog")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("catalog request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("catalog request should respond: {error}"));
        assert!(catalog.status().is_success());
        let body = axum::body::to_bytes(catalog.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("catalog body should collect: {error}"));
        let catalog_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("catalog body should be JSON: {error}"));
        let permission_ids = catalog_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("catalog should be an array"))
            .iter()
            .filter(|item| item["resource"] == "users" && matches!(item["action"].as_str(), Some("read" | "manage")))
            .filter_map(|item| item["id"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(permission_ids.len(), 2);
        let permission_json = serde_json::to_string(&permission_ids)
            .unwrap_or_else(|error| panic!("permission ids should serialize: {error}"));
        let role_name = "user-admin-test";
        let created = post_json_raw(
            app,
            "/api/v1/roles",
            &format!(
                r#"{{"name":"{role_name}","displayName":"User Admin Test","description":"Can manage users in tests","enabled":true,"permissionIds":{permission_json},"menuKeys":["/users"],"uiActionKeys":["users.create","users.edit","users.delete"]}}"#
            ),
            Some(&admin),
        )
        .await;
        assert_eq!(created["data"]["assignable"], true);
        role_name.to_owned()
    }

    async fn delete_user_and_assert_audit(app: axum::Router, user_id: &str) {
        let response = app
            .clone()
            .oneshot(
                admin_request_builder(app.clone(), "DELETE", format!("/api/v1/users/{user_id}"))
                    .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        for action in ["create", "update", "delete"] {
            let audit = app
                .clone()
                .oneshot(
                    admin_request_builder(
                        app.clone(),
                        "GET",
                        format!("/api/v1/audit-logs?action={action}&resource_type=user&resource_id={user_id}&page_size=1"),
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("user audit should respond: {error}"));
            assert!(audit.status().is_success());
            let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
                .await
                .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
            let json: Value = serde_json::from_slice(&body)
                .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
            assert_eq!(json["data"]["items"][0]["action"], action);
            assert_eq!(json["data"]["items"][0]["resource_type"], "user");
            assert_eq!(json["data"]["items"][0]["resource_id"], user_id);
        }
    }

    async fn router() -> axum::Router {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ))
    }

    async fn router_with_script_signature_secret_ref(secret_ref: &str) -> axum::Router {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        router_with_state(
            AppState::new(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_script_governance_config(ScriptGovernanceConfig {
                release_signature_secret_ref: Some(secret_ref.to_owned()),
            }),
        )
    }

    async fn router_with_leader_cluster() -> axum::Router {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
            StaticCoordinator::shared(ClusterStatus {
                mode: ClusterMode::Raft,
                role: ClusterRole::Leader,
                node_id: "tikeo-0".to_owned(),
                nodes: 3,
                can_schedule: true,
                leader_fencing_token: Some("raft:term:7:node:tikeo-0".to_owned()),
                detail: "test leader with persisted fencing token".to_owned(),
            }),
        ))
    }

    #[tokio::test]
    async fn gitops_manifest_exports_yaml_and_reports_drift_diff() {
        let app = router().await;
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"gitops.echo","scheduleType":"api","processorType":"sdk","processorName":"demo.echo","enabled":true}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("job create should respond: {error}"));
        assert!(created.status().is_success());

        let exported = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/gitops/manifest?namespace=default&app=billing&format=yaml",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("gitops export should respond: {error}"));
        assert!(exported.status().is_success());
        let body = axum::body::to_bytes(exported.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("export body should collect: {error}"));
        let mut json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("export body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["format"], "yaml");
        assert!(json["data"]["checksum"].as_str().is_some_and(|value| value.starts_with("sha256:")));
        assert!(json["data"]["manifestYaml"].as_str().is_some_and(|value| value.contains("apiVersion")));
        let job = json["data"]["manifest"]["resources"]
            .as_array_mut()
            .unwrap_or_else(|| panic!("manifest should contain resources"))
            .iter_mut()
            .find(|resource| resource["kind"] == "Job")
            .unwrap_or_else(|| panic!("manifest should export job resource"));
        assert_eq!(job["metadata"]["name"], "gitops.echo");
        job["spec"]["enabled"] = serde_json::Value::Bool(false);

        let desired = serde_json::json!({"manifest": json["data"]["manifest"].clone()});
        let diff = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/gitops/diff",
                    &desired.to_string(),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("gitops diff should respond: {error}"));
        assert!(diff.status().is_success());
        let body = axum::body::to_bytes(diff.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("diff body should collect: {error}"));
        let diff_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("diff body should be JSON: {error}"));
        assert_eq!(diff_json["code"], 0);
        assert_eq!(diff_json["data"]["summary"]["update"], 1);
        assert!(diff_json["data"]["changes"]
            .as_array()
            .unwrap_or_else(|| panic!("diff should contain changes"))
            .iter()
            .any(|change| change["action"] == "update"
                && change["kind"] == "Job"
                && change["diff"].as_str().is_some_and(|value| value.contains("enabled"))));
    }

    #[tokio::test]
    async fn calendar_management_crud_lists_and_audits_upsert() {
        let app = router().await;
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/calendars",
                    r#"{"namespace":"default","app":"billing","name":"cn-maintenance","timezone":"Asia/Shanghai","excludedDates":["2026-05-29"],"maintenanceWindows":[{"start":"2026-05-29T01:00:00Z","end":"2026-05-29T02:00:00Z"}]}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("calendar create should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("calendar body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("calendar body should be JSON: {error}"));
        let id = json["data"]["id"].as_str().unwrap_or_else(|| panic!("calendar should have id")).to_owned();
        assert_eq!(json["data"]["timezone"], "Asia/Shanghai");

        let listed = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/calendars?namespace=default&app=billing").await)
            .await
            .unwrap_or_else(|error| panic!("calendar list should respond: {error}"));
        assert!(listed.status().is_success());
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("calendar list body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("calendar list body should be JSON: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"][0]["name"], "cn-maintenance");

        let audit = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", format!("/api/v1/audit-logs?action=upsert&resource_type=calendar&resource_id={id}&page_size=1")).await)
            .await
            .unwrap_or_else(|error| panic!("calendar audit should respond: {error}"));
        assert!(audit.status().is_success());
    }
