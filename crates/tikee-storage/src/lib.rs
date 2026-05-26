//! Persistent storage repositories and migrations for tikee.

#![forbid(unsafe_code)]

pub mod entities;
pub mod migration;
pub mod repository;

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
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
    CompleteWorkflowShardInput, CompleteWorkflowShardResult, CreateAlertRule, CreateAuditLog,
    CreateAuthSession, CreateJob, CreateJobInstance, CreateJobInstanceAttempt, CreateOidcAuthState,
    CreateScript, CreateUser, CreateWorkflow, DispatchQueueClaim, DispatchQueueSloSummary,
    DispatchQueueSummary, InstanceEventSummary, JobInstanceAttemptRepository,
    JobInstanceAttemptSummary, JobInstanceLogRepository, JobInstanceLogSummary,
    JobInstanceRepository, JobInstanceSummary, JobRepository, JobSummary,
    MaterializeWorkflowNodeResult, NamespaceSummary, OidcAuthStateRepository, OidcAuthStateSummary,
    OidcIdentityRepository, OidcIdentitySummary, PermissionSummary, QueueOverview,
    RaftAppliedCommandSummary, RaftLogEntrySummary, RaftMemberSummary,
    RaftMembershipProposalSummary, RaftMetadataSummary, RaftRepository, RaftSnapshotSummary,
    RbacRepository, RecordAlertDeliveryAttempt, RecordRaftAppliedCommand,
    RecordRaftMembershipProposal, RecoverWorkflowNodeInput, RecoverWorkflowNodeResult,
    RegisterWorkerSession, ScopeRepository, ScriptReleaseGrantEvidenceSummary,
    ScriptReleaseSignatureSummary, ScriptRepository, ScriptSummary, ScriptVersionRepository,
    ScriptVersionSummary, UpdateJob, UpdateScript, UpdateUser, UpdateWorkflow, UpsertOidcIdentity,
    UpsertRaftLogEntry, UpsertRaftMember, UpsertRaftMetadata, UpsertRaftSnapshot, UserRepository,
    UserSummary, VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature, WorkerHeartbeat,
    WorkerLifecycleRepository, WorkerPoolSummary, WorkerSessionEventSummary, WorkerSessionSummary,
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
        .sqlx_logging(false)
        .idle_timeout(Duration::from_mins(1));
    configure_sqlite_connect_options(database_url, &mut options);

    let db = Database::connect(options).await?;
    migration::Migrator::up(&db, None).await?;
    ensure_sqlite_schema_compatibility(&db).await?;
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

async fn ensure_sqlite_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    ensure_broadcast_schema_compatibility(db).await?;
    ensure_auth_schema_compatibility(db).await?;
    ensure_oidc_auth_state_schema_compatibility(db).await?;
    ensure_oidc_identity_schema_compatibility(db).await?;
    ensure_rbac_schema_compatibility(db).await?;
    ensure_scope_schema_compatibility(db).await?;
    ensure_worker_lifecycle_schema_compatibility(db).await?;
    ensure_job_schema_compatibility(db).await?;
    ensure_scripts_schema_compatibility(db).await?;
    ensure_script_versions_schema_compatibility(db).await?;
    ensure_audit_logs_schema_compatibility(db).await?;
    ensure_alert_schema_compatibility(db).await?;
    ensure_workflow_schema_compatibility(db).await?;
    ensure_raft_schema_compatibility(db).await?;
    remove_sqlite_foreign_keys(db).await
}

