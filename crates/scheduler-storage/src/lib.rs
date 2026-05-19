//! Persistent storage repositories and migrations for scheduler.

#![forbid(unsafe_code)]

pub mod entities;
pub mod migration;
pub mod repository;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::time::Duration;
use thiserror::Error;

pub use repository::{
    AppendJobInstanceLog, CreateJob, CreateJobInstance, JobInstanceLogRepository,
    JobInstanceLogSummary, JobInstanceRepository, JobInstanceSummary, JobRepository, JobSummary,
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
    Ok(db)
}
