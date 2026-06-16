use super::{
    ClusterShardOwnership, DatabaseBackend, DispatchQueue, Statement, big_integer_col,
    big_integer_null, integer_null, string_col, string_null,
};
use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

pub(super) struct ShardOwnershipMigration;

impl MigrationName for ShardOwnershipMigration {
    fn name(&self) -> &'static str {
        "m20260616_000003_cluster_shard_ownership"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for ShardOwnershipMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ClusterShardOwnership::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ClusterShardOwnership::ShardId)
                            .integer()
                            .not_null()
                            .primary_key()
                            .take(),
                    )
                    .col(string_col(ClusterShardOwnership::OwnerNodeId))
                    .col(big_integer_col(ClusterShardOwnership::Epoch))
                    .col(big_integer_col(ClusterShardOwnership::RaftTerm))
                    .col(string_col(ClusterShardOwnership::FencingToken))
                    .col(string_col(ClusterShardOwnership::Status))
                    .col(string_null(ClusterShardOwnership::LeaseExpiresAt))
                    .col(string_col(ClusterShardOwnership::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        create_indexes(manager).await?;
        ensure_dispatch_queue_columns(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn create_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("idx_cluster_shard_ownership_owner")
                .table(ClusterShardOwnership::Table)
                .col(ClusterShardOwnership::OwnerNodeId)
                .col(ClusterShardOwnership::Status)
                .if_not_exists()
                .to_owned(),
        )
        .await?;
    if manager.get_database_backend() == DatabaseBackend::Sqlite
        && !sqlite_table_exists(manager, "dispatch_queue").await?
    {
        return Ok(());
    }
    manager
        .create_index(
            Index::create()
                .name("idx_dispatch_queue_shard_due")
                .table(DispatchQueue::Table)
                .col(DispatchQueue::ShardId)
                .col(DispatchQueue::Status)
                .col(DispatchQueue::RunAfter)
                .if_not_exists()
                .to_owned(),
        )
        .await
}

async fn ensure_dispatch_queue_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite {
        if !sqlite_table_exists(manager, "dispatch_queue").await? {
            return Ok(());
        }
        for (column, ddl) in [
            (
                "shard_id",
                "ALTER TABLE dispatch_queue ADD COLUMN shard_id integer",
            ),
            (
                "owner_epoch",
                "ALTER TABLE dispatch_queue ADD COLUMN owner_epoch bigint",
            ),
            (
                "owner_fencing_token",
                "ALTER TABLE dispatch_queue ADD COLUMN owner_fencing_token varchar(191)",
            ),
        ] {
            if !sqlite_column_exists(manager, "dispatch_queue", column).await? {
                manager
                    .get_connection()
                    .execute(Statement::from_string(DatabaseBackend::Sqlite, ddl))
                    .await?;
            }
        }
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(DispatchQueue::Table)
                .add_column_if_not_exists(integer_null(DispatchQueue::ShardId))
                .add_column_if_not_exists(big_integer_null(DispatchQueue::OwnerEpoch))
                .add_column_if_not_exists(string_null(DispatchQueue::OwnerFencingToken))
                .to_owned(),
        )
        .await
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
