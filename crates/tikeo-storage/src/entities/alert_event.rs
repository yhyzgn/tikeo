//! Alert event history entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "alert_events")]
/// Alert event persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Associated alert rule identifier.
    pub rule_id: String,
    /// Alert rule display name.
    pub rule_name: String,
    /// Alert severity.
    pub severity: String,
    /// Current status.
    pub status: String,
    /// Event type.
    pub event_type: String,
    /// Affected resource type.
    pub resource_type: String,
    /// Affected resource identifier.
    pub resource_id: String,
    /// Optional failure classification.
    pub failure_class: Option<String>,
    /// Human-readable message.
    pub message: Option<String>,
    /// Alert de-duplication key.
    pub dedupe_key: String,
    /// Creation timestamp.
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
