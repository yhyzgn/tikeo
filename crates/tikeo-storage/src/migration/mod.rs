//! Database schema migrations for tikeo storage.

mod columns;
mod iden;
mod indexes;
mod notification_center;
mod rbac_role_management;
mod sqlite_compat;

use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;
use sea_query::InsertStatement;

use self::{
    columns::{
        big_integer_col, big_integer_null, boolean_col, integer_col, integer_null,
        short_string_col, string_col, string_null, string_pk, text_col, text_null,
    },
    iden::{
        AlertDeliveryAttempts, AlertEvents, AlertRules, Apps, AuditLogs, AuthSessions,
        DispatchQueue, InstanceEvents, JobInstanceAttempts, JobInstanceLogs, JobInstances,
        JobVersions, Jobs, Namespaces, NotificationChannels, NotificationDeliveryAttempts,
        NotificationMessages, NotificationPolicies, NotificationTemplates, OidcAuthStates,
        OidcIdentities, Permissions, Plugins, RaftAppliedCommands, RaftLogEntries, RaftMembers,
        RaftMembershipProposals, RaftMetadata, RaftSnapshots, RoleMenuPermissions, RolePermissions,
        RoleUiActionPermissions, Roles, ScheduleCursors, ScriptVersions, Scripts, SdkApiKeys,
        Secrets, ServiceAccounts, UserRoles, Users, WorkerLogicalInstances, WorkerPools,
        WorkerSessionEvents, WorkerSessions, WorkflowEdges, WorkflowInstances,
        WorkflowNodeInstances, WorkflowNodes, WorkflowShards, Workflows,
    },
    indexes::create_indexes,
    sqlite_compat::LegacySqliteSchemaCompatibility,
};

pub(crate) use sqlite_compat::apply_sqlite_schema_compatibility;

/// Storage schema migrator.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(CreateMetadataTables),
            Box::new(LegacySqliteSchemaCompatibility),
            Box::new(RbacRoleManagementMigration),
            Box::new(NotificationCenterMigration),
            Box::new(NotificationTemplatesMigration),
            Box::new(NotificationChannelExamplesMigration),
        ]
    }
}

use notification_center::{
    NotificationCenterMigration, NotificationChannelExamplesMigration,
    NotificationTemplatesMigration,
};
use rbac_role_management::RbacRoleManagementMigration;

#[derive(DeriveMigrationName)]
struct CreateMetadataTables;

#[async_trait::async_trait]
impl MigrationTrait for CreateMetadataTables {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_namespaces(manager).await?;
        create_apps(manager).await?;
        create_worker_pools(manager).await?;
        create_worker_lifecycle_tables(manager).await?;
        create_jobs(manager).await?;
        create_job_versions(manager).await?;
        create_job_instances(manager).await?;
        create_schedule_cursors(manager).await?;
        create_job_instance_attempts(manager).await?;
        create_job_instance_logs(manager).await?;
        create_users(manager).await?;
        create_rbac_tables(manager).await?;
        create_auth_sessions(manager).await?;
        create_service_accounts(manager).await?;
        create_sdk_api_keys(manager).await?;
        create_secrets(manager).await?;
        create_oidc_auth_states(manager).await?;
        create_oidc_identities(manager).await?;
        create_plugins(manager).await?;
        create_scripts(manager).await?;
        create_script_versions(manager).await?;
        create_workflow_tables(manager).await?;
        create_workflow_shards(manager).await?;
        create_dispatch_queue(manager).await?;
        create_instance_events(manager).await?;
        create_raft_tables(manager).await?;
        create_audit_logs(manager).await?;
        create_alert_rules(manager).await?;
        create_alert_events(manager).await?;
        create_alert_delivery_attempts(manager).await?;
        create_indexes(manager).await?;

        seed_rbac_defaults(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_metadata_tables(manager).await
    }
}

