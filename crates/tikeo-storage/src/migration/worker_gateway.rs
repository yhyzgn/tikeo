use super::{DatabaseBackend, Statement, WorkerSessions, sea_query::Index};
use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::{
    DbErr, MigrationName, MigrationTrait, SchemaManager, async_trait,
};

pub(super) struct WorkerGatewayMigration;

impl MigrationName for WorkerGatewayMigration {
    fn name(&self) -> &'static str {
        "m20260616_000001_worker_gateway_node"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for WorkerGatewayMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DatabaseBackend::Sqlite {
            if !sqlite_table_exists(manager, "worker_sessions").await? {
                return Ok(());
            }
            if sqlite_column_exists(manager, "worker_sessions", "gateway_node_id").await? {
                return Ok(());
            }
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE worker_sessions ADD COLUMN gateway_node_id varchar(191) NOT NULL DEFAULT 'standalone'",
            ))
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_worker_sessions_gateway_status")
                    .table(WorkerSessions::Table)
                    .col(WorkerSessions::GatewayNodeId)
                    .col(WorkerSessions::Status)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
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
