    #[tokio::test]
    async fn script_repository_persists_verified_release_signature_metadata() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(CreateScript {
                name: "signed-release".to_owned(),
                language: "python".to_owned(),
                version: "1.0.0".to_owned(),
                content: "print(1)".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(3),
                max_memory_bytes: Some(4096),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));

        let signed = scripts
            .publish_version(
                &script.id,
                1,
                Some(VerifiedScriptReleaseSignature {
                    approval_ticket: "CAB-42".to_owned(),
                    signature: "sha256:verified".to_owned(),
                    verified_by: "tester".to_owned(),
                }),
                None,
            )
            .await
            .unwrap_or_else(|error| panic!("signed publish should persist: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        let signature = signed
            .release_signature
            .unwrap_or_else(|| panic!("verified signature metadata should be returned"));
        assert_eq!(signature.approval_ticket, "CAB-42");
        assert_eq!(signature.signature, "sha256:verified");
        assert_eq!(signature.verified_by, "tester");
        assert!(!signature.verified_at.is_empty());

        let reloaded = scripts
            .get(&script.id)
            .await
            .unwrap_or_else(|error| panic!("script should reload: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(
            reloaded
                .release_signature
                .map(|metadata| metadata.approval_ticket),
            Some("CAB-42".to_owned())
        );
    }

    #[tokio::test]
    async fn script_repository_persists_verified_release_grant_evidence() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(CreateScript {
                name: "grant-evidence".to_owned(),
                language: "python".to_owned(),
                version: "1.0.0".to_owned(),
                content: "print(1)".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(3),
                max_memory_bytes: Some(4096),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));

        let signed = scripts
            .publish_version(
                &script.id,
                1,
                None,
                Some(VerifiedScriptReleaseGrants {
                    grants: tikeo_core::ScriptReleaseGrantSet {
                        url: vec!["https://api.example.com".to_owned()],
                        file_read: vec!["/data/input".to_owned()],
                        file_write: vec!["/data/output".to_owned()],
                        secret: vec!["secret:db-readonly".to_owned()],
                    },
                    verified_by: "grant-verifier".to_owned(),
                }),
            )
            .await
            .unwrap_or_else(|error| panic!("grant evidence should persist: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        let evidence = signed
            .release_grants
            .unwrap_or_else(|| panic!("verified grant evidence should be returned"));
        assert_eq!(evidence.url, ["https://api.example.com"]);
        assert_eq!(evidence.secret, ["secret:db-readonly"]);
        assert_eq!(evidence.verified_by, "grant-verifier");
        assert!(!evidence.verified_at.is_empty());

        let reloaded = scripts
            .get(&script.id)
            .await
            .unwrap_or_else(|error| panic!("script should reload: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(
            reloaded.release_grants.map(|metadata| metadata.file_read),
            Some(vec!["/data/input".to_owned()])
        );
    }

    #[tokio::test]
    async fn raft_repository_upserts_metadata_and_members_without_foreign_keys() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let repository = RaftRepository::new(db);

        let metadata = repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                current_term: 1,
                voted_for: Some("tikeo-1".to_owned()),
                commit_index: 2,
                applied_index: 1,
                leader_fencing_token: Some("term-1-node-tikeo-1".to_owned()),
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should upsert: {error}"));
        assert_eq!(metadata.node_id, "tikeo-1");
        assert_eq!(metadata.current_term, 1);
        assert_eq!(
            metadata.leader_fencing_token.as_deref(),
            Some("term-1-node-tikeo-1")
        );

        let updated = repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                current_term: 2,
                voted_for: None,
                commit_index: 4,
                applied_index: 4,
                leader_fencing_token: None,
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should update: {error}"));
        assert_eq!(updated.id, metadata.id);
        assert_eq!(updated.current_term, 2);
        assert_eq!(updated.voted_for, None);
        assert_eq!(updated.leader_fencing_token, None);

        let member = repository
            .upsert_member(UpsertRaftMember {
                node_id: "tikeo-1".to_owned(),
                endpoint: "http://tikeo-1:9999".to_owned(),
                status: "configured".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("member should upsert: {error}"));
        assert_eq!(member.node_id, "tikeo-1");

        let members = repository
            .list_members()
            .await
            .unwrap_or_else(|error| panic!("members should list: {error}"));
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].endpoint, "http://tikeo-1:9999");

        let log = repository
            .upsert_log_entry(UpsertRaftLogEntry {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                log_index: 1,
                term: 2,
                entry_type: "EntryNormal".to_owned(),
                data: "cGl4ZWw=".to_owned(),
                context: None,
                sync_status: "persisted".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("raft log should upsert: {error}"));
        assert_eq!(log.log_index, 1);
        assert_eq!(log.term, 2);

        let logs = repository
            .list_log_entries("tikeo-1", 1, 10)
            .await
            .unwrap_or_else(|error| panic!("raft logs should list: {error}"));
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].entry_type, "EntryNormal");

        let snapshot = repository
            .upsert_snapshot(UpsertRaftSnapshot {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                snapshot_index: 4,
                term: 2,
                conf_state: Some("e30=".to_owned()),
                data: None,
            })
            .await
            .unwrap_or_else(|error| panic!("raft snapshot should upsert: {error}"));
        assert_eq!(snapshot.snapshot_index, 4);
        assert_eq!(snapshot.term, 2);
    }

    #[tokio::test]
    async fn raft_repository_updates_applied_index_and_fencing_token() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let repository = RaftRepository::new(db);
        repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                current_term: 2,
                voted_for: None,
                commit_index: 4,
                applied_index: 4,
                leader_fencing_token: None,
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should upsert: {error}"));

        let older_applied = repository
            .update_applied_index("tikeo-1", 3)
            .await
            .unwrap_or_else(|error| panic!("applied index should update: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(older_applied.applied_index, 4);
        let newer_applied = repository
            .update_applied_index("tikeo-1", 6)
            .await
            .unwrap_or_else(|error| panic!("applied index should update: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(newer_applied.applied_index, 6);

        let fenced = repository
            .update_leader_fencing_token("tikeo-1", Some("raft:term:2:node:tikeo-1".to_owned()))
            .await
            .unwrap_or_else(|error| panic!("fencing token should update: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(
            fenced.leader_fencing_token.as_deref(),
            Some("raft:term:2:node:tikeo-1")
        );
        let cleared = repository
            .update_leader_fencing_token("tikeo-1", None)
            .await
            .unwrap_or_else(|error| panic!("fencing token should clear: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(cleared.leader_fencing_token, None);

        let command = repository
            .record_applied_command(RecordRaftAppliedCommand {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                log_index: 7,
                term: 2,
                command_id: "cmd-noop-1".to_owned(),
                command_type: "noop".to_owned(),
                payload: Some(r#"{"source":"test"}"#.to_owned()),
                status: "applied".to_owned(),
                message: "noop command applied idempotently".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("applied command should record: {error}"));
        let duplicate = repository
            .record_applied_command(RecordRaftAppliedCommand {
                cluster_id: "default".to_owned(),
                node_id: "tikeo-1".to_owned(),
                log_index: 7,
                term: 2,
                command_id: "cmd-noop-1-duplicate".to_owned(),
                command_type: "noop".to_owned(),
                payload: None,
                status: "applied".to_owned(),
                message: "duplicate should return existing".to_owned(),
            })
            .await
            .unwrap_or_else(|error| {
                panic!("duplicate applied command should be idempotent: {error}")
            });
        let commands = repository
            .list_applied_commands("tikeo-1")
            .await
            .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
        assert_eq!(duplicate.id, command.id);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command_id, "cmd-noop-1");
    }

    #[tokio::test]
    async fn job_retry_policy_defaults_and_updates_are_versioned() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = super::JobRepository::new(db);
        let created = jobs
            .create_job(super::CreateJob {
                created_by: Some("admin".to_owned()),
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "retry-default".to_owned(),
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
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));

        assert_eq!(created.retry_policy.max_attempts, 3);
        assert_eq!(created.retry_policy.initial_delay_seconds, 5);
        assert_eq!(created.retry_policy.backoff_multiplier, 2);
        assert_eq!(created.retry_policy.max_delay_seconds, 60);

        let updated = jobs
            .update_job(
                &created.id,
                super::UpdateJob {
                    retry_policy: Some(super::job::JobRetryPolicy {
                        enabled: true,
                        max_attempts: 5,
                        initial_delay_seconds: 10,
                        backoff_multiplier: 3,
                        max_delay_seconds: 120,
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_else(|error| panic!("job should update: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        assert_eq!(updated.retry_policy.max_attempts, 5);
        assert_eq!(updated.version_number, 2);

        let version = jobs
            .versions()
            .get_version_by_number(&created.id, 2)
            .await
            .unwrap_or_else(|error| panic!("version should load: {error}"))
            .unwrap_or_else(|| panic!("version should exist"));
        assert_eq!(version.retry_policy.max_attempts, 5);
        assert_eq!(version.retry_policy.max_delay_seconds, 120);
    }

    #[tokio::test]
    async fn failed_single_instance_can_be_requeued_with_attempt_preserved_for_retry() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = super::JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let job = jobs
            .create_job(super::CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "retry-queue".to_owned(),
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
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(super::CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should claim: {error}"))
            .unwrap_or_else(|| panic!("queue item should exist"));
        assert_eq!(claim.item.attempt, 1);
        workflows
            .mark_dispatch_queue_running(&claim.item.id, "server-a")
            .await
            .unwrap_or_else(|error| panic!("queue should run: {error}"));
        instances
            .update_status(&instance.id, InstanceStatus::Running)
            .await
            .unwrap_or_else(|error| panic!("instance should run: {error}"));

        let requeued = workflows
            .requeue_dispatch_queue_for_retry(&instance.id, 7)
            .await
            .unwrap_or_else(|error| panic!("queue should requeue: {error}"))
            .unwrap_or_else(|| panic!("queue should be requeued"));
        assert_eq!(requeued.status, "pending");
        let retrying_instance = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should reload: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(retrying_instance.status, InstanceStatus::Retrying);
        assert_eq!(requeued.attempt, 1);
        assert!(requeued.run_after > claim.item.run_after);

        let retry_claim = workflows
            .claim_next_job_queue_item("server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("retry should claim: {error}"));
        assert!(
            retry_claim.is_none(),
            "retry must wait until run_after is due"
        );
    }

    #[tokio::test]
    async fn repository_creates_and_lists_jobs() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let repository = JobRepository::new(db);

        let created = repository
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "nightly".to_owned(),
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

        let jobs = repository
            .list_jobs()
            .await
            .unwrap_or_else(|error| panic!("jobs should list: {error}"));
        let scheduled = repository
            .list_enabled_scheduled_jobs()
            .await
            .unwrap_or_else(|error| panic!("scheduled jobs should list: {error}"));

        assert_eq!(created.name, "nightly");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].app, "billing");
        assert!(jobs[0].enabled);
        assert!(scheduled.is_empty());
    }

    #[tokio::test]
    async fn repository_creates_and_lists_job_instances() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db);

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

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));

        assert_eq!(instance.status, InstanceStatus::Pending);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trigger_type, TriggerType::Api);

        let pending = instances
            .list_pending_single(10)
            .await
            .unwrap_or_else(|error| panic!("pending instances should list: {error}"));
        assert_eq!(pending.len(), 1);

        let updated = instances
            .update_status(&instance.id, InstanceStatus::Running)
            .await
            .unwrap_or_else(|error| panic!("instance status should update: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn repository_appends_and_lists_job_instance_logs() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db.clone());
        let logs = super::JobInstanceLogRepository::new(db);
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
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        logs.append(AppendJobInstanceLog {
            instance_id: instance.id.clone(),
            worker_id: "worker-1".to_owned(),
            level: "info".to_owned(),
            message: "hello".to_owned(),
            sequence: 0,
        })
        .await
        .unwrap_or_else(|error| panic!("log should append: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));

        let listed = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should list: {error}"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].message, "hello");
    }

    #[tokio::test]
    async fn auth_session_repository_deletes_expired_rows() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db.clone());
        let admin = users
            .create_user(super::CreateUser {
                username: "bootstrap-admin".to_owned(),
                email: "bootstrap-admin@example.com".to_owned(),
                password: "$2b$10$adminhash".to_owned(),
                role: "owner".to_owned(),
                bootstrap_admin: true,
            })
            .await
            .unwrap_or_else(|error| panic!("admin should insert: {error}"));
        let sessions = super::AuthSessionRepository::new(db.clone());
        auth_session::ActiveModel {
            id: Set("expired-session".to_owned()),
            user_id: Set(admin.id),
            token_hash: Set("expired-token-hash".to_owned()),
            device_id: Set(None),
            device_name: Set(None),
            expires_at: Set("1970-01-01T00:00:00Z".to_owned()),
            created_at: Set("1970-01-01T00:00:00Z".to_owned()),
            updated_at: Set("1970-01-01T00:00:00Z".to_owned()),
        }
        .insert(&db)
        .await
        .unwrap_or_else(|error| panic!("expired session should insert: {error}"));

        let deleted = sessions
            .delete_expired()
            .await
            .unwrap_or_else(|error| panic!("expired session should delete: {error}"));
        assert_eq!(deleted, 1);
        let loaded = sessions
            .get_by_token_hash("expired-token-hash")
            .await
            .unwrap_or_else(|error| panic!("session lookup should work: {error}"));
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn auth_session_repository_renews_valid_session_expiry() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db.clone());
        let admin = users
            .create_user(super::CreateUser {
                username: "renew-admin".to_owned(),
                email: "renew-admin@example.com".to_owned(),
                password: "$2b$10$adminhash".to_owned(),
                role: "owner".to_owned(),
                bootstrap_admin: true,
            })
            .await
            .unwrap_or_else(|error| panic!("admin should insert: {error}"));
        let sessions = super::AuthSessionRepository::new(db.clone());
        let original_expires_at = "2099-01-01T00:00:00Z".to_owned();
        let next_expires_at = "2099-01-08T00:00:00Z".to_owned();
        auth_session::ActiveModel {
            id: Set("renew-session".to_owned()),
            user_id: Set(admin.id),
            token_hash: Set("renew-token-hash".to_owned()),
            device_id: Set(None),
            device_name: Set(None),
            expires_at: Set(original_expires_at),
            created_at: Set("2026-01-01T00:00:00Z".to_owned()),
            updated_at: Set("2026-01-01T00:00:00Z".to_owned()),
        }
        .insert(&db)
        .await
        .unwrap_or_else(|error| panic!("valid session should insert: {error}"));

        let renewed = sessions
            .renew_expires_at("renew-token-hash", next_expires_at.clone())
            .await
            .unwrap_or_else(|error| panic!("session expiry should renew: {error}"));
        assert!(renewed);

        let loaded = sessions
            .get_by_token_hash("renew-token-hash")
            .await
            .unwrap_or_else(|error| panic!("session lookup should work: {error}"))
            .unwrap_or_else(|| panic!("renewed session should remain valid"));
        assert_eq!(loaded.expires_at, next_expires_at);
    }

    #[tokio::test]
    async fn user_repository_crud_operations() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db);

        let listed_empty = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert!(listed_empty.is_empty());

        // Create user
        let user = users
            .create_user(super::CreateUser {
                username: "operator-1".to_owned(),
                email: "operator-1@example.com".to_owned(),
                password: "$2b$10$operatorhash".to_owned(),
                role: "operator".to_owned(),
                bootstrap_admin: false,
            })
            .await
            .unwrap_or_else(|error| panic!("should create user: {error}"));
        assert_eq!(user.username, "operator-1");
        assert_eq!(user.role, "operator");

        // List users
        let listed = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert_eq!(listed.len(), 1);

        // Update user
        let updated = users
            .update_user(
                &user.id,
                super::UpdateUser {
                    email: Some("operator-updated@example.com".to_owned()),
                    password: None,
                    role: Some("viewer".to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("should update user: {error}"))
            .unwrap_or_else(|| panic!("user should exist"));
        assert_eq!(updated.role, "viewer");
        assert_eq!(updated.email, "operator-updated@example.com");

        // Delete user
        let deleted = users
            .delete_user(&user.id)
            .await
            .unwrap_or_else(|error| panic!("should delete user: {error}"));
        assert!(deleted);

        // List users again
        let listed_again = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert!(listed_again.is_empty());
    }
