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
            .update_status(&succeeded.id, tikeo_core::InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("instance should update: {error}"));
        assert_eq!(pending.status, tikeo_core::InstanceStatus::Pending);

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

        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db.clone()),
        );
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        registry
            .register(worker("metrics-worker", "billing"), tx)
            .await;


        tikeo_storage::WorkerDispatchOutboxRepository::new(db.clone())
            .create(tikeo_storage::CreateWorkerDispatchOutbox {
                instance_id: "inst-metrics-outbox".to_owned(),
                attempt_id: "attempt-metrics-outbox".to_owned(),
                worker_id: "worker-metrics-outbox".to_owned(),
                logical_instance_id: "logical-metrics-outbox".to_owned(),
                gateway_node_id: "test-node".to_owned(),
                gateway_generation: 1,
                assignment_token: "asg-metrics-outbox".to_owned(),
                dispatch_payload: "payload".to_owned(),
                shard_id: 0,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "test-node".to_owned(),
                owner_epoch: 0,
                owner_fencing_token: "fence-metrics".to_owned(),
                next_delivery_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("outbox metric row should create: {error}"));
        let pending_queue = workflows
            .dispatch_queue_for_instance(&pending.id)
            .await
            .unwrap_or_else(|error| panic!("pending queue should load: {error}"))
            .unwrap_or_else(|| panic!("pending queue should exist"));
        tikeo_storage::ClusterShardOwnershipRepository::new(db.clone())
            .upsert_newer(tikeo_storage::UpsertClusterShardOwnership {
                shard_id: pending_queue
                    .shard_id
                    .unwrap_or_else(|| panic!("pending queue should have shard id")),
                shard_map_version: pending_queue.shard_map_version.unwrap_or(1),
                shard_count: pending_queue.shard_count.unwrap_or(64),
                owner_node_id: "test-node".to_owned(),
                epoch: 3,
                raft_term: 7,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("shard ownership metric row should create: {error}"))
            .unwrap_or_else(|| panic!("shard ownership row should be returned"));

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
            "tikeo-dispatcher",
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
        assert_eq!(json["data"]["outbox"]["total"], 1);
        assert_eq!(json["data"]["outbox"]["byStatus"]["queued"], 1);
        assert_eq!(json["data"]["shard_ownership"]["total"], 1);
        assert_eq!(json["data"]["shard_ownership"]["active"], 1);
        assert_eq!(json["data"]["shard_ownership"]["maxEpoch"], 3);
        assert_eq!(json["data"]["shard_ownership"]["activeOwnerCount"], 1);
        assert_eq!(json["data"]["shard_ownership"]["ownershipSkew"], 0);
        assert_eq!(
            json["data"]["shard_ownership"]["activeByOwner"]["test-node"],
            1
        );
        assert_eq!(
            json["data"]["queue"]["pendingByShardOwner"]["test-node"],
            2
        );
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
        let oldest_pending_age = json["data"]["queue"]["oldestPendingAgeSeconds"]
            .as_u64()
            .unwrap_or_else(|| panic!("queue summary should include oldest pending age: {json}"));
        let average_pending_age = json["data"]["queue"]["averagePendingAgeSeconds"]
            .as_u64()
            .unwrap_or_else(|| panic!("queue summary should include average pending age: {json}"));
        assert!(
            average_pending_age <= oldest_pending_age,
            "average pending age should not exceed oldest pending age: {json}"
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
            text.contains("tikeo_dispatch_queue_pending_age_seconds"),
            "metrics body should expose dispatch queue pending age histogram: {text}"
        );
        assert!(
            text.contains("tikeo_dispatch_queue_dispatch_latency_seconds"),
            "metrics body should expose dispatch latency histogram: {text}"
        );
        assert!(
            text.contains("tikeo_job_instances_current"),
            "metrics body should expose job instance status gauges: {text}"
        );
        assert!(
            text.contains("tikeo_job_instance_success_ratio"),
            "metrics body should expose job instance success ratio: {text}"
        );
        assert!(
            text.contains("tikeo_script_governance_failures_current"),
            "metrics body should expose script governance failure gauges: {text}"
        );
        assert!(
            text.contains("tikeo_workflow_instances_current"),
            "metrics body should expose workflow instance status gauges: {text}"
        );
        assert!(
            text.contains("tikeo_workflow_instance_duration_seconds"),
            "metrics body should expose workflow instance duration histogram: {text}"
        );
        assert!(
            text.contains("tikeo_cluster_shard_ownership_owner_count"),
            "metrics body should expose shard owner count gauge: {text}"
        );
        assert!(
            text.contains("tikeo_cluster_shard_ownership_skew"),
            "metrics body should expose shard ownership skew gauge: {text}"
        );
        assert!(
            text.contains("tikeo_dispatch_queue_pending_by_owner"),
            "metrics body should expose per-owner pending queue gauge: {text}"
        );
        assert!(
            text.contains("tikeo_dispatch_queue_oldest_pending_age_by_owner_seconds"),
            "metrics body should expose per-owner pending age gauge: {text}"
        );
        assert!(
            text.contains("tikeo_workflow_shard_duration_seconds"),
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
                        tikeo_storage::WorkflowEdgeSpec {
                            from: "extract".to_owned(),
                            to: "load".to_owned(),
                            condition: Some("on_success".to_owned()),
                        },
                        tikeo_storage::WorkflowEdgeSpec {
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
            "tikeo-webhook-v1\njob_id={job_id}\ntimestamp={timestamp}\nnonce={nonce}\npayload={}",
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
        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db.clone()),
        );
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
        worker.structured_capabilities = Some(tikeo_proto::worker::v1::WorkerCapabilities {
            sdk_processors: vec![tikeo_proto::worker::v1::SdkProcessorCapability {
                name: "billing.advice".to_owned(),
            }],
            ..tikeo_proto::worker::v1::WorkerCapabilities::default()
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
            .update_status(&first.id, tikeo_core::InstanceStatus::Succeeded)
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
            .update_status(&second.id, tikeo_core::InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("second should succeed: {error}"));
        instances
            .set_timestamps_for_test(&second.id, "2026-05-28T00:01:00Z", "2026-05-28T00:01:30Z")
            .await
            .unwrap_or_else(|error| panic!("second timestamps should update: {error}"));
        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db.clone()),
        );
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        let mut worker = RegisterWorker {
            client_instance_id: "predict-worker".to_owned(),
            app: "billing".to_owned(),
            namespace: "default".to_owned(),
            cluster: "local".to_owned(),
            region: "local".to_owned(),
            capabilities: Vec::new(),
            structured_capabilities: Some(tikeo_proto::worker::v1::WorkerCapabilities {
                sdk_processors: vec![tikeo_proto::worker::v1::SdkProcessorCapability {
                    name: "demo.predict".to_owned(),
                }],
                ..tikeo_proto::worker::v1::WorkerCapabilities::default()
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
                    edges: vec![tikeo_storage::WorkflowEdgeSpec { from: "extract".to_owned(), to: "normalize".to_owned(), condition: Some("on_success".to_owned()) }],
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
                    edges: vec![tikeo_storage::WorkflowEdgeSpec { from: "normalize".to_owned(), to: "publish".to_owned(), condition: Some("always".to_owned()) }],
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
