//! Raft log entry entity for future `raft-rs` durable storage.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Per-node Raft log entry persisted without database foreign keys.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "raft_log_entries")]
pub struct Model {
    /// Log row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Logical cluster identifier.
    pub cluster_id: String,
    /// Stable tikeo node id owning this local log.
    pub node_id: String,
    /// Raft log index.
    pub log_index: i64,
    /// Raft term for this entry.
    pub term: i64,
    /// raft-rs entry type name, e.g. `EntryNormal` or `EntryConfChange`.
    pub entry_type: String,
    /// Base64-encoded entry payload bytes.
    pub data: String,
    /// Base64-encoded entry context bytes.
    pub context: Option<String>,
    /// Persistence status used by the future Ready pipeline.
    pub sync_status: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
