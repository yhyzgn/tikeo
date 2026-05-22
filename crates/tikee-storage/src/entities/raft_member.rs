//! Raft member entity for configured server nodes.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Configured Raft member endpoint.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "raft_members")]
pub struct Model {
    /// Member row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Stable member node id.
    pub node_id: String,
    /// Peer endpoint reachable through Docker bridge / K8s Service / LB networking.
    pub endpoint: String,
    /// Member lifecycle status, for example `configured`, `active`, `removed`.
    pub status: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
