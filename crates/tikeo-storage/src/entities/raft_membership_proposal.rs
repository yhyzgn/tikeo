//! Raft membership proposal entity for gated dynamic membership changes.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted Raft membership proposal intent.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "raft_membership_proposals")]
pub struct Model {
    /// Proposal row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Logical cluster identifier.
    pub cluster_id: String,
    /// Client-provided proposal idempotency key.
    pub proposal_id: String,
    /// Membership action, for example `add_voter` or `remove_voter`.
    pub action: String,
    /// Target tikeo node id.
    pub node_id: String,
    /// Target peer endpoint for add/update proposals.
    pub endpoint: Option<String>,
    /// Proposal status.
    pub status: String,
    /// Human-readable proposal result.
    pub message: String,
    /// Authenticated actor that created the proposal.
    pub created_by: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