async fn drop_metadata_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    drop_tables(
        manager,
        &[
            AlertDeliveryAttempts::Table.into_iden(),
            AlertEvents::Table.into_iden(),
            AlertRules::Table.into_iden(),
            AuditLogs::Table.into_iden(),
            RaftAppliedCommands::Table.into_iden(),
            RaftMembershipProposals::Table.into_iden(),
            RaftSnapshots::Table.into_iden(),
            RaftLogEntries::Table.into_iden(),
            RaftMembers::Table.into_iden(),
            RaftMetadata::Table.into_iden(),
            InstanceEvents::Table.into_iden(),
            DispatchQueue::Table.into_iden(),
            WorkflowNodeInstances::Table.into_iden(),
            WorkflowInstances::Table.into_iden(),
            WorkflowEdges::Table.into_iden(),
            WorkflowNodes::Table.into_iden(),
            Workflows::Table.into_iden(),
            ScriptVersions::Table.into_iden(),
            Scripts::Table.into_iden(),
            Secrets::Table.into_iden(),
            SdkApiKeys::Table.into_iden(),
            ServiceAccounts::Table.into_iden(),
            Plugins::Table.into_iden(),
        ],
    )
    .await?;
    drop_auth_tables(manager).await?;
    drop_tables(
        manager,
        &[
            RolePermissions::Table.into_iden(),
            Permissions::Table.into_iden(),
            Roles::Table.into_iden(),
            Users::Table.into_iden(),
            JobInstanceLogs::Table.into_iden(),
            JobInstanceAttempts::Table.into_iden(),
            ScheduleCursors::Table.into_iden(),
            JobInstances::Table.into_iden(),
            JobVersions::Table.into_iden(),
            Jobs::Table.into_iden(),
            Apps::Table.into_iden(),
            WorkerSessionEvents::Table.into_iden(),
            WorkerSessions::Table.into_iden(),
            WorkerLogicalInstances::Table.into_iden(),
            WorkerPools::Table.into_iden(),
            Namespaces::Table.into_iden(),
        ],
    )
    .await
}

async fn drop_tables(manager: &SchemaManager<'_>, tables: &[DynIden]) -> Result<(), DbErr> {
    for table in tables {
        manager
            .drop_table(Table::drop().table(table.clone()).to_owned())
            .await?;
    }
    Ok(())
}

