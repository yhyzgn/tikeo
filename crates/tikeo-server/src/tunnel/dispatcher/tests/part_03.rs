    #[tokio::test]
    async fn dispatch_once_prefers_workflow_node_processor_name() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let outbox = tikeo_storage::WorkerDispatchOutboxRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("job.default".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(tikeo_storage::CreateWorkflow {
                name: "processor override".to_owned(),
                created_by: "test".to_owned(),
                definition: tikeo_storage::WorkflowDefinition {
                    nodes: vec![tikeo_storage::WorkflowNodeSpec {
                        key: "job-a".to_owned(),
                        name: Some("Job A".to_owned()),
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id.clone()),
                        processor_name: Some("workflow.override".to_owned()),
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: Vec::new(),
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"));
        workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("workflow node should materialize: {error}"));

        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(sdk_capabilities("workflow.override")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &outbox,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            "test-fence",
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.processor_name, "workflow.override");
            }
            other => panic!("unexpected server message: {other:?}"),
        }
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn dispatch_includes_wasm_binding_only_for_approved_policy_safe_script() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let outbox = tikeo_storage::WorkerDispatchOutboxRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikeo_storage::CreateScript {
                name: "wasm echo".to_owned(),
                language: "wasm".to_owned(),
                version: "1.0.0".to_owned(),
                content: "wasm-demo-module".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(5),
                max_memory_bytes: Some(1024 * 1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let version = scripts
            .versions()
            .get_version_by_number(&script.id, 1)
            .await
            .unwrap_or_else(|error| panic!("script version should load: {error}"))
            .unwrap_or_else(|| panic!("script version should exist"));
        let script = scripts
            .publish_version(&script.id, version.version_number, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(script.status, "approved");
        assert_eq!(
            script.released_version_id.as_deref(),
            Some(version.id.as_str())
        );
        assert_eq!(script.released_version_number, Some(version.version_number));

        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "wasm job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some(script.id.clone()),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(script_capabilities("wasm")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &outbox,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            "test-fence",
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, script.id);
                let binding = task
                    .processor_binding
                    .unwrap_or_else(|| panic!("wasm binding expected"));
                match binding.kind {
                    Some(task_processor_binding::Kind::Wasm(wasm)) => {
                        assert_eq!(wasm.script_id, script.id);
                        assert_eq!(wasm.runtime, "wasmtime");
                        assert_eq!(wasm.entrypoint, "_start");
                        assert_eq!(wasm.timeout_ms, 5_000);
                        assert_eq!(wasm.max_memory_bytes, 1024 * 1024);
                        assert!(!wasm.allow_network);
                        assert!(wasm.allowed_env_vars.is_empty());
                        assert_eq!(wasm.version_id, version.id);
                        assert_eq!(
                            wasm.version_number,
                            u64::try_from(version.version_number).unwrap_or_else(|error| panic!(
                                "version number should convert: {error}"
                            ))
                        );
                        assert_eq!(wasm.module_sha256, version.content_sha256);
                        assert_eq!(wasm.module_signature, "");
                        assert_eq!(wasm.module, version.content.as_bytes());
                    }
                    other => panic!("unexpected binding: {other:?}"),
                }
            }
            other => panic!("unexpected server message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_includes_non_wasm_script_binding_only_for_released_safe_script() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikeo_storage::CreateScript {
                name: "shell echo".to_owned(),
                language: "shell".to_owned(),
                version: "1.0.0".to_owned(),
                content: "printf ok".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(5),
                max_memory_bytes: Some(1024 * 1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: Some(r#"{"resources":{"timeout_ms":7000,"max_memory_bytes":33554432,"max_output_bytes":4096},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":["SAFE_ENV"]}"#.to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let version = scripts
            .versions()
            .get_version_by_number(&script.id, 1)
            .await
            .unwrap_or_else(|error| panic!("script version should load: {error}"))
            .unwrap_or_else(|| panic!("script version should exist"));
        let script = scripts
            .publish_version(&script.id, version.version_number, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));

        let task = match build_dispatch_task(
            &scripts,
            "instance-shell".to_owned(),
            "job-shell".to_owned(),
            JobExecutor::Script {
                script_id: script.id.clone(),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("task build should not error: {error}"))
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                panic!("released safe script should dispatch: {failure:?}")
            }
        };

        let binding = task
            .processor_binding
            .unwrap_or_else(|| panic!("script binding expected"));
        match binding.kind {
            Some(task_processor_binding::Kind::Script(script_binding)) => {
                assert_eq!(script_binding.script_id, script.id);
                assert_eq!(script_binding.language, "shell");
                assert_eq!(script_binding.content, version.content.as_bytes());
                assert_eq!(script_binding.version_id, version.id);
                assert_eq!(script_binding.content_sha256, version.content_sha256);
                assert_eq!(script_binding.timeout_ms, 7_000);
                assert_eq!(script_binding.max_memory_bytes, 33_554_432);
                assert_eq!(script_binding.max_output_bytes, 4_096);
                assert!(!script_binding.allow_network);
                assert_eq!(script_binding.allowed_env_vars, vec!["SAFE_ENV"]);
                assert!(script_binding.read_only_paths.is_empty());
                assert!(script_binding.writable_paths.is_empty());
                assert!(script_binding.secret_refs.is_empty());
                assert_eq!(script_binding.sandbox_backend, "auto");
            }
            other => panic!("unexpected binding: {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_copies_verified_release_grants_into_script_binding() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikeo_storage::CreateScript {
                name: "shell grants".to_owned(),
                language: "shell".to_owned(),
                version: "1.0.0".to_owned(),
                content: "printf ok".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(5),
                max_memory_bytes: Some(1024 * 1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: Some(r#"{"resources":{"timeout_ms":7000,"max_memory_bytes":33554432,"max_output_bytes":4096},"network":{"enabled":false,"allowed_hosts":["policy.example.invalid"]},"filesystem":{"read_only_paths":["/policy/read"],"writable_paths":["/policy/write"]},"secrets":{"refs":["secret:policy"]},"env_vars":["SAFE_ENV"]}"#.to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let version = scripts
            .versions()
            .get_version_by_number(&script.id, 1)
            .await
            .unwrap_or_else(|error| panic!("script version should load: {error}"))
            .unwrap_or_else(|| panic!("script version should exist"));
        let script = scripts
            .publish_version(
                &script.id,
                version.version_number,
                None,
                Some(tikeo_storage::VerifiedScriptReleaseGrants {
                    grants: tikeo_core::ScriptReleaseGrantSet {
                        url: vec!["api.example.com".to_owned()],
                        file_read: vec!["/data/input".to_owned()],
                        file_write: vec!["/data/output".to_owned()],
                        secret: vec!["secret:db-readonly".to_owned()],
                    },
                    verified_by: "grant-verifier".to_owned(),
                }),
            )
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));

        let task = match build_dispatch_task(
            &scripts,
            "instance-shell".to_owned(),
            "job-shell".to_owned(),
            JobExecutor::Script {
                script_id: script.id.clone(),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("task build should not error: {error}"))
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                panic!("released grant script should dispatch: {failure:?}")
            }
        };

        let binding = task
            .processor_binding
            .unwrap_or_else(|| panic!("script binding expected"));
        match binding.kind {
            Some(task_processor_binding::Kind::Script(script_binding)) => {
                assert!(script_binding.allow_network);
                assert_eq!(
                    script_binding.allowed_network_hosts,
                    vec!["api.example.com"]
                );
                assert_eq!(script_binding.read_only_paths, vec!["/data/input"]);
                assert_eq!(script_binding.writable_paths, vec!["/data/output"]);
                assert_eq!(script_binding.secret_refs, vec!["secret:db-readonly"]);
                assert_eq!(script_binding.allowed_env_vars, vec!["SAFE_ENV"]);
            }
            other => panic!("unexpected binding: {other:?}"),
        }
    }

    #[tokio::test]
    async fn approved_wasm_script_without_release_pointer_fails_closed() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db);
        let script = scripts
            .create_script(tikeo_storage::CreateScript {
                name: "wasm unreleased".to_owned(),
                language: "wasm".to_owned(),
                version: "1.0.0".to_owned(),
                content: "module".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(1),
                max_memory_bytes: Some(1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let approved = scripts
            .update_script(
                &script.id,
                tikeo_storage::UpdateScript {
                    name: None,
                    language: None,
                    version: None,
                    content: None,
                    status: Some("approved".to_owned()),
                    timeout_seconds: None,
                    max_memory_bytes: None,
                    allow_network: None,
                    allowed_env_vars: None,
                    policy_json: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("script should update: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(approved.status, "approved");
        assert_eq!(approved.released_version_number, None);

        let task = build_dispatch_task(
            &scripts,
            "instance-1".to_owned(),
            "job-1".to_owned(),
            JobExecutor::Script {
                script_id: script.id.clone(),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("task build should not error: {error}"));
        assert!(matches!(
            task,
            DispatchTaskBuild::Rejected(ScriptGovernanceFailure::MissingReleasePointer)
        ));
    }

    #[tokio::test]
    async fn wasm_script_dispatch_eligibility_requires_approval_and_safe_policy() {
        let mut script = ScriptSummary {
            id: "script_1".to_owned(),
            name: "demo".to_owned(),
            language: "wasm".to_owned(),
            version: "1.0.0".to_owned(),
            content: "module".to_owned(),
            content_sha256: "af67347816654d9b144b131e2c92b8b6f6ba3edecb7f1911ef6d8a81f8e08329"
                .to_owned(),
            status: "draft".to_owned(),
            released_version_id: None,
            released_version_number: None,
            release_signature: None,
            release_grants: None,
            timeout_seconds: Some(1),
            max_memory_bytes: Some(1024),
            allow_network: false,
            allowed_env_vars: None,
            policy: serde_json::json!({
                "resources": {"timeout_ms": 30_000, "max_memory_bytes": 64 * 1024 * 1024, "max_output_bytes": 1024 * 1024},
                "network": {"enabled": false, "allowed_hosts": []},
                "filesystem": {"read_only_paths": [], "writable_paths": []},
                "secrets": {"refs": []},
                "env_vars": []
            }),
            created_by: "tester".to_owned(),
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
        };
        assert!(!script_is_dispatchable(&script));

        script.status = "approved".to_owned();
        assert!(script_is_dispatchable(&script));

        script.language = "unknown".to_owned();
        assert!(!script_is_dispatchable(&script));

        let mut version = ScriptVersionSummary {
            id: "version_1".to_owned(),
            script_id: script.id.clone(),
            version_number: 1,
            content: "module".to_owned(),
            content_sha256: script.content_sha256.clone(),
            language: "wasm".to_owned(),
            status: "draft".to_owned(),
            timeout_seconds: Some(1),
            max_memory_bytes: Some(1024),
            allow_network: false,
            allowed_env_vars: None,
            policy: script.policy,
            created_by: "tester".to_owned(),
            created_at: "now".to_owned(),
        };
        assert!(script_version_is_dispatchable(&version));

        version.allow_network = true;
        assert!(!script_version_is_dispatchable(&version));
    }
    #[tokio::test]
    async fn file_cleanup_processor_defaults_to_dry_run_and_requires_allowed_roots() {
        let outcome = execute_file_cleanup_processor(&serde_json::json!({
            "paths": ["/tmp/tikeo-cleanup-demo"]
        }))
        .await;
        assert!(!outcome.success);
        assert!(outcome.message.contains("allowedRoots"));

        let outcome = execute_file_cleanup_processor(&serde_json::json!({
            "paths": ["/tmp/tikeo-cleanup-demo"],
            "allowedRoots": ["/tmp"]
        }))
        .await;
        assert!(outcome.success);
        assert!(outcome.message.contains("dry-run"));
    }

    #[tokio::test]
    async fn file_cleanup_processor_deletes_only_under_allowed_roots() {
        let temp_root =
            std::env::temp_dir().join(format!("tikeo-cleanup-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_root)
            .await
            .unwrap_or_else(|error| panic!("temp root should be created: {error}"));
        let target = temp_root.join("stale.log");
        tokio::fs::write(&target, b"stale")
            .await
            .unwrap_or_else(|error| panic!("target file should be written: {error}"));

        let rejected = execute_file_cleanup_processor(&serde_json::json!({
            "paths": [target.display().to_string()],
            "allowedRoots": ["/var/lib/tikeo"],
            "dryRun": false
        }))
        .await;
        assert!(!rejected.success);
        assert!(tokio::fs::metadata(&target).await.is_ok());

        let deleted = execute_file_cleanup_processor(&serde_json::json!({
            "paths": [target.display().to_string()],
            "allowedRoots": [temp_root.display().to_string()],
            "dryRun": false
        }))
        .await;
        assert!(deleted.success, "{}", deleted.message);
        assert!(tokio::fs::metadata(&target).await.is_err());
        let _ = tokio::fs::remove_dir_all(&temp_root).await;
    }

    #[tokio::test]
    async fn grpc_processor_fails_closed_without_required_fields_and_private_hosts() {
        let missing = execute_grpc_processor(&serde_json::json!({})).await;
        assert!(!missing.success);
        assert!(missing.message.contains("endpoint"));

        let private = execute_grpc_processor(&serde_json::json!({
            "endpoint": "http://127.0.0.1:50051",
            "service": "demo.Echo",
            "method": "Ping"
        }))
        .await;
        assert!(!private.success);
        assert!(private.message.contains("private"));
    }

    #[tokio::test]
    async fn sql_processor_enforces_allowlist_and_read_only_default() {
        let missing_allowlist = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "sql": "SELECT 1"
        }))
        .await;
        assert!(!missing_allowlist.success);
        assert!(missing_allowlist.message.contains("allowedDatabaseUrls"));

        let write_rejected = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "allowedDatabaseUrls": ["sqlite::memory:"],
            "sql": "DELETE FROM demo",
            "dryRun": false
        }))
        .await;
        assert!(!write_rejected.success);
        assert!(write_rejected.message.contains("readOnly"));

        let dry_run = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "allowedDatabaseUrls": ["sqlite::memory:"],
            "sql": "SELECT 1"
        }))
        .await;
        assert!(dry_run.success);
        assert!(dry_run.message.contains("dry-run"));
    }

    #[tokio::test]
    async fn sql_processor_executes_sqlite_read_only_query() {
        let outcome = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "allowedDatabaseUrls": ["sqlite::memory:"],
            "sql": "SELECT 1",
            "dryRun": false
        }))
        .await;
        assert!(outcome.success, "{}", outcome.message);
        assert!(outcome.message.contains("1 row"));
    }


    #[tokio::test]
    async fn dispatch_uses_persisted_online_worker_for_eligibility_and_live_transport_only_for_send() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let outbox = tikeo_storage::WorkerDispatchOutboxRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db.clone());
        let lifecycle = tikeo_storage::WorkerLifecycleRepository::new(db.clone());
        let registry = WorkerRegistry::with_lifecycle(lifecycle.clone());
        let (tx, mut rx) = mpsc::channel(1);
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-db-authority".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(sdk_capabilities("billing.manual")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;
        lifecycle
            .mark_transport_error(&worker.worker_id, "db authoritative offline")
            .await
            .unwrap_or_else(|error| panic!("db offline marker should persist: {error}"));
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.manual".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &outbox,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            "test-fence",
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        assert!(rx.try_recv().is_err(), "db-offline worker must not receive dispatch even if registry still has a sender");
        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Failed);
    }


    #[derive(Debug, Default)]
    struct FailingRelay;

    #[async_trait::async_trait]
    impl crate::tunnel::WorkerRelayDispatch for FailingRelay {
        async fn dispatch_to_gateway(
            &self,
            _gateway_node_id: &str,
            _worker_id: &str,
            _task: DispatchTask,
        ) -> Result<(), crate::tunnel::WorkerRelayError> {
            Err(crate::tunnel::WorkerRelayError::transient("gateway temporarily unavailable"))
        }
    }

    #[tokio::test]
    async fn dispatch_persists_outbox_before_remote_relay_and_keeps_it_when_relay_fails() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let outbox = tikeo_storage::WorkerDispatchOutboxRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db.clone());
        let lifecycle = tikeo_storage::WorkerLifecycleRepository::new(db.clone());
        let registry = WorkerRegistry::with_lifecycle(lifecycle.clone())
            .with_gateway_node_id("leader-node")
            .with_relay(std::sync::Arc::new(FailingRelay));
        lifecycle
            .register_session(tikeo_storage::RegisterWorkerSession {
                worker_id: "wrk-remote-outbox".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "remote-outbox".to_owned(),
                connection_id: "conn-remote-outbox".to_owned(),
                gateway_node_id: "gateway-node".to_owned(),
                fencing_token: "token-remote-outbox".to_owned(),
                lease_seconds: 30,
                capabilities_json: r"[]".to_owned(),
                structured_capabilities_json: r#"{"sdkProcessors":["billing.outbox"]}"#.to_owned(),
                labels_json: r"{}".to_owned(),
                master_json: r"{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("remote gateway worker should persist: {error}"));
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "outbox".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.outbox".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &outbox,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            "test-fence",
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let attempt_rows = attempts
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("attempts should load: {error}"));
        assert_eq!(attempt_rows.len(), 1);
        let token = attempt_rows[0]
            .assignment_token
            .as_deref()
            .unwrap_or_else(|| panic!("assignment token should be persisted"));
        let claimed = outbox
            .claim_next_for_gateway("gateway-node", 10)
            .await
            .unwrap_or_else(|error| panic!("outbox should be claimable: {error}"))
            .unwrap_or_else(|| panic!("relay failure should leave a queued durable outbox row"));
        assert_eq!(claimed.instance_id, instance.id);
        assert_eq!(claimed.worker_id, "wrk-remote-outbox");
        assert_eq!(claimed.assignment_token, token);
        assert_eq!(claimed.gateway_node_id, "gateway-node");
    }
