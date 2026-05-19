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
    AppendJobInstanceLog, CreateJob, CreateJobInstance, CreateJobInstanceAttempt, CreateUser,
    JobInstanceAttemptRepository, JobInstanceAttemptSummary, JobInstanceLogRepository,
    JobInstanceLogSummary, JobInstanceRepository, JobInstanceSummary, JobRepository, JobSummary,
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
    ensure_broadcast_schema_compatibility(&db).await?;
    Ok(db)
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
            updated_at varchar NOT NULL,
            FOREIGN KEY(instance_id) REFERENCES job_instances(id) ON DELETE CASCADE
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
