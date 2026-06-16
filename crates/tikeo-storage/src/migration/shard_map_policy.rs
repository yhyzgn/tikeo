use super::{
    ClusterShardOwnership, DatabaseBackend, DispatchQueue, Statement, WorkerDispatchOutbox,
    big_integer_col, big_integer_null, integer_col, integer_null,
};
use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

pub(super) struct ShardMapPolicyMigration;

impl MigrationName for ShardMapPolicyMigration {
    fn name(&self) -> &'static str {
        "m20260616_000004_scheduler_shard_map_policy"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for ShardMapPolicyMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_dispatch_queue_columns(manager).await?;
        ensure_cluster_shard_ownership_columns(manager).await?;
        ensure_worker_dispatch_outbox_columns(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn ensure_dispatch_queue_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite {
        if !sqlite_table_exists(manager, "dispatch_queue").await? {
            return Ok(());
        }
        for (column, ddl) in [
            (
                "shard_map_version",
                "ALTER TABLE dispatch_queue ADD COLUMN shard_map_version bigint",
            ),
            (
                "shard_count",
                "ALTER TABLE dispatch_queue ADD COLUMN shard_count integer",
            ),
        ] {
            add_sqlite_column_if_missing(manager, "dispatch_queue", column, ddl).await?;
        }
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(DispatchQueue::Table)
                .add_column_if_not_exists(big_integer_null(DispatchQueue::ShardMapVersion))
                .add_column_if_not_exists(integer_null(DispatchQueue::ShardCount))
                .to_owned(),
        )
        .await
}

async fn ensure_cluster_shard_ownership_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite {
        if !sqlite_table_exists(manager, "cluster_shard_ownership").await? {
            return Ok(());
        }
        for (column, ddl) in [
            (
                "shard_map_version",
                "ALTER TABLE cluster_shard_ownership ADD COLUMN shard_map_version bigint NOT NULL DEFAULT 1",
            ),
            (
                "shard_count",
                "ALTER TABLE cluster_shard_ownership ADD COLUMN shard_count integer NOT NULL DEFAULT 64",
            ),
        ] {
            add_sqlite_column_if_missing(manager, "cluster_shard_ownership", column, ddl).await?;
        }
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(ClusterShardOwnership::Table)
                .add_column_if_not_exists(
                    big_integer_col(ClusterShardOwnership::ShardMapVersion).default(1),
                )
                .add_column_if_not_exists(
                    integer_col(ClusterShardOwnership::ShardCount).default(64),
                )
                .to_owned(),
        )
        .await
}

async fn ensure_worker_dispatch_outbox_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite {
        if !sqlite_table_exists(manager, "worker_dispatch_outbox").await? {
            return Ok(());
        }
        for (column, ddl) in [
            (
                "shard_map_version",
                "ALTER TABLE worker_dispatch_outbox ADD COLUMN shard_map_version bigint NOT NULL DEFAULT 1",
            ),
            (
                "shard_count",
                "ALTER TABLE worker_dispatch_outbox ADD COLUMN shard_count bigint NOT NULL DEFAULT 64",
            ),
        ] {
            add_sqlite_column_if_missing(manager, "worker_dispatch_outbox", column, ddl).await?;
        }
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(WorkerDispatchOutbox::Table)
                .add_column_if_not_exists(
                    big_integer_col(WorkerDispatchOutbox::ShardMapVersion).default(1),
                )
                .add_column_if_not_exists(
                    big_integer_col(WorkerDispatchOutbox::ShardCount).default(64),
                )
                .to_owned(),
        )
        .await
}

async fn add_sqlite_column_if_missing(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<(), DbErr> {
    if !sqlite_column_exists(manager, table, column).await? {
        manager
            .get_connection()
            .execute(Statement::from_string(DatabaseBackend::Sqlite, ddl))
            .await?;
    }
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
