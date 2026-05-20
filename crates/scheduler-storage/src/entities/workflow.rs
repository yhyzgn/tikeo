#![allow(missing_docs)]

//! `SeaORM` entity definition for workflow definitions.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Workflow definition row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub definition: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
