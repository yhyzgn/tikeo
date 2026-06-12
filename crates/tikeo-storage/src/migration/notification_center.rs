use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::{
    DbErr, IntoIden, MigrationName, MigrationTrait, SchemaManager, Table, async_trait, sea_query,
};
use sea_query::Index;

use super::{
    NotificationChannels, NotificationDeliveryAttempts, NotificationMessages, NotificationPolicies,
    NotificationTemplates, Permissions, RoleMenuPermissions, big_integer_col, boolean_col,
    exec_seed_insert_if_missing, integer_col, integer_null, now_rfc3339, seed_role_permissions,
    string_col, string_null, string_pk, text_col, text_null,
};

pub(super) struct NotificationCenterMigration;

pub(super) struct NotificationTemplatesMigration;

pub(super) struct NotificationChannelExamplesMigration;

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

impl MigrationName for NotificationTemplatesMigration {
    fn name(&self) -> &'static str {
        "m20260611_000002_notification_templates"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for NotificationTemplatesMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_notification_templates(manager).await?;
        create_notification_template_indexes(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(NotificationTemplates::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

impl MigrationName for NotificationChannelExamplesMigration {
    fn name(&self) -> &'static str {
        "m20260612_000001_notification_channel_examples"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for NotificationChannelExamplesMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        seed_notification_channel_examples(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        delete_seed_notification_channel_examples(manager).await
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

async fn create_notification_templates(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(NotificationTemplates::Table)
                .if_not_exists()
                .col(string_pk(NotificationTemplates::Id))
                .col(string_col(NotificationTemplates::TemplateKey))
                .col(string_col(NotificationTemplates::Name))
                .col(text_null(NotificationTemplates::Description))
                .col(string_col(NotificationTemplates::Provider))
                .col(string_col(NotificationTemplates::MessageType))
                .col(boolean_col(NotificationTemplates::Enabled))
                .col(text_col(NotificationTemplates::BodyJson))
                .col(text_col(NotificationTemplates::VariablesJson))
                .col(string_null(NotificationTemplates::CreatedBy))
                .col(string_null(NotificationTemplates::UpdatedBy))
                .col(string_col(NotificationTemplates::CreatedAt))
                .col(string_col(NotificationTemplates::UpdatedAt))
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

async fn create_notification_template_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("idx_notification_templates_key")
                .table(NotificationTemplates::Table)
                .col(NotificationTemplates::TemplateKey)
                .if_not_exists()
                .unique()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_notification_templates_provider")
                .table(NotificationTemplates::Table)
                .col(NotificationTemplates::Provider)
                .col(NotificationTemplates::MessageType)
                .if_not_exists()
                .to_owned(),
        )
        .await
}

async fn seed_notification_channel_examples(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let now = now_rfc3339();
    for &(provider, message_type) in NOTIFICATION_CHANNEL_EXAMPLE_TYPES {
        let id = notification_channel_example_id(provider, message_type);
        let config = notification_channel_example_config(provider, message_type).to_string();
        let secret_refs = notification_channel_example_secret_refs(provider).to_string();
        let target_redacted = notification_channel_example_target_redacted(provider);
        let insert = sea_query::Query::insert()
            .into_table(NotificationChannels::Table)
            .columns([
                NotificationChannels::Id,
                NotificationChannels::ScopeType,
                NotificationChannels::Namespace,
                NotificationChannels::App,
                NotificationChannels::WorkerPool,
                NotificationChannels::Name,
                NotificationChannels::Provider,
                NotificationChannels::Enabled,
                NotificationChannels::ConfigJson,
                NotificationChannels::SecretRefsJson,
                NotificationChannels::TargetRedacted,
                NotificationChannels::SafetyPolicyJson,
                NotificationChannels::CreatedBy,
                NotificationChannels::UpdatedBy,
                NotificationChannels::CreatedAt,
                NotificationChannels::UpdatedAt,
            ])
            .values_panic([
                id.clone().into(),
                "global".into(),
                Option::<String>::None.into(),
                Option::<String>::None.into(),
                Option::<String>::None.into(),
                format!("{provider} {message_type} smoke channel").into(),
                provider.into(),
                false.into(),
                config.into(),
                secret_refs.into(),
                target_redacted.into(),
                Option::<String>::None.into(),
                Some("system".to_owned()).into(),
                Some("system".to_owned()).into(),
                now.clone().into(),
                now.clone().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "notification_channels", &id, insert).await?;
    }
    Ok(())
}

async fn delete_seed_notification_channel_examples(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    for &(provider, message_type) in NOTIFICATION_CHANNEL_EXAMPLE_TYPES {
        let id = notification_channel_example_id(provider, message_type).replace('\'', "''");
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                format!("DELETE FROM notification_channels WHERE id = '{id}'"),
            ))
            .await?;
    }
    Ok(())
}

fn notification_channel_example_id(provider: &str, message_type: &str) -> String {
    let suffix = message_type
        .chars()
        .flat_map(char::to_lowercase)
        .map(|item| {
            if item.is_ascii_alphanumeric() {
                item
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("notification-channel-example-{provider}-{suffix}")
}

fn notification_channel_example_config(provider: &str, message_type: &str) -> serde_json::Value {
    let template = notification_channel_example_template(provider, message_type);
    match provider {
        "dingtalk" => serde_json::json!({
            "messageType": message_type,
            "isAtAll": false,
            "atMobiles": [],
            "atUserIds": [],
            "template": template
        }),
        "wechat_work" => serde_json::json!({
            "messageType": message_type,
            "mentionedList": [],
            "mentionedMobileList": [],
            "template": template
        }),
        "pagerduty" => serde_json::json!({
            "messageType": message_type,
            "source": "tikeo",
            "severity": "critical",
            "dedupKey": "{{dedupeKey}}",
            "component": "{{resourceType}}",
            "class": "{{eventType}}",
            "client": "tikeo",
            "clientUrl": "https://tikeo.example.com/instances/{{resourceId}}",
            "template": template
        }),
        "email" => serde_json::json!({
            "messageType": message_type,
            "to": ["ops@example.com"],
            "from": "tikeo@example.com",
            "template": template
        }),
        _ => serde_json::json!({
            "messageType": message_type,
            "template": template
        }),
    }
}

fn notification_channel_example_secret_refs(provider: &str) -> serde_json::Value {
    match provider {
        "slack" => serde_json::json!({ "url": "env:SLACK_WEBHOOK_URL" }),
        "dingtalk" => serde_json::json!({
            "url": "env:DINGTALK_WEBHOOK_URL",
            "signingKey": "env:DINGTALK_SIGNING_SECRET"
        }),
        "feishu" => serde_json::json!({
            "url": "env:FEISHU_WEBHOOK_URL",
            "signingKey": "env:FEISHU_BOT_SECRET"
        }),
        "wechat_work" => serde_json::json!({ "url": "env:WECOM_WEBHOOK_URL" }),
        "pagerduty" => serde_json::json!({ "routingKey": "env:PAGERDUTY_ROUTING_KEY" }),
        "email" => serde_json::json!({
            "smtpUrl": "env:TIKEO_SMTP_URL",
            "password": "env:TIKEO_SMTP_PASSWORD"
        }),
        _ => serde_json::json!({
            "url": "env:TIKEO_NOTIFICATION_WEBHOOK_URL",
            "authorization": "env:TIKEO_NOTIFICATION_AUTHORIZATION"
        }),
    }
}

fn notification_channel_example_target_redacted(provider: &str) -> String {
    if provider == "email" {
        "ops@example.com".to_owned()
    } else if provider == "pagerduty" {
        "pagerduty:secret-ref".to_owned()
    } else {
        format!("{provider}:secret-ref")
    }
}

fn notification_channel_example_template(provider: &str, message_type: &str) -> serde_json::Value {
    match (provider, message_type) {
        ("slack", "blockKit") => serde_json::json!({
            "messageType": message_type,
            "text": "[tikeo] {{subject}}",
            "blocks": [
                { "type": "section", "text": { "type": "mrkdwn", "text": "*{{subject}}*\n{{body}}" } }
            ]
        }),
        ("slack", "attachments") => serde_json::json!({
            "messageType": message_type,
            "text": "[tikeo] {{subject}}",
            "attachments": [
                { "color": "#439FE0", "title": "{{subject}}", "text": "{{body}}" }
            ]
        }),
        ("slack", _) => serde_json::json!({
            "messageType": "text",
            "text": "[tikeo/{{severity}}] {{subject}}\n{{body}}"
        }),
        ("dingtalk", "markdown") => serde_json::json!({
            "messageType": message_type,
            "title": "{{subject}}",
            "text": "### {{subject}}\n\n{{body}}"
        }),
        ("dingtalk", "link") => serde_json::json!({
            "messageType": message_type,
            "title": "{{subject}}",
            "text": "{{body}}",
            "messageUrl": "https://tikeo.example.com/instances/{{resourceId}}",
            "picUrl": "https://tikeo.example.com/logo.png"
        }),
        ("dingtalk", "actionCard") => serde_json::json!({
            "messageType": message_type,
            "title": "{{subject}}",
            "text": "### {{subject}}\n\n{{body}}",
            "singleTitle": "Open Tikeo",
            "singleURL": "https://tikeo.example.com/instances/{{resourceId}}"
        }),
        ("dingtalk", "feedCard") => serde_json::json!({
            "messageType": message_type,
            "links": [
                {
                    "title": "{{subject}}",
                    "messageURL": "https://tikeo.example.com/instances/{{resourceId}}",
                    "picURL": "https://tikeo.example.com/logo.png"
                }
            ]
        }),
        ("dingtalk", _) => serde_json::json!({
            "messageType": "text",
            "content": "{{subject}}\n{{body}}"
        }),
        ("feishu", "post") => serde_json::json!({
            "messageType": message_type,
            "title": "{{subject}}",
            "content": [[{ "tag": "text", "text": "{{body}}" }]]
        }),
        ("feishu", "image") => serde_json::json!({
            "messageType": message_type,
            "imageKey": "img_v3_example_key"
        }),
        ("feishu", "share_chat") => serde_json::json!({
            "messageType": message_type,
            "shareChatId": "oc_example_chat_id"
        }),
        ("feishu", "interactive") => serde_json::json!({
            "messageType": message_type,
            "card": {
                "header": { "title": { "tag": "plain_text", "content": "{{subject}}" } },
                "elements": [
                    { "tag": "div", "text": { "tag": "lark_md", "content": "{{body}}" } }
                ]
            }
        }),
        ("feishu", _) => serde_json::json!({
            "messageType": "text",
            "text": "{{subject}}\n{{body}}"
        }),
        ("wechat_work", "markdown") => serde_json::json!({
            "messageType": message_type,
            "content": "### {{subject}}\n{{body}}"
        }),
        ("wechat_work", "markdown_v2") => serde_json::json!({
            "messageType": message_type,
            "content": "# {{subject}}\n{{body}}"
        }),
        ("wechat_work", "image") => serde_json::json!({
            "messageType": message_type,
            "base64": "iVBORw0KGgo=",
            "md5": "d41d8cd98f00b204e9800998ecf8427e"
        }),
        ("wechat_work", "news") => serde_json::json!({
            "messageType": message_type,
            "articles": [
                {
                    "title": "{{subject}}",
                    "description": "{{body}}",
                    "url": "https://tikeo.example.com/instances/{{resourceId}}"
                }
            ]
        }),
        ("wechat_work", "file" | "voice") => serde_json::json!({
            "messageType": message_type,
            "media_id": "MEDIA_ID_FROM_WECOM_UPLOAD"
        }),
        ("wechat_work", "template_card") => serde_json::json!({
            "messageType": message_type,
            "templateCard": {
                "card_type": "text_notice",
                "main_title": { "title": "{{subject}}", "desc": "{{body}}" },
                "card_action": {
                    "type": 1,
                    "url": "https://tikeo.example.com/instances/{{resourceId}}"
                }
            }
        }),
        ("wechat_work", _) => serde_json::json!({
            "messageType": "text",
            "content": "{{subject}}\n{{body}}"
        }),
        ("pagerduty", "acknowledge" | "resolve") => serde_json::json!({
            "messageType": message_type,
            "dedupKey": "{{dedupeKey}}",
            "customDetails": {
                "eventType": "{{eventType}}",
                "resourceId": "{{resourceId}}"
            }
        }),
        ("pagerduty", _) => serde_json::json!({
            "messageType": "trigger",
            "summary": "{{subject}}",
            "customDetails": {
                "body": "{{body}}",
                "eventType": "{{eventType}}"
            }
        }),
        ("email", "html") => serde_json::json!({
            "messageType": message_type,
            "subject": "[tikeo/{{severity}}] {{subject}}",
            "html": "<h1>{{subject}}</h1><p>{{body}}</p>",
            "body": "{{body}}"
        }),
        ("email", _) => serde_json::json!({
            "messageType": "plain",
            "subject": "[tikeo/{{severity}}] {{subject}}",
            "body": "{{body}}\n\nResource: {{resourceType}}/{{resourceId}}"
        }),
        ("webhook", _) => serde_json::json!({
            "messageType": "json",
            "body": {
                "text": "{{subject}}",
                "body": "{{body}}",
                "eventType": "{{eventType}}",
                "resourceId": "{{resourceId}}"
            }
        }),
        _ => serde_json::json!({
            "messageType": message_type,
            "text": "{{subject}}\n{{body}}"
        }),
    }
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

const NOTIFICATION_CHANNEL_EXAMPLE_TYPES: &[(&str, &str)] = &[
    ("webhook", "json"),
    ("slack", "text"),
    ("slack", "blockKit"),
    ("slack", "attachments"),
    ("dingtalk", "text"),
    ("dingtalk", "markdown"),
    ("dingtalk", "link"),
    ("dingtalk", "actionCard"),
    ("dingtalk", "feedCard"),
    ("feishu", "text"),
    ("feishu", "post"),
    ("feishu", "image"),
    ("feishu", "share_chat"),
    ("feishu", "interactive"),
    ("wechat_work", "text"),
    ("wechat_work", "markdown"),
    ("wechat_work", "markdown_v2"),
    ("wechat_work", "image"),
    ("wechat_work", "news"),
    ("wechat_work", "file"),
    ("wechat_work", "voice"),
    ("wechat_work", "template_card"),
    ("pagerduty", "trigger"),
    ("pagerduty", "acknowledge"),
    ("pagerduty", "resolve"),
    ("email", "plain"),
    ("email", "html"),
];