async fn create_schedule_cursors(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ScheduleCursors::Table)
                .if_not_exists()
                .col(string_pk(ScheduleCursors::Id))
                .col(string_col(ScheduleCursors::JobId))
                .col(string_col(ScheduleCursors::TriggerType))
                .col(string_col(ScheduleCursors::FireAt))
                .col(string_col(ScheduleCursors::InstanceId))
                .col(string_col(ScheduleCursors::CreatedAt))
                .to_owned(),
        )
        .await
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
                .col(
                    ColumnDef::new(JobInstanceAttempts::ResultSuccess)
                        .boolean()
                        .null(),
                )
                .col(text_null(JobInstanceAttempts::ResultMessage))
                .col(string_null(JobInstanceAttempts::ResultCompletedAt))
                .col(string_col(JobInstanceAttempts::CreatedAt))
                .col(string_col(JobInstanceAttempts::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RaftAppliedCommands::Table)
                .if_not_exists()
                .col(string_pk(RaftAppliedCommands::Id))
                .col(string_col(RaftAppliedCommands::ClusterId))
                .col(string_col(RaftAppliedCommands::NodeId))
                .col(big_integer_col(RaftAppliedCommands::LogIndex))
                .col(big_integer_col(RaftAppliedCommands::Term))
                .col(string_col(RaftAppliedCommands::CommandId))
                .col(string_col(RaftAppliedCommands::CommandType))
                .col(text_null(RaftAppliedCommands::Payload))
                .col(string_col(RaftAppliedCommands::Status))
                .col(text_col(RaftAppliedCommands::Message))
                .col(string_col(RaftAppliedCommands::CreatedAt))
                .col(string_col(RaftAppliedCommands::UpdatedAt))
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
                .col(string_col(Users::Email))
                .col(string_col(Users::Password))
                .col(string_col(Users::Role))
                .col(boolean_col(Users::BootstrapAdmin).default(false))
                .col(string_col(Users::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_rbac_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Roles::Table)
                .if_not_exists()
                .col(string_pk(Roles::Id))
                .col(string_col(Roles::Name))
                .col(string_col(Roles::Description))
                .col(string_col(Roles::CreatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Permissions::Table)
                .if_not_exists()
                .col(string_pk(Permissions::Id))
                .col(string_col(Permissions::Resource))
                .col(string_col(Permissions::Action))
                .col(string_col(Permissions::Description))
                .col(string_col(Permissions::CreatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RolePermissions::Table)
                .if_not_exists()
                .col(string_pk(RolePermissions::Id))
                .col(string_col(RolePermissions::RoleId))
                .col(string_col(RolePermissions::PermissionId))
                .col(string_col(RolePermissions::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_service_accounts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ServiceAccounts::Table)
                .if_not_exists()
                .col(string_pk(ServiceAccounts::Id))
                .col(string_col(ServiceAccounts::Name))
                .col(string_null(ServiceAccounts::Description))
                .col(string_col(ServiceAccounts::Namespace))
                .col(string_col(ServiceAccounts::App))
                .col(string_null(ServiceAccounts::WorkerPool))
                .col(string_col(ServiceAccounts::Status))
                .col(string_col(ServiceAccounts::CreatedBy))
                .col(string_null(ServiceAccounts::UpdatedBy))
                .col(string_col(ServiceAccounts::CreatedAt))
                .col(string_col(ServiceAccounts::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_sdk_api_keys(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SdkApiKeys::Table)
                .if_not_exists()
                .col(string_pk(SdkApiKeys::Id))
                .col(string_col(SdkApiKeys::Name))
                .col(string_col(SdkApiKeys::KeyHash))
                .col(string_col(SdkApiKeys::KeyPrefix))
                .col(string_col(SdkApiKeys::Namespace))
                .col(string_col(SdkApiKeys::App))
                .col(string_col(SdkApiKeys::ServiceAccountId))
                .col(string_col(SdkApiKeys::ServiceAccountName))
                .col(text_col(SdkApiKeys::Scopes))
                .col(string_col(SdkApiKeys::Status))
                .col(string_null(SdkApiKeys::ExpiresAt))
                .col(string_null(SdkApiKeys::LastUsedAt))
                .col(string_col(SdkApiKeys::CreatedBy))
                .col(string_null(SdkApiKeys::RevokedBy))
                .col(string_null(SdkApiKeys::RotatedFrom))
                .col(string_col(SdkApiKeys::CreatedAt))
                .col(string_col(SdkApiKeys::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_secrets(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Secrets::Table)
                .if_not_exists()
                .col(string_pk(Secrets::Id))
                .col(string_col(Secrets::Namespace))
                .col(string_col(Secrets::App))
                .col(string_col(Secrets::Name))
                .col(string_col(Secrets::ValueRef))
                .col(string_col(Secrets::Status))
                .col(string_col(Secrets::CreatedBy))
                .col(string_col(Secrets::CreatedAt))
                .col(string_col(Secrets::UpdatedAt))
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

async fn create_oidc_auth_states(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(OidcAuthStates::Table)
                .if_not_exists()
                .col(string_pk(OidcAuthStates::Id))
                .col(string_col(OidcAuthStates::StateHash))
                .col(string_col(OidcAuthStates::RedirectUri))
                .col(string_col(OidcAuthStates::ExpiresAt))
                .col(string_null(OidcAuthStates::ConsumedAt))
                .col(string_col(OidcAuthStates::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_oidc_identities(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(OidcIdentities::Table)
                .if_not_exists()
                .col(string_pk(OidcIdentities::Id))
                .col(string_col(OidcIdentities::Issuer))
                .col(string_col(OidcIdentities::Subject))
                .col(string_col(OidcIdentities::Username))
                .col(string_null(OidcIdentities::Namespace))
                .col(string_null(OidcIdentities::App))
                .col(string_null(OidcIdentities::WorkerPool))
                .col(string_col(OidcIdentities::CreatedAt))
                .col(string_col(OidcIdentities::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_plugins(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Plugins::Table)
                .if_not_exists()
                .col(string_pk(Plugins::Id))
                .col(string_col(Plugins::Name))
                .col(string_col(Plugins::Kind))
                .col(text_col(Plugins::ProcessorTypesJson))
                .col(text_col(Plugins::AlertChannelTypesJson))
                .col(boolean_col(Plugins::Enabled))
                .col(string_col(Plugins::CreatedAt))
                .col(string_col(Plugins::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_scripts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Scripts::Table)
                .if_not_exists()
                .col(string_pk(Scripts::Id))
                .col(string_col(Scripts::Name))
                .col(string_col(Scripts::Language))
                .col(string_col(Scripts::Version))
                .col(text_col(Scripts::Content))
                .col(string_col(Scripts::Status))
                .col(string_null(Scripts::ReleasedVersionId))
                .col(big_integer_null(Scripts::ReleasedVersionNumber))
                .col(string_null(Scripts::ReleaseApprovalTicket))
                .col(string_null(Scripts::ReleaseSignature))
                .col(string_null(Scripts::ReleaseSignatureVerifiedAt))
                .col(string_null(Scripts::ReleaseSignatureVerifiedBy))
                .col(text_null(Scripts::ReleaseGrantsJson))
                .col(string_null(Scripts::ReleaseGrantsVerifiedAt))
                .col(string_null(Scripts::ReleaseGrantsVerifiedBy))
                .col(big_integer_null(Scripts::TimeoutSeconds))
                .col(big_integer_null(Scripts::MaxMemoryBytes))
                .col(boolean_col(Scripts::AllowNetwork))
                .col(string_null(Scripts::AllowedEnvVars))
                .col(string_null(Scripts::PolicyJson))
                .col(string_col(Scripts::CreatedBy))
                .col(string_col(Scripts::CreatedAt))
                .col(string_col(Scripts::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn drop_auth_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .drop_table(Table::drop().table(OidcIdentities::Table).to_owned())
        .await?;
    manager
        .drop_table(Table::drop().table(OidcAuthStates::Table).to_owned())
        .await?;
    manager
        .drop_table(Table::drop().table(AuthSessions::Table).to_owned())
        .await
}

async fn create_audit_logs(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(AuditLogs::Table)
                .if_not_exists()
                .col(string_pk(AuditLogs::Id))
                .col(string_col(AuditLogs::Actor))
                .col(string_col(AuditLogs::Action))
                .col(string_col(AuditLogs::ResourceType))
                .col(string_col(AuditLogs::ResourceId))
                .col(text_null(AuditLogs::Detail))
                .col(text_null(AuditLogs::Before))
                .col(text_null(AuditLogs::After))
                .col(string_null(AuditLogs::TraceId))
                .col(string_col(AuditLogs::Result))
                .col(string_null(AuditLogs::FailureReason))
                .col(string_null(AuditLogs::IpAddress))
                .col(string_col(AuditLogs::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_alert_rules(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(AlertRules::Table)
                .if_not_exists()
                .col(string_pk(AlertRules::Id))
                .col(string_col(AlertRules::Name))
                .col(string_col(AlertRules::Severity))
                .col(text_col(AlertRules::ConditionJson))
                .col(text_col(AlertRules::ChannelsJson))
                .col(boolean_col(AlertRules::Enabled))
                .col(big_integer_col(AlertRules::DedupeSeconds))
                .col(string_null(AlertRules::SilencedUntil))
                .col(string_col(AlertRules::CreatedAt))
                .col(string_col(AlertRules::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_alert_events(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(AlertEvents::Table)
                .if_not_exists()
                .col(string_pk(AlertEvents::Id))
                .col(string_col(AlertEvents::RuleId))
                .col(string_col(AlertEvents::RuleName))
                .col(string_col(AlertEvents::Severity))
                .col(string_col(AlertEvents::Status))
                .col(string_col(AlertEvents::EventType))
                .col(string_col(AlertEvents::ResourceType))
                .col(string_col(AlertEvents::ResourceId))
                .col(string_null(AlertEvents::FailureClass))
                .col(string_null(AlertEvents::Message))
                .col(string_col(AlertEvents::DedupeKey))
                .col(string_col(AlertEvents::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_alert_delivery_attempts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(AlertDeliveryAttempts::Table)
                .if_not_exists()
                .col(string_pk(AlertDeliveryAttempts::Id))
                .col(string_col(AlertDeliveryAttempts::EventId))
                .col(string_col(AlertDeliveryAttempts::RuleId))
                .col(string_col(AlertDeliveryAttempts::Provider))
                .col(string_col(AlertDeliveryAttempts::Target))
                .col(boolean_col(AlertDeliveryAttempts::Delivered))
                .col(integer_null(AlertDeliveryAttempts::StatusCode))
                .col(text_null(AlertDeliveryAttempts::Error))
                .col(integer_col(AlertDeliveryAttempts::Attempt))
                .col(string_col(AlertDeliveryAttempts::RetryState))
                .col(string_null(AlertDeliveryAttempts::NextRetryAt))
                .col(string_col(AlertDeliveryAttempts::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_script_versions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ScriptVersions::Table)
                .if_not_exists()
                .col(string_pk(ScriptVersions::Id))
                .col(string_col(ScriptVersions::ScriptId))
                .col(big_integer_col(ScriptVersions::VersionNumber))
                .col(text_col(ScriptVersions::Content))
                .col(string_col(ScriptVersions::ContentSha256))
                .col(string_col(ScriptVersions::Language))
                .col(string_col(ScriptVersions::Status))
                .col(big_integer_null(ScriptVersions::TimeoutSeconds))
                .col(big_integer_null(ScriptVersions::MaxMemoryBytes))
                .col(boolean_col(ScriptVersions::AllowNetwork))
                .col(string_null(ScriptVersions::AllowedEnvVars))
                .col(string_null(ScriptVersions::PolicyJson))
                .col(string_col(ScriptVersions::CreatedBy))
                .col(string_col(ScriptVersions::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_workflow_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Workflows::Table)
                .if_not_exists()
                .col(string_pk(Workflows::Id))
                .col(string_col(Workflows::Name))
                .col(text_col(Workflows::Definition))
                .col(string_col(Workflows::Status))
                .col(string_col(Workflows::CreatedBy))
                .col(string_col(Workflows::CreatedAt))
                .col(string_col(Workflows::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(WorkflowNodes::Table)
                .if_not_exists()
                .col(string_pk(WorkflowNodes::Id))
                .col(string_col(WorkflowNodes::WorkflowId))
                .col(string_col(WorkflowNodes::NodeKey))
                .col(string_col(WorkflowNodes::Name))
                .col(string_col(WorkflowNodes::Kind))
                .col(string_null(WorkflowNodes::JobId))
                .col(string_null(WorkflowNodes::ProcessorName))
                .col(text_null(WorkflowNodes::Config))
                .col(string_col(WorkflowNodes::CreatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(WorkflowEdges::Table)
                .if_not_exists()
                .col(string_pk(WorkflowEdges::Id))
                .col(string_col(WorkflowEdges::WorkflowId))
                .col(string_col(WorkflowEdges::FromNodeKey))
                .col(string_col(WorkflowEdges::ToNodeKey))
                .col(text_col(WorkflowEdges::Condition))
                .col(string_col(WorkflowEdges::CreatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(WorkflowInstances::Table)
                .if_not_exists()
                .col(string_pk(WorkflowInstances::Id))
                .col(string_col(WorkflowInstances::WorkflowId))
                .col(string_col(WorkflowInstances::Status))
                .col(string_col(WorkflowInstances::TriggerType))
                .col(string_col(WorkflowInstances::CreatedAt))
                .col(string_col(WorkflowInstances::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(WorkflowNodeInstances::Table)
                .if_not_exists()
                .col(string_pk(WorkflowNodeInstances::Id))
                .col(string_col(WorkflowNodeInstances::WorkflowInstanceId))
                .col(string_col(WorkflowNodeInstances::NodeKey))
                .col(string_col(WorkflowNodeInstances::Status))
                .col(string_null(WorkflowNodeInstances::JobInstanceId))
                .col(string_null(WorkflowNodeInstances::ChildWorkflowInstanceId))
                .col(string_col(WorkflowNodeInstances::CreatedAt))
                .col(string_col(WorkflowNodeInstances::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_workflow_shards(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(WorkflowShards::Table)
                .if_not_exists()
                .col(string_pk(WorkflowShards::Id))
                .col(string_col(WorkflowShards::WorkflowInstanceId))
                .col(string_col(WorkflowShards::WorkflowNodeInstanceId))
                .col(string_col(WorkflowShards::NodeKey))
                .col(integer_col(WorkflowShards::ShardIndex))
                .col(string_col(WorkflowShards::Status))
                .col(text_col(WorkflowShards::Input))
                .col(text_null(WorkflowShards::Output))
                .col(text_null(WorkflowShards::Checkpoint))
                .col(integer_col(WorkflowShards::RetryCount))
                .col(string_null(WorkflowShards::JobInstanceId))
                .col(string_col(WorkflowShards::CreatedAt))
                .col(string_col(WorkflowShards::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_dispatch_queue(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(DispatchQueue::Table)
                .if_not_exists()
                .col(string_pk(DispatchQueue::Id))
                .col(string_null(DispatchQueue::JobInstanceId))
                .col(string_null(DispatchQueue::WorkflowNodeInstanceId))
                .col(integer_col(DispatchQueue::Priority))
                .col(string_col(DispatchQueue::RunAfter))
                .col(string_col(DispatchQueue::Status))
                .col(integer_col(DispatchQueue::Attempt))
                .col(string_null(DispatchQueue::LeaseOwner))
                .col(string_null(DispatchQueue::LeaseUntil))
                .col(string_null(DispatchQueue::FencingToken))
                .col(string_null(DispatchQueue::WorkerSelector))
                .col(string_null(DispatchQueue::Namespace))
                .col(string_null(DispatchQueue::App))
                .col(string_null(DispatchQueue::WorkerPool))
                .col(string_col(DispatchQueue::CreatedAt))
                .col(string_col(DispatchQueue::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_instance_events(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(InstanceEvents::Table)
                .if_not_exists()
                .col(string_pk(InstanceEvents::Id))
                .col(string_col(InstanceEvents::InstanceId))
                .col(string_col(InstanceEvents::InstanceType))
                .col(string_col(InstanceEvents::EventType))
                .col(string_col(InstanceEvents::Message))
                .col(text_null(InstanceEvents::Payload))
                .col(string_col(InstanceEvents::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_raft_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(RaftMetadata::Table)
                .if_not_exists()
                .col(string_pk(RaftMetadata::Id))
                .col(string_col(RaftMetadata::ClusterId))
                .col(string_col(RaftMetadata::NodeId))
                .col(big_integer_col(RaftMetadata::CurrentTerm))
                .col(string_null(RaftMetadata::VotedFor))
                .col(big_integer_col(RaftMetadata::CommitIndex))
                .col(big_integer_col(RaftMetadata::AppliedIndex))
                .col(string_null(RaftMetadata::LeaderFencingToken))
                .col(text_null(RaftMetadata::ConfState))
                .col(string_col(RaftMetadata::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RaftMembers::Table)
                .if_not_exists()
                .col(string_pk(RaftMembers::Id))
                .col(string_col(RaftMembers::NodeId))
                .col(string_col(RaftMembers::Endpoint))
                .col(string_col(RaftMembers::Status))
                .col(string_col(RaftMembers::CreatedAt))
                .col(string_col(RaftMembers::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RaftLogEntries::Table)
                .if_not_exists()
                .col(string_pk(RaftLogEntries::Id))
                .col(string_col(RaftLogEntries::ClusterId))
                .col(string_col(RaftLogEntries::NodeId))
                .col(big_integer_col(RaftLogEntries::LogIndex))
                .col(big_integer_col(RaftLogEntries::Term))
                .col(string_col(RaftLogEntries::EntryType))
                .col(text_col(RaftLogEntries::Data))
                .col(text_null(RaftLogEntries::Context))
                .col(string_col(RaftLogEntries::SyncStatus))
                .col(string_col(RaftLogEntries::CreatedAt))
                .col(string_col(RaftLogEntries::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RaftSnapshots::Table)
                .if_not_exists()
                .col(string_pk(RaftSnapshots::Id))
                .col(string_col(RaftSnapshots::ClusterId))
                .col(string_col(RaftSnapshots::NodeId))
                .col(big_integer_col(RaftSnapshots::SnapshotIndex))
                .col(big_integer_col(RaftSnapshots::Term))
                .col(text_null(RaftSnapshots::ConfState))
                .col(text_null(RaftSnapshots::Data))
                .col(string_col(RaftSnapshots::CreatedAt))
                .col(string_col(RaftSnapshots::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RaftMembershipProposals::Table)
                .if_not_exists()
                .col(string_pk(RaftMembershipProposals::Id))
                .col(string_col(RaftMembershipProposals::ClusterId))
                .col(string_col(RaftMembershipProposals::ProposalId))
                .col(string_col(RaftMembershipProposals::Action))
                .col(string_col(RaftMembershipProposals::NodeId))
                .col(string_null(RaftMembershipProposals::Endpoint))
                .col(string_col(RaftMembershipProposals::Status))
                .col(text_col(RaftMembershipProposals::Message))
                .col(string_col(RaftMembershipProposals::CreatedBy))
                .col(string_col(RaftMembershipProposals::CreatedAt))
                .col(string_col(RaftMembershipProposals::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn seed_rbac_defaults(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let now = now_rfc3339();
    for (id, name, description) in [
        (
            "role-owner",
            "owner",
            "Site owner and bootstrap recovery role",
        ),
        (
            "role-operator",
            "operator",
            "Operate tikeo jobs and instances",
        ),
        ("role-viewer", "viewer", "Read-only platform access"),
    ] {
        let insert = sea_query::Query::insert()
            .into_table(Roles::Table)
            .columns([Roles::Id, Roles::Name, Roles::Description, Roles::CreatedAt])
            .values_panic([
                id.into(),
                name.into(),
                description.into(),
                now.clone().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "roles", id, insert).await?;
    }

    for (id, resource, action, description) in DEFAULT_PERMISSIONS {
        let insert = sea_query::Query::insert()
            .into_table(Permissions::Table)
            .columns([
                Permissions::Id,
                Permissions::Resource,
                Permissions::Action,
                Permissions::Description,
                Permissions::CreatedAt,
            ])
            .values_panic([
                (*id).into(),
                (*resource).into(),
                (*action).into(),
                (*description).into(),
                now.clone().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "permissions", id, insert).await?;
    }

    let owner_permissions: Vec<&str> = DEFAULT_PERMISSIONS
        .iter()
        .map(|permission| permission.0)
        .collect();
    let operator_permissions = [
        "perm-jobs-read",
        "perm-jobs-write",
        "perm-instances-read",
        "perm-instances-execute",
        "perm-scripts-read",
        "perm-audit-read",
        "perm-audit-manage",
        "perm-workflows-read",
        "perm-workflows-execute",
        "perm-workers-read",
        "perm-notifications-read",
        "perm-notifications-manage",
        "perm-notifications-test",
    ];
    let viewer_permissions = [
        "perm-jobs-read",
        "perm-instances-read",
        "perm-scripts-read",
        "perm-workflows-read",
        "perm-workers-read",
        "perm-notifications-read",
    ];
    seed_role_permissions(manager, "role-owner", owner_permissions).await?;
    seed_role_permissions(manager, "role-operator", operator_permissions).await?;
    seed_role_permissions(manager, "role-viewer", viewer_permissions).await
}

async fn seed_role_permissions<'a>(
    manager: &SchemaManager<'_>,
    role_id: &str,
    permission_ids: impl IntoIterator<Item = &'a str>,
) -> Result<(), DbErr> {
    for permission_id in permission_ids {
        let binding_id = format!("rp-{role_id}-{permission_id}");
        let insert = sea_query::Query::insert()
            .into_table(RolePermissions::Table)
            .columns([
                RolePermissions::Id,
                RolePermissions::RoleId,
                RolePermissions::PermissionId,
                RolePermissions::CreatedAt,
            ])
            .values_panic([
                binding_id.clone().into(),
                role_id.into(),
                permission_id.into(),
                now_rfc3339().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "role_permissions", &binding_id, insert).await?;
    }
    Ok(())
}

async fn exec_seed_insert_if_missing(
    manager: &SchemaManager<'_>,
    table: &str,
    id: &str,
    insert: InsertStatement,
) -> Result<(), DbErr> {
    if seed_row_exists(manager, table, id).await? {
        return Ok(());
    }
    ignore_unique(manager.exec_stmt(insert).await)
}

async fn seed_row_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    id: &str,
) -> Result<bool, DbErr> {
    let escaped_id = id.replace('\'', "''");
    let sql = format!("SELECT id FROM {table} WHERE id = '{escaped_id}' LIMIT 1");
    let row = manager
        .get_connection()
        .query_one(Statement::from_string(manager.get_database_backend(), sql))
        .await?;
    Ok(row.is_some())
}

fn ignore_unique(result: Result<(), DbErr>) -> Result<(), DbErr> {
    match result {
        Ok(()) => Ok(()),
        Err(DbErr::Exec(error)) if is_unique_conflict(&error.to_string()) => Ok(()),
        Err(error) => Err(error),
    }
}

fn is_unique_conflict(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("unique")
        || normalized.contains("duplicate key")
        || normalized.contains("duplicate entry")
        || normalized.contains("already exists")
}

const DEFAULT_PERMISSIONS: &[(&str, &str, &str, &str)] = &[
    ("perm-system-read", "system", "read", "Read system metadata"),
    ("perm-cluster-read", "cluster", "read", "Read cluster state"),
    (
        "perm-cluster-manage",
        "cluster",
        "manage",
        "Manage cluster membership proposals",
    ),
    ("perm-users-read", "users", "read", "Read users"),
    ("perm-users-manage", "users", "manage", "Manage users"),
    (
        "perm-tenants-read",
        "tenants",
        "read",
        "Read tenants, apps, and worker pools",
    ),
    (
        "perm-tenants-manage",
        "tenants",
        "manage",
        "Manage tenants, apps, and worker pools",
    ),
    ("perm-jobs-read", "jobs", "read", "Read jobs"),
    ("perm-jobs-write", "jobs", "write", "Create and update jobs"),
    (
        "perm-instances-read",
        "instances",
        "read",
        "Read job instances",
    ),
    (
        "perm-instances-execute",
        "instances",
        "execute",
        "Trigger job instances",
    ),
    ("perm-scripts-read", "scripts", "read", "Read scripts"),
    ("perm-scripts-manage", "scripts", "manage", "Manage scripts"),
    ("perm-audit-read", "audit", "read", "Read audit logs"),
    (
        "perm-audit-manage",
        "audit",
        "manage",
        "Manage alert rules, alert recovery, and audit-governed operations",
    ),
    ("perm-workflows-read", "workflows", "read", "Read workflows"),
    (
        "perm-workflows-manage",
        "workflows",
        "manage",
        "Manage workflows",
    ),
    (
        "perm-workflows-execute",
        "workflows",
        "execute",
        "Run workflows",
    ),
    (
        "perm-workers-read",
        "workers",
        "read",
        "Read workers and queue state",
    ),
    (
        "perm-workers-manage",
        "workers",
        "manage",
        "Manage worker lifecycle operations",
    ),
    (
        "perm-notifications-read",
        "notifications",
        "read",
        "Read notification channels, policies, messages, and delivery state",
    ),
    (
        "perm-notifications-manage",
        "notifications",
        "manage",
        "Manage notification channels, policies, and provider readiness",
    ),
    (
        "perm-notifications-test",
        "notifications",
        "test",
        "Send notification channel test messages",
    ),
];

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

async fn create_worker_pools(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(WorkerPools::Table)
                .if_not_exists()
                .col(string_pk(WorkerPools::Id))
                .col(string_col(WorkerPools::NamespaceId))
                .col(string_col(WorkerPools::AppId))
                .col(string_col(WorkerPools::Name))
                .col(integer_col(WorkerPools::MaxQueueDepth))
                .col(integer_col(WorkerPools::MaxConcurrency))
                .col(string_col(WorkerPools::CreatedAt))
                .col(string_col(WorkerPools::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_worker_lifecycle_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(WorkerLogicalInstances::Table)
                .if_not_exists()
                .col(string_pk(WorkerLogicalInstances::Id))
                .col(short_string_col(WorkerLogicalInstances::NamespaceName))
                .col(short_string_col(WorkerLogicalInstances::AppName))
                .col(short_string_col(WorkerLogicalInstances::Cluster))
                .col(short_string_col(WorkerLogicalInstances::Region))
                .col(short_string_col(WorkerLogicalInstances::ClientInstanceId))
                .col(string_null(WorkerLogicalInstances::CurrentWorkerId))
                .col(big_integer_col(WorkerLogicalInstances::CurrentGeneration))
                .col(string_col(WorkerLogicalInstances::Status))
                .col(string_col(WorkerLogicalInstances::LastSeenAt))
                .col(string_col(WorkerLogicalInstances::CreatedAt))
                .col(string_col(WorkerLogicalInstances::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(WorkerSessions::Table)
                .if_not_exists()
                .col(string_pk(WorkerSessions::WorkerId))
                .col(string_col(WorkerSessions::LogicalInstanceId))
                .col(string_col(WorkerSessions::ConnectionId))
                .col(big_integer_col(WorkerSessions::Generation))
                .col(string_col(WorkerSessions::FencingTokenHash))
                .col(string_col(WorkerSessions::Status))
                .col(string_null(WorkerSessions::StatusReason))
                .col(text_null(WorkerSessions::StatusEvidence))
                .col(string_col(WorkerSessions::LeaseExpiresAt))
                .col(string_col(WorkerSessions::LastHeartbeatAt))
                .col(big_integer_col(WorkerSessions::LastSequence))
                .col(string_col(WorkerSessions::ConnectedAt))
                .col(string_null(WorkerSessions::DisconnectedAt))
                .col(string_null(WorkerSessions::ReplacedByWorkerId))
                .col(string_null(WorkerSessions::DrainRequestedAt))
                .col(text_col(WorkerSessions::CapabilitiesJson))
                .col(text_col(WorkerSessions::StructuredCapabilitiesJson))
                .col(text_col(WorkerSessions::LabelsJson))
                .col(text_col(WorkerSessions::MasterJson))
                .col(string_col(WorkerSessions::CreatedAt))
                .col(string_col(WorkerSessions::UpdatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(WorkerSessionEvents::Table)
                .if_not_exists()
                .col(string_pk(WorkerSessionEvents::Id))
                .col(string_col(WorkerSessionEvents::WorkerId))
                .col(string_col(WorkerSessionEvents::LogicalInstanceId))
                .col(string_col(WorkerSessionEvents::EventType))
                .col(string_null(WorkerSessionEvents::Reason))
                .col(text_null(WorkerSessionEvents::DetailJson))
                .col(string_col(WorkerSessionEvents::CreatedAt))
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
                .col(string_col(Jobs::MisfirePolicy))
                .col(string_null(Jobs::ScheduleStartAt))
                .col(string_null(Jobs::ScheduleEndAt))
                .col(text_null(Jobs::ScheduleCalendarJson))
                .col(string_null(Jobs::ProcessorName))
                .col(string_null(Jobs::ProcessorType))
                .col(string_null(Jobs::ScriptId))
                .col(boolean_col(Jobs::Enabled))
                .col(string_null(Jobs::CanaryJobId))
                .col(integer_col(Jobs::CanaryPercent))
                .col(text_col(Jobs::RetryPolicyJson))
                .col(string_col(Jobs::CreatedAt))
                .col(string_col(Jobs::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_job_versions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(JobVersions::Table)
                .if_not_exists()
                .col(string_pk(JobVersions::Id))
                .col(string_col(JobVersions::JobId))
                .col(big_integer_col(JobVersions::VersionNumber))
                .col(string_col(JobVersions::Name))
                .col(string_col(JobVersions::ScheduleType))
                .col(string_null(JobVersions::ScheduleExpr))
                .col(string_col(JobVersions::MisfirePolicy))
                .col(string_null(JobVersions::ScheduleStartAt))
                .col(string_null(JobVersions::ScheduleEndAt))
                .col(text_null(JobVersions::ScheduleCalendarJson))
                .col(string_null(JobVersions::ProcessorName))
                .col(string_null(JobVersions::ProcessorType))
                .col(string_null(JobVersions::ScriptId))
                .col(boolean_col(JobVersions::Enabled))
                .col(text_col(JobVersions::RetryPolicyJson))
                .col(string_col(JobVersions::CreatedBy))
                .col(string_col(JobVersions::ChangeReason))
                .col(big_integer_null(JobVersions::RolledBackFromVersion))
                .col(string_col(JobVersions::CreatedAt))
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
                .col(string_null(JobInstances::ResultWorkerId))
                .col(ColumnDef::new(JobInstances::ResultSuccess).boolean().null())
                .col(text_null(JobInstances::ResultMessage))
                .col(string_null(JobInstances::ResultCompletedAt))
                .col(string_col(JobInstances::CreatedAt))
                .col(string_col(JobInstances::UpdatedAt))
                .to_owned(),
        )
        .await
}

#[cfg(test)]
mod seed_error_tests {
    use super::is_unique_conflict;

    #[test]
    fn unique_conflict_detection_is_database_dialect_tolerant() {
        assert!(is_unique_conflict("UNIQUE constraint failed: roles.id"));
        assert!(is_unique_conflict(
            "duplicate key value violates unique constraint \"roles_pkey\""
        ));
        assert!(is_unique_conflict(
            "Duplicate entry 'role-owner' for key 'PRIMARY'"
        ));
        assert!(!is_unique_conflict("connection refused"));
    }
}
