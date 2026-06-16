use super::{
    WorkerDispatchOutbox, big_integer_col, integer_col, string_col, string_null, string_pk,
    text_col, text_null,
};
use sea_orm_migration::prelude::*;

pub(super) struct FsodOutboxMigration;

impl MigrationName for FsodOutboxMigration {
    fn name(&self) -> &'static str {
        "m20260616_000002_worker_dispatch_outbox"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for FsodOutboxMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkerDispatchOutbox::Table)
                    .if_not_exists()
                    .col(string_pk(WorkerDispatchOutbox::Id))
                    .col(string_col(WorkerDispatchOutbox::InstanceId))
                    .col(string_col(WorkerDispatchOutbox::AttemptId))
                    .col(string_col(WorkerDispatchOutbox::WorkerId))
                    .col(string_col(WorkerDispatchOutbox::LogicalInstanceId))
                    .col(string_col(WorkerDispatchOutbox::GatewayNodeId))
                    .col(big_integer_col(WorkerDispatchOutbox::GatewayGeneration))
                    .col(string_col(WorkerDispatchOutbox::AssignmentToken))
                    .col(text_col(WorkerDispatchOutbox::DispatchPayload))
                    .col(big_integer_col(WorkerDispatchOutbox::ShardId))
                    .col(big_integer_col(WorkerDispatchOutbox::ShardMapVersion))
                    .col(big_integer_col(WorkerDispatchOutbox::ShardCount))
                    .col(string_col(WorkerDispatchOutbox::OwnerNodeId))
                    .col(big_integer_col(WorkerDispatchOutbox::OwnerEpoch))
                    .col(string_col(WorkerDispatchOutbox::OwnerFencingToken))
                    .col(string_col(WorkerDispatchOutbox::Status))
                    .col(integer_col(WorkerDispatchOutbox::DeliveryAttempts))
                    .col(string_col(WorkerDispatchOutbox::NextDeliveryAt))
                    .col(string_null(WorkerDispatchOutbox::VisibilityDeadline))
                    .col(text_null(WorkerDispatchOutbox::LastError))
                    .col(string_col(WorkerDispatchOutbox::CreatedAt))
                    .col(string_col(WorkerDispatchOutbox::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        create_outbox_indexes(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn create_outbox_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("idx_worker_dispatch_outbox_attempt")
                .table(WorkerDispatchOutbox::Table)
                .col(WorkerDispatchOutbox::InstanceId)
                .col(WorkerDispatchOutbox::AttemptId)
                .unique()
                .if_not_exists()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_worker_dispatch_outbox_gateway_due")
                .table(WorkerDispatchOutbox::Table)
                .col(WorkerDispatchOutbox::GatewayNodeId)
                .col(WorkerDispatchOutbox::Status)
                .col(WorkerDispatchOutbox::NextDeliveryAt)
                .if_not_exists()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_worker_dispatch_outbox_logical_status")
                .table(WorkerDispatchOutbox::Table)
                .col(WorkerDispatchOutbox::LogicalInstanceId)
                .col(WorkerDispatchOutbox::Status)
                .if_not_exists()
                .to_owned(),
        )
        .await
}