async fn ensure_scope_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS namespaces (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS apps (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS worker_pools (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            app_id varchar NOT NULL,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    Ok(())
}

async fn ensure_worker_lifecycle_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    for sql in [
        r"CREATE TABLE IF NOT EXISTS worker_logical_instances (
            id varchar NOT NULL PRIMARY KEY,
            namespace_name varchar NOT NULL,
            app_name varchar NOT NULL,
            cluster varchar NOT NULL,
            region varchar NOT NULL,
            client_instance_id varchar NOT NULL,
            current_worker_id varchar,
            current_generation bigint NOT NULL,
            status varchar NOT NULL,
            last_seen_at varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        r"CREATE TABLE IF NOT EXISTS worker_sessions (
            worker_id varchar NOT NULL PRIMARY KEY,
            logical_instance_id varchar NOT NULL,
            connection_id varchar NOT NULL,
            generation bigint NOT NULL,
            fencing_token_hash varchar NOT NULL,
            status varchar NOT NULL,
            status_reason varchar,
            status_evidence text,
            lease_expires_at varchar NOT NULL,
            last_heartbeat_at varchar NOT NULL,
            last_sequence bigint NOT NULL,
            connected_at varchar NOT NULL,
            disconnected_at varchar,
            replaced_by_worker_id varchar,
            drain_requested_at varchar,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        r"CREATE TABLE IF NOT EXISTS worker_session_events (
            id varchar NOT NULL PRIMARY KEY,
            worker_id varchar NOT NULL,
            logical_instance_id varchar NOT NULL,
            event_type varchar NOT NULL,
            reason varchar,
            detail_json text,
            created_at varchar NOT NULL
        )",
    ] {
        db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await?;
    }
    Ok(())
}

async fn ensure_job_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    if !sqlite_column_exists(db, "jobs", "processor_name").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE jobs ADD COLUMN processor_name varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "jobs", "script_id").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE jobs ADD COLUMN script_id varchar",
        ))
        .await?;
    }
    Ok(())
}

async fn ensure_alert_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS alert_delivery_attempts (
            id varchar NOT NULL PRIMARY KEY,
            event_id varchar NOT NULL,
            rule_id varchar NOT NULL,
            provider varchar NOT NULL,
            target varchar NOT NULL,
            delivered boolean NOT NULL,
            status_code integer,
            error text,
            attempt integer NOT NULL,
            retry_state varchar NOT NULL,
            next_retry_at varchar,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_alert_delivery_attempts_event ON alert_delivery_attempts (event_id, created_at)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_alert_delivery_attempts_retry ON alert_delivery_attempts (retry_state, next_retry_at)",
    ))
    .await?;
    Ok(())
}

async fn ensure_workflow_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    for sql in [
        r"CREATE TABLE IF NOT EXISTS workflows (id varchar NOT NULL PRIMARY KEY, name varchar NOT NULL, definition varchar NOT NULL, status varchar NOT NULL, created_by varchar NOT NULL, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS workflow_nodes (id varchar NOT NULL PRIMARY KEY, workflow_id varchar NOT NULL, node_key varchar NOT NULL, name varchar NOT NULL, kind varchar NOT NULL, job_id varchar, processor_name varchar, config varchar, created_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS workflow_edges (id varchar NOT NULL PRIMARY KEY, workflow_id varchar NOT NULL, from_node_key varchar NOT NULL, to_node_key varchar NOT NULL, condition varchar NOT NULL, created_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS workflow_instances (id varchar NOT NULL PRIMARY KEY, workflow_id varchar NOT NULL, status varchar NOT NULL, trigger_type varchar NOT NULL, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS workflow_node_instances (id varchar NOT NULL PRIMARY KEY, workflow_instance_id varchar NOT NULL, node_key varchar NOT NULL, status varchar NOT NULL, job_instance_id varchar, child_workflow_instance_id varchar, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS workflow_shards (id varchar NOT NULL PRIMARY KEY, workflow_instance_id varchar NOT NULL, workflow_node_instance_id varchar NOT NULL, node_key varchar NOT NULL, shard_index integer NOT NULL, status varchar NOT NULL, input varchar NOT NULL, output varchar, job_instance_id varchar, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS dispatch_queue (id varchar NOT NULL PRIMARY KEY, job_instance_id varchar, workflow_node_instance_id varchar, priority integer NOT NULL, run_after varchar NOT NULL, status varchar NOT NULL, attempt integer NOT NULL, lease_owner varchar, lease_until varchar, fencing_token varchar, worker_selector varchar, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS instance_events (id varchar NOT NULL PRIMARY KEY, instance_id varchar NOT NULL, instance_type varchar NOT NULL, event_type varchar NOT NULL, message varchar NOT NULL, payload varchar, created_at varchar NOT NULL)",
        "CREATE INDEX IF NOT EXISTS idx_workflows_name ON workflows (name)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_workflow_nodes_workflow_key ON workflow_nodes (workflow_id, node_key)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_edges_workflow ON workflow_edges (workflow_id)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_instances_workflow_created ON workflow_instances (workflow_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_node_instances_instance ON workflow_node_instances (workflow_instance_id)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_shards_node ON workflow_shards (workflow_node_instance_id)",
        "CREATE INDEX IF NOT EXISTS idx_dispatch_queue_status_run_after ON dispatch_queue (status, run_after)",
        "CREATE INDEX IF NOT EXISTS idx_instance_events_instance_created ON instance_events (instance_id, created_at)",
    ] {
        db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await?;
    }

    if !sqlite_column_exists(db, "workflow_nodes", "processor_name").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE workflow_nodes ADD COLUMN processor_name varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "workflow_shards", "job_instance_id").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE workflow_shards ADD COLUMN job_instance_id varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "workflow_node_instances", "child_workflow_instance_id").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE workflow_node_instances ADD COLUMN child_workflow_instance_id varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "dispatch_queue", "lease_owner").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE dispatch_queue ADD COLUMN lease_owner varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "dispatch_queue", "lease_until").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE dispatch_queue ADD COLUMN lease_until varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "dispatch_queue", "fencing_token").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE dispatch_queue ADD COLUMN fencing_token varchar",
        ))
        .await?;
    }
    Ok(())
}

