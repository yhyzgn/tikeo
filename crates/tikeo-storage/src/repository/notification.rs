#![allow(missing_docs)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::entities::{
    notification_channel, notification_delivery_attempt, notification_message, notification_policy,
};

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone)]
pub struct CreateNotificationChannel {
    pub scope_type: String,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
    pub name: String,
    pub provider: String,
    pub enabled: bool,
    pub config_json: String,
    pub secret_refs_json: String,
    pub safety_policy_json: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateNotificationChannel {
    pub scope_type: Option<String>,
    pub namespace: Option<Option<String>>,
    pub app: Option<Option<String>>,
    pub worker_pool: Option<Option<String>>,
    pub name: Option<String>,
    pub provider: Option<String>,
    pub enabled: Option<bool>,
    pub config_json: Option<String>,
    pub secret_refs_json: Option<String>,
    pub safety_policy_json: Option<Option<String>>,
    pub updated_by: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationChannelFilters {
    pub scope_type: Option<String>,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
    pub provider: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannelSummary {
    pub id: String,
    pub scope_type: String,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
    pub name: String,
    pub provider: String,
    pub enabled: bool,
    pub config_json: String,
    #[serde(skip_serializing)]
    pub secret_refs_json: String,
    pub target_redacted: String,
    pub safety_policy_json: Option<String>,
    pub target_configured: bool,
    pub secret_configured: bool,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannelDeleteResult {
    pub deleted: bool,
    pub referenced_by_policies: u64,
}

#[derive(Debug, Clone)]
pub struct NotificationChannelDeliveryConfig {
    pub id: String,
    pub provider: String,
    pub enabled: bool,
    pub config_json: String,
    pub secret_refs_json: String,
    pub target_redacted: String,
    pub safety_policy_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateNotificationPolicy {
    pub owner_type: String,
    pub owner_id: Option<String>,
    pub name: String,
    pub event_family: String,
    pub event_filter_json: String,
    pub channel_refs_json: String,
    pub template_ref: Option<String>,
    pub severity: String,
    pub enabled: bool,
    pub dedupe_seconds: i64,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateNotificationPolicy {
    pub owner_type: Option<String>,
    pub owner_id: Option<Option<String>>,
    pub name: Option<String>,
    pub event_family: Option<String>,
    pub event_filter_json: Option<String>,
    pub channel_refs_json: Option<String>,
    pub template_ref: Option<Option<String>>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub dedupe_seconds: Option<i64>,
    pub throttle_json: Option<Option<String>>,
    pub quiet_hours_json: Option<Option<String>>,
    pub escalation_json: Option<Option<String>>,
    pub updated_by: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationPolicyFilters {
    pub owner_type: Option<String>,
    pub owner_id: Option<String>,
    pub event_family: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPolicySummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub owner_type: String,
    pub owner_id: Option<String>,
    pub event_family: String,
    pub event_filter_json: String,
    pub channel_refs_json: String,
    pub template_ref: Option<String>,
    pub severity: String,
    pub dedupe_seconds: i64,
    pub throttle_json: Option<String>,
    pub quiet_hours_json: Option<String>,
    pub escalation_json: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPolicyValidationSummary {
    pub policy_id: String,
    pub valid: bool,
    pub channel_count: u64,
    pub missing_channel_ids: Vec<String>,
    pub disabled_channel_ids: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateNotificationMessage {
    pub source_type: String,
    pub source_id: String,
    pub policy_id: String,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub severity: String,
    pub subject: String,
    pub body: String,
    pub payload_json: String,
    pub dedupe_key: String,
    pub trace_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationMessageFilters {
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub policy_id: Option<String>,
    pub event_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationMessageSummary {
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub policy_id: String,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub severity: String,
    pub subject: String,
    pub body: String,
    pub payload_json: String,
    pub dedupe_key: String,
    pub trace_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct RecordNotificationDeliveryAttempt {
    pub message_id: String,
    pub policy_id: String,
    pub channel_id: String,
    pub provider: String,
    pub target_redacted: String,
    pub attempt: i32,
    pub delivered: bool,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub retry_state: String,
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationDeliveryAttemptFilters {
    pub message_id: Option<String>,
    pub policy_id: Option<String>,
    pub channel_id: Option<String>,
    pub provider: Option<String>,
    pub retry_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryAttemptSummary {
    pub id: String,
    pub message_id: String,
    pub policy_id: String,
    pub channel_id: String,
    pub provider: String,
    pub target_redacted: String,
    pub attempt: i32,
    pub delivered: bool,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub retry_state: String,
    pub next_retry_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NotificationChannelRepository {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct NotificationPolicyRepository {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct NotificationMessageRepository {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct NotificationDeliveryAttemptRepository {
    db: DatabaseConnection,
}

impl NotificationChannelRepository {
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_channel(
        &self,
        input: CreateNotificationChannel,
    ) -> Result<NotificationChannelSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let target_redacted =
            target_redacted(&input.provider, &input.config_json, &input.secret_refs_json);
        let model = notification_channel::ActiveModel {
            id: Set(new_id("notification-channel")),
            scope_type: Set(input.scope_type),
            namespace: Set(input.namespace),
            app: Set(input.app),
            worker_pool: Set(input.worker_pool),
            name: Set(input.name),
            provider: Set(input.provider),
            enabled: Set(input.enabled),
            config_json: Set(input.config_json),
            secret_refs_json: Set(input.secret_refs_json),
            target_redacted: Set(target_redacted),
            safety_policy_json: Set(input.safety_policy_json),
            created_by: Set(None),
            updated_by: Set(None),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(NotificationChannelSummary::from(model))
    }

    pub async fn update_channel(
        &self,
        id: &str,
        input: UpdateNotificationChannel,
    ) -> Result<Option<NotificationChannelSummary>, sea_orm::DbErr> {
        let Some(row) = notification_channel::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let current_provider = row.provider.clone();
        let current_config_json = row.config_json.clone();
        let current_secret_refs_json = row.secret_refs_json.clone();
        let mut active: notification_channel::ActiveModel = row.into();
        let mut effective_provider = input.provider.clone().unwrap_or(current_provider);
        let provider_changed = input.provider.is_some();
        let mut effective_config_json = current_config_json;
        let mut effective_secret_refs_json = current_secret_refs_json;
        let mut target_changed = provider_changed;
        if let Some(value) = input.scope_type {
            active.scope_type = Set(value);
        }
        if let Some(value) = input.namespace {
            active.namespace = Set(value);
        }
        if let Some(value) = input.app {
            active.app = Set(value);
        }
        if let Some(value) = input.worker_pool {
            active.worker_pool = Set(value);
        }
        if let Some(value) = input.name {
            active.name = Set(value);
        }
        if let Some(value) = input.provider {
            effective_provider.clone_from(&value);
            active.provider = Set(value);
        }
        if let Some(value) = input.enabled {
            active.enabled = Set(value);
        }
        if let Some(value) = input.config_json {
            effective_config_json.clone_from(&value);
            active.config_json = Set(value);
            target_changed = true;
        }
        if let Some(value) = input.secret_refs_json {
            effective_secret_refs_json.clone_from(&value);
            active.secret_refs_json = Set(value);
            target_changed = true;
        }
        if target_changed {
            active.target_redacted = Set(target_redacted(
                &effective_provider,
                &effective_config_json,
                &effective_secret_refs_json,
            ));
        }
        if let Some(value) = input.safety_policy_json {
            active.safety_policy_json = Set(value);
        }
        if let Some(value) = input.updated_by {
            active.updated_by = Set(value);
        }
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(NotificationChannelSummary::from)
            .map(Some)
    }

    pub async fn get_channel(
        &self,
        id: &str,
    ) -> Result<Option<NotificationChannelSummary>, sea_orm::DbErr> {
        notification_channel::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(NotificationChannelSummary::from))
    }

    pub async fn get_channel_delivery_config(
        &self,
        id: &str,
    ) -> Result<Option<NotificationChannelDeliveryConfig>, sea_orm::DbErr> {
        notification_channel::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(NotificationChannelDeliveryConfig::from))
    }

    pub async fn list_channels(
        &self,
        filters: NotificationChannelFilters,
    ) -> Result<Vec<NotificationChannelSummary>, sea_orm::DbErr> {
        let mut query = notification_channel::Entity::find();
        if let Some(value) = filters.scope_type {
            query = query.filter(notification_channel::Column::ScopeType.eq(value));
        }
        if let Some(value) = filters.namespace {
            query = query.filter(notification_channel::Column::Namespace.eq(value));
        }
        if let Some(value) = filters.app {
            query = query.filter(notification_channel::Column::App.eq(value));
        }
        if let Some(value) = filters.worker_pool {
            query = query.filter(notification_channel::Column::WorkerPool.eq(value));
        }
        if let Some(value) = filters.provider {
            query = query.filter(notification_channel::Column::Provider.eq(value));
        }
        if let Some(value) = filters.enabled {
            query = query.filter(notification_channel::Column::Enabled.eq(value));
        }
        let rows = query
            .order_by_desc(notification_channel::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(NotificationChannelSummary::from)
            .collect())
    }

    pub async fn delete_channel(
        &self,
        id: &str,
    ) -> Result<NotificationChannelDeleteResult, sea_orm::DbErr> {
        let policies = NotificationPolicyRepository::new(self.db.clone())
            .list_policies(NotificationPolicyFilters::default())
            .await?;
        let referenced_by_policies = policies
            .iter()
            .filter(|policy| channel_refs_contain(&policy.channel_refs_json, id))
            .count() as u64;
        if referenced_by_policies > 0 {
            return Ok(NotificationChannelDeleteResult {
                deleted: false,
                referenced_by_policies,
            });
        }
        let result = notification_channel::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(NotificationChannelDeleteResult {
            deleted: result.rows_affected > 0,
            referenced_by_policies,
        })
    }
}

impl NotificationPolicyRepository {
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_policy(
        &self,
        input: CreateNotificationPolicy,
    ) -> Result<NotificationPolicySummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = notification_policy::ActiveModel {
            id: Set(new_id("notification-policy")),
            name: Set(input.name),
            enabled: Set(input.enabled),
            owner_type: Set(input.owner_type),
            owner_id: Set(input.owner_id),
            event_family: Set(input.event_family),
            event_filter_json: Set(input.event_filter_json),
            channel_refs_json: Set(input.channel_refs_json),
            template_ref: Set(input.template_ref),
            severity: Set(input.severity),
            dedupe_seconds: Set(input.dedupe_seconds),
            throttle_json: Set(None),
            quiet_hours_json: Set(None),
            escalation_json: Set(None),
            created_by: Set(None),
            updated_by: Set(None),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(NotificationPolicySummary::from(model))
    }

    pub async fn update_policy(
        &self,
        id: &str,
        input: UpdateNotificationPolicy,
    ) -> Result<Option<NotificationPolicySummary>, sea_orm::DbErr> {
        let Some(row) = notification_policy::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: notification_policy::ActiveModel = row.into();
        if let Some(value) = input.owner_type {
            active.owner_type = Set(value);
        }
        if let Some(value) = input.owner_id {
            active.owner_id = Set(value);
        }
        if let Some(value) = input.name {
            active.name = Set(value);
        }
        if let Some(value) = input.event_family {
            active.event_family = Set(value);
        }
        if let Some(value) = input.event_filter_json {
            active.event_filter_json = Set(value);
        }
        if let Some(value) = input.channel_refs_json {
            active.channel_refs_json = Set(value);
        }
        if let Some(value) = input.template_ref {
            active.template_ref = Set(value);
        }
        if let Some(value) = input.severity {
            active.severity = Set(value);
        }
        if let Some(value) = input.enabled {
            active.enabled = Set(value);
        }
        if let Some(value) = input.dedupe_seconds {
            active.dedupe_seconds = Set(value);
        }
        if let Some(value) = input.throttle_json {
            active.throttle_json = Set(value);
        }
        if let Some(value) = input.quiet_hours_json {
            active.quiet_hours_json = Set(value);
        }
        if let Some(value) = input.escalation_json {
            active.escalation_json = Set(value);
        }
        if let Some(value) = input.updated_by {
            active.updated_by = Set(value);
        }
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(NotificationPolicySummary::from)
            .map(Some)
    }

    pub async fn get_policy(
        &self,
        id: &str,
    ) -> Result<Option<NotificationPolicySummary>, sea_orm::DbErr> {
        notification_policy::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(NotificationPolicySummary::from))
    }

    pub async fn list_policies(
        &self,
        filters: NotificationPolicyFilters,
    ) -> Result<Vec<NotificationPolicySummary>, sea_orm::DbErr> {
        let mut query = notification_policy::Entity::find();
        if let Some(value) = filters.owner_type {
            query = query.filter(notification_policy::Column::OwnerType.eq(value));
        }
        if let Some(value) = filters.owner_id {
            query = query.filter(notification_policy::Column::OwnerId.eq(value));
        }
        if let Some(value) = filters.event_family {
            query = query.filter(notification_policy::Column::EventFamily.eq(value));
        }
        if let Some(value) = filters.enabled {
            query = query.filter(notification_policy::Column::Enabled.eq(value));
        }
        let rows = query
            .order_by_desc(notification_policy::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(NotificationPolicySummary::from)
            .collect())
    }

    pub async fn delete_policy(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = notification_policy::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn validate_policy(
        &self,
        id: &str,
    ) -> Result<Option<NotificationPolicyValidationSummary>, sea_orm::DbErr> {
        let Some(policy) = self.get_policy(id).await? else {
            return Ok(None);
        };
        let channels = NotificationChannelRepository::new(self.db.clone())
            .list_channels(NotificationChannelFilters::default())
            .await?;
        let channel_ids = extract_channel_refs(&policy.channel_refs_json);
        let mut missing_channel_ids = Vec::new();
        let mut disabled_channel_ids = Vec::new();
        for channel_id in &channel_ids {
            match channels.iter().find(|channel| &channel.id == channel_id) {
                Some(channel) if !channel.enabled => disabled_channel_ids.push(channel_id.clone()),
                Some(_) => {}
                None => missing_channel_ids.push(channel_id.clone()),
            }
        }
        let mut issues = Vec::new();
        if channel_ids.is_empty() {
            issues.push("policy must reference at least one notification channel".to_owned());
        }
        for channel_id in &missing_channel_ids {
            issues.push(format!("channel does not exist: {channel_id}"));
        }
        for channel_id in &disabled_channel_ids {
            issues.push(format!("channel is disabled: {channel_id}"));
        }
        Ok(Some(NotificationPolicyValidationSummary {
            policy_id: policy.id,
            valid: issues.is_empty(),
            channel_count: channel_ids.len() as u64,
            missing_channel_ids,
            disabled_channel_ids,
            issues,
        }))
    }
}

impl NotificationMessageRepository {
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_message(
        &self,
        input: CreateNotificationMessage,
    ) -> Result<NotificationMessageSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = notification_message::ActiveModel {
            id: Set(new_id("notification-message")),
            source_type: Set(input.source_type),
            source_id: Set(input.source_id),
            policy_id: Set(input.policy_id),
            event_type: Set(input.event_type),
            resource_type: Set(input.resource_type),
            resource_id: Set(input.resource_id),
            severity: Set(input.severity),
            subject: Set(input.subject),
            body: Set(input.body),
            payload_json: Set(input.payload_json),
            dedupe_key: Set(input.dedupe_key),
            trace_id: Set(input.trace_id),
            status: Set(input.status),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(NotificationMessageSummary::from(model))
    }

    pub async fn update_message_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<Option<NotificationMessageSummary>, sea_orm::DbErr> {
        let Some(row) = notification_message::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: notification_message::ActiveModel = row.into();
        active.status = Set(status.to_owned());
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(NotificationMessageSummary::from)
            .map(Some)
    }

    pub async fn latest_message_by_dedupe_key(
        &self,
        dedupe_key: &str,
    ) -> Result<Option<NotificationMessageSummary>, sea_orm::DbErr> {
        notification_message::Entity::find()
            .filter(notification_message::Column::DedupeKey.eq(dedupe_key.to_owned()))
            .order_by_desc(notification_message::Column::CreatedAt)
            .one(&self.db)
            .await
            .map(|row| row.map(NotificationMessageSummary::from))
    }

    pub async fn list_messages(
        &self,
        filters: NotificationMessageFilters,
    ) -> Result<Vec<NotificationMessageSummary>, sea_orm::DbErr> {
        let mut query = notification_message::Entity::find();
        if let Some(value) = filters.source_type {
            query = query.filter(notification_message::Column::SourceType.eq(value));
        }
        if let Some(value) = filters.source_id {
            query = query.filter(notification_message::Column::SourceId.eq(value));
        }
        if let Some(value) = filters.policy_id {
            query = query.filter(notification_message::Column::PolicyId.eq(value));
        }
        if let Some(value) = filters.event_type {
            query = query.filter(notification_message::Column::EventType.eq(value));
        }
        if let Some(value) = filters.severity {
            query = query.filter(notification_message::Column::Severity.eq(value));
        }
        if let Some(value) = filters.status {
            query = query.filter(notification_message::Column::Status.eq(value));
        }
        let rows = query
            .order_by_desc(notification_message::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(NotificationMessageSummary::from)
            .collect())
    }

    pub async fn get_message(
        &self,
        id: &str,
    ) -> Result<Option<NotificationMessageSummary>, sea_orm::DbErr> {
        notification_message::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(NotificationMessageSummary::from))
    }
}

impl NotificationDeliveryAttemptRepository {
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn record_attempt(
        &self,
        input: RecordNotificationDeliveryAttempt,
    ) -> Result<NotificationDeliveryAttemptSummary, sea_orm::DbErr> {
        let model = notification_delivery_attempt::ActiveModel {
            id: Set(new_id("notification-delivery")),
            message_id: Set(input.message_id),
            policy_id: Set(input.policy_id),
            channel_id: Set(input.channel_id),
            provider: Set(input.provider),
            target_redacted: Set(input.target_redacted),
            attempt: Set(input.attempt),
            delivered: Set(input.delivered),
            status_code: Set(input.status_code),
            error: Set(input.error),
            retry_state: Set(input.retry_state),
            next_retry_at: Set(input.next_retry_at),
            created_at: Set(now_rfc3339()),
        }
        .insert(&self.db)
        .await?;
        Ok(NotificationDeliveryAttemptSummary::from(model))
    }

    pub async fn list_due_attempts(
        &self,
        limit: u64,
    ) -> Result<Vec<NotificationDeliveryAttemptSummary>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let rows = notification_delivery_attempt::Entity::find()
            .filter(
                notification_delivery_attempt::Column::RetryState
                    .eq("retry_pending")
                    .or(notification_delivery_attempt::Column::RetryState.eq("pending")),
            )
            .filter(
                notification_delivery_attempt::Column::NextRetryAt
                    .is_null()
                    .or(notification_delivery_attempt::Column::NextRetryAt.lte(now)),
            )
            .order_by_asc(notification_delivery_attempt::Column::NextRetryAt)
            .order_by_asc(notification_delivery_attempt::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(NotificationDeliveryAttemptSummary::from)
            .collect())
    }

    pub async fn mark_attempt_retry_state(
        &self,
        id: &str,
        retry_state: &str,
        error: Option<&str>,
        next_retry_at: Option<&str>,
    ) -> Result<Option<NotificationDeliveryAttemptSummary>, sea_orm::DbErr> {
        let Some(row) = notification_delivery_attempt::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: notification_delivery_attempt::ActiveModel = row.into();
        active.retry_state = Set(retry_state.to_owned());
        if let Some(error) = error {
            active.error = Set(Some(error.to_owned()));
        }
        active.next_retry_at = Set(next_retry_at.map(ToOwned::to_owned));
        active
            .update(&self.db)
            .await
            .map(NotificationDeliveryAttemptSummary::from)
            .map(Some)
    }

    pub async fn list_attempts(
        &self,
        filters: NotificationDeliveryAttemptFilters,
    ) -> Result<Vec<NotificationDeliveryAttemptSummary>, sea_orm::DbErr> {
        let mut query = notification_delivery_attempt::Entity::find();
        if let Some(value) = filters.message_id {
            query = query.filter(notification_delivery_attempt::Column::MessageId.eq(value));
        }
        if let Some(value) = filters.policy_id {
            query = query.filter(notification_delivery_attempt::Column::PolicyId.eq(value));
        }
        if let Some(value) = filters.channel_id {
            query = query.filter(notification_delivery_attempt::Column::ChannelId.eq(value));
        }
        if let Some(value) = filters.provider {
            query = query.filter(notification_delivery_attempt::Column::Provider.eq(value));
        }
        if let Some(value) = filters.retry_state {
            query = query.filter(notification_delivery_attempt::Column::RetryState.eq(value));
        }
        let rows = query
            .order_by_desc(notification_delivery_attempt::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(NotificationDeliveryAttemptSummary::from)
            .collect())
    }
}

impl From<notification_channel::Model> for NotificationChannelSummary {
    fn from(value: notification_channel::Model) -> Self {
        let redacted_config = redact_config_json(&value.config_json);
        Self {
            target_configured: !value.target_redacted.is_empty()
                && value.target_redacted != "unconfigured",
            secret_configured: secret_refs_configured(&value.secret_refs_json)
                || secret_configured_in_redacted_config(&redacted_config),
            id: value.id,
            scope_type: value.scope_type,
            namespace: value.namespace,
            app: value.app,
            worker_pool: value.worker_pool,
            name: value.name,
            provider: value.provider,
            enabled: value.enabled,
            config_json: redacted_config,
            secret_refs_json: value.secret_refs_json,
            target_redacted: value.target_redacted,
            safety_policy_json: value.safety_policy_json,
            created_by: value.created_by,
            updated_by: value.updated_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<notification_channel::Model> for NotificationChannelDeliveryConfig {
    fn from(value: notification_channel::Model) -> Self {
        Self {
            id: value.id,
            provider: value.provider,
            enabled: value.enabled,
            config_json: value.config_json,
            secret_refs_json: value.secret_refs_json,
            target_redacted: value.target_redacted,
            safety_policy_json: value.safety_policy_json,
        }
    }
}

impl From<notification_policy::Model> for NotificationPolicySummary {
    fn from(value: notification_policy::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            enabled: value.enabled,
            owner_type: value.owner_type,
            owner_id: value.owner_id,
            event_family: value.event_family,
            event_filter_json: value.event_filter_json,
            channel_refs_json: value.channel_refs_json,
            template_ref: value.template_ref,
            severity: value.severity,
            dedupe_seconds: value.dedupe_seconds,
            throttle_json: value.throttle_json,
            quiet_hours_json: value.quiet_hours_json,
            escalation_json: value.escalation_json,
            created_by: value.created_by,
            updated_by: value.updated_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<notification_message::Model> for NotificationMessageSummary {
    fn from(value: notification_message::Model) -> Self {
        Self {
            id: value.id,
            source_type: value.source_type,
            source_id: value.source_id,
            policy_id: value.policy_id,
            event_type: value.event_type,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            severity: value.severity,
            subject: value.subject,
            body: value.body,
            payload_json: value.payload_json,
            dedupe_key: value.dedupe_key,
            trace_id: value.trace_id,
            status: value.status,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<notification_delivery_attempt::Model> for NotificationDeliveryAttemptSummary {
    fn from(value: notification_delivery_attempt::Model) -> Self {
        Self {
            id: value.id,
            message_id: value.message_id,
            policy_id: value.policy_id,
            channel_id: value.channel_id,
            provider: value.provider,
            target_redacted: value.target_redacted,
            attempt: value.attempt,
            delivered: value.delivered,
            status_code: value.status_code,
            error: value.error,
            retry_state: value.retry_state,
            next_retry_at: value.next_retry_at,
            created_at: value.created_at,
        }
    }
}

fn redact_config_json(raw: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(raw) else {
        return "{}".to_owned();
    };
    redact_value(&mut value, false);
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_owned())
}

fn redact_value(value: &mut Value, in_headers: bool) {
    match value {
        Value::Object(map) => {
            for (key, field) in map.iter_mut() {
                if in_headers {
                    redact_header_value(field);
                } else if sensitive_key(key) {
                    *field = Value::String("***redacted***".to_owned());
                } else if url_like_config_key(key) {
                    if let Some(raw_url) = field.as_str() {
                        *field = Value::String(redact_url_like(raw_url));
                    }
                } else {
                    redact_value(field, key.eq_ignore_ascii_case("headers"));
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item, in_headers);
            }
        }
        _ => {}
    }
}

fn redact_header_value(value: &mut Value) {
    match value {
        Value::String(_) => *value = Value::String("***redacted***".to_owned()),
        Value::Array(items) => {
            for item in items {
                redact_header_value(item);
            }
        }
        Value::Object(map) => {
            for field in map.values_mut() {
                redact_header_value(field);
            }
        }
        _ => {}
    }
}

fn sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("authorization")
        || key == "routing_key"
        || key == "routingkey"
        || key == "integration_key"
        || key == "integrationkey"
        || key == "signing_key"
        || key == "signingkey"
        || key == "smtp_url"
        || key == "smtpurl"
}

fn url_like_config_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
    matches!(normalized.as_str(), "url" | "webhookurl")
}

fn target_redacted(provider: &str, raw_config: &str, raw_secret_refs: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(raw_config) else {
        return "unconfigured".to_owned();
    };
    if provider == "email" {
        return ["to", "recipients"]
            .iter()
            .find_map(|key| value.get(key))
            .map_or_else(|| "unconfigured".to_owned(), redact_email_target);
    }
    ["url", "webhook_url", "webhookUrl"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .map(redact_url_like)
        .or_else(|| {
            let keys: &[&str] = if provider == "pagerduty" {
                &[
                    "target",
                    "routing_key",
                    "routingKey",
                    "integration_key",
                    "integrationKey",
                ]
            } else {
                &["target"]
            };
            keys.iter()
                .find_map(|key| value.get(*key).and_then(Value::as_str))
                .map(|_| format!("{provider}:***redacted***"))
        })
        .or_else(|| secret_ref_target_redacted(provider, raw_secret_refs))
        .unwrap_or_else(|| "unconfigured".to_owned())
}

fn secret_ref_target_redacted(provider: &str, raw_secret_refs: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw_secret_refs).ok()?;
    let object = value.as_object()?;
    let keys: &[&str] = if provider == "pagerduty" {
        &[
            "url",
            "webhook_url",
            "webhookUrl",
            "target",
            "routing_key",
            "routingKey",
            "integration_key",
            "integrationKey",
        ]
    } else {
        &["url", "webhook_url", "webhookUrl", "target"]
    };
    keys.iter()
        .any(|key| object.get(*key).is_some_and(value_has_present_field))
        .then(|| format!("{provider}:secret-ref"))
}

fn redact_email_target(value: &Value) -> String {
    match value {
        Value::String(item) => item.to_owned(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(","),
        _ => "email".to_owned(),
    }
}

fn redact_url_like(value: &str) -> String {
    url::Url::parse(value).map_or_else(
        |_| "invalid-url".to_owned(),
        |url| {
            let mut redacted = format!(
                "{}://{}",
                url.scheme(),
                url.host_str().unwrap_or("unknown-host")
            );
            if let Some(port) = url.port() {
                redacted.push(':');
                redacted.push_str(&port.to_string());
            }
            if url.path() != "/" && !url.path().is_empty() {
                redacted.push_str("/...");
            }
            redacted
        },
    )
}

fn secret_refs_configured(raw: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    value_has_present_field(&value)
}

fn secret_configured_in_redacted_config(raw: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    value_contains_redaction(&value)
}

fn value_has_present_field(value: &Value) -> bool {
    match value {
        Value::String(item) => !item.trim().is_empty(),
        Value::Array(items) => items.iter().any(value_has_present_field),
        Value::Object(map) => map.values().any(value_has_present_field),
        Value::Bool(value) => *value,
        Value::Number(_) => true,
        Value::Null => false,
    }
}

fn value_contains_redaction(value: &Value) -> bool {
    match value {
        Value::String(item) => item == "***redacted***",
        Value::Array(items) => items.iter().any(value_contains_redaction),
        Value::Object(map) => map.values().any(value_contains_redaction),
        _ => false,
    }
}

fn channel_refs_contain(raw: &str, channel_id: &str) -> bool {
    extract_channel_refs(raw).iter().any(|id| id == channel_id)
}

fn extract_channel_refs(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_str().map(ToOwned::to_owned).or_else(|| {
                    item.get("channelId")
                        .or_else(|| item.get("channel_id"))
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
            })
            .collect(),
        Value::Object(map) => map
            .get("channelId")
            .or_else(|| map.get("channel_id"))
            .and_then(Value::as_str)
            .map(|item| vec![item.to_owned()])
            .unwrap_or_default(),
        Value::String(item) => vec![item],
        _ => Vec::new(),
    }
}
