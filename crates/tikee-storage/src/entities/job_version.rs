//! Job version history entity.

use sea_orm::entity::prelude::*;

/// Immutable snapshot of a job definition at a point in time.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job_versions")]
pub struct Model {
    /// Version record identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Soft-linked to `jobs.id`.
    pub job_id: String,
    /// Monotonically increasing job version number.
    pub version_number: i64,
    /// Snapshot of display name.
    pub name: String,
    /// Snapshot of schedule type.
    pub schedule_type: String,
    /// Snapshot of optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Snapshot of misfire policy.
    pub misfire_policy: String,
    /// Snapshot of optional inclusive schedule start timestamp.
    pub schedule_start_at: Option<String>,
    /// Snapshot of optional exclusive schedule end timestamp.
    pub schedule_end_at: Option<String>,
    /// Snapshot of optional lifecycle calendar JSON with maintenance/freeze windows and excluded dates.
    pub schedule_calendar_json: Option<String>,
    /// Snapshot of optional SDK processor binding.
    pub processor_name: Option<String>,
    /// Optional custom plugin processor type for capability resolution.
    pub processor_type: Option<String>,
    /// Snapshot of optional managed script binding.
    pub script_id: Option<String>,
    /// Snapshot of enabled flag.
    pub enabled: bool,
    /// Snapshot of structured failure retry policy JSON.
    pub retry_policy_json: String,
    /// Actor that created the snapshot.
    pub created_by: String,
    /// Creation reason such as `create`, `update`, or `rollback`.
    pub change_reason: String,
    /// Source version number when this snapshot was created by rollback.
    pub rolled_back_from_version: Option<i64>,
    /// Timestamp in RFC3339 format.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