async fn ensure_raft_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    for sql in [
        r"CREATE TABLE IF NOT EXISTS raft_metadata (id varchar NOT NULL PRIMARY KEY, cluster_id varchar NOT NULL, node_id varchar NOT NULL, current_term bigint NOT NULL, voted_for varchar, commit_index bigint NOT NULL, applied_index bigint NOT NULL, leader_fencing_token varchar, conf_state text, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS raft_members (id varchar NOT NULL PRIMARY KEY, node_id varchar NOT NULL, endpoint varchar NOT NULL, status varchar NOT NULL, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS raft_log_entries (id varchar NOT NULL PRIMARY KEY, cluster_id varchar NOT NULL, node_id varchar NOT NULL, log_index bigint NOT NULL, term bigint NOT NULL, entry_type varchar NOT NULL, data text NOT NULL, context text, sync_status varchar NOT NULL, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS raft_snapshots (id varchar NOT NULL PRIMARY KEY, cluster_id varchar NOT NULL, node_id varchar NOT NULL, snapshot_index bigint NOT NULL, term bigint NOT NULL, conf_state text, data text, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS raft_applied_commands (id varchar NOT NULL PRIMARY KEY, cluster_id varchar NOT NULL, node_id varchar NOT NULL, log_index bigint NOT NULL, term bigint NOT NULL, command_id varchar NOT NULL, command_type varchar NOT NULL, payload text, status varchar NOT NULL, message text NOT NULL, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS raft_membership_proposals (id varchar NOT NULL PRIMARY KEY, cluster_id varchar NOT NULL, proposal_id varchar NOT NULL, action varchar NOT NULL, node_id varchar NOT NULL, endpoint varchar, status varchar NOT NULL, message text NOT NULL, created_by varchar NOT NULL, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_metadata_node ON raft_metadata (node_id)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_members_node ON raft_members (node_id)",
        "CREATE INDEX IF NOT EXISTS idx_raft_members_status ON raft_members (status)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_log_entries_node_index ON raft_log_entries (node_id, log_index)",
        "CREATE INDEX IF NOT EXISTS idx_raft_log_entries_node_term ON raft_log_entries (node_id, term)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_snapshots_node_index ON raft_snapshots (node_id, snapshot_index)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_applied_commands_node_index ON raft_applied_commands (node_id, log_index)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_applied_commands_command ON raft_applied_commands (cluster_id, command_id)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raft_membership_proposals_proposal ON raft_membership_proposals (cluster_id, proposal_id)",
        "CREATE INDEX IF NOT EXISTS idx_raft_membership_proposals_node ON raft_membership_proposals (node_id, status)",
    ] {
        db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await?;
    }
    if !sqlite_column_exists(db, "raft_metadata", "leader_fencing_token").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE raft_metadata ADD COLUMN leader_fencing_token varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "raft_metadata", "conf_state").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE raft_metadata ADD COLUMN conf_state text",
        ))
        .await?;
    }
    Ok(())
}

