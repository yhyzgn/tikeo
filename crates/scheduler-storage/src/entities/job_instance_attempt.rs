//! Job instance attempt entity.

use sea_orm::entity::prelude::*;

/// Per-worker execution row for broadcast instances.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_instance_attempts")]
pub struct Model {
    /// Attempt identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Parent instance identifier.
    pub instance_id: String,
    /// Target worker identifier.
    pub worker_id: String,
    /// Current attempt status.
    pub status: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance attempt relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Parent instance.
    #[sea_orm(
        belongs_to = "super::job_instance::Entity",
        from = "Column::InstanceId",
        to = "super::job_instance::Column::Id"
    )]
    JobInstance,
}

impl Related<super::job_instance::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobInstance.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
