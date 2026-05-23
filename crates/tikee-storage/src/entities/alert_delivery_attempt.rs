//! Alert delivery attempt entity.

use sea_orm::entity::prelude::*;

/// One provider delivery attempt for an alert event.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "alert_delivery_attempts")]
pub struct Model {
    /// Attempt identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Soft-linked alert event id.
    pub event_id: String,
    /// Soft-linked alert rule id.
    pub rule_id: String,
    /// Provider name.
    pub provider: String,
    /// Redacted provider target.
    pub target: String,
    /// Whether the provider accepted the attempt.
    pub delivered: bool,
    /// HTTP status when available.
    pub status_code: Option<i32>,
    /// Error message when delivery failed.
    pub error: Option<String>,
    /// Attempt number for this event/provider/target.
    pub attempt: i32,
    /// Retry state: `delivered`, `retry_pending`, or `failed_permanent`.
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
