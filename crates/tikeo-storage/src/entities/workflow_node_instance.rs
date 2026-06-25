//! `SeaORM` entity definition for workflow node instances.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_node_instances")]
/// Workflow node instance persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Workflow instance identifier.
    pub workflow_instance_id: String,
    /// Workflow node key.
    pub node_key: String,
    /// Current status.
    pub status: String,
    /// Related job instance identifier.
    pub job_instance_id: Option<String>,
    /// Child workflow instance identifier.
    pub child_workflow_instance_id: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