async fn ensure_scripts_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS scripts (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            language varchar NOT NULL,
            version varchar NOT NULL,
            content varchar NOT NULL,
            status varchar NOT NULL,
            release_approval_ticket varchar,
            release_signature varchar,
            release_signature_verified_at varchar,
            release_signature_verified_by varchar,
            release_grants_json text,
            release_grants_verified_at varchar,
            release_grants_verified_by varchar,
            timeout_seconds bigint,
            max_memory_bytes bigint,
            allow_network boolean NOT NULL DEFAULT 0,
            allowed_env_vars varchar,
            policy_json varchar,
            created_by varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    if !sqlite_column_exists(db, "scripts", "released_version_id").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE scripts ADD COLUMN released_version_id varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "scripts", "released_version_number").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE scripts ADD COLUMN released_version_number bigint",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "scripts", "policy_json").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE scripts ADD COLUMN policy_json varchar",
        ))
        .await?;
    }
    for column in [
        "release_approval_ticket",
        "release_signature",
        "release_signature_verified_at",
        "release_signature_verified_by",
        "release_grants_verified_at",
        "release_grants_verified_by",
    ] {
        if !sqlite_column_exists(db, "scripts", column).await? {
            db.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("ALTER TABLE scripts ADD COLUMN {column} varchar"),
            ))
            .await?;
        }
    }
    if !sqlite_column_exists(db, "scripts", "release_grants_json").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE scripts ADD COLUMN release_grants_json text",
        ))
        .await?;
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_scripts_status ON scripts (status)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_scripts_name ON scripts (name)",
    ))
    .await?;
    Ok(())
}

async fn ensure_script_versions_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS script_versions (
            id varchar NOT NULL PRIMARY KEY,
            script_id varchar NOT NULL,
            version_number bigint NOT NULL,
            content varchar NOT NULL,
            content_sha256 varchar NOT NULL DEFAULT '',
            language varchar NOT NULL,
            status varchar NOT NULL,
            release_approval_ticket varchar,
            release_signature varchar,
            release_signature_verified_at varchar,
            release_signature_verified_by varchar,
            release_grants_json text,
            release_grants_verified_at varchar,
            release_grants_verified_by varchar,
            timeout_seconds bigint,
            max_memory_bytes bigint,
            allow_network boolean NOT NULL DEFAULT 0,
            allowed_env_vars varchar,
            policy_json varchar,
            created_by varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    if !sqlite_column_exists(db, "script_versions", "content_sha256").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE script_versions ADD COLUMN content_sha256 varchar NOT NULL DEFAULT ''",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "script_versions", "policy_json").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE script_versions ADD COLUMN policy_json varchar",
        ))
        .await?;
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_script_versions_script_id ON script_versions (script_id)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_script_versions_script_version ON script_versions (script_id, version_number)",
    ))
    .await?;
    Ok(())
}

async fn ensure_broadcast_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }

    if !sqlite_column_exists(db, "job_instances", "execution_mode").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE job_instances ADD COLUMN execution_mode varchar NOT NULL DEFAULT 'single'",
        ))
        .await?;
    }

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS job_instance_attempts (
            id varchar NOT NULL PRIMARY KEY,
            instance_id varchar NOT NULL,
            worker_id varchar NOT NULL,
            status varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_job_instance_attempts_instance_worker ON job_instance_attempts (instance_id, worker_id)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_job_instance_attempts_status ON job_instance_attempts (status)",
    ))
    .await?;

    Ok(())
}

async fn ensure_audit_logs_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS audit_logs (
            id varchar NOT NULL PRIMARY KEY,
            actor varchar NOT NULL,
            action varchar NOT NULL,
            resource_type varchar NOT NULL,
            resource_id varchar NOT NULL,
            detail varchar,
            before varchar,
            after varchar,
            trace_id varchar,
            result varchar NOT NULL DEFAULT 'success',
            failure_reason varchar,
            ip_address varchar,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    for statement in [
        "ALTER TABLE audit_logs ADD COLUMN before varchar",
        "ALTER TABLE audit_logs ADD COLUMN after varchar",
        "ALTER TABLE audit_logs ADD COLUMN trace_id varchar",
        "ALTER TABLE audit_logs ADD COLUMN result varchar NOT NULL DEFAULT 'success'",
        "ALTER TABLE audit_logs ADD COLUMN failure_reason varchar",
    ] {
        let _ = db
            .execute(Statement::from_string(DatabaseBackend::Sqlite, statement))
            .await;
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs (created_at)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs (actor)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs (resource_type, resource_id)",
    ))
    .await?;
    Ok(())
}

