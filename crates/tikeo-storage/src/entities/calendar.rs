//! `SeaORM` entity definition for centralized schedule calendars.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "calendars")]
/// Centralized schedule calendar scoped to namespace/app.
pub struct Model {
    /// Unique calendar id.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Namespace scope.
    pub namespace: String,
    /// App scope.
    pub app: String,
    /// Calendar logical name unique within namespace/app.
    pub name: String,
    /// IANA timezone used by date-only exclusions.
    pub timezone: String,
    /// JSON array of date-only exclusions.
    pub excluded_dates_json: String,
    /// JSON array of holiday dates.
    pub holidays_json: String,
    /// JSON array of maintenance windows.
    pub maintenance_windows_json: String,
    /// JSON array of freeze windows.
    pub freeze_windows_json: String,
    /// Actor who created the calendar.
    pub created_by: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Database-level foreign keys are forbidden; relationships are soft-linked by scope/name.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
