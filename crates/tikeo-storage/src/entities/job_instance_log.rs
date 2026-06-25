//! Job instance log entity.

use sea_orm::entity::prelude::*;

/// Job instance log row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_instance_logs")]
pub struct Model {
    /// Log identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Parent instance identifier, soft-linked to `job_instances.id`.
    pub instance_id: String,
    /// Worker that emitted the log.
    pub worker_id: String,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Worker-local monotonic sequence.
    pub sequence: i64,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
