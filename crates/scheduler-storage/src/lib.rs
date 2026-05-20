//! Persistent storage repositories and migrations for scheduler.

#![forbid(unsafe_code)]

pub mod entities;
pub mod migration;
pub mod repository;

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use sea_orm_migration::MigratorTrait;
use std::time::Duration;
use thiserror::Error;

pub use repository::{
    AppendJobInstanceLog, AuthSessionRepository, AuthSessionSummary, CreateAuthSession, CreateJob,
    CreateJobInstance, CreateJobInstanceAttempt, CreateScript, CreateUser,
    JobInstanceAttemptRepository, JobInstanceAttemptSummary, JobInstanceLogRepository,
    JobInstanceLogSummary, JobInstanceRepository, JobInstanceSummary, JobRepository, JobSummary,
    ScriptRepository, ScriptSummary, ScriptVersionRepository, ScriptVersionSummary, UpdateScript,
    UpdateUser, UserRepository, UserSummary,
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
        .idle_timeout(Duration::from_mins(1));

    let db = Database::connect(options).await?;
    migration::Migrator::up(&db, None).await?;
    ensure_sqlite_schema_compatibility(&db).await?;
    Ok(db)
}

async fn ensure_sqlite_schema_compatibility(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    ensure_broadcast_schema_compatibility(db).await?;
    ensure_auth_schema_compatibility(db).await?;
    ensure_scripts_schema_compatibility(db).await?;
    ensure_script_versions_schema_compatibility(db).await?;
    remove_sqlite_foreign_keys(db).await
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
            timeout_seconds bigint,
            max_memory_bytes bigint,
            allow_network boolean NOT NULL DEFAULT 0,
            allowed_env_vars varchar,
            created_by varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
    ))
    .await?;
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
            language varchar NOT NULL,
            status varchar NOT NULL,
            timeout_seconds bigint,
            max_memory_bytes bigint,
            allow_network boolean NOT NULL DEFAULT 0,
            allowed_env_vars varchar,
            created_by varchar NOT NULL,
            created_at varchar NOT NULL
        )",
    ))
    .await?;
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
            "INSERT OR IGNORE INTO users (id, username, password, role, created_at) VALUES ('usr-admin', 'scheduler_init', '$2b$10$/rflKev/thG2Je1e.2/7leHSg8Z/LYdSTqdpwsPKTyJMO5ajpysLW', 'admin', '{}')",
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
        "jobs",
        r"CREATE TABLE jobs (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            app_id varchar NOT NULL,
            name varchar NOT NULL,
            schedule_type varchar NOT NULL,
            schedule_expr varchar,
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
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_app_name ON jobs (app_id, name)",
        "CREATE INDEX IF NOT EXISTS idx_jobs_enabled ON jobs (enabled)",
        "CREATE INDEX IF NOT EXISTS idx_job_instances_job_created ON job_instances (job_id, created_at)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_job_instance_attempts_instance_worker ON job_instance_attempts (instance_id, worker_id)",
        "CREATE INDEX IF NOT EXISTS idx_job_instance_attempts_status ON job_instance_attempts (status)",
        "CREATE INDEX IF NOT EXISTS idx_job_instance_logs_instance_seq ON job_instance_logs (instance_id, sequence)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_sessions_token_hash ON auth_sessions (token_hash)",
        "CREATE INDEX IF NOT EXISTS idx_auth_sessions_user ON auth_sessions (user_id)",
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
