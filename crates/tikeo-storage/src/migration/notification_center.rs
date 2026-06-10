use sea_orm_migration::prelude::{
    DbErr, IntoIden, MigrationName, MigrationTrait, SchemaManager, Table, async_trait, sea_query,
};
use sea_query::Index;

use super::{
    NotificationChannels, NotificationDeliveryAttempts, NotificationMessages, NotificationPolicies,
    Permissions, RoleMenuPermissions, big_integer_col, boolean_col, exec_seed_insert_if_missing,
    integer_col, integer_null, now_rfc3339, seed_role_permissions, string_col, string_null,
    string_pk, text_col, text_null,
};

pub(super) struct NotificationCenterMigration;

impl MigrationName for NotificationCenterMigration {
    fn name(&self) -> &'static str {
        "m20260611_000001_notification_center"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for NotificationCenterMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_notification_channels(manager).await?;
        create_notification_policies(manager).await?;
        create_notification_messages(manager).await?;
        create_notification_delivery_attempts(manager).await?;
        create_notification_indexes(manager).await?;
        seed_notification_permissions(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            NotificationDeliveryAttempts::Table.into_iden(),
            NotificationMessages::Table.into_iden(),
            NotificationPolicies::Table.into_iden(),
            NotificationChannels::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }
}

async fn create_notification_channels(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(NotificationChannels::Table)
                .if_not_exists()
                .col(string_pk(NotificationChannels::Id))
                .col(string_col(NotificationChannels::ScopeType))
                .col(string_null(NotificationChannels::Namespace))
                .col(string_null(NotificationChannels::App))
                .col(string_null(NotificationChannels::WorkerPool))
                .col(string_col(NotificationChannels::Name))
                .col(string_col(NotificationChannels::Provider))
                .col(boolean_col(NotificationChannels::Enabled))
                .col(text_col(NotificationChannels::ConfigJson))
                .col(text_col(NotificationChannels::SecretRefsJson))
                .col(string_col(NotificationChannels::TargetRedacted))
                .col(text_null(NotificationChannels::SafetyPolicyJson))
                .col(string_null(NotificationChannels::CreatedBy))
                .col(string_null(NotificationChannels::UpdatedBy))
                .col(string_col(NotificationChannels::CreatedAt))
                .col(string_col(NotificationChannels::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_notification_policies(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(NotificationPolicies::Table)
                .if_not_exists()
                .col(string_pk(NotificationPolicies::Id))
                .col(string_col(NotificationPolicies::Name))
                .col(boolean_col(NotificationPolicies::Enabled))
                .col(string_col(NotificationPolicies::OwnerType))
                .col(string_null(NotificationPolicies::OwnerId))
                .col(string_col(NotificationPolicies::EventFamily))
                .col(text_col(NotificationPolicies::EventFilterJson))
                .col(text_col(NotificationPolicies::ChannelRefsJson))
                .col(string_null(NotificationPolicies::TemplateRef))
                .col(string_col(NotificationPolicies::Severity))
                .col(big_integer_col(NotificationPolicies::DedupeSeconds))
                .col(text_null(NotificationPolicies::ThrottleJson))
                .col(text_null(NotificationPolicies::QuietHoursJson))
                .col(text_null(NotificationPolicies::EscalationJson))
                .col(string_null(NotificationPolicies::CreatedBy))
                .col(string_null(NotificationPolicies::UpdatedBy))
                .col(string_col(NotificationPolicies::CreatedAt))
                .col(string_col(NotificationPolicies::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_notification_messages(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(NotificationMessages::Table)
                .if_not_exists()
                .col(string_pk(NotificationMessages::Id))
                .col(string_col(NotificationMessages::SourceType))
                .col(string_col(NotificationMessages::SourceId))
                .col(string_col(NotificationMessages::PolicyId))
                .col(string_col(NotificationMessages::EventType))
                .col(string_col(NotificationMessages::ResourceType))
                .col(string_col(NotificationMessages::ResourceId))
                .col(string_col(NotificationMessages::Severity))
                .col(string_col(NotificationMessages::Subject))
                .col(text_col(NotificationMessages::Body))
                .col(text_col(NotificationMessages::PayloadJson))
                .col(string_col(NotificationMessages::DedupeKey))
                .col(string_null(NotificationMessages::TraceId))
                .col(string_col(NotificationMessages::Status))
                .col(string_col(NotificationMessages::CreatedAt))
                .col(string_col(NotificationMessages::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_notification_delivery_attempts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(NotificationDeliveryAttempts::Table)
                .if_not_exists()
                .col(string_pk(NotificationDeliveryAttempts::Id))
                .col(string_col(NotificationDeliveryAttempts::MessageId))
                .col(string_col(NotificationDeliveryAttempts::PolicyId))
                .col(string_col(NotificationDeliveryAttempts::ChannelId))
                .col(string_col(NotificationDeliveryAttempts::Provider))
                .col(string_col(NotificationDeliveryAttempts::TargetRedacted))
                .col(integer_col(NotificationDeliveryAttempts::Attempt))
                .col(boolean_col(NotificationDeliveryAttempts::Delivered))
                .col(integer_null(NotificationDeliveryAttempts::StatusCode))
                .col(text_null(NotificationDeliveryAttempts::Error))
                .col(string_col(NotificationDeliveryAttempts::RetryState))
                .col(string_null(NotificationDeliveryAttempts::NextRetryAt))
                .col(string_col(NotificationDeliveryAttempts::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_notification_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("idx_notification_channels_scope_name")
                .table(NotificationChannels::Table)
                .col(NotificationChannels::ScopeType)
                .col(NotificationChannels::Namespace)
                .col(NotificationChannels::App)
                .col(NotificationChannels::WorkerPool)
                .col(NotificationChannels::Name)
                .if_not_exists()
                .unique()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_notification_policies_owner")
                .table(NotificationPolicies::Table)
                .col(NotificationPolicies::OwnerType)
                .col(NotificationPolicies::OwnerId)
                .if_not_exists()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_notification_messages_status")
                .table(NotificationMessages::Table)
                .col(NotificationMessages::Status)
                .col(NotificationMessages::CreatedAt)
                .if_not_exists()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_notification_delivery_attempts_retry")
                .table(NotificationDeliveryAttempts::Table)
                .col(NotificationDeliveryAttempts::RetryState)
                .col(NotificationDeliveryAttempts::NextRetryAt)
                .if_not_exists()
                .to_owned(),
        )
        .await
}

async fn seed_notification_permissions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let now = now_rfc3339();
    for (id, resource, action, description) in RBAC_BACKFILL_PERMISSIONS {
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
    seed_role_permissions(
        manager,
        "role-owner",
        [
            "perm-audit-manage",
            "perm-notifications-read",
            "perm-notifications-manage",
            "perm-notifications-test",
        ],
    )
    .await?;
    seed_role_permissions(
        manager,
        "role-operator",
        [
            "perm-audit-read",
            "perm-audit-manage",
            "perm-notifications-read",
            "perm-notifications-manage",
            "perm-notifications-test",
        ],
    )
    .await?;
    seed_role_permissions(manager, "role-viewer", ["perm-notifications-read"]).await?;
    seed_notification_menu_permissions(manager).await
}

async fn seed_notification_menu_permissions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for role_id in ["role-owner", "role-operator", "role-viewer"] {
        let binding_id = format!("rmp-{role_id}-_notifications");
        let insert = sea_query::Query::insert()
            .into_table(RoleMenuPermissions::Table)
            .columns([
                RoleMenuPermissions::Id,
                RoleMenuPermissions::RoleId,
                RoleMenuPermissions::MenuKey,
                RoleMenuPermissions::CreatedAt,
            ])
            .values_panic([
                binding_id.clone().into(),
                role_id.into(),
                "/notifications".into(),
                now_rfc3339().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "role_menu_permissions", &binding_id, insert).await?;
    }
    Ok(())
}

const RBAC_BACKFILL_PERMISSIONS: &[(&str, &str, &str, &str)] = &[
    (
        "perm-audit-manage",
        "audit",
        "manage",
        "Manage alert rules, alert recovery, and audit-governed operations",
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
