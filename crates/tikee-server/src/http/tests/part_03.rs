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
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "metrics-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
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
                        checkpoint: None,
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
        assert!(
            json["data"]["queue"]["oldestPendingAgeSeconds"]
                .as_u64()
                .is_some_and(|value| value <= 1),
            "oldest pending age should be freshly created and near zero"
        );
        assert!(
            json["data"]["queue"]["averagePendingAgeSeconds"]
                .as_u64()
                .is_some_and(|value| value <= 1),
            "average pending age should be freshly created and near zero"
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
    async fn job_topology_api_discovers_workflow_dependencies_and_unresolved_refs() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let upstream = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "extract".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.extract".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("upstream job should create: {error}"));
        let downstream = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "load".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.load".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("downstream job should create: {error}"));
        let workflows = WorkflowRepository::new(db.clone());
        let workflow = workflows
            .create_workflow(CreateWorkflow {
                name: "billing-etl".to_owned(),
                definition: WorkflowDefinition {
                    nodes: vec![
                        WorkflowNodeSpec {
                            key: "extract".to_owned(),
                            name: Some("Extract".to_owned()),
                            kind: Some("job".to_owned()),
                            job_id: Some(upstream.id.clone()),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                        WorkflowNodeSpec {
                            key: "load".to_owned(),
                            name: Some("Load".to_owned()),
                            kind: Some("job".to_owned()),
                            job_id: Some(downstream.id.clone()),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                        WorkflowNodeSpec {
                            key: "notify".to_owned(),
                            name: Some("Notify".to_owned()),
                            kind: Some("job".to_owned()),
                            job_id: Some("job_missing_notify".to_owned()),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![
                        tikee_storage::WorkflowEdgeSpec {
                            from: "extract".to_owned(),
                            to: "load".to_owned(),
                            condition: Some("on_success".to_owned()),
                        },
                        tikee_storage::WorkflowEdgeSpec {
                            from: "load".to_owned(),
                            to: "notify".to_owned(),
                            condition: Some("always".to_owned()),
                        },
                    ],
                },
                created_by: "admin".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should create: {error}"));

        let app = router_with_state(AppState::new(
            jobs,
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            workflows,
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/jobs/topology").await)
            .await
            .unwrap_or_else(|error| panic!("topology route should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["code"], 0);
        assert!(
            json["data"]["nodes"]
                .as_array()
                .unwrap_or_else(|| panic!("nodes should be array"))
                .iter()
                .any(|node| node["id"] == upstream.id && node["type"] == "job")
        );
        assert!(
            json["data"]["nodes"]
                .as_array()
                .unwrap_or_else(|| panic!("nodes should be array"))
                .iter()
                .any(|node| node["id"] == workflow.id && node["type"] == "workflow")
        );
        assert!(
            json["data"]["edges"]
                .as_array()
                .unwrap_or_else(|| panic!("edges should be array"))
                .iter()
                .any(|edge| edge["from"] == upstream.id
                    && edge["to"] == downstream.id
                    && edge["condition"] == "on_success")
        );
        assert!(
            json["data"]["unresolved"]
                .as_array()
                .unwrap_or_else(|| panic!("unresolved should be array"))
                .iter()
                .any(|item| item["workflowId"] == workflow.id
                    && item["nodeKey"] == "notify"
                    && item["missingJobId"] == "job_missing_notify")
        );
    }


    #[tokio::test]
    async fn tenant_secret_store_creates_lists_and_deletes_scoped_secret_refs() {
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
        let create_response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/secrets",
                    r#"{"namespace":"default","app":"billing","name":"db.password","reference":{"kind":"env","name":"APP_DB_PASSWORD"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("create secret should respond: {error}"));
        assert!(create_response.status().is_success());
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let id = created["data"]["id"].as_str().unwrap_or_else(|| panic!("secret id should exist")).to_owned();
        assert_eq!(created["data"]["namespace"], "default");
        assert_eq!(created["data"]["app"], "billing");
        let secret_reference: Value = serde_json::from_str(
            created["data"]["valueRef"]
                .as_str()
                .unwrap_or_else(|| panic!("secret valueRef should be a structured JSON string")),
        )
        .unwrap_or_else(|error| panic!("secret valueRef should contain structured JSON: {error}"));
        assert_eq!(secret_reference["kind"], "env");
        assert_eq!(secret_reference["name"], "APP_DB_PASSWORD");

        let list_response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/secrets?namespace=default&app=billing").await)
            .await
            .unwrap_or_else(|error| panic!("list secrets should respond: {error}"));
        assert!(list_response.status().is_success());
        let body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let listed: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(listed["data"].as_array().is_some_and(|items| items.iter().any(|item| item["id"] == id)));

        let delete_response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "DELETE", format!("/api/v1/secrets/{id}")).await)
            .await
            .unwrap_or_else(|error| panic!("delete secret should respond: {error}"));
        assert!(delete_response.status().is_success());

        let list_response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/secrets?namespace=default&app=billing").await)
            .await
            .unwrap_or_else(|error| panic!("list secrets after delete should respond: {error}"));
        assert!(list_response.status().is_success());
        let body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let listed_after_delete: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(!listed_after_delete["data"].as_array().unwrap_or(&Vec::new()).iter().any(|item| item["id"] == id));

        let read_audit = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/audit-logs?action=read&resource_type=secret&page_size=1").await)
            .await
            .unwrap_or_else(|error| panic!("read audit should respond: {error}"));
        assert!(read_audit.status().is_success());
        let read_body = axum::body::to_bytes(read_audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
        let read_json: Value = serde_json::from_slice(&read_body)
            .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
        assert_eq!(read_json["data"]["items"][0]["resource_type"], "secret");
        assert_eq!(read_json["data"]["items"][0]["action"], "read");

        let create_audit = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/audit-logs?action=create&resource_type=secret&page_size=1").await)
            .await
            .unwrap_or_else(|error| panic!("create audit should respond: {error}"));
        assert!(create_audit.status().is_success());
        let create_body = axum::body::to_bytes(create_audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
        let create_json: Value = serde_json::from_slice(&create_body)
            .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
        assert_eq!(create_json["data"]["items"][0]["resource_type"], "secret");
        assert_eq!(create_json["data"]["items"][0]["action"], "create");

        let delete_audit = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/audit-logs?action=delete&resource_type=secret&page_size=1").await)
            .await
            .unwrap_or_else(|error| panic!("delete audit should respond: {error}"));
        assert!(delete_audit.status().is_success());
        let delete_body = axum::body::to_bytes(delete_audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
        let delete_json: Value = serde_json::from_slice(&delete_body)
            .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
        assert_eq!(delete_json["data"]["items"][0]["resource_type"], "secret");
        assert_eq!(delete_json["data"]["items"][0]["action"], "delete");
    }


    #[tokio::test]
    async fn inbound_webhook_event_source_triggers_job_and_records_payload_log() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "webhook-target".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.webhook".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instances = JobInstanceRepository::new(db.clone());
        let logs = JobInstanceLogRepository::new(db.clone());
        let app = router_with_state(AppState::new(
            jobs,
            instances.clone(),
            logs.clone(),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/events/webhooks/{}:trigger", job.id),
                    r#"{"source":"gitlab","eventType":"push","payload":{"ref":"main","sha":"abc123"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("webhook route should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["accepted"], true);
        assert_eq!(json["data"]["jobId"], job.id);
        assert_eq!(json["data"]["triggerType"], "webhook");
        let instance_id = json["data"]["instanceId"]
            .as_str()
            .unwrap_or_else(|| panic!("instanceId should be returned"));
        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trigger_type, TriggerType::Webhook);
        let instance_logs = logs
            .list_by_instance(instance_id)
            .await
            .unwrap_or_else(|error| panic!("logs should list: {error}"));
        assert!(
            instance_logs
                .iter()
                .any(|log| log.message.contains("webhook_event_source")
                    && log.message.contains("abc123")),
            "webhook payload should be recorded in instance logs: {instance_logs:?}"
        );
    }

    #[tokio::test]
    async fn inbound_webhook_rejects_replayed_signed_nonce() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "signed-webhook-target".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.webhook".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instances = JobInstanceRepository::new(db.clone());
        let app = router_with_state(AppState::new(
            jobs,
            instances.clone(),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));
        let timestamp = chrono::Utc::now().timestamp();
        let nonce = "nonce-1";
        let payload = serde_json::json!({"sha":"abc123"});
        let signature = inbound_webhook_signature(&std::env::var("PATH").unwrap_or_default(), &job.id, timestamp, nonce, &payload);
        let body = serde_json::json!({
            "source": "gitlab",
            "eventType": "push",
            "payload": payload,
            "secretRef": "env:PATH",
            "signature": signature,
            "timestamp": timestamp,
            "nonce": nonce
        })
        .to_string();

        let first = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/events/webhooks/{}:trigger", job.id),
                    &body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("webhook route should respond: {error}"));
        assert!(first.status().is_success());

        let replay = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/events/webhooks/{}:trigger", job.id),
                    &body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("webhook replay should respond: {error}"));
        assert_eq!(replay.status(), axum::http::StatusCode::BAD_REQUEST);
        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(listed.len(), 1, "replay must not create another instance");
    }

    fn inbound_webhook_signature(
        secret: &str,
        job_id: &str,
        timestamp: i64,
        nonce: &str,
        payload: &serde_json::Value,
    ) -> String {
        use sha2::{Digest as _, Sha256};
        let canonical = format!(
            "tikee-webhook-v1\njob_id={job_id}\ntimestamp={timestamp}\nnonce={nonce}\npayload={}",
            serde_json::to_string(payload).unwrap_or_else(|_| "null".to_owned())
        );
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(b"\n");
        hasher.update(canonical.as_bytes());
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }


    #[tokio::test]
    async fn scheduling_advice_reports_worker_capability_readiness() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "advice-target".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.advice".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let registry = crate::tunnel::WorkerRegistry::default();
        let app = router_with_state(AppState::new(
            jobs,
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            registry.clone(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let not_ready = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/jobs/{}/scheduling-advice", job.id),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("advice route should respond: {error}"));
        let body = axum::body::to_bytes(not_ready.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["ready"], false);
        assert_eq!(json["data"]["severity"], "error");
        assert_eq!(json["data"]["requiredCapability"], "SDK processor 'billing.advice'");

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let mut worker = worker("advice-worker", "billing");
        worker.structured_capabilities = Some(tikee_proto::worker::v1::WorkerCapabilities {
            sdk_processors: vec![tikee_proto::worker::v1::SdkProcessorCapability {
                name: "billing.advice".to_owned(),
            }],
            ..tikee_proto::worker::v1::WorkerCapabilities::default()
        });
        registry.register(worker, tx).await;

        let ready = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/jobs/{}/scheduling-advice", job.id),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("advice route should respond: {error}"));
        let body = axum::body::to_bytes(ready.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["ready"], true);
        assert_eq!(json["data"]["severity"], "ok");
        assert_eq!(json["data"]["eligibleWorkers"].as_array().map(Vec::len), Some(1));
    }


    #[tokio::test]
    async fn job_trigger_routes_to_canary_job_when_percent_is_full() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let canary = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "canary-target".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.canary".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("canary job should create: {error}"));
        let main = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "main-target".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.main".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: Some(canary.id.clone()),
                canary_percent: 100,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("main job should create: {error}"));
        let instances = JobInstanceRepository::new(db.clone());
        let app = router_with_state(AppState::new(
            jobs,
            instances.clone(),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/jobs/{}:trigger", main.id),
                    r#"{"triggerType":"api","executionMode":"single"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("trigger route should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["data"]["jobId"], canary.id);
        assert_eq!(json["data"]["canaryRouting"]["enabled"], true);
        assert_eq!(json["data"]["canaryRouting"]["routed"], true);
        assert_eq!(json["data"]["canaryRouting"]["originalJobId"], main.id);
        assert_eq!(json["data"]["canaryRouting"]["routedJobId"], canary.id);
        let main_instances = instances
            .list_by_job(&main.id)
            .await
            .unwrap_or_else(|error| panic!("main instances should load: {error}"));
        let canary_instances = instances
            .list_by_job(&canary.id)
            .await
            .unwrap_or_else(|error| panic!("canary instances should load: {error}"));
        assert_eq!(main_instances.len(), 0);
        assert_eq!(canary_instances.len(), 1);
    }



    #[tokio::test]
    async fn scheduling_advice_reports_history_duration_and_resource_prediction() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "predictable-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.predict".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let first = instances
            .create_pending(CreateJobInstance { job_id: job.id.clone(), trigger_type: TriggerType::Api, execution_mode: ExecutionMode::Single })
            .await
            .unwrap_or_else(|error| panic!("first instance should create: {error}"))
            .unwrap_or_else(|| panic!("first instance should exist"));
        instances
            .update_status(&first.id, tikee_core::InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("first should succeed: {error}"));
        instances
            .set_timestamps_for_test(&first.id, "2026-05-28T00:00:00Z", "2026-05-28T00:00:10Z")
            .await
            .unwrap_or_else(|error| panic!("first timestamps should update: {error}"));
        let second = instances
            .create_pending(CreateJobInstance { job_id: job.id.clone(), trigger_type: TriggerType::Api, execution_mode: ExecutionMode::Single })
            .await
            .unwrap_or_else(|error| panic!("second instance should create: {error}"))
            .unwrap_or_else(|| panic!("second instance should exist"));
        instances
            .update_status(&second.id, tikee_core::InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("second should succeed: {error}"));
        instances
            .set_timestamps_for_test(&second.id, "2026-05-28T00:01:00Z", "2026-05-28T00:01:30Z")
            .await
            .unwrap_or_else(|error| panic!("second timestamps should update: {error}"));
        let registry = crate::tunnel::WorkerRegistry::default();
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        let mut worker = RegisterWorker {
            client_instance_id: "predict-worker".to_owned(),
            app: "billing".to_owned(),
            namespace: "default".to_owned(),
            cluster: "local".to_owned(),
            region: "local".to_owned(),
            capabilities: Vec::new(),
            structured_capabilities: Some(tikee_proto::worker::v1::WorkerCapabilities {
                sdk_processors: vec![tikee_proto::worker::v1::SdkProcessorCapability {
                    name: "demo.predict".to_owned(),
                }],
                ..tikee_proto::worker::v1::WorkerCapabilities::default()
            }),
            election: None,
            labels: std::collections::HashMap::default(),
        };
        worker.labels.insert("cpu".to_owned(), "4".to_owned());
        worker.labels.insert("memory_mb".to_owned(), "8192".to_owned());
        registry.register(worker, sender).await;
        let app = router_with_state(AppState::new(
            jobs,
            instances,
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", format!("/api/v1/jobs/{}/scheduling-advice", job.id)).await)
            .await
            .unwrap_or_else(|error| panic!("advice route should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["data"]["history"]["completedInstances"], 2);
        assert_eq!(json["data"]["history"]["averageDurationSeconds"], 20);
        assert_eq!(json["data"]["history"]["p95DurationSeconds"], 30);
        assert_eq!(json["data"]["prediction"]["estimatedDurationSeconds"], 30);
        assert_eq!(json["data"]["prediction"]["recommendedConcurrency"], 1);
        assert_eq!(json["data"]["prediction"]["workerCapacity"]["eligibleWorkerCount"], 1);
        let Some(reasons) = json["data"]["prediction"]["reasons"].as_array() else {
            panic!("prediction reasons should be an array");
        };
        assert!(reasons
            .iter()
            .any(|item| item.as_str().unwrap_or_default().contains("history")));
    }

    #[tokio::test]
    async fn job_impact_api_reports_cross_workflow_upstream_and_downstream() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let extract = jobs
            .create_job(CreateJob { created_by: Some("admin".to_owned()), namespace: "default".to_owned(), app: "billing".to_owned(), name: "extract".to_owned(), schedule_type: "api".to_owned(), schedule_expr: None, misfire_policy: "fire_once".to_owned(), schedule_start_at: None, schedule_end_at: None, schedule_calendar_json: None, processor_name: Some("billing.extract".to_owned()), processor_type: None, script_id: None, enabled: true, canary_job_id: None, canary_percent: 0, retry_policy: None })
            .await
            .unwrap_or_else(|error| panic!("extract job should create: {error}"));
        let normalize = jobs
            .create_job(CreateJob { created_by: Some("admin".to_owned()), namespace: "default".to_owned(), app: "billing".to_owned(), name: "normalize".to_owned(), schedule_type: "api".to_owned(), schedule_expr: None, misfire_policy: "fire_once".to_owned(), schedule_start_at: None, schedule_end_at: None, schedule_calendar_json: None, processor_name: Some("billing.normalize".to_owned()), processor_type: None, script_id: None, enabled: true, canary_job_id: None, canary_percent: 0, retry_policy: None })
            .await
            .unwrap_or_else(|error| panic!("normalize job should create: {error}"));
        let publish = jobs
            .create_job(CreateJob { created_by: Some("admin".to_owned()), namespace: "default".to_owned(), app: "billing".to_owned(), name: "publish".to_owned(), schedule_type: "api".to_owned(), schedule_expr: None, misfire_policy: "fire_once".to_owned(), schedule_start_at: None, schedule_end_at: None, schedule_calendar_json: None, processor_name: Some("billing.publish".to_owned()), processor_type: None, script_id: None, enabled: true, canary_job_id: None, canary_percent: 0, retry_policy: None })
            .await
            .unwrap_or_else(|error| panic!("publish job should create: {error}"));
        let workflows = WorkflowRepository::new(db.clone());
        let first = workflows
            .create_workflow(CreateWorkflow {
                name: "billing-ingest".to_owned(),
                definition: WorkflowDefinition {
                    nodes: vec![workflow_node("extract", &extract.id), workflow_node("normalize", &normalize.id)],
                    edges: vec![tikee_storage::WorkflowEdgeSpec { from: "extract".to_owned(), to: "normalize".to_owned(), condition: Some("on_success".to_owned()) }],
                },
                created_by: "admin".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("first workflow should create: {error}"));
        let second = workflows
            .create_workflow(CreateWorkflow {
                name: "billing-publish".to_owned(),
                definition: WorkflowDefinition {
                    nodes: vec![workflow_node("normalize", &normalize.id), workflow_node("publish", &publish.id)],
                    edges: vec![tikee_storage::WorkflowEdgeSpec { from: "normalize".to_owned(), to: "publish".to_owned(), condition: Some("always".to_owned()) }],
                },
                created_by: "admin".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("second workflow should create: {error}"));
        let app = router_with_state(AppState::new(
            jobs,
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            workflows,
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", format!("/api/v1/jobs/{}/impact", normalize.id)).await)
            .await
            .unwrap_or_else(|error| panic!("impact route should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["data"]["targetJob"]["id"], normalize.id);
        let Some(referencing_workflows) = json["data"]["referencingWorkflows"].as_array() else {
            panic!("referencingWorkflows should be an array");
        };
        let Some(upstream_jobs) = json["data"]["upstreamJobs"].as_array() else {
            panic!("upstreamJobs should be an array");
        };
        let Some(downstream_jobs) = json["data"]["downstreamJobs"].as_array() else {
            panic!("downstreamJobs should be an array");
        };
        assert!(referencing_workflows.iter().any(|item| item["id"] == first.id));
        assert!(referencing_workflows.iter().any(|item| item["id"] == second.id));
        assert!(upstream_jobs.iter().any(|item| item["id"] == extract.id));
        assert!(downstream_jobs.iter().any(|item| item["id"] == publish.id));
        assert_eq!(json["data"]["riskSummary"]["workflowCount"], 2);
    }

    #[tokio::test]
    async fn workflow_replay_api_returns_instance_events_and_graph_bundle() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob { created_by: Some("admin".to_owned()), namespace: "default".to_owned(), app: "billing".to_owned(), name: "replay-job".to_owned(), schedule_type: "api".to_owned(), schedule_expr: None, misfire_policy: "fire_once".to_owned(), schedule_start_at: None, schedule_end_at: None, schedule_calendar_json: None, processor_name: Some("billing.replay".to_owned()), processor_type: None, script_id: None, enabled: true, canary_job_id: None, canary_percent: 0, retry_policy: None })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let workflows = WorkflowRepository::new(db.clone());
        let workflow = workflows
            .create_workflow(CreateWorkflow {
                name: "replay-flow".to_owned(),
                definition: WorkflowDefinition { nodes: vec![workflow_node("run", &job.id)], edges: vec![] },
                created_by: "admin".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should create: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let app = router_with_state(AppState::new(
            jobs,
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            workflows,
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", format!("/api/v1/workflow-instances/{}/replay", instance.id)).await)
            .await
            .unwrap_or_else(|error| panic!("replay route should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["data"]["instance"]["id"], instance.id);
        assert_eq!(json["data"]["workflow"]["id"], workflow.id);
        assert!(json["data"]["events"].as_array().is_some_and(|events| !events.is_empty()));
        assert!(json["data"]["graph"]["nodes"].as_array().is_some_and(|nodes| nodes.len() == 1));
    }

    fn workflow_node(key: &str, job_id: &str) -> WorkflowNodeSpec {
        WorkflowNodeSpec {
            key: key.to_owned(),
            name: Some(key.to_owned()),
            kind: Some("job".to_owned()),
            job_id: Some(job_id.to_owned()),
            processor_name: None,
            child_workflow_id: None,
            map_items: None,
            config: None,
        }
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
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "governed-script".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some("script-missing-runtime".to_owned()),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
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
        ensure_bootstrap_admin(app.clone()).await;
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            ADMIN_LOGIN,
        )
        .await;

        assert_eq!(login["code"], 0);
        let token = login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("login should return token"))
            .to_owned();
        assert_eq!(token.len(), 48);
        assert!(token.chars().all(|value| value.is_ascii_alphanumeric()));
        assert_eq!(login["data"]["roles"][0], "owner");

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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let me: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(me["code"], 0);
        assert_eq!(me["data"]["username"], "bootstrap_admin");
    }




    #[tokio::test]
    async fn cancel_instance_route_records_audit_log() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"cancel-audit","scheduleType":"api","processorName":"demo.echo"}"#,
            Some(&admin),
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("job id should be present"));
        let triggered = post_json_raw(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"triggerType":"api","executionMode":"single"}"#,
            Some(&admin),
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("instance id should be present"));
        let cancelled = post_json_raw(
            app.clone(),
            &format!("/api/v1/instances/{instance_id}/cancel"),
            "{}",
            Some(&admin),
        )
        .await;
        assert_eq!(cancelled["data"]["status"], "cancelled");

        let audit = get_json_with_auth(
            app,
            "/api/v1/audit-logs?action=cancel&resource_type=job_instance&page_size=1",
            &admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["action"], "cancel");
        assert_eq!(audit["data"]["items"][0]["resource_type"], "job_instance");
        assert_eq!(audit["data"]["items"][0]["resource_id"], instance_id);
    }

    #[tokio::test]
    async fn job_version_api_lists_and_rolls_back_snapshots() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"versioned-job","scheduleType":"api","processorName":"demo.echo"}"#,
            Some(&admin),
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("job id should be present"))
            .to_owned();
        assert_eq!(created["data"]["versionNumber"], 1);

        let updated = patch_json_raw(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}"),
            r#"{"name":"versioned-job-v2","enabled":false}"#,
            &admin,
        )
        .await;
        assert_eq!(updated["data"]["versionNumber"], 2);
        assert_eq!(updated["data"]["enabled"], false);

        let versions = get_json_with_auth(app.clone(), &format!("/api/v1/jobs/{job_id}/versions"), &admin).await;
        let items = versions["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("versions should be an array"));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["version_number"], 2);
        assert_eq!(items[1]["version_number"], 1);

        let rolled_back = post_json_raw(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/rollback"),
            r#"{"versionNumber":1}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(rolled_back["data"]["versionNumber"], 3);
        assert_eq!(rolled_back["data"]["name"], "versioned-job");
        assert_eq!(rolled_back["data"]["enabled"], true);

        let audit = get_json_with_auth(
            app,
            "/api/v1/audit-logs?action=rollback&resource_type=job&page_size=1",
            &admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["action"], "rollback");
        assert_eq!(audit["data"]["items"][0]["resource_type"], "job");
        assert_eq!(audit["data"]["items"][0]["resource_id"], job_id);
    }

    #[tokio::test]
    async fn plugin_registry_supports_custom_processor_types_and_alert_channels() {
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

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/plugins",
                    r#"{"name":"Ops Plugin","kind":"processor","processorTypes":[{"type":"sql","label":"SQL Processor","capability":"sql","processorNames":["billing.sql-sync"],"description":"Runs governed SQL processor tasks"},{"type":"external_jar","label":"External JAR Processor","capability":"external_jar","processorNames":["billing.jar-sync"],"description":"Runs versioned JAR in container sandbox","artifactRef":"s3://plugins/billing-jar-sync-1.0.0.jar","containerImage":"registry.example.com/tikee/jar-runner:1.0.0","entrypoint":["java","-jar","/plugins/billing-jar-sync.jar"],"checksum":"sha256:abc123"}],"alertChannelTypes":[{"type":"ops_webhook","label":"Ops Webhook","targetKind":"webhook","description":"Routes alerts to the ops bridge","template":{"headers":{"X-Tikee-Plugin":"ops"},"body":{"text":"{{message}}","resource":"{{resource_id}}"}}}],"enabled":true}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin route should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["processorTypes"][0]["capability"], "sql");
        assert_eq!(json["data"]["processorTypes"][0]["processorNames"][0], "billing.sql-sync");
        assert_eq!(json["data"]["processorTypes"][1]["type"], "external_jar");
        assert_eq!(json["data"]["processorTypes"][1]["artifactRef"], "s3://plugins/billing-jar-sync-1.0.0.jar");
        assert_eq!(json["data"]["processorTypes"][1]["containerImage"], "registry.example.com/tikee/jar-runner:1.0.0");
        assert_eq!(json["data"]["alertChannelTypes"][0]["type"], "ops_webhook");

        let plugins = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/plugins").await)
            .await
            .unwrap_or_else(|error| panic!("plugin list should respond: {error}"));
        assert!(plugins.status().is_success());
        let body = axum::body::to_bytes(plugins.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));

        let job = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"sql-sync","scheduleType":"api","processorType":"sql","processorName":"billing.sql-sync"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("job create should respond: {error}"));
        assert!(job.status().is_success());
        let body = axum::body::to_bytes(job.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["processorType"], "sql");

        let jar_job = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"jar-sync","scheduleType":"api","processorType":"external_jar","processorName":"billing.jar-sync"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("external jar job create should respond: {error}"));
        assert!(jar_job.status().is_success());

        let invalid_job = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"bad-sql-sync","scheduleType":"api","processorType":"sql","processorName":"mixed.sql"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("invalid job create should respond: {error}"));
        assert_eq!(invalid_job.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(invalid_job.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let invalid_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(invalid_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("plugin processorName is not declared")));

        let advice = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!(
                        "/api/v1/jobs/{}/scheduling-advice",
                        json["data"]["id"].as_str().unwrap_or_else(|| panic!("job id"))
                    ),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("advice route should respond: {error}"));
        assert!(advice.status().is_success());
        let body = axum::body::to_bytes(advice.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(
            json["data"]["requiredCapability"],
            "plugin processor type 'sql' name 'billing.sql-sync'"
        );

        let status = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Ops plugin alert","severity":"warning","condition":{"type":"script_governance_failure","failure_class":"plugin_test","threshold":1},"channels":[{"type":"ops_webhook","url":"https://ops.example.invalid/hook","enabled":true}],"enabled":true,"dedupe_seconds":30}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule route should respond: {error}"));
        assert!(status.status().is_success());
        let body = axum::body::to_bytes(status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let rule_id = json["data"]["id"].as_str().unwrap_or_else(|| panic!("rule id"));

        let readiness = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/alert-rules/{rule_id}/delivery-status"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert readiness route should respond: {error}"));
        assert!(readiness.status().is_success());
        let body = axum::body::to_bytes(readiness.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["ready"], true);
        assert_eq!(json["data"]["channels"][0]["provider"], "ops_webhook");
        assert_eq!(json["data"]["channels"][0]["transport_security"], "https");
    }

    #[tokio::test]
    async fn sdk_api_key_lifecycle_uses_header_and_app_scope() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let (api_key, key_id) = create_billing_sdk_api_key(app.clone(), &admin).await;

        assert_sdk_api_key_list_redacted(app.clone(), &admin, &api_key, &key_id).await;
        assert_sdk_key_audit_action(app.clone(), &admin, &key_id, "sdk_api_key_create").await;
        seed_sdk_key_scope_jobs(app.clone(), &admin).await;
        assert_sdk_key_lists_only_bound_app(app.clone(), &api_key).await;
        assert_sdk_key_authentication_is_audited(app.clone(), &admin, &key_id).await;
        assert_sdk_key_cannot_write_other_app(app.clone(), &api_key).await;
        update_sdk_api_key(app.clone(), &admin, &key_id).await;
        assert_sdk_key_audit_action(app.clone(), &admin, &key_id, "sdk_api_key_update").await;
        assert_sdk_key_lists_only_bound_app(app.clone(), &api_key).await;
        assert_sdk_key_cannot_write_bound_app_after_scope_edit(app.clone(), &api_key).await;
        assert_sdk_api_key_list_still_contains_updated_key(app.clone(), &admin, &key_id).await;
        revoke_sdk_api_key(app.clone(), &admin, &key_id).await;
        assert_sdk_key_audit_action(app.clone(), &admin, &key_id, "sdk_api_key_revoke").await;
        assert_revoked_sdk_key_rejected(app, &api_key).await;
    }

    #[tokio::test]
    async fn disabling_service_account_revokes_bound_sdk_keys() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let (api_key, key_id) = create_billing_sdk_api_key(app.clone(), &admin).await;
        let list = get_json_with_auth(app.clone(), "/api/v1/management/api-keys", &admin).await;
        let service_account_id = list["data"]
            .as_array()
            .unwrap_or_else(|| panic!("api key list should be an array"))
            .iter()
            .find(|item| item["id"] == key_id)
            .and_then(|item| item["service_account_id"].as_str())
            .unwrap_or_else(|| panic!("bound service account should be listed"))
            .to_owned();

        let disabled = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/management/service-accounts/{service_account_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(disabled.status().is_success());

        let service_accounts = get_json_with_auth(app.clone(), "/api/v1/management/service-accounts", &admin).await;
        let disabled_account = service_accounts["data"]
            .as_array()
            .unwrap_or_else(|| panic!("service account list should be an array"))
            .iter()
            .find(|item| item["id"] == service_account_id)
            .unwrap_or_else(|| panic!("disabled service account should remain visible"));
        assert_eq!(disabled_account["status"], "disabled");

        let keys = get_json_with_auth(app.clone(), "/api/v1/management/api-keys", &admin).await;
        assert!(keys["data"]
            .as_array()
            .unwrap_or_else(|| panic!("api key list should be an array"))
            .iter()
            .all(|item| item["id"] != key_id));
        assert_revoked_sdk_key_rejected(app, &api_key).await;
    }


    async fn patch_json_raw(app: axum::Router, uri: &str, body: &str, token: &str) -> Value {
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(uri)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(status.is_success(), "unexpected status {status}: {json}");
        json
    }

    async fn get_json_with_auth(app: axum::Router, uri: &str, token: &str) -> Value {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(status.is_success(), "unexpected status {status}: {json}");
        json
    }

    async fn create_billing_sdk_api_key(app: axum::Router, admin: &str) -> (String, String) {
        let service_account = create_billing_service_account(app.clone(), admin).await;
        let service_account_id = service_account["id"]
            .as_str()
            .unwrap_or_else(|| panic!("service account id should be present"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/management/api-keys")
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"name":"java demo","namespace":"default","app":"billing","service_account_id":"{service_account_id}","scopes":["jobs:read","jobs:write","instances:execute"]}}"#
                    )))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body)}));
        assert!(status.is_success(), "unexpected status {status}: {created}");
        assert_eq!(created["code"], 0);
        let api_key = created["data"]["api_key"]
            .as_str()
            .unwrap_or_else(|| panic!("api key should be returned once"))
            .to_owned();
        assert!(api_key.starts_with("tk-"));
        assert_eq!(api_key.len(), 67);
        assert!(api_key[3..].chars().all(|ch| ch.is_ascii_alphanumeric()));
        assert_eq!(created["data"]["key"]["namespace"], "default");
        assert_eq!(created["data"]["key"]["app"], "billing");
        assert_eq!(created["data"]["key"]["service_account_name"], "java-demo-sa");
        assert_eq!(created["data"]["key"]["service_account_id"], service_account_id);
        let key_id = created["data"]["key"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("key id should be present"))
            .to_owned();
        (api_key, key_id)
    }

    async fn create_billing_service_account(app: axum::Router, admin: &str) -> Value {
        let created = post_json_raw(
            app.clone(),
            "/api/v1/management/service-accounts",
            r#"{"name":"java-demo-sa","description":"Java demo SDK identity","namespace":"default","app":"billing"}"#,
            Some(admin),
        )
        .await;
        assert_eq!(created["data"]["name"], "java-demo-sa");
        assert_eq!(created["data"]["namespace"], "default");
        assert_eq!(created["data"]["app"], "billing");
        let list = get_json_with_auth(app, "/api/v1/management/service-accounts", admin).await;
        assert!(list["data"]
            .as_array()
            .unwrap_or_else(|| panic!("service account list should be an array"))
            .iter()
            .any(|item| item["id"] == created["data"]["id"]));
        created["data"].clone()
    }

    async fn assert_sdk_api_key_list_redacted(app: axum::Router, admin: &str, api_key: &str, key_id: &str) {
        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/management/api-keys")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(list.status().is_success());
        let list_body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let list_text = String::from_utf8(list_body.to_vec())
            .unwrap_or_else(|error| panic!("body should be utf8: {error}"));
        assert!(!list_text.contains(api_key));
        assert!(!list_text.contains("key_hash"));
        let list_json: Value = serde_json::from_str(&list_text)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(list_json["data"][0]["id"], key_id);
        assert_eq!(list_json["data"][0]["service_account_name"], "java-demo-sa");
    }

    async fn seed_sdk_key_scope_jobs(app: axum::Router, admin: &str) {
        let _billing = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"sdk-visible"}"#,
            Some(admin),
        )
        .await;
        let _other = post_json_raw(
            app,
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"other","name":"sdk-hidden"}"#,
            Some(admin),
        )
        .await;
    }

    async fn assert_sdk_key_lists_only_bound_app(app: axum::Router, api_key: &str) {
        let visible = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs")
                    .header("x-tikee-api-key", api_key)
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(visible.status().is_success());
        let visible_body = axum::body::to_bytes(visible.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let visible_json: Value = serde_json::from_slice(&visible_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let visible_items = visible_json["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("items should be an array"));
        assert!(visible_items.iter().any(|item| item["name"] == "sdk-visible"));
        assert!(visible_items.iter().all(|item| item["name"] != "sdk-hidden"));
    }



    async fn assert_sdk_key_audit_action(
        app: axum::Router,
        admin: &str,
        key_id: &str,
        action: &str,
    ) {
        let audit = get_json_with_auth(
            app,
            &format!("/api/v1/audit-logs?action={action}&resource_type=sdk_api_key&resource_id={key_id}&page_size=1"),
            admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["resource_type"], "sdk_api_key");
        assert_eq!(audit["data"]["items"][0]["resource_id"], key_id);
        assert!(!audit["data"]["items"][0]["actor"]
            .as_str()
            .unwrap_or_default()
            .is_empty());
    }

    async fn assert_sdk_key_authentication_is_audited(app: axum::Router, admin: &str, key_id: &str) {
        let audit = get_json_with_auth(
            app,
            &format!("/api/v1/audit-logs?action=sdk_api_key_authenticate&resource_type=sdk_api_key&resource_id={key_id}&pageSize=20"),
            admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["resource_type"], "sdk_api_key");
        assert_eq!(audit["data"]["items"][0]["resource_id"], key_id);
        assert!(
            audit["data"]["items"]
                .as_array()
                .unwrap_or_else(|| panic!("audit items should be an array"))
                .iter()
                .any(|item| item["actor"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("service_account:sa_"))
        );
    }

    async fn assert_sdk_key_cannot_write_other_app(app: axum::Router, api_key: &str) {
        let denied = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("x-tikee-api-key", api_key)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"default","app":"other","name":"blocked"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(denied.status(), axum::http::StatusCode::FORBIDDEN);
    }


    async fn update_sdk_api_key(app: axum::Router, admin: &str, key_id: &str) {
        let updated = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/management/api-keys/{key_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"updated-management-key","scopes":["jobs:read"],"expires_at":null}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(updated.status().is_success());
        let body = axum::body::to_bytes(updated.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["id"], key_id);
        assert_eq!(json["data"]["name"], "updated-management-key");
        assert_eq!(json["data"]["scopes"][0], "jobs:read");
        assert_eq!(json["data"]["expires_at"], serde_json::Value::Null);
    }

    async fn assert_sdk_key_cannot_write_bound_app_after_scope_edit(app: axum::Router, api_key: &str) {
        let denied = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("x-tikee-api-key", api_key)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"default","app":"billing","name":"blocked-by-scope"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(denied.status(), axum::http::StatusCode::FORBIDDEN);
    }

    async fn assert_sdk_api_key_list_still_contains_updated_key(
        app: axum::Router,
        admin: &str,
        key_id: &str,
    ) {
        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/management/api-keys")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(list.status().is_success());
        let body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let item = json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("api key list should be an array"))
            .iter()
            .find(|item| item["id"] == key_id)
            .unwrap_or_else(|| panic!("updated api key should remain listed"));
        assert_eq!(item["scopes"][0], "jobs:read");
    }

    async fn revoke_sdk_api_key(app: axum::Router, admin: &str, key_id: &str) {
        let revoked = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/management/api-keys/{key_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(revoked.status().is_success());
    }

    async fn assert_revoked_sdk_key_rejected(app: axum::Router, api_key: &str) {
        let rejected = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs")
                    .header("x-tikee-api-key", api_key)
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(rejected.status(), axum::http::StatusCode::UNAUTHORIZED);
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
        assert_eq!(api_token.len(), 48);
        assert!(api_token.chars().all(|value| value.is_ascii_alphanumeric()));

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
                        r#"{"username":"scoped-denied","email":"scoped-denied@example.com","password":"Secret123!","role":"viewer"}"#,
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
    async fn workers_list_shows_latest_generation_for_reconnected_logical_instance() {
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
        assert_eq!(json["data"]["items"][0]["clientInstanceId"], "pod-1");
        assert_eq!(first.worker_id, second.worker_id);
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
                        r#"{"username":"bootstrap_admin","password":"wrong"}"#,
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

    #[tokio::test]
    async fn login_accepts_email_identifier_for_password_session() {
        let app = router().await;
        ensure_bootstrap_admin(app.clone()).await;

        let login = post_json_without_auth(
            app,
            "/api/v1/auth/login",
            r#"{"username":"bootstrap.admin@example.com","password":"Tikee@2026!"}"#,
        )
        .await;

        assert_eq!(login["code"], 0);
        assert_eq!(login["data"]["username"], "bootstrap_admin");
        assert!(login["data"]["token"].as_str().is_some_and(|token| !token.is_empty()));
    }
