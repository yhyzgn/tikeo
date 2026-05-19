//! Namespace entity.

use sea_orm::entity::prelude::*;

/// Namespace row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "namespaces")]
pub struct Model {
    /// Namespace identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Unique namespace name.
    #[sea_orm(unique)]
    pub name: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Namespace relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Apps belonging to the namespace.
    #[sea_orm(has_many = "super::app::Entity")]
    App,
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
