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
        create_job_instance_attempts(manager).await?;
        create_job_instance_logs(manager).await?;
        create_users(manager).await?;
        create_auth_sessions(manager).await?;
        create_indexes(manager).await?;

        // Seed default admin
        seed_admin_user(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthSessions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(JobInstanceLogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(JobInstanceAttempts::Table).to_owned())
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

async fn create_job_instance_attempts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(JobInstanceAttempts::Table)
                .if_not_exists()
                .col(string_pk(JobInstanceAttempts::Id))
                .col(string_col(JobInstanceAttempts::InstanceId))
                .col(string_col(JobInstanceAttempts::WorkerId))
                .col(string_col(JobInstanceAttempts::Status))
                .col(string_col(JobInstanceAttempts::CreatedAt))
                .col(string_col(JobInstanceAttempts::UpdatedAt))
                .to_owned(),
        )
        .await
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
                .to_owned(),
        )
        .await
}

async fn create_users(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Users::Table)
                .if_not_exists()
                .col(string_pk(Users::Id))
                .col(string_col(Users::Username))
                .col(string_col(Users::Password))
                .col(string_col(Users::Role))
                .col(string_col(Users::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_auth_sessions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(AuthSessions::Table)
                .if_not_exists()
                .col(string_pk(AuthSessions::Id))
                .col(string_col(AuthSessions::UserId))
                .col(string_col(AuthSessions::TokenHash))
                .col(string_null(AuthSessions::DeviceId))
                .col(string_null(AuthSessions::DeviceName))
                .col(string_col(AuthSessions::ExpiresAt))
                .col(string_col(AuthSessions::CreatedAt))
                .col(string_col(AuthSessions::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn seed_admin_user(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // Seed initial admin user using credentials documented in README: scheduler_init / Scheduler@2026!
    let insert = sea_query::Query::insert()
        .into_table(Users::Table)
        .columns([
            Users::Id,
            Users::Username,
            Users::Password,
            Users::Role,
            Users::CreatedAt,
        ])
        .values_panic([
            "usr-admin".into(),
            "scheduler_init".into(),
            "$2b$10$/rflKev/thG2Je1e.2/7leHSg8Z/LYdSTqdpwsPKTyJMO5ajpysLW".into(), // hash for "Scheduler@2026!"
            "admin".into(),
            now_rfc3339().into(),
        ])
        .to_owned();

    match manager.exec_stmt(insert).await {
        Ok(()) => Ok(()),
        Err(DbErr::Exec(error)) if error.to_string().contains("UNIQUE") => Ok(()),
        Err(error) => Err(error),
    }
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
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
                .col(string_col(JobInstances::ExecutionMode))
                .col(string_col(JobInstances::CreatedAt))
                .col(string_col(JobInstances::UpdatedAt))
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
    create_attempt_indexes(manager).await?;
    create_index(
        manager,
        Index::create()
            .name("idx_job_instance_logs_instance_seq")
            .table(JobInstanceLogs::Table)
            .col(JobInstanceLogs::InstanceId)
            .col(JobInstanceLogs::Sequence)
            .to_owned(),
    )
    .await?;

    create_index(
        manager,
        Index::create()
            .name("idx_users_username")
            .table(Users::Table)
            .col(Users::Username)
            .unique()
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_auth_sessions_token_hash")
            .table(AuthSessions::Table)
            .col(AuthSessions::TokenHash)
            .unique()
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_auth_sessions_user")
            .table(AuthSessions::Table)
            .col(AuthSessions::UserId)
            .to_owned(),
    )
    .await
}

#[derive(DeriveIden)]
enum AuthSessions {
    Table,
    Id,
    UserId,
    TokenHash,
    DeviceId,
    DeviceName,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    Password,
    Role,
    CreatedAt,
}

async fn create_attempt_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    create_index(
        manager,
        Index::create()
            .name("idx_job_instance_attempts_instance_worker")
            .table(JobInstanceAttempts::Table)
            .col(JobInstanceAttempts::InstanceId)
            .col(JobInstanceAttempts::WorkerId)
            .unique()
            .to_owned(),
    )
    .await?;
    create_index(
        manager,
        Index::create()
            .name("idx_job_instance_attempts_status")
            .table(JobInstanceAttempts::Table)
            .col(JobInstanceAttempts::Status)
            .to_owned(),
    )
    .await
}

async fn create_index(
    manager: &SchemaManager<'_>,
    mut statement: IndexCreateStatement,
) -> Result<(), DbErr> {
    statement.if_not_exists();
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
    ExecutionMode,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum JobInstanceAttempts {
    Table,
    Id,
    InstanceId,
    WorkerId,
    Status,
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
