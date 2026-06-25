//! Alert rule entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "alert_rules")]
/// Alert rule persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Alert severity.
    pub severity: String,
    /// Serialized alert condition.
    pub condition_json: String,
    /// Serialized alert channel configuration.
    pub channels_json: String,
    /// Whether the record is enabled.
    pub enabled: bool,
    /// De-duplication window in seconds.
    pub dedupe_seconds: i64,
    /// Optional silence expiration timestamp.
    pub silenced_until: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
