//! `SeaORM` entity definition for workflow shard rows.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_shards")]
/// Workflow shard persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Workflow instance identifier.
    pub workflow_instance_id: String,
    /// Related workflow node instance identifier.
    pub workflow_node_instance_id: String,
    /// Workflow node key.
    pub node_key: String,
    /// Workflow shard index.
    pub shard_index: i32,
    /// Current status.
    pub status: String,
    /// Serialized shard input.
    pub input: String,
    /// Serialized shard output.
    pub output: Option<String>,
    /// Serialized shard checkpoint.
    pub checkpoint: Option<String>,
    /// Shard retry count.
    pub retry_count: i32,
    /// Related job instance identifier.
    pub job_instance_id: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
