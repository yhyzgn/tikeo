use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub(super) struct LegacySqliteSchemaCompatibility;

#[async_trait::async_trait]
impl MigrationTrait for LegacySqliteSchemaCompatibility {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        apply_sqlite_schema_compatibility(manager.get_connection()).await
    }
}

pub(super) async fn apply_sqlite_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    ensure_broadcast_schema_compatibility(db).await?;
    ensure_auth_schema_compatibility(db).await?;
    ensure_service_account_schema_compatibility(db).await?;
    ensure_sdk_api_key_schema_compatibility(db).await?;
    ensure_oidc_auth_state_schema_compatibility(db).await?;
    ensure_oidc_identity_schema_compatibility(db).await?;
    ensure_rbac_schema_compatibility(db).await?;
    ensure_scope_schema_compatibility(db).await?;
    ensure_calendar_schema_compatibility(db).await?;
    ensure_plugin_schema_compatibility(db).await?;
    ensure_worker_lifecycle_schema_compatibility(db).await?;
    ensure_job_schema_compatibility(db).await?;
    ensure_schedule_cursor_schema_compatibility(db).await?;
    ensure_scripts_schema_compatibility(db).await?;
    ensure_script_versions_schema_compatibility(db).await?;
    ensure_audit_logs_schema_compatibility(db).await?;
    ensure_alert_schema_compatibility(db).await?;
    ensure_workflow_schema_compatibility(db).await?;
    ensure_raft_schema_compatibility(db).await?;
    foreign_keys::remove_sqlite_foreign_keys(db).await
}

async fn ensure_calendar_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS calendars (
            id varchar NOT NULL PRIMARY KEY,
            namespace varchar NOT NULL,
            app varchar NOT NULL,
            name varchar NOT NULL,
            timezone varchar NOT NULL,
            excluded_dates_json text NOT NULL,
            holidays_json text NOT NULL,
            maintenance_windows_json text NOT NULL,
            freeze_windows_json text NOT NULL,
            created_by varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_calendars_scope_name ON calendars (namespace, app, name)",
    ))
    .await?;
    Ok(())
}

async fn ensure_plugin_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS plugins (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            kind varchar NOT NULL,
            processor_types_json text NOT NULL,
            alert_channel_types_json text NOT NULL,
            enabled boolean NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_plugins_name ON plugins (name)",
    ))
    .await?;
    Ok(())
}

async fn ensure_scope_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS namespaces (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            max_queue_depth integer NOT NULL DEFAULT 0,
            max_concurrency integer NOT NULL DEFAULT 0,
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
    db: &impl ConnectionTrait,
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
            capabilities_json text NOT NULL DEFAULT '[]',
            structured_capabilities_json text NOT NULL DEFAULT '{}',
            labels_json text NOT NULL DEFAULT '{}',
            master_json text NOT NULL DEFAULT '{}',
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
    for (column, default_json) in [
        ("capabilities_json", "'[]'"),
        ("structured_capabilities_json", "'{}'"),
        ("labels_json", "'{}'"),
        ("master_json", "'{}'"),
    ] {
        if !sqlite_column_exists(db, "worker_sessions", column).await? {
            db.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("ALTER TABLE worker_sessions ADD COLUMN {column} text NOT NULL DEFAULT {default_json}"),
            ))
            .await?;
        }
    }
    Ok(())
}

async fn ensure_job_schema_compatibility(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
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
    if !sqlite_column_exists(db, "jobs", "processor_type").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE jobs ADD COLUMN processor_type varchar",
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
    if !sqlite_column_exists(db, "jobs", "schedule_calendar_json").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE jobs ADD COLUMN schedule_calendar_json varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "jobs", "canary_job_id").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE jobs ADD COLUMN canary_job_id varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "jobs", "canary_percent").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE jobs ADD COLUMN canary_percent integer NOT NULL DEFAULT 0",
        ))
        .await?;
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS job_versions (
            id varchar NOT NULL PRIMARY KEY,
            job_id varchar NOT NULL,
            version_number bigint NOT NULL,
            name varchar NOT NULL,
            schedule_type varchar NOT NULL,
            schedule_expr varchar,
            misfire_policy varchar NOT NULL DEFAULT 'fire_once',
            schedule_start_at varchar,
            schedule_end_at varchar,
            schedule_calendar_json varchar,
            processor_name varchar,
            processor_type varchar,
            script_id varchar,
            enabled boolean NOT NULL,
            created_by varchar NOT NULL,
            change_reason varchar NOT NULL,
            rolled_back_from_version bigint,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    if !sqlite_column_exists(db, "job_versions", "schedule_calendar_json").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE job_versions ADD COLUMN schedule_calendar_json varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "job_versions", "processor_type").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE job_versions ADD COLUMN processor_type varchar",
        ))
        .await?;
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_job_versions_job_number ON job_versions (job_id, version_number)",
    ))
    .await?;
    Ok(())
}

