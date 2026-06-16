//! Persistent storage repositories and migrations for tikeo.

#![forbid(unsafe_code)]

pub mod entities;
pub mod migration;
pub mod repository;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use sqlx::sqlite::{SqliteJournalMode, SqliteSynchronous};
use std::{path::Path, time::Duration};
use thiserror::Error;

#[doc(hidden)]
pub use repository::reset_scheduler_shard_policy_for_test;
pub use repository::util::{parse_timestamp_offset, set_timestamp_offset};
pub use repository::{
    AdvanceWorkflowInput, AdvanceWorkflowResult, AlertDeliveryAttemptFilters,
    AlertDeliveryAttemptSummary, AlertEventFilters, AlertEventSummary, AlertRepository,
    AlertRuleSummary, AppSummary, AppendJobInstanceLog, AuditLogFilters, AuditLogPageSummary,
    AuditLogRepository, AuditLogSummary, AuthSessionRepository, AuthSessionSummary,
    CalendarRepository, CalendarSummary, CalendarWindowSummary, ClusterShardOwnershipRepository,
    ClusterShardOwnershipSloSummary, ClusterShardOwnershipSummary, CompleteWorkflowShardInput,
    CompleteWorkflowShardResult, CreateAlertRule, CreateAuditLog, CreateAuthSession, CreateJob,
    CreateJobInstance, CreateJobInstanceAttempt, CreateNotificationChannel,
    CreateNotificationMessage, CreateNotificationPolicy, CreateNotificationTemplate,
    CreateOidcAuthState, CreatePlugin, CreateRole, CreateScript, CreateSdkApiKey, CreateSecret,
    CreateServiceAccount, CreateUser, CreateWorkerDispatchOutbox, CreateWorkflow,
    DispatchQueueClaim, DispatchQueueShardOwner, DispatchQueueSloSummary, DispatchQueueSummary,
    InstanceEventSummary, JobDurationHistory, JobInstanceAttemptRepository,
    JobInstanceAttemptSummary, JobInstanceLogRepository, JobInstanceLogSummary,
    JobInstanceRepository, JobInstanceResult, JobInstanceSummary, JobRepository, JobRetryPolicy,
    JobSummary, JobVersionRepository, JobVersionSummary, MaterializeWorkflowNodeResult,
    NamespaceSummary, NotificationChannelDeleteResult, NotificationChannelDeliveryConfig,
    NotificationChannelFilters, NotificationChannelRepository, NotificationChannelSummary,
    NotificationDeliveryAttemptFilters, NotificationDeliveryAttemptRepository,
    NotificationDeliveryAttemptSummary, NotificationMessageFilters, NotificationMessageRepository,
    NotificationMessageSummary, NotificationPolicyFilters, NotificationPolicyRepository,
    NotificationPolicySummary, NotificationPolicyValidationSummary, NotificationTemplateFilters,
    NotificationTemplateRepository, NotificationTemplateSummary, OidcAuthStateRepository,
    OidcAuthStateSummary, OidcIdentityRepository, OidcIdentitySummary, PermissionCatalogItem,
    PermissionSummary, PersistedOnlineWorkerSummary, PluginAlertChannelTypeSummary,
    PluginProcessorTypeSummary, PluginRepository, PluginSummary, QueueOverview,
    RaftAppliedCommandSummary, RaftLogEntrySummary, RaftMemberSummary,
    RaftMembershipProposalSummary, RaftMetadataSummary, RaftRepository, RaftSnapshotSummary,
    RbacRepository, RebalanceWorkflowShardsInput, RebalanceWorkflowShardsResult,
    RecordAlertDeliveryAttempt, RecordNotificationDeliveryAttempt, RecordRaftAppliedCommand,
    RecordRaftMembershipProposal, RecoverWorkflowNodeInput, RecoverWorkflowNodeResult,
    RegisterWorkerSession, RoleSummary, ScheduleCursorRepository, ScopeRepository,
    ScriptReleaseGrantEvidenceSummary, ScriptReleaseSignatureSummary, ScriptRepository,
    ScriptSummary, ScriptVersionRepository, ScriptVersionSummary, SdkApiKeyRepository,
    SdkApiKeySummary, SecretRepository, SecretSummary, ServiceAccountRepository,
    ServiceAccountSummary, UpdateJob, UpdateNotificationChannel, UpdateNotificationPolicy,
    UpdateNotificationTemplate, UpdatePlugin, UpdateRole, UpdateScript, UpdateSdkApiKey,
    UpdateServiceAccount, UpdateUser, UpdateWorkerPoolQuota, UpdateWorkflow, UpsertCalendar,
    UpsertClusterShardOwnership, UpsertOidcIdentity, UpsertRaftLogEntry, UpsertRaftMember,
    UpsertRaftMetadata, UpsertRaftSnapshot, UserRepository, UserSummary,
    VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature, WorkerDispatchOutboxRepository,
    WorkerDispatchOutboxSloSummary, WorkerDispatchOutboxSummary, WorkerHeartbeat,
    WorkerLifecycleRepository, WorkerPoolSummary, WorkerSessionEventSummary,
    WorkerSessionSnapshotUpdate, WorkerSessionSummary, WorkflowDefinition, WorkflowEdgeSpec,
    WorkflowInstanceSummary, WorkflowJobResultOutcome, WorkflowNodeInstanceSummary,
    WorkflowNodeSpec, WorkflowRepository, WorkflowShardSummary, WorkflowSloSummary,
    WorkflowSummary, WorkflowValidationResult, scheduler_shard_policy, set_scheduler_shard_policy,
    validate_workflow_definition,
};
pub use sea_orm::DbErr;

