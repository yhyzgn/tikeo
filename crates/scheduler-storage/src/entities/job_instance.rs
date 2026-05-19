//! Job instance entity.

use sea_orm::entity::prelude::*;

/// Job instance row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_instances")]
pub struct Model {
    /// Instance identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Parent job identifier.
    pub job_id: String,
    /// Current instance status.
    pub status: String,
    /// Trigger source.
    pub trigger_type: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Parent job.
    #[sea_orm(
        belongs_to = "super::job::Entity",
        from = "Column::JobId",
        to = "super::job::Column::Id"
    )]
    Job,
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
