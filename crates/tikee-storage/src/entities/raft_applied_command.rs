//! Raft applied command entity for idempotent state-machine bookkeeping.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Per-node applied Raft command record.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "raft_applied_commands")]
pub struct Model {
    /// Applied command row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Logical cluster identifier.
    pub cluster_id: String,
    /// Stable local node id.
    pub node_id: String,
    /// Raft log index that carried the command.
    pub log_index: i64,
    /// Raft term that carried the command.
    pub term: i64,
    /// Client-provided command id for replay idempotency.
    pub command_id: String,
    /// Command type, e.g. `noop`.
    pub command_type: String,
    /// JSON/base64 payload captured by the state-machine layer.
    pub payload: Option<String>,
    /// Apply status, e.g. `applied` or `rejected`.
    pub status: String,
    /// Human-readable apply result.
    pub message: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
