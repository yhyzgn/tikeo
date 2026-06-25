//! Generic notification delivery attempt entity.

use sea_orm::entity::prelude::*;

/// One provider attempt for one normalized notification message.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_delivery_attempts")]
pub struct Model {
    /// Stable attempt identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Message soft-link id.
    pub message_id: String,
    /// Policy soft-link id.
    pub policy_id: String,
    /// Channel soft-link id.
    pub channel_id: String,
    /// Provider key.
    pub provider: String,
    /// Redacted target display value.
    pub target_redacted: String,
    /// Attempt number.
    pub attempt: i32,
    /// Whether the provider accepted the attempt.
    pub delivered: bool,
    /// HTTP status when available.
    pub status_code: Option<i32>,
    /// Error message when delivery failed.
    pub error: Option<String>,
    /// `delivered`, `retry_pending`, `retry_consumed`, `failed_permanent`, or `dead_letter`.
    pub retry_state: String,
    /// Optional next retry timestamp.
    pub next_retry_at: Option<String>,
    /// RFC3339 creation timestamp.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
