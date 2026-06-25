//! Worker session event entity.

use sea_orm::entity::prelude::*;

/// Append-only worker session lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "worker_session_events")]
pub struct Model {
    /// Event identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Worker session id.
    pub worker_id: String,
    /// Logical instance id at event time.
    pub logical_instance_id: String,
    /// Event type.
    pub event_type: String,
    /// Optional reason code.
    pub reason: Option<String>,
    /// Optional JSON event detail.
    pub detail_json: Option<String>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
