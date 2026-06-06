#![allow(missing_docs)]

//! `SeaORM` entity definition for workflow shard rows.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_shards")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_instance_id: String,
    pub workflow_node_instance_id: String,
    pub node_key: String,
    pub shard_index: i32,
    pub status: String,
    pub input: String,
    pub output: Option<String>,
    pub checkpoint: Option<String>,
    pub retry_count: i32,
    pub job_instance_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
