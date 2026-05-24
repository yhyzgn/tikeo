//! Worker logical instance entity.

use sea_orm::entity::prelude::*;

/// Stable logical worker row grouping many ephemeral worker sessions.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "worker_logical_instances")]
pub struct Model {
    /// Logical instance identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Namespace name scope.
    pub namespace_name: String,
    /// Application name scope.
    pub app_name: String,
    /// Deployment cluster name.
    pub cluster: String,
    /// Deployment region.
    pub region: String,
    /// Client-provided stable instance hint.
    pub client_instance_id: String,
    /// Current authoritative worker session id, when any.
    pub current_worker_id: Option<String>,
    /// Latest known generation for this logical instance.
    pub current_generation: i64,
    /// Aggregated logical status.
    pub status: String,
    /// Last observed session or heartbeat time.
    pub last_seen_at: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
