//! Raft snapshot entity for future `raft-rs` durable storage.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Per-node Raft snapshot metadata/payload pointer persisted without database foreign keys.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "raft_snapshots")]
pub struct Model {
    /// Snapshot row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Logical cluster identifier.
    pub cluster_id: String,
    /// Stable scheduler node id owning this local snapshot.
    pub node_id: String,
    /// Snapshot index.
    pub snapshot_index: i64,
    /// Snapshot term.
    pub term: i64,
    /// Base64-encoded `ConfState` bytes or JSON marker for future migration.
    pub conf_state: Option<String>,
    /// Base64-encoded snapshot payload or object-store pointer.
    pub data: Option<String>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