/// Errors raised by storage initialization and repository operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// `SQLite` database parent directory could not be prepared.
    #[error("sqlite database directory preparation failed for {path}: {source}")]
    PrepareSqliteFile {
        /// Configured `SQLite` database path.
        path: String,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// Database connection failed.
    #[error("database connection failed: {0}")]
    Connect(#[from] sea_orm::DbErr),
}

/// Connect to the configured database and run schema migrations.
///
/// # Errors
///
/// Returns an error when the database cannot be opened or migrations fail.
pub async fn connect_and_migrate(database_url: &str) -> Result<DatabaseConnection, StorageError> {
    ensure_sqlite_database_parent(database_url)?;

    let mut options = ConnectOptions::new(database_url.to_owned());
    options
        .max_connections(16)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .sqlx_logging(std::env::var_os("TIKEO_SQLX_LOGGING").is_some())
        .idle_timeout(Duration::from_mins(1));
    configure_sqlite_connect_options(database_url, &mut options);

    let db = Database::connect(options).await?;
    migration::Migrator::up(&db, None).await?;
    migration::apply_sqlite_schema_compatibility(&db).await?;
    Ok(db)
}

fn ensure_sqlite_database_parent(database_url: &str) -> Result<(), StorageError> {
    let Some(path) = sqlite_database_file_path(database_url) else {
        return Ok(());
    };
    let parent = Path::new(&path).parent();
    if let Some(parent) = parent.filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|source| StorageError::PrepareSqliteFile {
            path: path.clone(),
            source,
        })?;
    }
    Ok(())
}

fn sqlite_database_file_path(database_url: &str) -> Option<String> {
    let path_with_query = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))?;
    let path = path_with_query
        .split_once('?')
        .map_or(path_with_query, |(path, _)| path);
    if path.is_empty() || path == ":memory:" || path.ends_with(":memory:") {
        return None;
    }
    Some(path.to_owned())
}

