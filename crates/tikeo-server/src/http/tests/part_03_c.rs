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
            worker_pool: None,
                script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let first = instances
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
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
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
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
            normal_processors: vec![tikeo_proto::worker::v1::ProcessorCapability {
                name: "demo.predict".to_owned(),
                description: String::new(),
            }],
            ..tikeo_proto::worker::v1::WorkerCapabilities::default()
        }),
        election: None,
        labels: std::collections::HashMap::default(),
    };
    worker.labels.insert("cpu".to_owned(), "4".to_owned());
    worker
        .labels
        .insert("memory_mb".to_owned(), "8192".to_owned());
    registry.register(worker, sender).await;
    let app = router_with_state(app_state!(
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
    assert_eq!(
        json["data"]["prediction"]["workerCapacity"]["eligibleWorkerCount"],
        1
    );
    let Some(reasons) = json["data"]["prediction"]["reasons"].as_array() else {
        panic!("prediction reasons should be an array");
    };
    assert!(
        reasons
            .iter()
            .any(|item| item.as_str().unwrap_or_default().contains("history"))
    );
}

#[tokio::test]
async fn job_impact_api_reports_cross_workflow_upstream_and_downstream() {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let extract = jobs
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
            worker_pool: None,
                script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("extract job should create: {error}"));
    let normalize = jobs
        .create_job(CreateJob {
            created_by: Some("admin".to_owned()),
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "normalize".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("billing.normalize".to_owned()),
            processor_type: None,
            worker_pool: None,
                script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("normalize job should create: {error}"));
    let publish = jobs
        .create_job(CreateJob {
            created_by: Some("admin".to_owned()),
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "publish".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("billing.publish".to_owned()),
            processor_type: None,
            worker_pool: None,
                script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("publish job should create: {error}"));
    let workflows = WorkflowRepository::new(db.clone());
    let first = workflows
        .create_workflow(CreateWorkflow {
            name: "billing-ingest".to_owned(),
            definition: WorkflowDefinition {
                nodes: vec![
                    workflow_node("extract", &extract.id),
                    workflow_node("normalize", &normalize.id),
                ],
                edges: vec![tikeo_storage::WorkflowEdgeSpec {
                    from: "extract".to_owned(),
                    to: "normalize".to_owned(),
                    condition: Some("on_success".to_owned()),
                }],
            },
            created_by: "admin".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("first workflow should create: {error}"));
    let second = workflows
        .create_workflow(CreateWorkflow {
            name: "billing-publish".to_owned(),
            definition: WorkflowDefinition {
                nodes: vec![
                    workflow_node("normalize", &normalize.id),
                    workflow_node("publish", &publish.id),
                ],
                edges: vec![tikeo_storage::WorkflowEdgeSpec {
                    from: "normalize".to_owned(),
                    to: "publish".to_owned(),
                    condition: Some("always".to_owned()),
                }],
            },
            created_by: "admin".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("second workflow should create: {error}"));
    let app = router_with_state(app_state!(
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
        .oneshot(
            admin_request_builder(app, "GET", format!("/api/v1/jobs/{}/impact", normalize.id))
                .await,
        )
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
    assert!(
        referencing_workflows
            .iter()
            .any(|item| item["id"] == first.id)
    );
    assert!(
        referencing_workflows
            .iter()
            .any(|item| item["id"] == second.id)
    );
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
        .create_job(CreateJob {
            created_by: Some("admin".to_owned()),
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "replay-job".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("billing.replay".to_owned()),
            processor_type: None,
            worker_pool: None,
                script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let workflows = WorkflowRepository::new(db.clone());
    let workflow = workflows
        .create_workflow(CreateWorkflow {
            name: "replay-flow".to_owned(),
            definition: WorkflowDefinition {
                nodes: vec![workflow_node("run", &job.id)],
                edges: vec![],
            },
            created_by: "admin".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("workflow should create: {error}"));
    let instance = workflows
        .run_workflow(&workflow.id, "api")
        .await
        .unwrap_or_else(|error| panic!("workflow should run: {error}"))
        .unwrap_or_else(|| panic!("workflow should exist"));
    let app = router_with_state(app_state!(
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
        .oneshot(
            admin_request_builder(
                app,
                "GET",
                format!("/api/v1/workflow-instances/{}/replay", instance.id),
            )
            .await,
        )
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
    assert!(
        json["data"]["events"]
            .as_array()
            .is_some_and(|events| !events.is_empty())
    );
    assert!(
        json["data"]["graph"]["nodes"]
            .as_array()
            .is_some_and(|nodes| nodes.len() == 1)
    );
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
