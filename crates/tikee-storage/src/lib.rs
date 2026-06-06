//! Persistent storage repositories and migrations for tikee.

#![forbid(unsafe_code)]

pub mod entities;
pub mod migration;
pub mod repository;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use sqlx::sqlite::{SqliteJournalMode, SqliteSynchronous};
use std::time::Duration;
use thiserror::Error;

pub use repository::util::{parse_timestamp_offset, set_timestamp_offset};
pub use repository::{
    AdvanceWorkflowInput, AdvanceWorkflowResult, AlertDeliveryAttemptFilters,
    AlertDeliveryAttemptSummary, AlertEventFilters, AlertEventSummary, AlertRepository,
    AlertRuleSummary, AppSummary, AppendJobInstanceLog, AuditLogFilters, AuditLogPageSummary,
    AuditLogRepository, AuditLogSummary, AuthSessionRepository, AuthSessionSummary,
    CalendarRepository, CalendarSummary, CalendarWindowSummary, CompleteWorkflowShardInput,
    CompleteWorkflowShardResult, CreateAlertRule, CreateAuditLog, CreateAuthSession, CreateJob,
    CreateJobInstance, CreateJobInstanceAttempt, CreateOidcAuthState, CreatePlugin, CreateScript,
    CreateSdkApiKey, CreateSecret, CreateServiceAccount, CreateUser, CreateWorkflow,
    DispatchQueueClaim, DispatchQueueSloSummary, DispatchQueueSummary, InstanceEventSummary,
    JobDurationHistory, JobInstanceAttemptRepository, JobInstanceAttemptSummary,
    JobInstanceLogRepository, JobInstanceLogSummary, JobInstanceRepository, JobInstanceResult,
    JobInstanceSummary, JobRepository, JobRetryPolicy, JobSummary, JobVersionRepository,
    JobVersionSummary, MaterializeWorkflowNodeResult, NamespaceSummary, OidcAuthStateRepository,
    OidcAuthStateSummary, OidcIdentityRepository, OidcIdentitySummary, PermissionSummary,
    PersistedOnlineWorkerSummary, PluginAlertChannelTypeSummary, PluginProcessorTypeSummary,
    PluginRepository, PluginSummary, QueueOverview, RaftAppliedCommandSummary, RaftLogEntrySummary,
    RaftMemberSummary, RaftMembershipProposalSummary, RaftMetadataSummary, RaftRepository,
    RaftSnapshotSummary, RbacRepository, RebalanceWorkflowShardsInput,
    RebalanceWorkflowShardsResult, RecordAlertDeliveryAttempt, RecordRaftAppliedCommand,
    RecordRaftMembershipProposal, RecoverWorkflowNodeInput, RecoverWorkflowNodeResult,
    RegisterWorkerSession, ScheduleCursorRepository, ScopeRepository,
    ScriptReleaseGrantEvidenceSummary, ScriptReleaseSignatureSummary, ScriptRepository,
    ScriptSummary, ScriptVersionRepository, ScriptVersionSummary, SdkApiKeyRepository,
    SdkApiKeySummary, SecretRepository, SecretSummary, ServiceAccountRepository,
    ServiceAccountSummary, UpdateJob, UpdatePlugin, UpdateScript, UpdateSdkApiKey,
    UpdateServiceAccount, UpdateUser, UpdateWorkerPoolQuota, UpdateWorkflow, UpsertCalendar,
    UpsertOidcIdentity, UpsertRaftLogEntry, UpsertRaftMember, UpsertRaftMetadata,
    UpsertRaftSnapshot, UserRepository, UserSummary, VerifiedScriptReleaseGrants,
    VerifiedScriptReleaseSignature, WorkerHeartbeat, WorkerLifecycleRepository, WorkerPoolSummary,
    WorkerSessionEventSummary, WorkerSessionSnapshotUpdate, WorkerSessionSummary,
    WorkflowDefinition, WorkflowEdgeSpec, WorkflowInstanceSummary, WorkflowJobResultOutcome,
    WorkflowNodeInstanceSummary, WorkflowNodeSpec, WorkflowRepository, WorkflowShardSummary,
    WorkflowSloSummary, WorkflowSummary, WorkflowValidationResult, validate_workflow_definition,
};
pub use sea_orm::DbErr;

/// Errors raised by storage initialization and repository operations.
#[derive(Debug, Error)]
pub enum StorageError {
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
    let mut options = ConnectOptions::new(database_url.to_owned());
    options
        .max_connections(16)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .sqlx_logging(std::env::var_os("TIKEE_SQLX_LOGGING").is_some())
        .idle_timeout(Duration::from_mins(1));
    configure_sqlite_connect_options(database_url, &mut options);

    let db = Database::connect(options).await?;
    migration::Migrator::up(&db, None).await?;
    migration::apply_sqlite_schema_compatibility(&db).await?;
    Ok(db)
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
    async fn connect_and_migrate_backfills_retry_columns_when_sqlite_compat_was_already_recorded() {
        let path = std::env::temp_dir().join(format!(
            "tikee-legacy-compat-{}-{}.db",
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
