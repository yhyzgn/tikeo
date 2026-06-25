//! Persistent schedule trigger cursor and idempotency rows.

use sea_orm::entity::prelude::*;

/// One successfully claimed automatic schedule fire window.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "schedule_cursors")]
pub struct Model {
    /// Cursor row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Scheduled job id.
    pub job_id: String,
    /// Trigger source, for example `cron` or `fixed_rate`.
    pub trigger_type: String,
    /// Logical scheduled fire timestamp/window in RFC3339 UTC.
    pub fire_at: String,
    /// Job instance created for this fire window.
    pub instance_id: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
