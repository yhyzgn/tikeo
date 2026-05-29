//! Repository APIs over tikee metadata tables.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    missing_docs
)]

mod alert;
mod attempt;
mod audit;
mod auth;
mod instance;
mod job;
mod job_repo;
mod job_version;
mod log;
mod oidc;
mod oidc_identity;
mod plugin;
mod raft;
mod scope;
mod script;
mod sdk_api_key;
mod secret;
mod user;
pub mod util;
mod worker_lifecycle;
mod workflow;

pub use alert::{
    AlertDeliveryAttemptFilters, AlertDeliveryAttemptSummary, AlertEventFilters, AlertEventSummary,
    AlertRepository, AlertRuleSummary, CreateAlertRule, RecordAlertDeliveryAttempt,
};
pub use attempt::{
    CreateJobInstanceAttempt, JobInstanceAttemptRepository, JobInstanceAttemptSummary,
};
pub use audit::{
    AuditLogFilters, AuditLogPageSummary, AuditLogRepository, AuditLogSummary, CreateAuditLog,
};
pub use auth::{
    AuthSessionRepository, AuthSessionSummary, CreateAuthSession, PermissionSummary, RbacRepository,
};
pub use instance::{
    CreateJobInstance, JobDurationHistory, JobInstanceRepository, JobInstanceSummary,
};
pub use job::{CreateJob, JobSummary, UpdateJob};
pub use job_repo::JobRepository;
pub use job_version::{JobVersionRepository, JobVersionSummary};
pub use log::{AppendJobInstanceLog, JobInstanceLogRepository, JobInstanceLogSummary};
pub use oidc::{CreateOidcAuthState, OidcAuthStateRepository, OidcAuthStateSummary};
pub use oidc_identity::{OidcIdentityRepository, OidcIdentitySummary, UpsertOidcIdentity};
pub use plugin::{
    CreatePlugin, PluginAlertChannelTypeSummary, PluginProcessorTypeSummary, PluginRepository,
    PluginSummary, UpdatePlugin,
};
pub use raft::{
    RaftAppliedCommandSummary, RaftLogEntrySummary, RaftMemberSummary,
    RaftMembershipProposalSummary, RaftMetadataSummary, RaftRepository, RaftSnapshotSummary,
    RecordRaftAppliedCommand, RecordRaftMembershipProposal, UpsertRaftLogEntry, UpsertRaftMember,
    UpsertRaftMetadata, UpsertRaftSnapshot,
};
pub use scope::{
    AppSummary, NamespaceSummary, ScopeRepository, UpdateWorkerPoolQuota, WorkerPoolSummary,
};
pub use script::{
    CreateScript, ScriptReleaseGrantEvidenceSummary, ScriptReleaseSignatureSummary,
    ScriptRepository, ScriptSummary, ScriptVersionRepository, ScriptVersionSummary, UpdateScript,
    VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature,
};
pub use sdk_api_key::{CreateSdkApiKey, SdkApiKeyRepository, SdkApiKeySummary, UpdateSdkApiKey};
pub use secret::{CreateSecret, SecretRepository, SecretSummary};
pub use user::{CreateUser, UpdateUser, UserRepository, UserSummary};
pub use worker_lifecycle::{
    RegisterWorkerSession, WorkerHeartbeat, WorkerLifecycleRepository, WorkerSessionEventSummary,
    WorkerSessionSummary,
};
pub use workflow::{
    AdvanceWorkflowInput, AdvanceWorkflowResult, CompleteWorkflowShardInput,
    CompleteWorkflowShardResult, CreateWorkflow, DispatchQueueClaim, DispatchQueueSloSummary,
    DispatchQueueSummary, InstanceEventSummary, MaterializeWorkflowNodeResult, QueueOverview,
    RebalanceWorkflowShardsInput, RebalanceWorkflowShardsResult, RecoverWorkflowNodeInput,
    RecoverWorkflowNodeResult, UpdateWorkflow, WorkflowDefinition, WorkflowEdgeSpec,
    WorkflowInstanceSummary, WorkflowJobResultOutcome, WorkflowNodeInstanceSummary,
    WorkflowNodeSpec, WorkflowRepository, WorkflowShardSummary, WorkflowSloSummary,
    WorkflowSummary, WorkflowValidationResult, validate_workflow_definition,
};