fn configure_sqlite_connect_options(database_url: &str, options: &mut ConnectOptions) {
    if !database_url.starts_with("sqlite:") {
        return;
    }
    options.map_sqlx_sqlite_opts(|sqlite_options| {
        sqlite_options
            .busy_timeout(Duration::from_secs(5))
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .pragma("foreign_keys", "ON")
    });
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use sea_orm_migration::MigratorTrait;

    #[tokio::test]
    async fn raft_tables_keep_soft_relationships_without_foreign_keys() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));

        for table in [
            "raft_metadata",
            "raft_members",
            "raft_log_entries",
            "raft_snapshots",
            "raft_applied_commands",
            "raft_membership_proposals",
        ] {
            let rows = db
                .query_all(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    format!("PRAGMA foreign_key_list({table})"),
                ))
                .await
                .unwrap_or_else(|error| panic!("foreign key list should query: {error}"));
            assert!(
                rows.is_empty(),
                "table {table} must use soft relationships only"
            );
        }
    }

    #[tokio::test]
    async fn notification_templates_upgrade_is_tracked_as_versioned_migration() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));

        let migration_rows = db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT version FROM seaql_migrations ORDER BY version",
            ))
            .await
            .unwrap_or_else(|error| panic!("migration history should query: {error}"));
        let versions = migration_rows
            .iter()
            .map(|row| row.try_get::<String>("", "version"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|error| panic!("migration version rows should decode: {error}"));

        assert!(
            versions
                .iter()
                .any(|version| version == "m20260611_000002_notification_templates"),
            "notification_templates must be added by a distinct versioned migration, got {versions:?}"
        );
        assert!(
            sqlite_table_has_column(&db, "notification_templates", "template_key").await,
            "notification_templates.template_key should exist after migrations"
        );
    }

    #[tokio::test]
    async fn sqlite_schema_compatibility_upgrade_is_tracked_as_versioned_migration() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));

        let migration_rows = db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT version FROM seaql_migrations ORDER BY version",
            ))
            .await
            .unwrap_or_else(|error| panic!("migration history should query: {error}"));
        let versions = migration_rows
            .iter()
            .map(|row| row.try_get::<String>("", "version"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|error| panic!("migration version rows should decode: {error}"));

        assert!(
            versions.len() >= 2,
            "schema compatibility upgrades must be tracked as explicit SeaORM migrations, got {versions:?}"
        );
        assert!(
            versions.iter().any(|version| version == "sqlite_compat"),
            "schema compatibility upgrade must be recorded by migration/sqlite_compat.rs, got {versions:?}"
        );
    }

    #[tokio::test]
    async fn sqlite_file_migrations_create_parent_directory_and_preserve_local_rows_on_rerun() {
        let base = std::env::temp_dir().join(format!(
            "tikeo-storage-persist-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let db_path = base.join("nested").join("tikeo-dev.db");
        let database_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let db = crate::connect_and_migrate(&database_url)
            .await
            .unwrap_or_else(|error| panic!("sqlite file db should initialize: {error}"));
        assert!(
            db_path.exists(),
            "connect_and_migrate should create missing sqlite parent directories"
        );

        let namespace = format!("operator-local-{}", std::process::id());
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO namespaces (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
            vec![
                "ns-operator-local".into(),
                namespace.clone().into(),
                "2026-06-14T00:00:00Z".into(),
                "2026-06-14T00:00:00Z".into(),
            ],
        ))
        .await
        .unwrap_or_else(|error| panic!("local namespace should insert: {error}"));
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r"INSERT INTO notification_channels (
                id, scope_type, namespace, app, worker_pool, name, provider, enabled,
                config_json, secret_refs_json, target_redacted, safety_policy_json,
                created_by, updated_by, created_at, updated_at
            ) VALUES (?, ?, NULL, NULL, NULL, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?, ?)",
            vec![
                "notification-channel-local-operator".into(),
                "global".into(),
                "Operator customized Feishu card".into(),
                "feishu".into(),
                false.into(),
                r#"{"messageType":"interactive","template":{"body":{"msg_type":"interactive"}}}"#
                    .into(),
                r#"{"webhook":"env:LOCAL_FEISHU_WEBHOOK"}"#.into(),
                "feishu:env:LOCAL_FEISHU_WEBHOOK".into(),
                "2026-06-14T00:00:00Z".into(),
                "2026-06-14T00:00:00Z".into(),
            ],
        ))
        .await
        .unwrap_or_else(|error| panic!("local notification channel should insert: {error}"));
        db.close()
            .await
            .unwrap_or_else(|error| panic!("sqlite file db should close: {error}"));

        let reopened = crate::connect_and_migrate(&database_url)
            .await
            .unwrap_or_else(|error| panic!("sqlite file db should reopen and migrate: {error}"));
        let namespace_count = reopened
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM namespaces WHERE name = ?",
                vec![namespace.into()],
            ))
            .await
            .unwrap_or_else(|error| panic!("namespace should query: {error}"))
            .unwrap_or_else(|| panic!("namespace count row should exist"))
            .try_get::<i64>("", "count")
            .unwrap_or_else(|error| panic!("namespace count should decode: {error}"));
        assert_eq!(
            namespace_count, 1,
            "rerunning migrations must not clear locally created rows"
        );

        let customized_channel = reopened
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT name, enabled FROM notification_channels WHERE id = 'notification-channel-local-operator'",
            ))
            .await
            .unwrap_or_else(|error| panic!("notification channel should query: {error}"))
            .unwrap_or_else(|| panic!("local notification channel should exist"));
        let name = customized_channel
            .try_get::<String>("", "name")
            .unwrap_or_else(|error| panic!("notification name should decode: {error}"));
        let enabled = customized_channel
            .try_get::<bool>("", "enabled")
            .unwrap_or_else(|error| panic!("notification enabled should decode: {error}"));
        assert_eq!(name, "Operator customized Feishu card");
        assert!(
            !enabled,
            "rerunning migrations must not refresh edited seed rows"
        );

        reopened
            .close()
            .await
            .unwrap_or_else(|error| panic!("sqlite file db should close: {error}"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[tokio::test]
    async fn connect_and_migrate_backfills_retry_columns_when_sqlite_compat_was_already_recorded() {
        let path = std::env::temp_dir().join(format!(
            "tikeo-legacy-compat-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let database_url = format!("sqlite://{}?mode=rwc", path.display());
        let legacy = sea_orm::Database::connect(&database_url)
            .await
            .unwrap_or_else(|error| panic!("legacy sqlite db should open: {error}"));
        for statement in [
            r"CREATE TABLE seaql_migrations (version varchar NOT NULL PRIMARY KEY, applied_at bigint NOT NULL)",
            r"INSERT INTO seaql_migrations (version, applied_at) VALUES ('mod', 1), ('sqlite_compat', 2)",
            r"CREATE TABLE jobs (
                id varchar NOT NULL PRIMARY KEY,
                namespace_id varchar NOT NULL,
                app_id varchar NOT NULL,
                name varchar NOT NULL,
                schedule_type varchar NOT NULL,
                schedule_expr varchar,
                misfire_policy varchar NOT NULL,
                processor_name varchar,
                enabled boolean NOT NULL,
                created_at varchar NOT NULL,
                updated_at varchar NOT NULL
            )",
            r"CREATE TABLE job_versions (
                id varchar NOT NULL PRIMARY KEY,
                job_id varchar NOT NULL,
                version_number bigint NOT NULL,
                name varchar NOT NULL,
                schedule_type varchar NOT NULL,
                schedule_expr varchar,
                misfire_policy varchar NOT NULL,
                processor_name varchar,
                enabled boolean NOT NULL,
                created_by varchar NOT NULL,
                change_reason varchar NOT NULL,
                created_at varchar NOT NULL
            )",
            r"CREATE TABLE job_instances (
                id varchar NOT NULL PRIMARY KEY,
                job_id varchar NOT NULL,
                status varchar NOT NULL,
                trigger_type varchar NOT NULL,
                execution_mode varchar NOT NULL,
                created_at varchar NOT NULL,
                updated_at varchar NOT NULL
            )",
            r"CREATE TABLE job_instance_logs (
                id varchar NOT NULL PRIMARY KEY,
                instance_id varchar NOT NULL,
                worker_id varchar NOT NULL,
                level varchar NOT NULL,
                message text NOT NULL,
                sequence bigint NOT NULL,
                created_at varchar NOT NULL
            )",
        ] {
            legacy
                .execute(Statement::from_string(DatabaseBackend::Sqlite, statement))
                .await
                .unwrap_or_else(|error| panic!("legacy schema statement should run: {error}"));
        }
        legacy
            .close()
            .await
            .unwrap_or_else(|error| panic!("legacy sqlite db should close: {error}"));

        let migrated = crate::connect_and_migrate(&database_url)
            .await
            .unwrap_or_else(|error| panic!("legacy sqlite db should migrate: {error}"));

        for (table, column) in [
            ("jobs", "retry_policy_json"),
            ("job_versions", "retry_policy_json"),
            ("job_instances", "result_worker_id"),
            ("job_instances", "result_success"),
            ("job_instances", "result_message"),
            ("job_instances", "result_completed_at"),
            ("job_instance_attempts", "assignment_token"),
            ("job_instance_attempts", "result_success"),
            ("job_instance_attempts", "result_message"),
            ("job_instance_attempts", "result_completed_at"),
        ] {
            assert!(
                sqlite_table_has_column(&migrated, table, column).await,
                "{table}.{column} should be backfilled even when sqlite_compat migration was already recorded"
            );
        }
        migrated
            .close()
            .await
            .unwrap_or_else(|error| panic!("migrated sqlite db should close: {error}"));
        let _ = std::fs::remove_file(path);
    }

    async fn sqlite_table_has_column(
        db: &sea_orm::DatabaseConnection,
        table: &str,
        column: &str,
    ) -> bool {
        let rows = db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("PRAGMA table_info({table})"),
            ))
            .await
            .unwrap_or_else(|error| panic!("table info should query: {error}"));
        rows.iter().any(|row| {
            row.try_get::<String>("", "name")
                .is_ok_and(|name| name == column)
        })
    }

    #[tokio::test]
    async fn sqlite_compatibility_creates_scope_tables_before_indexes_for_existing_dev_db() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        for sql in [
            r"CREATE TABLE jobs (
                id varchar NOT NULL PRIMARY KEY,
                namespace_id varchar NOT NULL,
                app_id varchar NOT NULL,
                name varchar NOT NULL,
                schedule_type varchar NOT NULL,
                schedule_expr varchar,
                processor_name varchar,
                processor_type varchar,
                script_id varchar,
                enabled boolean NOT NULL,
                created_at varchar NOT NULL,
                updated_at varchar NOT NULL
            )",
            r"CREATE TABLE job_instances (
                id varchar NOT NULL PRIMARY KEY,
                job_id varchar NOT NULL,
                status varchar NOT NULL,
                trigger_type varchar NOT NULL,
                execution_mode varchar NOT NULL,
                created_at varchar NOT NULL,
                updated_at varchar NOT NULL
            )",
            r"CREATE TABLE job_instance_attempts (
                id varchar NOT NULL PRIMARY KEY,
                instance_id varchar NOT NULL,
                worker_id varchar NOT NULL,
                status varchar NOT NULL,
                assignment_token varchar,
                started_at varchar NOT NULL,
                finished_at varchar,
                error_message text,
                created_at varchar NOT NULL
            )",
            r"CREATE TABLE job_instance_logs (
                id varchar NOT NULL PRIMARY KEY,
                instance_id varchar NOT NULL,
                worker_id varchar,
                level varchar NOT NULL,
                message text NOT NULL,
                sequence bigint NOT NULL,
                created_at varchar NOT NULL
            )",
        ] {
            db.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql.to_owned(),
            ))
            .await
            .unwrap_or_else(|error| panic!("legacy indexed table should create: {error}"));
        }

        crate::migration::Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| {
                panic!("compatibility migration should create scope tables before indexes: {error}")
            });

        for table in ["namespaces", "apps", "worker_pools"] {
            let row = db
                .query_one(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"),
                ))
                .await
                .unwrap_or_else(|error| panic!("sqlite_master query should run: {error}"));
            assert!(
                row.is_some(),
                "{table} should exist after compatibility migration"
            );
        }
    }
}
