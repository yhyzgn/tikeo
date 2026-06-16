    #[tokio::test]
    async fn http_processor_retries_and_signs_requests() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("http listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("http listener should expose addr: {error}"));
        let captured = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let captured_server = captured.clone();
        let server = tokio::spawn(async move {
            for attempt in 0..2 {
                let (mut stream, _) = listener
                    .accept()
                    .await
                    .unwrap_or_else(|error| panic!("http client should connect: {error}"));
                let mut buffer = [0_u8; 4096];
                let read = stream
                    .read(&mut buffer)
                    .await
                    .unwrap_or_else(|error| panic!("http request should read: {error}"));
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                captured_server.lock().await.push(request);
                let response = if attempt == 0 {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"
                };
                stream
                    .write_all(response.as_bytes())
                    .await
                    .unwrap_or_else(|error| panic!("http response should write: {error}"));
            }
        });

        let body = serde_json::json!({"hello":"world"});
        let mut hasher = Sha256::new();
        hasher.update(b"test-secret");
        hasher.update(b"\n");
        hasher.update(body.to_string().as_bytes());
        let expected_signature = format!("sha256:{}", hex::encode(hasher.finalize()));
        let outcome = execute_http_processor(&serde_json::json!({
            "url": format!("http://{address}/hook"),
            "method": "POST",
            "allowedHosts": ["127.0.0.1"],
            "allowInsecureLoopback": true,
            "maxRetries": 1,
            "retryBackoffMs": 1,
            "signature": {
                "type": "sha256",
                "secret": "test-secret",
                "header": "X-Tikeo-Signature"
            },
            "body": body,
        }))
        .await;
        server
            .await
            .unwrap_or_else(|error| panic!("http test server should finish: {error}"));

        assert!(outcome.success, "unexpected outcome: {outcome:?}");
        assert!(outcome.message.contains("attempts=2"));
        let captured_requests = captured.lock().await.clone();
        assert_eq!(captured_requests.len(), 2);
        assert!(captured_requests[0].contains("POST /hook HTTP/1.1"));
        assert!(captured_requests[0].contains(&format!("x-tikeo-signature: {expected_signature}")));
        assert!(captured_requests[1].contains(&format!("x-tikeo-signature: {expected_signature}")));
    }

    #[tokio::test]
    async fn http_processor_enforces_denylist_and_circuit_breaker() {
        let denied = execute_http_processor(&serde_json::json!({
            "url": "https://203.0.113.10/hook",
            "allowedHosts": ["203.0.113.10"],
            "deniedCidrs": ["203.0.113.0/24"],
        }))
        .await;
        assert!(!denied.success);
        assert!(denied.message.contains("deniedCidrs"));

        let tripped = execute_http_processor(&serde_json::json!({
            "url": "http://127.0.0.1:9/hook",
            "allowedHosts": ["127.0.0.1"],
            "allowInsecureLoopback": true,
            "maxRetries": 3,
            "retryBackoffMs": 1,
            "circuitBreaker": { "failureThreshold": 1 },
        }))
        .await;
        assert!(!tripped.success);
        assert!(tripped.message.contains("circuit breaker open"));
        assert!(tripped.message.contains("after 1 failures"));
    }

    #[tokio::test]
    async fn dispatch_once_sends_pending_instance_to_registered_worker() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
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
                processor_name: Some("billing.manual".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                    structured_capabilities: Some(sdk_capabilities("billing.manual")),
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
                assert_eq!(task.processor_name, "billing.manual");
                let persisted_attempts = attempts
                    .list_by_instance(&instance.id)
                    .await
                    .unwrap_or_else(|error| panic!("attempts should load: {error}"));
                assert_eq!(persisted_attempts.len(), 1);
                assert_eq!(
                    persisted_attempts[0].assignment_token.as_deref(),
                    Some(task.assignment_token.as_str()),
                    "assignment token must be durable before the Worker can answer"
                );
                assert_eq!(persisted_attempts[0].status, InstanceStatus::Running);
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn dispatch_once_backoffs_unmatched_queue_item_without_starving_later_work() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);

        let blocked_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "offline".to_owned(),
                name: "blocked".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.blocked".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("blocked job should be created: {error}"));
        let blocked_instance = instances
            .create_pending(CreateJobInstance {
                job_id: blocked_job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("blocked instance should be created: {error}"))
            .unwrap_or_else(|| panic!("blocked job should exist"));

        let valid_job = jobs
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
                processor_name: Some("demo.echo".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("valid job should be created: {error}"));
        let valid_instance = instances
            .create_pending(CreateJobInstance {
                job_id: valid_job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("valid instance should be created: {error}"))
            .unwrap_or_else(|| panic!("valid job should exist"));

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
                    structured_capabilities: Some(sdk_capabilities("demo.echo")),
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
            .unwrap_or_else(|| panic!("matched worker should receive later valid dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, valid_instance.id);
                assert_eq!(task.processor_name, "demo.echo");
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let blocked = instances
            .get(&blocked_instance.id)
            .await
            .unwrap_or_else(|error| panic!("blocked instance should load: {error}"))
            .unwrap_or_else(|| panic!("blocked instance should exist"));
        assert_eq!(blocked.status, InstanceStatus::Failed);
        let valid = instances
            .get(&valid_instance.id)
            .await
            .unwrap_or_else(|error| panic!("valid instance should load: {error}"))
            .unwrap_or_else(|| panic!("valid instance should exist"));
        assert_eq!(valid.status, InstanceStatus::Running);

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        let blocked_queue = overview
            .items
            .iter()
            .find(|item| item.job_instance_id.as_deref() == Some(blocked_instance.id.as_str()))
            .unwrap_or_else(|| panic!("blocked queue item should exist"));
        assert_eq!(blocked_queue.status, "failed");
    }

    #[tokio::test]
    async fn dispatch_once_logs_retry_attempt_dispatch_progress() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "retry-dispatch-log".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.retry".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: Some(JobRetryPolicy {
                    enabled: true,
                    max_attempts: 3,
                    initial_delay_seconds: 0,
                    backoff_multiplier: 2,
                    max_delay_seconds: 60,
                }),
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
        let first_claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should claim: {error}"))
            .unwrap_or_else(|| panic!("queue item should exist"));
        workflows
            .mark_dispatch_queue_running(&first_claim.item.id, "server-a")
            .await
            .unwrap_or_else(|error| panic!("queue should be running: {error}"));
        instances
            .update_status(&instance.id, InstanceStatus::Running)
            .await
            .unwrap_or_else(|error| panic!("instance should be running: {error}"));

        complete_builtin_processor_outcome(
            &instances,
            &workflows,
            &logs,
            &job,
            &instance.id,
            first_claim.item.attempt,
            "builtin.test",
            false,
            "first attempt failed".to_owned(),
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("failure should schedule retry: {error}"));

        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-retry".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(sdk_capabilities("demo.retry")),
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
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            "test-fence",
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("retry dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive retry dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "demo.retry");
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let persisted_logs = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should load: {error}"));
        assert!(
            persisted_logs
                .iter()
                .any(|log| log.message.contains(&format!(
                    "retry attempt 2/3 dispatching to worker {}",
                    worker.worker_id
                ))),
            "retry dispatch progress should be written to execution logs: {persisted_logs:?}"
        );
    }

    #[tokio::test]
    async fn dispatch_once_fails_script_instance_when_no_script_worker_capability_exists() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
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
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "shell job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some(script.id),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
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
            &workflows,
            &scripts,
            &logs,
            &audit,
            &WorkerRegistry::default(),
            "test-fence",
            &notification_center(&jobs),
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Failed);
        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        let queue_item = overview
            .items
            .iter()
            .find(|item| item.job_instance_id.as_deref() == Some(instance.id.as_str()))
            .unwrap_or_else(|| panic!("queue item should exist"));
        assert_eq!(queue_item.status, "failed");
        let instance_logs = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should load: {error}"));
        assert!(
            instance_logs
                .iter()
                .any(|log| { log.message.contains("script_no_eligible_worker_capability") })
        );
    }

    #[tokio::test]
    async fn dispatch_script_uses_unified_script_worker_capability() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikeo_storage::CreateScript {
                name: "python example".to_owned(),
                language: "python".to_owned(),
                version: "1.0.0".to_owned(),
                content: "print('ok')".to_owned(),
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
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "python job".to_owned(),
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
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
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
                    client_instance_id: "script-worker".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(script_capabilities("python")),
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
            .unwrap_or_else(|| panic!("script worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                let binding = task
                    .processor_binding
                    .unwrap_or_else(|| panic!("script binding expected"));
                match binding.kind {
                    Some(task_processor_binding::Kind::Script(script_binding)) => {
                        assert_eq!(script_binding.script_id, script.id);
                        assert_eq!(script_binding.language, "python");
                    }
                    other => panic!("unexpected binding: {other:?}"),
                }
            }
            other => panic!("unexpected server message: {other:?}"),
        }
        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn dispatch_once_filters_by_namespace_and_app() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
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
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
        let (tx1, mut rx1) = mpsc::channel(1);
        let (tx2, _rx2) = mpsc::channel(1);

        // This worker should match
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(sdk_capabilities("manual")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx1,
            )
            .await;

        // This worker should NOT match
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-2".to_owned(),
                    app: "analytics".to_owned(), // Different app
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(sdk_capabilities("manual")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx2,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
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

        let message = rx1
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker-1 should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, job.name);
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }
