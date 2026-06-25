//! `SeaORM` entity definition for workflow edges.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_edges")]
/// Workflow edge persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Workflow identifier.
    pub workflow_id: String,
    /// Source workflow node key.
    pub from_node_key: String,
    /// Target workflow node key.
    pub to_node_key: String,
    /// Edge condition expression.
    pub condition: String,
    /// Creation timestamp.
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
