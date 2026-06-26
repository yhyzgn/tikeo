use super::{DatabaseBackend, JobVersions, Jobs, Statement, string_null};
use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

pub(super) struct WorkerPoolJobScopeMigration;

impl MigrationName for WorkerPoolJobScopeMigration {
    fn name(&self) -> &'static str {
        "m20260626_000001_worker_pool_job_scope"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for WorkerPoolJobScopeMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_job_worker_pool_column(manager).await?;
        ensure_job_version_worker_pool_column(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn ensure_job_worker_pool_column(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite {
        add_sqlite_column_if_missing(
            manager,
            "jobs",
            "worker_pool",
            "ALTER TABLE jobs ADD COLUMN worker_pool varchar",
        )
        .await
    } else {
        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_column_if_not_exists(string_null(Jobs::WorkerPool))
                    .to_owned(),
            )
            .await
    }
}

async fn ensure_job_version_worker_pool_column(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite {
        add_sqlite_column_if_missing(
            manager,
            "job_versions",
            "worker_pool",
            "ALTER TABLE job_versions ADD COLUMN worker_pool varchar",
        )
        .await
    } else {
        manager
            .alter_table(
                Table::alter()
                    .table(JobVersions::Table)
                    .add_column_if_not_exists(string_null(JobVersions::WorkerPool))
                    .to_owned(),
            )
            .await
    }
}

async fn add_sqlite_column_if_missing(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<(), DbErr> {
    if !sqlite_table_exists(manager, table).await?
        || sqlite_column_exists(manager, table, column).await?
    {
        return Ok(());
    }
    manager
        .get_connection()
        .execute(Statement::from_string(DatabaseBackend::Sqlite, ddl))
        .await?;
    Ok(())
}

async fn sqlite_column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let rows = manager
        .get_connection()
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

async fn sqlite_table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let rows = manager
        .get_connection()
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"),
        ))
        .await?;
    Ok(!rows.is_empty())
}
