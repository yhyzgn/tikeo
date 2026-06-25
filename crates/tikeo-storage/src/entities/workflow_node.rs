//! `SeaORM` entity definition for workflow nodes.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_nodes")]
/// Workflow node persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Workflow identifier.
    pub workflow_id: String,
    /// Workflow node key.
    pub node_key: String,
    /// Display name.
    pub name: String,
    /// Workflow node kind.
    pub kind: String,
    /// Related job identifier.
    pub job_id: Option<String>,
    /// Worker processor name.
    pub processor_name: Option<String>,
    /// Serialized node configuration.
    pub config: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
