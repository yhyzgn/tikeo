//! Job instance log entity.

use sea_orm::entity::prelude::*;

/// Job instance log row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_instance_logs")]
pub struct Model {
    /// Log identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Parent instance identifier.
    pub instance_id: String,
    /// Worker that emitted the log.
    pub worker_id: String,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Worker-local monotonic sequence.
    pub sequence: i64,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Job instance log relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Parent job instance.
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