async fn ensure_rbac_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS roles (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            description varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS permissions (
            id varchar NOT NULL PRIMARY KEY,
            resource varchar NOT NULL,
            action varchar NOT NULL,
            description varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS role_permissions (
            id varchar NOT NULL PRIMARY KEY,
            role_id varchar NOT NULL,
            permission_id varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_roles_name ON roles (name)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_permissions_resource_action ON permissions (resource, action)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_role_permissions_role_permission ON role_permissions (role_id, permission_id)",
    ))
    .await?;
    seed_sqlite_rbac_defaults(db).await
}

const SQLITE_DEFAULT_PERMISSIONS: &[(&str, &str, &str, &str)] = &[
    ("perm-system-read", "system", "read", "Read system metadata"),
    ("perm-cluster-read", "cluster", "read", "Read cluster state"),
    (
        "perm-cluster-manage",
        "cluster",
        "manage",
        "Manage cluster membership proposals",
    ),
    ("perm-users-read", "users", "read", "Read users"),
    ("perm-users-manage", "users", "manage", "Manage users"),
    (
        "perm-tenants-read",
        "tenants",
        "read",
        "Read tenants, apps, and worker pools",
    ),
    (
        "perm-tenants-manage",
        "tenants",
        "manage",
        "Manage tenants, apps, and worker pools",
    ),
    ("perm-jobs-read", "jobs", "read", "Read jobs"),
    ("perm-jobs-write", "jobs", "write", "Create and update jobs"),
    (
        "perm-instances-read",
        "instances",
        "read",
        "Read job instances",
    ),
    (
        "perm-instances-execute",
        "instances",
        "execute",
        "Trigger job instances",
    ),
    ("perm-scripts-read", "scripts", "read", "Read scripts"),
    ("perm-scripts-manage", "scripts", "manage", "Manage scripts"),
    ("perm-audit-read", "audit", "read", "Read audit logs"),
    ("perm-workflows-read", "workflows", "read", "Read workflows"),
    (
        "perm-workflows-manage",
        "workflows",
        "manage",
        "Manage workflows",
    ),
    (
        "perm-workflows-execute",
        "workflows",
        "execute",
        "Run workflows",
    ),
];

async fn seed_sqlite_rbac_defaults(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());
    for (id, name, description) in [
        ("role-admin", "admin", "Full platform administration"),
        (
            "role-operator",
            "operator",
            "Operate tikee jobs and instances",
        ),
        ("role-viewer", "viewer", "Read-only platform access"),
    ] {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                "INSERT OR IGNORE INTO roles (id, name, description, created_at) VALUES ('{id}', '{name}', '{description}', '{now}')"
            ),
        ))
        .await?;
    }
    for (id, resource, action, description) in SQLITE_DEFAULT_PERMISSIONS {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                "INSERT OR IGNORE INTO permissions (id, resource, action, description, created_at) VALUES ('{id}', '{resource}', '{action}', '{description}', '{now}')"
            ),
        ))
        .await?;
    }
    let admin_permissions = SQLITE_DEFAULT_PERMISSIONS
        .iter()
        .map(|(id, _, _, _)| *id)
        .collect::<Vec<_>>();
    seed_sqlite_role_permissions(db, "role-admin", &admin_permissions, &now).await?;
    seed_sqlite_role_permissions(
        db,
        "role-operator",
        &[
            "perm-tenants-read",
            "perm-jobs-read",
            "perm-jobs-write",
            "perm-instances-read",
            "perm-instances-execute",
            "perm-scripts-read",
            "perm-workflows-read",
            "perm-workflows-execute",
        ],
        &now,
    )
    .await?;
    seed_sqlite_role_permissions(
        db,
        "role-viewer",
        &[
            "perm-tenants-read",
            "perm-jobs-read",
            "perm-instances-read",
            "perm-scripts-read",
            "perm-workflows-read",
        ],
        &now,
    )
    .await
}

