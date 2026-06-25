//! Job entity.

use sea_orm::entity::prelude::*;

/// Job row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    /// Job identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Owning namespace identifier, soft-linked to `namespaces.id`.
    pub namespace_id: String,
    /// Owning application identifier, soft-linked to `apps.id`.
    pub app_id: String,
    /// Display name unique within app.
    pub name: String,
    /// Schedule type, for example `api`, `cron`, `fixed_rate`.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Misfire policy for automatic schedules.
    pub misfire_policy: String,
    /// Optional inclusive schedule start timestamp.
    pub schedule_start_at: Option<String>,
    /// Optional exclusive schedule end timestamp.
    pub schedule_end_at: Option<String>,
    /// Optional lifecycle calendar JSON with maintenance/freeze windows and excluded dates.
    pub schedule_calendar_json: Option<String>,
    /// Optional SDK worker processor binding.
    pub processor_name: Option<String>,
    /// Optional custom plugin processor type for capability resolution.
    pub processor_type: Option<String>,
    /// Optional managed script binding.
    pub script_id: Option<String>,
    /// Enabled flag.
    pub enabled: bool,
    /// Optional canary target job id for explicit trigger routing.
    pub canary_job_id: Option<String>,
    /// Canary traffic percentage in 0..=100.
    pub canary_percent: i32,
    /// Canary metrics gate and rollback policy JSON.
    pub canary_policy_json: String,
    /// Structured failure retry policy JSON.
    pub retry_policy_json: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
