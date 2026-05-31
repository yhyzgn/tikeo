//! `SeaORM` entity definition for app-scoped secret references.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted app-scoped secret reference metadata. Secret plaintext is never stored.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "secrets")]
pub struct Model {
    /// Unique secret id.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Namespace scope.
    pub namespace: String,
    /// App scope.
    pub app: String,
    /// Secret logical name unique within namespace/app.
    pub name: String,
    /// External value reference such as `env:APP_SECRET`.
    pub value_ref: String,
    /// Secret status: active or deleted.
    pub status: String,
    /// Actor who created the reference.
    pub created_by: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by names.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
