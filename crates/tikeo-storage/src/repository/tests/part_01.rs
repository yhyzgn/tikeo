    #[tokio::test]
    async fn alert_rules_apply_threshold_dedupe_window_and_silence() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = crate::repository::AlertRepository::new(db.clone());
        let threshold_rule = repository
            .create_rule(crate::repository::CreateAlertRule {
                name: "Windowed script failures".to_owned(),
                severity: "warning".to_owned(),
                condition_json: serde_json::json!({
                    "type": "script_governance_failure",
                    "failure_class": "runtime_missing",
                    "threshold": 2,
                })
                .to_string(),
                channels_json: "[]".to_owned(),
                enabled: true,
                dedupe_seconds: 300,
                silenced_until: None,
            })
            .await
            .unwrap_or_else(|error| panic!("threshold rule should create: {error}"));

        let first = repository
            .record_script_governance_failure("inst-a", "runtime_missing", "first miss")
            .await
            .unwrap_or_else(|error| panic!("first event should record: {error}"));
        assert_eq!(first[0].status, "suppressed");

        let second = repository
            .record_script_governance_failure("inst-b", "runtime_missing", "second miss")
            .await
            .unwrap_or_else(|error| panic!("second event should record: {error}"));
        assert_eq!(second[0].status, "firing");

        let duplicate = repository
            .record_script_governance_failure("inst-c", "runtime_missing", "duplicate miss")
            .await
            .unwrap_or_else(|error| panic!("duplicate event should record: {error}"));
        assert_eq!(duplicate[0].status, "suppressed");

        let firing_row = crate::entities::alert_event::Entity::find_by_id(second[0].id.clone())
            .one(&db)
            .await
            .unwrap_or_else(|error| panic!("firing row should load: {error}"))
            .unwrap_or_else(|| panic!("firing row should exist"));
        let mut active: crate::entities::alert_event::ActiveModel = firing_row.into();
        active.created_at = Set("1970-01-01T00:00:00Z".to_owned());
        active
            .update(&db)
            .await
            .unwrap_or_else(|error| panic!("firing row should age out: {error}"));

        let after_window = repository
            .record_script_governance_failure("inst-d", "runtime_missing", "new window miss")
            .await
            .unwrap_or_else(|error| panic!("new window event should record: {error}"));
        assert_eq!(after_window[0].rule_id, threshold_rule.id);
        assert_eq!(after_window[0].status, "firing");

        let silenced_until = time::OffsetDateTime::now_utc()
            .saturating_add(time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2999-01-01T00:00:00Z".to_owned());
        repository
            .create_rule(crate::repository::CreateAlertRule {
                name: "Silenced script failures".to_owned(),
                severity: "critical".to_owned(),
                condition_json: serde_json::json!({
                    "type": "script_governance_failure",
                    "failure_class": "policy_denied",
                    "threshold": 1,
                })
                .to_string(),
                channels_json: "[]".to_owned(),
                enabled: true,
                dedupe_seconds: 300,
                silenced_until: Some(silenced_until),
            })
            .await
            .unwrap_or_else(|error| panic!("silenced rule should create: {error}"));
        let silenced = repository
            .record_script_governance_failure("inst-s", "policy_denied", "policy denied")
            .await
            .unwrap_or_else(|error| panic!("silenced event should record: {error}"));
        assert_eq!(silenced[0].status, "silenced");
    }

    #[tokio::test]
    async fn plugin_repository_resolves_custom_processor_and_alert_channel_types() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = crate::repository::PluginRepository::new(db);
        let created = repository
            .create_plugin(crate::repository::CreatePlugin {
                name: "Ops Plugin".to_owned(),
                kind: "mixed".to_owned(),
                processor_types: vec![crate::repository::PluginProcessorTypeSummary {
                    r#type: "sql".to_owned(),
                    label: "SQL Processor".to_owned(),
                    capability: "sql".to_owned(),
                    processor_names: vec!["billing.sql-sync".to_owned()],
                    description: Some("custom SQL handler".to_owned()),
                    artifact_ref: None,
                    container_image: None,
                    entrypoint: None,
                    checksum: None,
                }],
                alert_channel_types: vec![crate::repository::PluginAlertChannelTypeSummary {
                    r#type: "ops_webhook".to_owned(),
                    label: "Ops Webhook".to_owned(),
                    target_kind: "webhook".to_owned(),
                    description: None,
                    template: serde_json::json!({"body":{"text":"{{message}}"}}),
                }],
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("plugin should create: {error}"));

        assert_eq!(created.processor_types[0].capability, "sql");
        let processor_type = repository
            .resolve_processor_type("sql")
            .await
            .unwrap_or_else(|error| panic!("processor type should resolve: {error}"));
        assert!(processor_type.is_some());
        let alert_channel_type = repository
            .resolve_alert_channel_type("ops_webhook")
            .await
            .unwrap_or_else(|error| panic!("alert channel type should resolve: {error}"));
        assert!(alert_channel_type.is_some());
    }

    #[tokio::test]
    async fn job_version_history_tracks_updates_and_rollbacks() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = JobRepository::new(db);
        let created = repository
            .create_job(CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "versioned".to_owned(),
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
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        assert_eq!(created.version_number, 1);

        let updated = repository
            .update_job(
                &created.id,
                UpdateJob {
                    updated_by: Some("editor".to_owned()),
                    name: Some("versioned-v2".to_owned()),
                    enabled: Some(false),
                    ..UpdateJob::default()
                },
            )
            .await
            .unwrap_or_else(|error| panic!("job should update: {error}"))
            .unwrap_or_else(|| panic!("updated job should exist"));
        assert_eq!(updated.version_number, 2);

        let rolled_back = repository
            .rollback_job(&created.id, 1, Some("operator".to_owned()))
            .await
            .unwrap_or_else(|error| panic!("job should rollback: {error}"))
            .unwrap_or_else(|| panic!("rolled back job should exist"));
        assert_eq!(rolled_back.version_number, 3);
        assert_eq!(rolled_back.name, "versioned");
        assert!(rolled_back.enabled);

        let versions = repository
            .versions()
            .list_versions(&created.id)
            .await
            .unwrap_or_else(|error| panic!("versions should list: {error}"));
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].change_reason, "rollback");
        assert_eq!(versions[0].rolled_back_from_version, Some(1));
        assert_eq!(versions[1].created_by, "editor");
        assert_eq!(versions[2].created_by, "admin");
    }

    #[tokio::test]
    async fn worker_lifecycle_repository_replaces_generations_and_fences_stale_heartbeats() {
        use crate::repository::{
            RegisterWorkerSession, WorkerHeartbeat, WorkerLifecycleRepository,
        };

        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = WorkerLifecycleRepository::new(db);

        let first = repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-one".to_owned(),
                namespace_name: "finance".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                client_instance_id: "host-a#slot-1".to_owned(),
                connection_id: "conn-one".to_owned(),
                gateway_node_id: "tikeo-1".to_owned(),
                fencing_token: "token-one".to_owned(),
                lease_seconds: 30,
                capabilities_json: r#"["java"]"#.to_owned(),
                structured_capabilities_json: r#"{"tags":["java"],"sdkProcessors":["demo.echo"],"scriptRunners":[],"pluginProcessors":[]}"#.to_owned(),
                labels_json: r#"{"worker_pool":"blue"}"#.to_owned(),
                master_json: r#"{"domain":"dev-alpha/orders/local/local","isMaster":true,"masterWorkerId":"wrk-persisted-online","term":1,"fencingToken":"wmf-test"}"#.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("first session should persist: {error}"));
        let second = repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-two".to_owned(),
                namespace_name: "finance".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                client_instance_id: "host-a#slot-1".to_owned(),
                connection_id: "conn-two".to_owned(),
                gateway_node_id: "tikeo-2".to_owned(),
                fencing_token: "token-two".to_owned(),
                lease_seconds: 30,
                capabilities_json: r#"["java"]"#.to_owned(),
                structured_capabilities_json: r#"{"tags":["java"],"sdkProcessors":["demo.echo"],"scriptRunners":[],"pluginProcessors":[]}"#.to_owned(),
                labels_json: r#"{"worker_pool":"blue"}"#.to_owned(),
                master_json: r#"{"domain":"dev-alpha/orders/local/local","isMaster":true,"masterWorkerId":"wrk-persisted-online","term":1,"fencingToken":"wmf-test"}"#.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("replacement session should persist: {error}"));

        assert_eq!(first.generation, 1);
        assert_eq!(second.generation, 2);
        assert_eq!(second.gateway_node_id, "tikeo-2");
        assert_eq!(second.current_worker_id.as_deref(), Some("wrk-two"));

        let old = repository
            .get_session("wrk-one")
            .await
            .unwrap_or_else(|error| panic!("old session lookup should run: {error}"))
            .unwrap_or_else(|| panic!("old session should remain inspectable"));
        assert_eq!(old.status, "replaced");
        assert_eq!(
            old.status_reason.as_deref(),
            Some("replaced_by_new_generation")
        );
        assert_eq!(old.replaced_by_worker_id.as_deref(), Some("wrk-two"));

        assert!(
            repository
                .heartbeat(WorkerHeartbeat {
                    worker_id: "wrk-one".to_owned(),
                    generation: first.generation,
                    fencing_token: "token-one".to_owned(),
                    sequence: 7,
                    lease_seconds: 30,
                })
                .await
                .unwrap_or_else(|error| panic!("stale heartbeat should be handled: {error}"))
                .is_none(),
            "stale replaced heartbeat must not renew the old session"
        );
        let renewed = repository
            .heartbeat(WorkerHeartbeat {
                worker_id: "wrk-two".to_owned(),
                generation: second.generation,
                fencing_token: "token-two".to_owned(),
                sequence: 8,
                lease_seconds: 30,
            })
            .await
            .unwrap_or_else(|error| panic!("current heartbeat should persist: {error}"))
            .unwrap_or_else(|| panic!("current heartbeat should be accepted"));
        assert_eq!(renewed.last_sequence, 8);

        let events = repository
            .list_session_events("wrk-one")
            .await
            .unwrap_or_else(|error| panic!("events should load: {error}"));
        assert!(
            events
                .iter()
                .any(|event| event.event_type == "session_replaced"
                    && event.reason.as_deref() == Some("replaced_by_new_generation"))
        );
    }

    #[tokio::test]
    async fn worker_lifecycle_lists_online_workers_from_persistent_sessions_after_registry_restart()
    {
        use crate::repository::{RegisterWorkerSession, WorkerLifecycleRepository};

        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = WorkerLifecycleRepository::new(db);
        repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-persisted-online".to_owned(),
                namespace_name: "dev-alpha".to_owned(),
                app_name: "orders".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "spring-boot3-worker-demo-fedora".to_owned(),
                connection_id: "conn-persisted-online".to_owned(),
                gateway_node_id: "tikeo-gateway-1".to_owned(),
                fencing_token: "token-persisted-online".to_owned(),
                lease_seconds: 30,
                capabilities_json: r#"["java"]"#.to_owned(),
                structured_capabilities_json: r#"{"tags":["java"],"sdkProcessors":["demo.echo"],"scriptRunners":[],"pluginProcessors":[]}"#.to_owned(),
                labels_json: r#"{"worker_pool":"blue"}"#.to_owned(),
                master_json: r#"{"domain":"dev-alpha/orders/local/local","isMaster":true,"masterWorkerId":"wrk-persisted-online","term":1,"fencingToken":"wmf-test"}"#.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("online session should persist: {error}"));
        repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-persisted-expired".to_owned(),
                namespace_name: "dev-alpha".to_owned(),
                app_name: "orders".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "expired-demo".to_owned(),
                connection_id: "conn-persisted-expired".to_owned(),
                gateway_node_id: "tikeo-gateway-2".to_owned(),
                fencing_token: "token-persisted-expired".to_owned(),
                lease_seconds: -1,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("expired session should persist: {error}"));

        let online = repository
            .list_online_workers(20)
            .await
            .unwrap_or_else(|error| panic!("online workers should load: {error}"));

        assert_eq!(online.len(), 1);
        assert_eq!(online[0].worker_id, "wrk-persisted-online");
        assert_eq!(online[0].gateway_node_id, "tikeo-gateway-1");
        assert_eq!(online[0].namespace_name, "dev-alpha");
        assert_eq!(online[0].app_name, "orders");
        assert_eq!(online[0].cluster, "local");
        assert_eq!(online[0].region, "local");
        assert_eq!(
            online[0].client_instance_id.as_deref(),
            Some("spring-boot3-worker-demo-fedora")
        );
        assert!(online[0].capabilities_json.contains("java"));
        assert!(online[0].structured_capabilities_json.contains("demo.echo"));
        assert!(online[0].labels_json.contains("worker_pool"));
        assert!(online[0].master_json.contains("isMaster"));
    }


    #[tokio::test]
    async fn worker_lifecycle_get_online_current_worker_rejects_registry_only_or_expired_state() {
        use crate::repository::{RegisterWorkerSession, WorkerLifecycleRepository};

        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = WorkerLifecycleRepository::new(db);
        repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-online-current".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "pod-a".to_owned(),
                connection_id: "conn-online".to_owned(),
                gateway_node_id: "tikeo-test".to_owned(),
                fencing_token: "token-online".to_owned(),
                lease_seconds: 30,
                capabilities_json: r#"["java"]"#.to_owned(),
                structured_capabilities_json: r#"{"tags":["java"],"sdkProcessors":["demo.echo"],"scriptRunners":[],"pluginProcessors":[]}"#.to_owned(),
                labels_json: r#"{"worker_pool":"blue"}"#.to_owned(),
                master_json: r#"{"domain":"default/billing/local/local","isMaster":true,"masterWorkerId":"wrk-online-current","term":1,"fencingToken":"wmf-test"}"#.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("online session should persist: {error}"));
        repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-expired-current".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "pod-expired".to_owned(),
                connection_id: "conn-expired".to_owned(),
                gateway_node_id: "tikeo-test".to_owned(),
                fencing_token: "token-expired".to_owned(),
                lease_seconds: -1,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("expired session should persist: {error}"));

        let online = repository
            .get_online_current_worker("wrk-online-current")
            .await
            .unwrap_or_else(|error| panic!("online worker lookup should run: {error}"))
            .unwrap_or_else(|| panic!("online current worker should be returned"));
        assert_eq!(online.worker_id, "wrk-online-current");
        assert_eq!(online.generation, 1);
        assert_eq!(
            repository
                .get_online_current_worker("wrk-expired-current")
                .await
                .unwrap_or_else(|error| panic!("expired worker lookup should run: {error}")),
            None
        );
        assert_eq!(
            repository
                .get_online_current_worker("wrk-registry-only")
                .await
                .unwrap_or_else(|error| panic!("missing worker lookup should run: {error}")),
            None
        );
    }

    #[tokio::test]
    async fn job_instance_attempt_assignment_token_is_persisted_and_validated() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "assignment-token".to_owned(),
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
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: tikeo_core::TriggerType::Api,
                execution_mode: tikeo_core::ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let created = attempts
            .create_pending_for_workers(&instance.id, &["wrk-one".to_owned()])
            .await
            .unwrap_or_else(|error| panic!("attempt should create: {error}"));
        assert_eq!(created[0].assignment_token, None);

        assert!(
            attempts
                .record_assignment_token(&instance.id, "wrk-one", "asg-db-token")
                .await
                .unwrap_or_else(|error| panic!("assignment token should persist: {error}"))
        );
        assert!(
            attempts
                .accepts_assignment_token(&instance.id, "wrk-one", "asg-db-token")
                .await
                .unwrap_or_else(|error| panic!("assignment token should validate: {error}"))
        );
        assert!(
            !attempts
                .accepts_assignment_token(&instance.id, "wrk-one", "wrong-token")
                .await
                .unwrap_or_else(|error| panic!("wrong token should validate false: {error}"))
        );
    }

    #[tokio::test]
    async fn worker_lifecycle_graceful_unregister_stops_current_session_with_evidence() {
        use crate::repository::{RegisterWorkerSession, WorkerLifecycleRepository};

        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = WorkerLifecycleRepository::new(db);
        let registered = repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-stop".to_owned(),
                namespace_name: "finance".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                client_instance_id: "host-a#slot-1".to_owned(),
                connection_id: "conn-stop".to_owned(),
                gateway_node_id: "tikeo-test".to_owned(),
                fencing_token: "token-stop".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("session should persist: {error}"));

        let stopped = repository
            .graceful_unregister(&registered.worker_id, registered.generation, "token-stop")
            .await
            .unwrap_or_else(|error| panic!("graceful unregister should run: {error}"))
            .unwrap_or_else(|| panic!("current fenced session should stop"));

        assert_eq!(stopped.status, "stopped");
        assert_eq!(stopped.status_reason.as_deref(), Some("graceful_shutdown"));
        let session = repository
            .get_session(&registered.worker_id)
            .await
            .unwrap_or_else(|error| panic!("stopped session should load: {error}"))
            .unwrap_or_else(|| panic!("stopped session should exist"));
        assert_eq!(session.status, "stopped");
        let events = repository
            .list_session_events(&registered.worker_id)
            .await
            .unwrap_or_else(|error| panic!("events should load: {error}"));
        assert!(
            events
                .iter()
                .any(|event| event.event_type == "graceful_shutdown"
                    && event.reason.as_deref() == Some("graceful_shutdown"))
        );
    }

    #[tokio::test]
    async fn worker_lifecycle_marks_expired_online_sessions_unknown_without_calling_them_crashes() {
        use crate::repository::{RegisterWorkerSession, WorkerLifecycleRepository};

        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = WorkerLifecycleRepository::new(db);
        let registered = repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-expired".to_owned(),
                namespace_name: "finance".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                client_instance_id: "host-a#slot-1".to_owned(),
                connection_id: "conn-expired".to_owned(),
                gateway_node_id: "tikeo-test".to_owned(),
                fencing_token: "token-expired".to_owned(),
                lease_seconds: -1,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("expired test session should persist: {error}"));

        let expired = repository
            .mark_expired_online_sessions(10)
            .await
            .unwrap_or_else(|error| panic!("lease scan should run: {error}"));

        assert_eq!(expired, vec![registered.worker_id.clone()]);
        let session = repository
            .get_session(&registered.worker_id)
            .await
            .unwrap_or_else(|error| panic!("expired session should load: {error}"))
            .unwrap_or_else(|| panic!("expired session should remain inspectable"));
        assert_eq!(session.status, "offline");
        assert_eq!(
            session.status_reason.as_deref(),
            Some("lease_expired_unknown")
        );
        assert!(
            session
                .status_evidence
                .as_deref()
                .is_some_and(
                    |evidence| evidence.contains("lease expired") && !evidence.contains("crash")
                ),
            "timeout evidence must be explicit but must not claim a crash"
        );

        let events = repository
            .list_session_events(&registered.worker_id)
            .await
            .unwrap_or_else(|error| panic!("events should load: {error}"));
        assert!(
            events
                .iter()
                .any(|event| event.event_type == "lease_expired"
                    && event.reason.as_deref() == Some("lease_expired_unknown"))
        );
    }

    #[tokio::test]
    async fn worker_lifecycle_marks_transport_errors_with_evidence() {
        use crate::repository::{RegisterWorkerSession, WorkerLifecycleRepository};

        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let repository = WorkerLifecycleRepository::new(db);
        let registered = repository
            .register_session(RegisterWorkerSession {
                worker_id: "wrk-transport".to_owned(),
                namespace_name: "finance".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                client_instance_id: "host-a#slot-1".to_owned(),
                connection_id: "conn-transport".to_owned(),
                gateway_node_id: "tikeo-test".to_owned(),
                fencing_token: "token-transport".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("transport test session should persist: {error}"));

        let offline = repository
            .mark_transport_error(&registered.worker_id, "grpc stream returned unavailable")
            .await
            .unwrap_or_else(|error| panic!("transport mark should run: {error}"))
            .unwrap_or_else(|| panic!("online session should be marked offline"));

        assert_eq!(offline.status, "offline");
        assert_eq!(offline.status_reason.as_deref(), Some("transport_error"));
        assert_eq!(
            offline.status_evidence.as_deref(),
            Some("grpc stream returned unavailable")
        );
        let events = repository
            .list_session_events(&registered.worker_id)
            .await
            .unwrap_or_else(|error| panic!("events should load: {error}"));
        assert!(events.iter().any(|event| {
            event.event_type == "transport_error"
                && event.reason.as_deref() == Some("transport_error")
                && event
                    .detail_json
                    .as_deref()
                    .is_some_and(|detail| detail.contains("grpc stream returned unavailable"))
        }));
    }

    #[tokio::test]
    async fn migration_creates_metadata_tables() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT name FROM sqlite_master WHERE type='table' AND name='jobs'".to_owned(),
            ))
            .await
            .unwrap_or_else(|error| panic!("sqlite_master query should run: {error}"));

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn script_repository_publishes_and_rolls_back_immutable_versions() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(CreateScript {
                name: "wasm-release".to_owned(),
                language: "wasm".to_owned(),
                version: "1.0.0".to_owned(),
                content: "module-v1".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(3),
                max_memory_bytes: Some(4096),
                allow_network: false,
                allowed_env_vars: Some(r#"["SAFE_ENV"]"#.to_owned()),
                policy_json: Some(r#"{"resources":{"timeout_ms":12000,"max_memory_bytes":33554432,"max_output_bytes":524288},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":["SAFE_ENV"]}"#.to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        assert_eq!(script.released_version_number, None);
        assert_eq!(script.policy["network"]["enabled"], false);
        assert_eq!(script.policy["resources"]["timeout_ms"], 12_000);
        assert_eq!(script.policy["env_vars"], serde_json::json!(["SAFE_ENV"]));
        assert_eq!(
            script.policy["filesystem"]["read_only_paths"],
            serde_json::json!([])
        );

        scripts
            .update_script(
                &script.id,
                UpdateScript {
                    name: None,
                    language: None,
                    version: Some("1.0.1".to_owned()),
                    content: Some("module-v2".to_owned()),
                    status: None,
                    timeout_seconds: None,
                    max_memory_bytes: None,
                    allow_network: None,
                    allowed_env_vars: None,
                    policy_json: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("script should update: {error}"));

        let versions = scripts
            .versions()
            .list_versions(&script.id)
            .await
            .unwrap_or_else(|error| panic!("versions should list: {error}"));
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version_number, 2);
        assert_eq!(versions[0].content, "module-v2");
        assert_eq!(versions[1].version_number, 1);
        assert_eq!(versions[1].content, "module-v1");
        assert_eq!(versions[0].policy["network"]["enabled"], false);
        assert_eq!(versions[1].policy["resources"]["timeout_ms"], 12_000);

        let published = scripts
            .publish_version(&script.id, 2, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(published.status, "approved");
        assert_eq!(
            published.released_version_id.as_deref(),
            Some(versions[0].id.as_str())
        );
        assert_eq!(published.released_version_number, Some(2));
        assert!(published.release_signature.is_none());

        let rolled_back = scripts
            .rollback_release(&script.id, 1, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should roll back: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(rolled_back.status, "approved");
        assert_eq!(
            rolled_back.released_version_id.as_deref(),
            Some(versions[1].id.as_str())
        );
        assert_eq!(rolled_back.released_version_number, Some(1));
        assert!(rolled_back.release_signature.is_none());
    }