async fn seed_sqlite_role_permissions(
    db: &DatabaseConnection,
    role_id: &str,
    permission_ids: &[&str],
    now: &str,
) -> Result<(), sea_orm::DbErr> {
    for permission_id in permission_ids {
        let id = format!("rp-{role_id}-{permission_id}");
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                "INSERT OR IGNORE INTO role_permissions (id, role_id, permission_id, created_at) VALUES ('{id}', '{role_id}', '{permission_id}', '{now}')"
            ),
        ))
        .await?;
    }
    Ok(())
}

async fn ensure_oidc_auth_state_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS oidc_auth_states (
            id varchar NOT NULL PRIMARY KEY,
            state_hash varchar NOT NULL,
            redirect_uri varchar NOT NULL,
            expires_at varchar NOT NULL,
            consumed_at varchar,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_oidc_auth_states_state_hash ON oidc_auth_states (state_hash)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_oidc_auth_states_expires ON oidc_auth_states (expires_at)",
    ))
    .await?;
    Ok(())
}

async fn ensure_oidc_identity_schema_compatibility(
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS oidc_identities (
            id varchar NOT NULL PRIMARY KEY,
            issuer varchar NOT NULL,
            subject varchar NOT NULL,
            username varchar NOT NULL,
            namespace varchar,
            app varchar,
            worker_pool varchar,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_oidc_identities_issuer_subject ON oidc_identities (issuer, subject)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_oidc_identities_username ON oidc_identities (username)",
    ))
    .await?;
    Ok(())
}

async fn ensure_auth_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS users (
            id varchar NOT NULL PRIMARY KEY,
            username varchar NOT NULL,
            password varchar NOT NULL,
            role varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    if sqlite_column_exists(db, "users", "password_hash").await?
        && !sqlite_column_exists(db, "users", "password").await?
    {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE users RENAME COLUMN password_hash TO password",
        ))
        .await?;
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users (username)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS auth_sessions (
            id varchar NOT NULL PRIMARY KEY,
            user_id varchar NOT NULL,
            token_hash varchar NOT NULL,
            device_id varchar,
            device_name varchar,
            expires_at varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_sessions_token_hash ON auth_sessions (token_hash)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_auth_sessions_user ON auth_sessions (user_id)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!(
            "INSERT OR IGNORE INTO users (id, username, password, role, created_at) VALUES ('usr-admin', 'tikee_init', '$2b$10$vslUa5GAP.Mk3s4PPclu..miTj/beUTaSCR/HSZdfPVXmhA/7lmpm', 'admin', '{}')",
            time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
        ),
    ))
    .await?;

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn remove_sqlite_foreign_keys(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }

    rebuild_sqlite_table_without_foreign_keys(
        db,
        "apps",
        r"CREATE TABLE apps (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &["id", "namespace_id", "name", "created_at", "updated_at"],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "worker_pools",
        r"CREATE TABLE worker_pools (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            app_id varchar NOT NULL,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "namespace_id",
            "app_id",
            "name",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "jobs",
        r"CREATE TABLE jobs (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            app_id varchar NOT NULL,
            name varchar NOT NULL,
            schedule_type varchar NOT NULL,
            schedule_expr varchar,
            processor_name varchar,
            script_id varchar,
            enabled boolean NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "namespace_id",
            "app_id",
            "name",
            "schedule_type",
            "schedule_expr",
            "processor_name",
            "script_id",
            "enabled",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "job_instances",
        r"CREATE TABLE job_instances (
            id varchar NOT NULL PRIMARY KEY,
            job_id varchar NOT NULL,
            status varchar NOT NULL,
            trigger_type varchar NOT NULL,
            execution_mode varchar NOT NULL DEFAULT 'single',
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "job_id",
            "status",
            "trigger_type",
            "execution_mode",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "job_instance_attempts",
        r"CREATE TABLE job_instance_attempts (
            id varchar NOT NULL PRIMARY KEY,
            instance_id varchar NOT NULL,
            worker_id varchar NOT NULL,
            status varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "instance_id",
            "worker_id",
            "status",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "job_instance_logs",
        r"CREATE TABLE job_instance_logs (
            id varchar NOT NULL PRIMARY KEY,
            instance_id varchar NOT NULL,
            worker_id varchar NOT NULL,
            level varchar NOT NULL,
            message varchar NOT NULL,
            sequence bigint NOT NULL,
            created_at varchar NOT NULL
        )",
        &[
            "id",
            "instance_id",
            "worker_id",
            "level",
            "message",
            "sequence",
            "created_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "auth_sessions",
        r"CREATE TABLE auth_sessions (
            id varchar NOT NULL PRIMARY KEY,
            user_id varchar NOT NULL,
            token_hash varchar NOT NULL,
            device_id varchar,
            device_name varchar,
            expires_at varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "user_id",
            "token_hash",
            "device_id",
            "device_name",
            "expires_at",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    ensure_sqlite_indexes(db).await
}

