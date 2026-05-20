//! Job instance entity.

use sea_orm::entity::prelude::*;

/// Job instance row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_instances")]
pub struct Model {
    /// Instance identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Parent job identifier, soft-linked to `jobs.id`.
    pub job_id: String,
    /// Current instance status.
    pub status: String,
    /// Trigger source.
    pub trigger_type: String,
    /// Execution mode, for example `single` or `broadcast`.
    pub execution_mode: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
