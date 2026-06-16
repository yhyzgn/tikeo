//! `SeaORM` entity definition for scheduler shard ownership projection rows.

use sea_orm::entity::prelude::*;

/// Raft-applied scheduler shard ownership projection.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "cluster_shard_ownership")]
pub struct Model {
    /// Scheduler shard id.
    #[sea_orm(primary_key, auto_increment = false)]
    pub shard_id: i32,
    /// Shard map version used to compute this shard id.
    pub shard_map_version: i64,
    /// Total shard count for this shard map version.
    pub shard_count: i32,
    /// Current owner node id.
    pub owner_node_id: String,
    /// Monotonic ownership epoch.
    pub epoch: i64,
    /// Raft term that produced this projection.
    pub raft_term: i64,
    /// Epoch-scoped fencing token.
    pub fencing_token: String,
    /// Ownership status: active/transferring/revoked.
    pub status: String,
    /// Optional lease hint for diagnostics only.
    pub lease_expires_at: Option<String>,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
