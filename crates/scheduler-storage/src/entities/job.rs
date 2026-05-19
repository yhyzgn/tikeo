//! Job entity.

use sea_orm::entity::prelude::*;

/// Job row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    /// Job identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Owning namespace identifier.
    pub namespace_id: String,
    /// Owning application identifier.
    pub app_id: String,
    /// Display name unique within app.
    pub name: String,
    /// Schedule type, for example `api`, `cron`, `fixed_rate`.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Enabled flag.
    pub enabled: bool,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Parent namespace.
    #[sea_orm(
        belongs_to = "super::namespace::Entity",
        from = "Column::NamespaceId",
        to = "super::namespace::Column::Id"
    )]
    Namespace,
    /// Parent application.
    #[sea_orm(
        belongs_to = "super::app::Entity",
        from = "Column::AppId",
        to = "super::app::Column::Id"
    )]
    App,
    /// Job instances.
    #[sea_orm(has_many = "super::job_instance::Entity")]
    JobInstance,
}

impl Related<super::namespace::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Namespace.def()
    }
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<super::job_instance::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobInstance.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
