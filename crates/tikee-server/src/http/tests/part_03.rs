    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn metrics_summary_reports_storage_registry_and_alert_counts() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "metrics-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let pending = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let succeeded = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        instances
            .update_status(&succeeded.id, tikee_core::InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("instance should update: {error}"));
        assert_eq!(pending.status, tikee_core::InstanceStatus::Pending);

        let workflows = WorkflowRepository::new(db.clone());
        let workflow = workflows
            .create_workflow(CreateWorkflow {
                name: "metrics-map".to_owned(),
                definition: WorkflowDefinition {
                    nodes: vec![WorkflowNodeSpec {
                        key: "fanout".to_owned(),
                        name: Some("Fanout".to_owned()),
                        kind: Some("map".to_owned()),
                        job_id: Some("job-metrics-shard".to_owned()),
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: Some(vec![
                            serde_json::json!({"item": 1}),
                            serde_json::json!({"item": 2}),
                        ]),
                        config: None,
                    }],
                    edges: vec![],
                },
                created_by: "metrics-test".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should create: {error}"));
        let workflow_instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("workflow node should materialize: {error}"))
            .unwrap_or_else(|| panic!("workflow node should materialize"));
        assert_eq!(materialized.shards.len(), 2);
        for shard in materialized.shards {
            workflows
                .complete_workflow_shard(
                    &shard.id,
                    CompleteWorkflowShardInput {
                        status: "succeeded".to_owned(),
                        output: Some(serde_json::json!({"ok": true})),
                        message: Some("shard succeeded".to_owned()),
                    },
                )
                .await
                .unwrap_or_else(|error| panic!("workflow shard should complete: {error}"));
        }
        let completed_workflow = workflows
            .get_workflow_instance(&workflow_instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should reload: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(completed_workflow.status, "succeeded");

        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        registry
            .register(worker("metrics-worker", "billing"), tx)
            .await;

        let app = router_with_state(AppState::new(
            jobs,
            instances,
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            workflows,
            AuditLogRepository::new(db.clone()),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        app.clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Runtime governance","severity":"warning","condition":{"type":"script_governance_failure","failure_class":"script_runtime_unavailable","threshold":1},"channels":[],"enabled":true,"dedupe_seconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule route should respond: {error}"));
        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikee-dispatcher",
            "inst-metrics",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));

        let summary = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/metrics/summary").await)
            .await
            .unwrap_or_else(|error| panic!("metrics summary route should respond: {error}"));
        let status = summary.status();
        let body = axum::body::to_bytes(summary.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["workers"]["online"], 1);
        assert_eq!(json["data"]["instances"]["total"], 4);
        assert_eq!(json["data"]["instances"]["by_status"]["pending"], 3);
        assert_eq!(json["data"]["instances"]["by_status"]["succeeded"], 1);
        assert_eq!(json["data"]["queue"]["pending"], 2);
        assert_eq!(json["data"]["queue"]["running"], 0);
        assert!(
            json["data"]["queue"]["completedDispatches"]
                .as_u64()
                .is_some_and(|value| value >= 1),
            "queue summary should count completed dispatch rows"
        );
        assert!(
            json["data"]["queue"]["longestDispatchLatencySeconds"]
                .as_u64()
                .is_some(),
            "queue summary should include completed dispatch latency rollups"
        );
        assert_eq!(
            json["data"]["queue"]["oldestPendingAgeSeconds"].as_u64(),
            Some(0)
        );
        assert_eq!(
            json["data"]["queue"]["averagePendingAgeSeconds"].as_u64(),
            Some(0)
        );
        assert_eq!(json["data"]["alerts"]["total_events"], 1);
        assert_eq!(json["data"]["alerts"]["by_status"]["firing"], 1);
        assert_eq!(json["data"]["governance"]["script_failure_events"], 1);
        assert_eq!(
            json["data"]["governance"]["by_failure_class"]["script_runtime_unavailable"],
            1
        );
        assert_eq!(json["data"]["workflows"]["instancesTotal"], 1);
        assert_eq!(
            json["data"]["workflows"]["instancesByStatus"]["succeeded"],
            1
        );
        assert_eq!(json["data"]["workflows"]["shardsTotal"], 2);
        assert_eq!(
            json["data"]["workflows"]["shardsByStatus"]["succeeded"],
            2
        );
        assert_eq!(
            json["data"]["workflows"]["instanceSuccessRatio"].as_f64(),
            Some(1.0)
        );
        assert_eq!(
            json["data"]["workflows"]["shardSuccessRatio"].as_f64(),
            Some(1.0)
        );

        let metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("metrics route should respond: {error}"));
        let body = axum::body::to_bytes(metrics.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let text = String::from_utf8(body.to_vec())
            .unwrap_or_else(|error| panic!("metrics body should be utf8: {error}"));
        assert!(
            text.contains("tikee_dispatch_queue_pending_age_seconds"),
            "metrics body should expose dispatch queue pending age histogram: {text}"
        );
        assert!(
            text.contains("tikee_dispatch_queue_dispatch_latency_seconds"),
            "metrics body should expose dispatch latency histogram: {text}"
        );
        assert!(
            text.contains("tikee_job_instances_current"),
            "metrics body should expose job instance status gauges: {text}"
        );
        assert!(
            text.contains("tikee_job_instance_success_ratio"),
            "metrics body should expose job instance success ratio: {text}"
        );
        assert!(
            text.contains("tikee_script_governance_failures_current"),
            "metrics body should expose script governance failure gauges: {text}"
        );
        assert!(
            text.contains("tikee_workflow_instances_current"),
            "metrics body should expose workflow instance status gauges: {text}"
        );
        assert!(
            text.contains("tikee_workflow_instance_duration_seconds"),
            "metrics body should expose workflow instance duration histogram: {text}"
        );
        assert!(
            text.contains("tikee_workflow_shard_duration_seconds"),
            "metrics body should expose workflow shard duration histogram: {text}"
        );
    }

    #[tokio::test]
    async fn script_governance_audit_logs_filter_by_failure_reason() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "governed-script".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some("script:script-missing-runtime".to_owned()),
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("parent job should exist"));

        crate::tunnel::governance::materialize_script_governance_audit(
            &audit,
            "tikee-dispatcher",
            &instance.id,
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance audit should append: {error}"));

        let app = router_with_state(AppState::new(
            jobs,
            instances,
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            audit,
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    "/api/v1/audit-logs?resource_type=script_execution_governance&failure_reason=script_runtime_unavailable",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["total"], 1);
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            json["data"]["items"][0]["action"],
            "script_governance_failure"
        );
        assert_eq!(
            json["data"]["items"][0]["resource_type"],
            "script_execution_governance"
        );
        assert_eq!(json["data"]["items"][0]["resource_id"], instance.id);
        assert_eq!(
            json["data"]["items"][0]["failure_reason"],
            "script_runtime_unavailable"
        );
        assert_eq!(json["data"]["items"][0]["result"], "failed");
    }

    #[tokio::test]
    async fn login_succeeds_and_me_returns_principal() {
        let app = router().await;
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"tikee_init","password":"Tikee@2026!"}"#,
        )
        .await;

        assert_eq!(login["code"], 0);
        let token = login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("login should return token"))
            .to_owned();
        assert!(token.starts_with("atk_"));
        assert_eq!(login["data"]["roles"][0], "admin");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let me: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(me["code"], 0);
        assert_eq!(me["data"]["username"], "tikee_init");
    }

    #[tokio::test]
    async fn api_token_lifecycle_creates_lists_authenticates_and_revokes() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;

        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"nightly automation"}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["token"]["name"], "nightly automation");
        assert_eq!(
            created["data"]["token"]["scopes"].as_array().map(Vec::len),
            Some(0)
        );
        let token_id = created["data"]["token"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("api token id should be present"))
            .to_owned();
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();
        assert!(api_token.starts_with("atk_"));

        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/api-tokens")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(list_response.status().is_success());
        let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let list_text = String::from_utf8(list_body.to_vec())
            .unwrap_or_else(|error| panic!("body should be utf8: {error}"));
        assert!(!list_text.contains("token_hash"));
        assert!(!list_text.contains(&api_token));
        let list: Value = serde_json::from_str(&list_text)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(list["code"], 0);
        assert_eq!(list["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(list["data"][0]["id"], token_id);

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(me.status().is_success());

        let revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/auth/api-tokens/{token_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(revoke.status().is_success());

        let rejected = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(rejected.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn scoped_api_token_limits_effective_permissions() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"read only users","scopes":["users:read"]}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["token"]["scopes"][0], "users:read");
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();

        let scoped_read = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(scoped_read.status().is_success());

        let scoped_write = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {api_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"scoped-denied","password":"Secret123!","role":"viewer"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(scoped_write.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_token_policy_bounds_expiry_and_rotation_revokes_old_token() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"short lived automation","scopes":["users:read"],"expires_in_seconds":900}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["token"]["scopes"][0], "users:read");
        let first_token_id = created["data"]["token"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("api token id should be present"))
            .to_owned();
        let first_access_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();
        let first_expires_at = DateTime::parse_from_rfc3339(
            created["data"]["token"]["expires_at"]
                .as_str()
                .unwrap_or_else(|| panic!("expires_at should be present")),
        )
        .unwrap_or_else(|error| panic!("expires_at should be RFC3339: {error}"))
        .with_timezone(&Utc);
        assert!(
            first_expires_at <= Utc::now() + chrono::Duration::seconds(1_000),
            "requested 900 second token should not receive the default long session TTL"
        );

        let rotated = post_json_raw(
            app.clone(),
            &format!("/api/v1/auth/api-tokens/{first_token_id}/rotate"),
            r#"{"name":"rotated automation","expires_in_seconds":1800}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(rotated["code"], 0);
        assert_ne!(rotated["data"]["token"]["id"], first_token_id);
        assert_eq!(rotated["data"]["token"]["name"], "rotated automation");
        assert_eq!(rotated["data"]["token"]["scopes"][0], "users:read");
        let rotated_access_token = rotated["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("rotated token value should be returned once"))
            .to_owned();
        assert_ne!(rotated_access_token, first_access_token);

        let old_token_rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {first_access_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(
            old_token_rejected.status(),
            axum::http::StatusCode::UNAUTHORIZED
        );

        let rotated_read = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {rotated_access_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(rotated_read.status().is_success());

        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/api-tokens")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let list_body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let list_json: Value = serde_json::from_slice(&list_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(list_json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(list_json["data"][0]["name"], "rotated automation");
    }

    #[tokio::test]
    async fn api_token_policy_rejects_ttl_outside_configured_bounds() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/api-tokens")
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"too long","expires_in_seconds":31536000}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_ne!(json["code"], 0);
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|message| message.contains("expires_in_seconds"))
        );
    }

    #[tokio::test]
    async fn api_token_scope_bindings_limit_job_namespace_and_app_access() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let _billing = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"billing-visible"}"#,
            Some(&admin),
        )
        .await;
        let _payroll = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"finance","app":"payroll","name":"payroll-hidden"}"#,
            Some(&admin),
        )
        .await;

        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"billing automation","scopes":["jobs:read","jobs:write"],"scope_bindings":[{"namespace":"default","app":"billing","worker_pool":"pool-a"}]}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(
            created["data"]["token"]["scope_bindings"][0]["namespace"],
            "default"
        );
        assert_eq!(
            created["data"]["token"]["scope_bindings"][0]["app"],
            "billing"
        );
        assert_eq!(
            created["data"]["token"]["scope_bindings"][0]["worker_pool"],
            "pool-a"
        );
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();

        let visible_jobs = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(visible_jobs.status().is_success());
        let body = axum::body::to_bytes(visible_jobs.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"]["items"][0]["name"], "billing-visible");

        let denied_create = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("authorization", format!("Bearer {api_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"finance","app":"payroll","name":"blocked"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(denied_create.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_token_scope_bindings_filter_worker_pool_visibility() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        let mut pool_a = worker("pool-a-worker", "billing");
        pool_a
            .labels
            .insert("worker_pool".to_owned(), "pool-a".to_owned());
        let mut pool_b = worker("pool-b-worker", "billing");
        pool_b
            .labels
            .insert("worker_pool".to_owned(), "pool-b".to_owned());
        let registered_a = registry.register(pool_a, tx1).await;
        let _registered_b = registry.register(pool_b, tx2).await;
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
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"pool a worker reader","scopes":["workers:read"],"scope_bindings":[{"namespace":"default","app":"billing","worker_pool":"pool-a"}]}"#,
            Some(&admin),
        )
        .await;
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workers")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["online"], 1);
        assert_eq!(
            json["data"]["items"][0]["workerId"],
            registered_a.worker_id
        );
    }

    #[tokio::test]
    async fn workers_list_shows_latest_generation_for_replaced_logical_instance() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        let first = registry.register(worker("pod-1", "billing"), tx1).await;
        let second = registry.register(worker("pod-1", "billing"), tx2).await;
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

        let response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/workers").await)
            .await
            .unwrap_or_else(|error| panic!("workers route should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["data"]["online"], 1);
        assert_eq!(json["data"]["items"][0]["workerId"], second.worker_id);
        assert_eq!(json["data"]["items"][0]["generation"], 2);
        assert_eq!(json["data"]["items"][0]["status"], "online");
        assert_eq!(json["data"]["items"][0]["client_instance_id"], "pod-1");
        assert_ne!(first.worker_id, second.worker_id);
    }

    #[tokio::test]
    async fn login_failure_uses_unauthorized_envelope() {
        let app = router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"tikee_init","password":"wrong"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 40101);
        assert!(json.get("data").is_some());
    }

