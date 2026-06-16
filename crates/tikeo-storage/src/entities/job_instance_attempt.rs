//! Job instance attempt entity.

use sea_orm::entity::prelude::*;

/// Per-worker execution row for broadcast instances.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_instance_attempts")]
pub struct Model {
    /// Attempt identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Parent instance identifier, soft-linked to `job_instances.id`.
    pub instance_id: String,
    /// Target worker identifier.
    pub worker_id: String,
    /// Current attempt status.
    pub status: String,
    /// Persisted assignment token issued by the scheduling owner for Worker logs/results.
    pub assignment_token: Option<String>,
    /// Whether the worker-reported result succeeded.
    pub result_success: Option<bool>,
    /// Worker-reported result message.
    pub result_message: Option<String>,
    /// Result completion timestamp.
    pub result_completed_at: Option<String>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