#[cfg(test)]
mod tests {
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, EntityTrait, QueryFilter, Set,
        Statement,
    };
    use sea_orm_migration::MigratorTrait;

    use tikee_core::{ExecutionMode, InstanceStatus, TriggerType};

    use crate::{
        entities::auth_session,
        migration::Migrator,
        repository::{
            AppendJobInstanceLog, CreateJob, CreateJobInstance, CreateScript, RaftRepository,
            RecordRaftAppliedCommand, ScriptRepository, UpdateJob, UpdateScript,
            UpsertRaftLogEntry, UpsertRaftMember, UpsertRaftMetadata, UpsertRaftSnapshot,
            VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature,
        },
    };

    use super::{JobInstanceRepository, JobRepository};

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
        assert!(
            repository
                .resolve_processor_type("sql")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            repository
                .resolve_alert_channel_type("ops_webhook")
                .await
                .unwrap()
                .is_some()
        );
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
                processor_name: Some("demo.echo".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                fencing_token: "token-one".to_owned(),
                lease_seconds: 30,
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
                fencing_token: "token-two".to_owned(),
                lease_seconds: 30,
            })
            .await
            .unwrap_or_else(|error| panic!("replacement session should persist: {error}"));

        assert_eq!(first.generation, 1);
        assert_eq!(second.generation, 2);
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
                fencing_token: "token-stop".to_owned(),
                lease_seconds: 30,
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
                fencing_token: "token-expired".to_owned(),
                lease_seconds: -1,
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
                fencing_token: "token-transport".to_owned(),
                lease_seconds: 30,
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
                    grants: tikee_core::ScriptReleaseGrantSet {
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
                node_id: "tikee-1".to_owned(),
                current_term: 1,
                voted_for: Some("tikee-1".to_owned()),
                commit_index: 2,
                applied_index: 1,
                leader_fencing_token: Some("term-1-node-tikee-1".to_owned()),
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should upsert: {error}"));
        assert_eq!(metadata.node_id, "tikee-1");
        assert_eq!(metadata.current_term, 1);
        assert_eq!(
            metadata.leader_fencing_token.as_deref(),
            Some("term-1-node-tikee-1")
        );

        let updated = repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: "default".to_owned(),
                node_id: "tikee-1".to_owned(),
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
                node_id: "tikee-1".to_owned(),
                endpoint: "http://tikee-1:9999".to_owned(),
                status: "configured".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("member should upsert: {error}"));
        assert_eq!(member.node_id, "tikee-1");

        let members = repository
            .list_members()
            .await
            .unwrap_or_else(|error| panic!("members should list: {error}"));
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].endpoint, "http://tikee-1:9999");

        let log = repository
            .upsert_log_entry(UpsertRaftLogEntry {
                cluster_id: "default".to_owned(),
                node_id: "tikee-1".to_owned(),
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
            .list_log_entries("tikee-1", 1, 10)
            .await
            .unwrap_or_else(|error| panic!("raft logs should list: {error}"));
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].entry_type, "EntryNormal");

        let snapshot = repository
            .upsert_snapshot(UpsertRaftSnapshot {
                cluster_id: "default".to_owned(),
                node_id: "tikee-1".to_owned(),
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
                node_id: "tikee-1".to_owned(),
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
            .update_applied_index("tikee-1", 3)
            .await
            .unwrap_or_else(|error| panic!("applied index should update: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(older_applied.applied_index, 4);
        let newer_applied = repository
            .update_applied_index("tikee-1", 6)
            .await
            .unwrap_or_else(|error| panic!("applied index should update: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(newer_applied.applied_index, 6);

        let fenced = repository
            .update_leader_fencing_token("tikee-1", Some("raft:term:2:node:tikee-1".to_owned()))
            .await
            .unwrap_or_else(|error| panic!("fencing token should update: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(
            fenced.leader_fencing_token.as_deref(),
            Some("raft:term:2:node:tikee-1")
        );
        let cleared = repository
            .update_leader_fencing_token("tikee-1", None)
            .await
            .unwrap_or_else(|error| panic!("fencing token should clear: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        assert_eq!(cleared.leader_fencing_token, None);

        let command = repository
            .record_applied_command(RecordRaftAppliedCommand {
                cluster_id: "default".to_owned(),
                node_id: "tikee-1".to_owned(),
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
                node_id: "tikee-1".to_owned(),
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
            .list_applied_commands("tikee-1")
            .await
            .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
        assert_eq!(duplicate.id, command.id);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command_id, "cmd-noop-1");
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
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
            .get_by_username("tikee_init")
            .await
            .unwrap_or_else(|error| panic!("admin lookup should work: {error}"))
            .unwrap_or_else(|| panic!("seeded admin should exist"));
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
    async fn user_repository_crud_operations() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db);

        // Seeding checked
        let admin = users
            .get_by_username("tikee_init")
            .await
            .unwrap_or_else(|error| panic!("should load admin user: {error}"));
        let admin = admin.unwrap_or_else(|| panic!("seeded admin should exist"));
        assert_eq!(admin.role, "admin");

        // Create user
        let user = users
            .create_user(super::CreateUser {
                username: "operator-1".to_owned(),
                password: "$2b$10$operatorhash".to_owned(),
                role: "operator".to_owned(),
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
        assert_eq!(listed.len(), 2); // admin + operator-1

        // Update user
        let updated = users
            .update_user(
                &user.id,
                super::UpdateUser {
                    password: None,
                    role: Some("viewer".to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("should update user: {error}"))
            .unwrap_or_else(|| panic!("user should exist"));
        assert_eq!(updated.role, "viewer");

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
        assert_eq!(listed_again.len(), 1); // just admin
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_job_result_auto_advances_next_node() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let first_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "first".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("first job should be created: {error}"));
        let second_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "second".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("second job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "auto-advance".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "first".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(first_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                        super::WorkflowNodeSpec {
                            key: "second".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(second_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "first".to_owned(),
                        to: "second".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("node should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued node should exist"));
        assert_eq!(materialized.queue_item.status, "done");
        assert_eq!(materialized.queue_item.lease_owner, None);
        let job_claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("job queue should claim: {error}"))
            .unwrap_or_else(|| panic!("job queue item should exist"));
        assert_eq!(job_claim.item.attempt, 1);
        let job_instance_id = materialized
            .node
            .job_instance_id
            .clone()
            .unwrap_or_else(|| panic!("job node should create job instance"));

        let running_marked = workflows
            .mark_dispatch_queue_running(&job_claim.item.id, "server-a")
            .await
            .unwrap_or_else(|error| panic!("job queue should mark running: {error}"));
        assert!(running_marked);

        let outcome = workflows
            .complete_job_node_from_result(&job_instance_id, InstanceStatus::Succeeded, None)
            .await
            .unwrap_or_else(|error| panic!("workflow should advance from job result: {error}"))
            .unwrap_or_else(|| panic!("job should be linked to workflow node"));

        assert_eq!(outcome.node_key, "first");
        assert_eq!(outcome.status, "succeeded");
        assert_eq!(outcome.queued_nodes, vec!["second".to_owned()]);
        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.status, "running");
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn dispatch_queue_can_close_by_terminal_job_instance() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "terminal-close".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
            .unwrap_or_else(|error| panic!("instance should become terminal: {error}"));
        assert!(
            workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await
                .unwrap_or_else(|error| panic!("queue should close: {error}"))
        );

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue overview should load: {error}"));
        assert_eq!(overview.pending, 0);
        assert_eq!(overview.running, 0);
        assert_eq!(overview.done, 1);
        assert_eq!(overview.items[0].status, "done");
        assert!(overview.items[0].lease_owner.is_none());
    }

    #[tokio::test]
    async fn dispatch_queue_claim_sets_and_releases_lease() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "claimable".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "claim-flow".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "start".to_owned(),
                        name: None,
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id),
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let _instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let claim = workflows
            .claim_next_dispatch_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should claim: {error}"))
            .unwrap_or_else(|| panic!("queue item should be claimable"));
        assert_eq!(claim.lease_owner, "server-a");
        assert_eq!(claim.item.lease_owner.as_deref(), Some("server-a"));
        assert_eq!(
            claim.item.fencing_token.as_deref(),
            Some(claim.fencing_token.as_str())
        );
        assert!(claim.fencing_token.starts_with("lease:server-a:"));
        assert_eq!(claim.item.attempt, 1);
        assert!(claim.item.workflow_node_instance_id.is_some());

        let cleared = workflows
            .clear_expired_dispatch_queue_leases()
            .await
            .unwrap_or_else(|error| panic!("expired lease cleanup should run: {error}"));
        assert_eq!(cleared, 0);

        let second_claim = workflows
            .claim_dispatch_queue_item(&claim.item.id, "server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("second claim should not error: {error}"));
        assert!(second_claim.is_none());
        assert!(
            workflows
                .release_dispatch_queue_item(&claim.item.id, "server-a")
                .await
                .unwrap_or_else(|error| panic!("release should succeed: {error}"))
        );
        let reclaimed = workflows
            .claim_dispatch_queue_item_with_fencing(
                &claim.item.id,
                "server-b",
                30,
                Some("raft:server-b:term-2"),
            )
            .await
            .unwrap_or_else(|error| panic!("reclaim should succeed: {error}"))
            .unwrap_or_else(|| panic!("released item should be claimable"));
        assert_eq!(reclaimed.lease_owner, "server-b");
        assert_eq!(reclaimed.fencing_token, "raft:server-b:term-2");
        assert_eq!(
            reclaimed.item.fencing_token.as_deref(),
            Some("raft:server-b:term-2")
        );
        assert_eq!(reclaimed.item.attempt, 2);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_shards_complete_and_advance_successor() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let reduce_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "reduce".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "shards".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "map".to_owned(),
                            name: None,
                            kind: Some("map".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: Some(vec![
                                serde_json::json!({"n": 1}),
                                serde_json::json!({"n": 2}),
                            ]),
                            config: None,
                        },
                        super::WorkflowNodeSpec {
                            key: "reduce".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(reduce_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "map".to_owned(),
                        to: "reduce".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map should materialize: {error}"))
            .unwrap_or_else(|| panic!("map queue should exist"));
        assert_eq!(materialized.shards.len(), 2);
        assert!(
            materialized
                .shards
                .iter()
                .all(|shard| shard.job_instance_id.is_some())
        );

        let first = workflows
            .complete_workflow_shard(
                &materialized.shards[0].id,
                super::CompleteWorkflowShardInput {
                    status: "succeeded".to_owned(),
                    output: Some(serde_json::json!({"ok": 1})),
                    checkpoint: None,
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("first shard should complete: {error}"))
            .unwrap_or_else(|| panic!("first shard should exist"));
        assert!(!first.node_completed);
        assert!(first.advance.is_none());

        let second = workflows
            .complete_workflow_shard(
                &materialized.shards[1].id,
                super::CompleteWorkflowShardInput {
                    status: "succeeded".to_owned(),
                    output: Some(serde_json::json!({"ok": 2})),
                    checkpoint: None,
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("second shard should complete: {error}"))
            .unwrap_or_else(|| panic!("second shard should exist"));
        assert!(second.node_completed);
        assert_eq!(second.node_status.as_deref(), Some("succeeded"));
        assert_eq!(
            second
                .advance
                .as_ref()
                .map(|advance| advance.queued_nodes.as_slice()),
            Some(&["reduce".to_owned()][..])
        );

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn cancel_job_instance_closes_dispatch_queue() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cancel-me".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
            .unwrap_or_else(|| panic!("job should exist"));

        assert!(
            workflows
                .cancel_job_instance(&instance.id)
                .await
                .unwrap_or_else(|error| panic!("cancel should persist: {error}"))
        );
        let reloaded = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should reload: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(reloaded.status, InstanceStatus::Cancelled);
        let queue = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue overview should load: {error}"));
        assert_eq!(queue.items[0].status, "cancelled");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_map_reduce_writes_reduce_chunks_and_manifest() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db.clone());
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "map-reduce-manifest".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "reduce".to_owned(),
                        name: None,
                        kind: Some("map_reduce".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: Some(vec![
                            serde_json::json!({"n": 1}),
                            serde_json::json!({"n": 2}),
                            serde_json::json!({"n": 3}),
                        ]),
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map_reduce should materialize: {error}"))
            .unwrap_or_else(|| panic!("map_reduce queue should exist"));
        for (index, shard) in materialized.shards.iter().enumerate() {
            workflows
                .complete_workflow_shard(
                    &shard.id,
                    super::CompleteWorkflowShardInput {
                        status: "succeeded".to_owned(),
                        output: Some(serde_json::json!({"ok": index})),
                        checkpoint: Some(serde_json::json!({"offset": index})),
                        message: None,
                    },
                )
                .await
                .unwrap_or_else(|error| panic!("shard should complete: {error}"));
        }
        let events = crate::entities::instance_event::Entity::find()
            .filter(crate::entities::instance_event::Column::InstanceId.eq(instance.id))
            .all(&db)
            .await
            .unwrap_or_else(|error| panic!("events should load: {error}"));
        assert!(
            events
                .iter()
                .any(|event| event.event_type == "workflow.map_reduce.chunk")
        );
        let manifest = events
            .iter()
            .find(|event| event.event_type == "workflow.map_reduce.manifest")
            .unwrap_or_else(|| panic!("manifest event should exist"));
        let payload: serde_json::Value =
            serde_json::from_str(manifest.payload.as_deref().unwrap_or("{}"))
                .unwrap_or_else(|error| panic!("manifest payload should parse: {error}"));
        assert_eq!(payload["totalShards"], 3);
        assert_eq!(payload["spilled"], true);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_failed_shard_rebalance_preserves_checkpoint_and_requeues() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "rebalance-shards".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "map".to_owned(),
                        name: None,
                        kind: Some("map".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: Some(vec![serde_json::json!({"n": 1})]),
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map should materialize: {error}"))
            .unwrap_or_else(|| panic!("map queue should exist"));
        let failed = workflows
            .complete_workflow_shard(
                &materialized.shards[0].id,
                super::CompleteWorkflowShardInput {
                    status: "failed".to_owned(),
                    output: Some(serde_json::json!({"error": "boom"})),
                    checkpoint: Some(serde_json::json!({"offset": 42})),
                    message: Some("failed with checkpoint".to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("shard should fail: {error}"))
            .unwrap_or_else(|| panic!("shard should exist"));
        assert_eq!(
            failed.shard.checkpoint,
            Some(serde_json::json!({"offset": 42}))
        );

        let rebalanced = workflows
            .rebalance_workflow_shards(
                &instance.id,
                super::RebalanceWorkflowShardsInput {
                    node_key: Some("map".to_owned()),
                    statuses: Some(vec!["failed".to_owned()]),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("shards should rebalance: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));

        assert_eq!(rebalanced.requeued_shards.len(), 1);
        assert_eq!(rebalanced.requeued_shards[0].status, "pending");
        assert_eq!(rebalanced.requeued_shards[0].retry_count, 1);
        assert_eq!(
            rebalanced.requeued_shards[0].checkpoint,
            Some(serde_json::json!({"offset": 42}))
        );
        assert!(rebalanced.requeued_shards[0].job_instance_id.is_some());
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn child_workflow_completion_advances_parent_node() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let child_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "child-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let child = workflows
            .create_workflow(super::CreateWorkflow {
                name: "child".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "child-task".to_owned(),
                        name: None,
                        kind: Some("job".to_owned()),
                        job_id: Some(child_job.id),
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("child workflow should be created: {error}"));
        let parent = workflows
            .create_workflow(super::CreateWorkflow {
                name: "parent".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "child".to_owned(),
                        name: None,
                        kind: Some("sub_workflow".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: Some(child.id),
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("parent workflow should be created: {error}"));
        let parent_instance = workflows
            .run_workflow(&parent.id, "api")
            .await
            .unwrap_or_else(|error| panic!("parent should run: {error}"))
            .unwrap_or_else(|| panic!("parent should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("sub workflow should materialize: {error}"))
            .unwrap_or_else(|| panic!("sub workflow queue should exist"));
        let child_instance_id = materialized
            .node
            .child_workflow_instance_id
            .clone()
            .unwrap_or_else(|| panic!("child instance id should exist"));

        let advanced = workflows
            .advance_workflow(
                &child_instance_id,
                super::AdvanceWorkflowInput {
                    node_key: "child-task".to_owned(),
                    status: "succeeded".to_owned(),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("child should advance: {error}"))
            .unwrap_or_else(|| panic!("child should exist"));
        assert!(advanced.completed);

        let refreshed = workflows
            .get_workflow_instance(&parent_instance.id)
            .await
            .unwrap_or_else(|error| panic!("parent should load: {error}"))
            .unwrap_or_else(|| panic!("parent should exist"));
        assert_eq!(refreshed.status, "succeeded");
        assert_eq!(refreshed.nodes[0].status, "succeeded");
    }

    #[tokio::test]
    async fn workflow_condition_node_routes_failure_branch_and_auto_advances() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let false_branch_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "false-branch".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "condition-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "gate".to_owned(),
                            name: None,
                            kind: Some("condition".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({"expression": "false"})),
                        },
                        super::WorkflowNodeSpec {
                            key: "false-task".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(false_branch_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "gate".to_owned(),
                        to: "false-task".to_owned(),
                        condition: Some("on_failure".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("condition should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued condition should exist"));

        assert_eq!(materialized.node.node_key, "gate");
        assert_eq!(materialized.node.status, "failed");
        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "failed");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn workflow_condition_node_evaluates_safe_typed_expression() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "typed-condition-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "gate".to_owned(),
                            name: None,
                            kind: Some("condition".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({
                                "expression": "vars.env == 'prod' && vars.progress >= 90 && vars.approved == true",
                                "vars": {
                                    "env": "prod",
                                    "progress": 95,
                                    "approved": true
                                }
                            })),
                        },
                        super::WorkflowNodeSpec {
                            key: "end".to_owned(),
                            name: None,
                            kind: Some("end".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "gate".to_owned(),
                        to: "end".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("condition should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued condition should exist"));
        assert_eq!(materialized.node.node_key, "gate");
        assert_eq!(materialized.node.status, "succeeded");

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn workflow_compensation_node_auto_advances_after_failure_branch() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "compensation-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "gate".to_owned(),
                            name: None,
                            kind: Some("condition".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({"expression": "false"})),
                        },
                        super::WorkflowNodeSpec {
                            key: "rollback".to_owned(),
                            name: None,
                            kind: Some("compensation".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(
                                serde_json::json!({"compensates": "gate", "strategy": "saga"}),
                            ),
                        },
                        super::WorkflowNodeSpec {
                            key: "end".to_owned(),
                            name: None,
                            kind: Some("end".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![
                        super::WorkflowEdgeSpec {
                            from: "gate".to_owned(),
                            to: "rollback".to_owned(),
                            condition: Some("on_failure".to_owned()),
                        },
                        super::WorkflowEdgeSpec {
                            from: "rollback".to_owned(),
                            to: "end".to_owned(),
                            condition: Some("on_success".to_owned()),
                        },
                    ],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("condition should materialize: {error}"));
        let compensation = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("compensation should materialize: {error}"))
            .unwrap_or_else(|| panic!("compensation should queue"));
        assert_eq!(compensation.node.node_key, "rollback");
        assert_eq!(compensation.node.status, "succeeded");

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "failed");
        assert_eq!(refreshed.nodes[1].status, "succeeded");
        assert_eq!(refreshed.nodes[2].status, "queued");
    }

    #[tokio::test]
    async fn workflow_delay_node_uses_run_after_before_materializing() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "delay-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "wait".to_owned(),
                        name: None,
                        kind: Some("delay".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: None,
                        config: Some(serde_json::json!({"seconds": 60})),
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("delay claim should not fail: {error}"));
        assert!(
            materialized.is_none(),
            "delay node must wait until run_after"
        );
    }
}
