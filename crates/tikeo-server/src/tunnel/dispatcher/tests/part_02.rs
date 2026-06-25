
    #[derive(Debug, Default)]
    struct RecordingRelay {
        calls: tokio::sync::Mutex<Vec<(String, String, DispatchTask)>>,
    }

    #[async_trait::async_trait]
    impl crate::tunnel::WorkerRelayDispatch for RecordingRelay {
        async fn dispatch_to_gateway(
            &self,
            gateway_node_id: &str,
            worker_id: &str,
            task: DispatchTask,
        ) -> Result<(), crate::tunnel::WorkerRelayError> {
            self.calls.lock().await.push((
                gateway_node_id.to_owned(),
                worker_id.to_owned(),
                task,
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn dispatch_relays_to_remote_worker_gateway_from_persisted_owner() {
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
        let relay = std::sync::Arc::new(RecordingRelay::default());
        let registry = WorkerRegistry::with_lifecycle(lifecycle.clone())
            .with_gateway_node_id("leader-node")
            .with_relay(relay.clone());
        lifecycle
            .register_session(tikeo_storage::RegisterWorkerSession {
                worker_id: "wrk-remote-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "remote-worker".to_owned(),
                connection_id: "conn-remote".to_owned(),
                gateway_node_id: "gateway-node".to_owned(),
                fencing_token: "token-remote".to_owned(),
                lease_seconds: 30,
                capabilities_json: r"[]".to_owned(),
                structured_capabilities_json: r#"{"normalProcessors":[{"name":"billing.manual","description":""}]}"#.to_owned(),
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
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notification_center(&jobs),
            ),
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let calls = relay.calls.lock().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "gateway-node");
        assert_eq!(calls[0].1, "wrk-remote-gateway");
        assert_eq!(calls[0].2.instance_id, instance.id);
        assert!(!calls[0].2.assignment_token.is_empty());
        drop(calls);
        let persisted_attempts = attempts
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("attempts should load: {error}"));
        assert_eq!(persisted_attempts.len(), 1);
        assert_eq!(persisted_attempts[0].worker_id, "wrk-remote-gateway");
        assert!(persisted_attempts[0].assignment_token.is_some());
        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }


    #[tokio::test]
    async fn dispatch_lasso_prefers_local_worker_but_still_persists_outbox() {
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
        let lifecycle = tikeo_storage::WorkerLifecycleRepository::new(db);
        let registry = WorkerRegistry::with_lifecycle(lifecycle.clone()).with_gateway_node_id("node-a");
        lifecycle
            .register_session(tikeo_storage::RegisterWorkerSession {
                worker_id: "wrk-remote-master".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "remote-master".to_owned(),
                connection_id: "conn-remote".to_owned(),
                gateway_node_id: "node-b".to_owned(),
                fencing_token: "token-remote".to_owned(),
                lease_seconds: 30,
                capabilities_json: r"[]".to_owned(),
                structured_capabilities_json: r#"{"normalProcessors":[{"name":"billing.manual","description":""}]}"#.to_owned(),
                labels_json: r"{}".to_owned(),
                master_json: r#"{"isMaster":true,"domain":"default/billing/local/local","masterWorkerId":"wrk-remote-master"}"#.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("remote worker should persist: {error}"));
        let (tx, mut rx) = mpsc::channel(1);
        let local = registry
            .register(
                RegisterWorker {
                    client_instance_id: "local-follower".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(normal_capabilities("billing.manual")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "lasso-locality".to_owned(),
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
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notification_center(&jobs),
            ),
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("local worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "billing.manual");
            }
            other => panic!("unexpected server message: {other:?}"),
        }
        let not_claimable = outbox
            .claim_next_for_gateway("node-a", 10)
            .await
            .unwrap_or_else(|error| panic!("inline hinted outbox should load: {error}"));
        assert!(
            not_claimable.is_none(),
            "inline hinted dispatch must not remain immediately claimable and replay the same assignment"
        );
        let summary = outbox
            .summary()
            .await
            .unwrap_or_else(|error| panic!("outbox summary should load: {error}"));
        assert_eq!(summary.by_status.get("delivered"), Some(&1));
        let _ = local;
    }

    #[tokio::test]
    async fn dispatch_once_closes_terminal_instance_queue_item() {
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
                name: "already-done".to_owned(),
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
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        instances
            .update_status(&instance.id, InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("instance should be terminal: {error}"));
        let registry = WorkerRegistry::default();

        dispatch_once(
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notification_center(&jobs),
            ),
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert_eq!(overview.pending, 0);
        assert_eq!(overview.running, 0);
        assert_eq!(overview.done, 1);
        assert_eq!(
            overview.items[0].job_instance_id.as_deref(),
            Some(instance.id.as_str())
        );
    }

    #[tokio::test]
    async fn raft_leader_without_projected_shards_does_not_fallback_claim_queue_items() {
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
                name: "raft-leader-no-shards".to_owned(),
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
        instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let leader = StaticCoordinator::shared(ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Leader,
            node_id: "node-a".to_owned(),
            nodes: 3,
            can_schedule: true,
            leader_fencing_token: Some("raft:term:9:node:node-a".to_owned()),
            detail: "test leader without projected shards".to_owned(),
        });

        let notifications = notification_center(&jobs);
        dispatch_once_if_owner(
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notifications,
            ),
            &leader,
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch gate should run: {error}"));

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert_eq!(overview.pending, 1);
        assert_eq!(overview.running, 0);
        assert!(overview.items[0].lease_owner.is_none());
    }

    #[tokio::test]
    async fn follower_dispatch_claims_owned_shard_queue_items() {
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
                name: "follower-dispatch".to_owned(),
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
        let queued = workflows
            .dispatch_queue_for_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("dispatch queue should load: {error}"))
            .unwrap_or_else(|| panic!("dispatch queue item should exist"));
        let shard_id = queued.shard_id.unwrap_or_else(|| panic!("queue item should be sharded"));
        let owner = tikeo_storage::ClusterShardOwnershipRepository::new(jobs.db())
            .upsert_newer(tikeo_storage::UpsertClusterShardOwnership {
                shard_id,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "node-b".to_owned(),
                epoch: 11,
                raft_term: 6,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("shard owner should persist: {error}"))
            .unwrap_or_else(|| panic!("newer shard owner should be accepted"));
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-follower".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(normal_capabilities("billing.manual")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;
        let follower = StaticCoordinator::shared(ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Follower,
            node_id: "node-b".to_owned(),
            nodes: 3,
            can_schedule: false,
            leader_fencing_token: None,
            detail: "test follower".to_owned(),
        });

        let notifications = notification_center(&jobs);
        dispatch_once_if_owner(
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notifications,
            ),
            &follower,
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch gate should run: {error}"));

        let message = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("worker should receive follower-owned shard dispatch before timeout"))
            .unwrap_or_else(|| panic!("worker channel should stay open"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "billing.manual");
                assert!(!task.assignment_token.is_empty());
            }
            other => panic!("unexpected server message: {other:?}"),
        }
        let updated_queue = workflows
            .dispatch_queue_for_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("dispatch queue should reload: {error}"))
            .unwrap_or_else(|| panic!("dispatch queue item should still exist"));
        assert_eq!(updated_queue.status, "running");
        assert_eq!(updated_queue.owner_epoch, Some(owner.epoch));
        assert_eq!(
            updated_queue.owner_fencing_token.as_deref(),
            Some(owner.fencing_token.as_str())
        );
    }

    #[tokio::test]
    async fn explicit_shard_owner_dispatches_only_owned_queue_items() {
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
        let ownership = tikeo_storage::ClusterShardOwnershipRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "shard-owner-dispatch".to_owned(),
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
        let queued = workflows
            .dispatch_queue_for_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("dispatch queue should load: {error}"))
            .unwrap_or_else(|| panic!("dispatch queue item should exist"));
        let shard_id = queued
            .shard_id
            .unwrap_or_else(|| panic!("api job dispatch queue should have stable shard id"));
        let owner = ownership
            .upsert_newer(tikeo_storage::UpsertClusterShardOwnership {
                shard_id,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "node-b".to_owned(),
                epoch: 9,
                raft_term: 4,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("shard owner should persist: {error}"))
            .unwrap_or_else(|| panic!("newer shard owner should be accepted"));

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
                    structured_capabilities: Some(normal_capabilities("billing.manual")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;
        let notifications = notification_center(&jobs);
        dispatch_once_with_shards(
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notifications,
            ),
            "node-b",
            "",
            std::slice::from_ref(&owner),
        )
        .await
        .unwrap_or_else(|error| panic!("explicit shard owner dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "billing.manual");
                assert!(!task.assignment_token.is_empty());
            }
            other => panic!("unexpected server message: {other:?}"),
        }
        let updated_queue = workflows
            .dispatch_queue_for_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("dispatch queue should reload: {error}"))
            .unwrap_or_else(|| panic!("dispatch queue item should still exist"));
        assert_eq!(updated_queue.status, "running");
        assert_eq!(updated_queue.shard_id, Some(shard_id));
        assert_eq!(updated_queue.owner_epoch, Some(owner.epoch));
        assert_eq!(
            updated_queue.owner_fencing_token.as_deref(),
            Some(owner.fencing_token.as_str())
        );
        let not_claimable = outbox
            .claim_next_for_gateway("standalone", 10)
            .await
            .unwrap_or_else(|error| panic!("inline hinted outbox should load: {error}"));
        assert!(
            not_claimable.is_none(),
            "inline hinted shard dispatch must not remain immediately claimable"
        );
        let summary = outbox
            .summary()
            .await
            .unwrap_or_else(|error| panic!("outbox summary should load: {error}"));
        assert_eq!(summary.by_status.get("delivered"), Some(&1));
    }

    #[tokio::test]
    async fn explicit_shard_owner_materializes_owned_workflow_nodes() {
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
        let ownership = tikeo_storage::ClusterShardOwnershipRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "workflow-node-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.workflow".to_owned()),
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
                name: "shard follower workflow boundary".to_owned(),
                created_by: "test".to_owned(),
                definition: tikeo_storage::WorkflowDefinition {
                    nodes: vec![tikeo_storage::WorkflowNodeSpec {
                        key: "job-a".to_owned(),
                        name: Some("Job A".to_owned()),
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id.clone()),
                        processor_name: Some("billing.workflow".to_owned()),
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
        let overview_before = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        let workflow_queue_shard = overview_before.items[0]
            .shard_id
            .unwrap_or_else(|| panic!("workflow node queue should have stable shard id"));
        let owner = ownership
            .upsert_newer(tikeo_storage::UpsertClusterShardOwnership {
                shard_id: workflow_queue_shard,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "node-b".to_owned(),
                epoch: 11,
                raft_term: 5,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("matching shard owner should persist: {error}"))
            .unwrap_or_else(|| panic!("newer matching shard owner should be accepted"));
        let registry = WorkerRegistry::default();
        let notifications = notification_center(&jobs);
        dispatch_once_with_shards(
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notifications,
            ),
            "node-b",
            "",
            std::slice::from_ref(&owner),
        )
        .await
        .unwrap_or_else(|error| panic!("explicit shard owner dispatch should run: {error}"));

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert_eq!(overview.running, 0);
        assert!(
            overview.items.iter().any(|item| item.job_instance_id.is_some()),
            "shard-owner follower should create a job dispatch item for the owned workflow node; the same dispatcher tick may already advance it beyond pending: {overview:?}"
        );
        assert!(
            overview.items.iter().any(|item| {
                item.workflow_node_instance_id.is_some()
                    && item.status == "done"
                    && item.owner_epoch == Some(owner.epoch)
            }),
            "owned workflow node queue should be fenced and closed after materialization: {overview:?}"
        );
    }




    #[tokio::test]
    async fn explicit_shard_owner_dispatches_only_owned_broadcast_attempts() {
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
        let ownership = tikeo_storage::ClusterShardOwnershipRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "broadcast-owner-dispatch".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.broadcast".to_owned()),
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
                execution_mode: ExecutionMode::Broadcast,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        let registered = registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-broadcast-client".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(normal_capabilities("billing.broadcast")),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;
        let created_attempts = attempts
            .create_pending_for_workers(&instance.id, std::slice::from_ref(&registered.worker_id))
            .await
            .unwrap_or_else(|error| panic!("broadcast attempt should be created: {error}"));
        let attempt = created_attempts
            .first()
            .unwrap_or_else(|| panic!("attempt should exist"));
        let target_shard = tikeo_storage::scheduler_shard_policy().shard_id_for(
            &job.namespace,
            &job.app,
            &format!("{}:{}", instance.id, attempt.id),
        );
        let owner = ownership
            .upsert_newer(tikeo_storage::UpsertClusterShardOwnership {
                shard_id: target_shard,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "node-b".to_owned(),
                epoch: 12,
                raft_term: 6,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("broadcast owner should persist: {error}"))
            .unwrap_or_else(|| panic!("newer broadcast owner should be accepted"));

        let notifications = notification_center(&jobs);
        dispatch_once_with_shards(
            dispatcher_refs!(
                &jobs,
                &instances,
                &attempts,
                &outbox,
                &workflows,
                &scripts,
                &logs,
                &audit,
                &registry,
                &notifications,
            ),
            "node-b",
            "fallback-not-used",
            std::slice::from_ref(&owner),
        )
        .await
        .unwrap_or_else(|error| panic!("owned broadcast dispatch should run: {error}"));

        let message = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("worker should receive owned broadcast before timeout"))
            .unwrap_or_else(|| panic!("worker channel should stay open"))
            .unwrap_or_else(|error| panic!("broadcast dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "billing.broadcast");
                assert!(!task.assignment_token.is_empty());
            }
            other => panic!("unexpected server message: {other:?}"),
        }
        let not_claimable = outbox
            .claim_next_for_gateway("standalone", 10)
            .await
            .unwrap_or_else(|error| panic!("inline hinted broadcast outbox should load: {error}"));
        assert!(
            not_claimable.is_none(),
            "inline hinted broadcast dispatch must not remain immediately claimable"
        );
        let summary = outbox
            .summary()
            .await
            .unwrap_or_else(|error| panic!("outbox summary should load: {error}"));
        assert_eq!(summary.by_status.get("delivered"), Some(&1));
    }