async fn ensure_alert_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
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
    db: &impl ConnectionTrait,
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
        r"CREATE TABLE IF NOT EXISTS workflow_shards (id varchar NOT NULL PRIMARY KEY, workflow_instance_id varchar NOT NULL, workflow_node_instance_id varchar NOT NULL, node_key varchar NOT NULL, shard_index integer NOT NULL, status varchar NOT NULL, input varchar NOT NULL, output varchar, checkpoint varchar, retry_count integer NOT NULL DEFAULT 0, job_instance_id varchar, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
        r"CREATE TABLE IF NOT EXISTS dispatch_queue (id varchar NOT NULL PRIMARY KEY, job_instance_id varchar, workflow_node_instance_id varchar, priority integer NOT NULL, run_after varchar NOT NULL, status varchar NOT NULL, attempt integer NOT NULL, lease_owner varchar, lease_until varchar, fencing_token varchar, worker_selector varchar, namespace varchar, app varchar, worker_pool varchar, created_at varchar NOT NULL, updated_at varchar NOT NULL)",
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
    if !sqlite_column_exists(db, "workflow_shards", "checkpoint").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE workflow_shards ADD COLUMN checkpoint varchar",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "workflow_shards", "retry_count").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE workflow_shards ADD COLUMN retry_count integer NOT NULL DEFAULT 0",
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

async fn ensure_raft_schema_compatibility(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
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

async fn ensure_schedule_cursor_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS schedule_cursors (
            id varchar NOT NULL PRIMARY KEY,
            job_id varchar NOT NULL,
            trigger_type varchar NOT NULL,
            fire_at varchar NOT NULL,
            instance_id varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_schedule_cursors_job_trigger_fire ON schedule_cursors (job_id, trigger_type, fire_at)",
    ))
    .await?;
    Ok(())
}

async fn ensure_scripts_schema_compatibility(
    db: &impl ConnectionTrait,
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
    db: &impl ConnectionTrait,
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
    db: &impl ConnectionTrait,
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
    db: &impl ConnectionTrait,
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

async fn ensure_rbac_schema_compatibility(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
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

async fn seed_sqlite_rbac_defaults(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
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
    db: &impl ConnectionTrait,
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
    db: &impl ConnectionTrait,
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
    db: &impl ConnectionTrait,
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

async fn ensure_service_account_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS service_accounts (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            description varchar,
            namespace varchar NOT NULL,
            app varchar NOT NULL,
            worker_pool varchar,
            status varchar NOT NULL,
            created_by varchar NOT NULL,
            updated_by varchar,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_service_accounts_scope_name ON service_accounts (namespace, app, name)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_service_accounts_status ON service_accounts (status)",
    ))
    .await?;
    Ok(())
}

async fn ensure_sdk_api_key_schema_compatibility(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS sdk_api_keys (
            id varchar NOT NULL PRIMARY KEY,
            name varchar NOT NULL,
            key_hash varchar NOT NULL,
            key_prefix varchar NOT NULL,
            namespace varchar NOT NULL,
            app varchar NOT NULL,
            service_account_id varchar NOT NULL,
            service_account_name varchar NOT NULL,
            scopes text NOT NULL,
            status varchar NOT NULL,
            expires_at varchar,
            last_used_at varchar,
            created_by varchar NOT NULL,
            revoked_by varchar,
            rotated_from varchar,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
    if !sqlite_column_exists(db, "sdk_api_keys", "service_account_id").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE sdk_api_keys ADD COLUMN service_account_id varchar NOT NULL DEFAULT ''",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "sdk_api_keys", "service_account_name").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE sdk_api_keys ADD COLUMN service_account_name varchar NOT NULL DEFAULT ''",
        ))
        .await?;
    }
    backfill_service_accounts_from_sdk_keys(db).await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_sdk_api_keys_hash ON sdk_api_keys (key_hash)",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE INDEX IF NOT EXISTS idx_sdk_api_keys_scope ON sdk_api_keys (namespace, app, status)",
    ))
    .await?;
    Ok(())
}

async fn ensure_auth_schema_compatibility(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r"CREATE TABLE IF NOT EXISTS users (
            id varchar NOT NULL PRIMARY KEY,
            username varchar NOT NULL,
            email varchar NOT NULL DEFAULT '',
            password varchar NOT NULL,
            role varchar NOT NULL,
            bootstrap_admin boolean NOT NULL DEFAULT FALSE,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
    if !sqlite_column_exists(db, "users", "email").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE users ADD COLUMN email varchar NOT NULL DEFAULT ''",
        ))
        .await?;
    }
    if !sqlite_column_exists(db, "users", "bootstrap_admin").await? {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "ALTER TABLE users ADD COLUMN bootstrap_admin boolean NOT NULL DEFAULT FALSE",
        ))
        .await?;
    }
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
    Ok(())
}

async fn backfill_service_accounts_from_sdk_keys(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            r"SELECT service_account_id, service_account_name, namespace, app, created_by, MIN(created_at) AS created_at
              FROM sdk_api_keys
              WHERE service_account_id IS NOT NULL AND service_account_id != ''
              GROUP BY service_account_id, service_account_name, namespace, app, created_by",
        ))
        .await?;
    for row in rows {
        let id: String = row.try_get("", "service_account_id")?;
        let name: String = row.try_get("", "service_account_name")?;
        let namespace: String = row.try_get("", "namespace")?;
        let app: String = row.try_get("", "app")?;
        let created_by: String = row.try_get("", "created_by")?;
        let created_at: String = row.try_get("", "created_at")?;
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r"INSERT OR IGNORE INTO service_accounts
              (id, name, description, namespace, app, worker_pool, status, created_by, updated_by, created_at, updated_at)
              VALUES (?, ?, NULL, ?, ?, NULL, 'active', ?, NULL, ?, ?)",
            vec![
                id.into(),
                name.into(),
                namespace.into(),
                app.into(),
                created_by.into(),
                created_at.clone().into(),
                created_at.into(),
            ],
        ))
        .await?;
    }
    Ok(())
}

mod foreign_keys;

async fn sqlite_column_exists(
    db: &impl ConnectionTrait,
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