async fn rebuild_sqlite_table_without_foreign_keys(
    db: &DatabaseConnection,
    table: &str,
    create_sql: &str,
    columns: &[&str],
) -> Result<(), sea_orm::DbErr> {
    if !sqlite_table_has_foreign_keys(db, table).await? {
        return Ok(());
    }

    let backup = format!("{table}__soft_rel_tmp");
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys=OFF",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("ALTER TABLE {table} RENAME TO {backup}"),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        create_sql.to_owned(),
    ))
    .await?;
    let column_list = columns.join(", ");
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("INSERT INTO {table} ({column_list}) SELECT {column_list} FROM {backup}"),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("DROP TABLE {backup}"),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys=ON",
    ))
    .await?;
    Ok(())
}

async fn sqlite_table_has_foreign_keys(
    db: &DatabaseConnection,
    table: &str,
) -> Result<bool, sea_orm::DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA foreign_key_list({table})"),
        ))
        .await?;
    Ok(!rows.is_empty())
}

async fn ensure_sqlite_indexes(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    for sql in [
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_apps_namespace_name ON apps (namespace_id, name)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_pools_app_name ON worker_pools (app_id, name)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_app_name ON jobs (app_id, name)",
        "CREATE INDEX IF NOT EXISTS idx_jobs_enabled ON jobs (enabled)",
        "CREATE INDEX IF NOT EXISTS idx_job_instances_job_created ON job_instances (job_id, created_at)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_job_instance_attempts_instance_worker ON job_instance_attempts (instance_id, worker_id)",
        "CREATE INDEX IF NOT EXISTS idx_job_instance_attempts_status ON job_instance_attempts (status)",
        "CREATE INDEX IF NOT EXISTS idx_job_instance_logs_instance_seq ON job_instance_logs (instance_id, sequence)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_sessions_token_hash ON auth_sessions (token_hash)",
        "CREATE INDEX IF NOT EXISTS idx_auth_sessions_user ON auth_sessions (user_id)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_oidc_auth_states_state_hash ON oidc_auth_states (state_hash)",
        "CREATE INDEX IF NOT EXISTS idx_oidc_auth_states_expires ON oidc_auth_states (expires_at)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_oidc_identities_issuer_subject ON oidc_identities (issuer, subject)",
        "CREATE INDEX IF NOT EXISTS idx_oidc_identities_username ON oidc_identities (username)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_logical_instances_key ON worker_logical_instances (namespace_name, app_name, cluster, region, client_instance_id)",
        "CREATE INDEX IF NOT EXISTS idx_worker_sessions_status_lease ON worker_sessions (status, lease_expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_worker_sessions_logical_generation ON worker_sessions (logical_instance_id, generation)",
        "CREATE INDEX IF NOT EXISTS idx_worker_session_events_worker_created ON worker_session_events (worker_id, created_at)",
    ] {
        db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await?;
    }
    Ok(())
}

async fn sqlite_column_exists(
    db: &DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool, sea_orm::DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA table_info({table})"),
        ))
        .await?;

    for row in rows {
        let name: String = row.try_get("", "name")?;
        if name == column {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

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

        crate::ensure_sqlite_schema_compatibility(&db)
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
