//! Application entity.

use sea_orm::entity::prelude::*;

/// Application row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "apps")]
pub struct Model {
    /// Application identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Owning namespace identifier.
    pub namespace_id: String,
    /// Application name unique within a namespace.
    pub name: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Application relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Parent namespace.
    #[sea_orm(
        belongs_to = "super::namespace::Entity",
        from = "Column::NamespaceId",
        to = "super::namespace::Column::Id"
    )]
    Namespace,
    /// Jobs belonging to the application.
    #[sea_orm(has_many = "super::job::Entity")]
    Job,
}

impl Related<super::namespace::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Namespace.def()
    }
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
