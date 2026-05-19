//! Database schema migrations for scheduler storage.

use sea_orm_migration::prelude::*;

/// Storage schema migrator.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(CreateMetadataTables)]
    }
}

#[derive(DeriveMigrationName)]
struct CreateMetadataTables;

#[async_trait::async_trait]
impl MigrationTrait for CreateMetadataTables {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_namespaces(manager).await?;
        create_apps(manager).await?;
        create_jobs(manager).await?;
        create_job_instances(manager).await?;
        create_job_instance_logs(manager).await?;
        create_indexes(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JobInstanceLogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(JobInstances::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Apps::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Namespaces::Table).to_owned())
            .await?;
        Ok(())
    }
}

async fn create_job_instance_logs(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(JobInstanceLogs::Table)
                .if_not_exists()
                .col(string_pk(JobInstanceLogs::Id))
                .col(string_col(JobInstanceLogs::InstanceId))
                .col(string_col(JobInstanceLogs::WorkerId))
                .col(string_col(JobInstanceLogs::Level))
                .col(string_col(JobInstanceLogs::Message))
                .col(big_integer_col(JobInstanceLogs::Sequence))
                .col(string_col(JobInstanceLogs::CreatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_job_instance_logs_instance")
                        .from(JobInstanceLogs::Table, JobInstanceLogs::InstanceId)
                        .to(JobInstances::Table, JobInstances::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_namespaces(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Namespaces::Table)
                .if_not_exists()
                .col(string_pk(Namespaces::Id))
                .col(string_col(Namespaces::Name))
                .col(string_col(Namespaces::CreatedAt))
                .col(string_col(Namespaces::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_apps(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Apps::Table)
                .if_not_exists()
                .col(string_pk(Apps::Id))
                .col(string_col(Apps::NamespaceId))
                .col(string_col(Apps::Name))
                .col(string_col(Apps::CreatedAt))
                .col(string_col(Apps::UpdatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_apps_namespace")
                        .from(Apps::Table, Apps::NamespaceId)
                        .to(Namespaces::Table, Namespaces::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_jobs(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Jobs::Table)
                .if_not_exists()
                .col(string_pk(Jobs::Id))
                .col(string_col(Jobs::NamespaceId))
                .col(string_col(Jobs::AppId))
                .col(string_col(Jobs::Name))
                .col(string_col(Jobs::ScheduleType))
                .col(string_null(Jobs::ScheduleExpr))
                .col(boolean_col(Jobs::Enabled))
                .col(string_col(Jobs::CreatedAt))
                .col(string_col(Jobs::UpdatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_jobs_namespace")
                        .from(Jobs::Table, Jobs::NamespaceId)
                        .to(Namespaces::Table, Namespaces::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_jobs_app")
                        .from(Jobs::Table, Jobs::AppId)
                        .to(Apps::Table, Apps::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_job_instances(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(JobInstances::Table)
                .if_not_exists()
                .col(string_pk(JobInstances::Id))
                .col(string_col(JobInstances::JobId))
                .col(string_col(JobInstances::Status))
                .col(string_col(JobInstances::TriggerType))
                .col(string_col(JobInstances::CreatedAt))
                .col(string_col(JobInstances::UpdatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_job_instances_job")
                        .from(JobInstances::Table, JobInstances::JobId)
                        .to(Jobs::Table, Jobs::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    create_index(
        manager,
        Index::create()
            .name("idx_namespaces_name")
            .table(Namespaces::Table)
            .col(Namespaces::Name)
            .unique()
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_apps_namespace_name")
            .table(Apps::Table)
            .col(Apps::NamespaceId)
            .col(Apps::Name)
            .unique()
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_jobs_app_name")
            .table(Jobs::Table)
            .col(Jobs::AppId)
            .col(Jobs::Name)
            .unique()
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_jobs_enabled")
            .table(Jobs::Table)
            .col(Jobs::Enabled)
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_job_instances_job_created")
            .table(JobInstances::Table)
            .col(JobInstances::JobId)
            .col(JobInstances::CreatedAt)
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_job_instance_logs_instance_seq")
            .table(JobInstanceLogs::Table)
            .col(JobInstanceLogs::InstanceId)
            .col(JobInstanceLogs::Sequence)
            .to_owned(),
    )
    .await
}

async fn create_index(
    manager: &SchemaManager<'_>,
    statement: IndexCreateStatement,
) -> Result<(), DbErr> {
    manager.create_index(statement).await
}

#[derive(DeriveIden)]
enum Namespaces {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Apps {
    Table,
    Id,
    NamespaceId,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    Id,
    NamespaceId,
    AppId,
    Name,
    ScheduleType,
    ScheduleExpr,
    Enabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum JobInstances {
    Table,
    Id,
    JobId,
    Status,
    TriggerType,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum JobInstanceLogs {
    Table,
    Id,
    InstanceId,
    WorkerId,
    Level,
    Message,
    Sequence,
    CreatedAt,
}

fn string_pk<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column)
        .string()
        .not_null()
        .primary_key()
        .take()
}

fn string_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string().not_null().take()
}

fn string_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string().null().take()
}

fn boolean_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).boolean().not_null().take()
}

fn big_integer_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).big_integer().not_null().take()
}
