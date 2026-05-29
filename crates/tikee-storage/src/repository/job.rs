/// Minimal job creation input.
#[derive(Debug, Clone)]
pub struct CreateJob {
    /// Actor creating the initial immutable version snapshot.
    pub created_by: Option<String>,
    /// Namespace name. Defaults to `default` at HTTP boundary.
    pub namespace: String,
    /// Application name. Defaults to `default` at HTTP boundary.
    pub app: String,
    /// Job display name.
    pub name: String,
    /// Schedule type such as `api`, `cron`, `fixed_rate`; `api` means explicit API/SDK/UI trigger.
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
    /// Optional SDK worker processor binding. When absent, dispatch falls back to job name.
    pub processor_name: Option<String>,
    /// Optional custom plugin processor type.
    pub processor_type: Option<String>,
    /// Optional managed script binding. Mutually exclusive with `processor_name`.
    pub script_id: Option<String>,
    /// Whether the job is enabled.
    pub enabled: bool,
    /// Optional canary target job id for explicit trigger routing.
    pub canary_job_id: Option<String>,
    /// Canary traffic percentage in 0..=100.
    pub canary_percent: i32,
}

/// Minimal job update input. `None` leaves the field unchanged.
#[derive(Debug, Clone, Default)]
pub struct UpdateJob {
    /// Actor creating the update version snapshot.
    pub updated_by: Option<String>,
    /// Optional job display name.
    pub name: Option<String>,
    /// Optional schedule type.
    pub schedule_type: Option<String>,
    /// Optional schedule expression. Outer `None` leaves unchanged; inner `None` clears it.
    pub schedule_expr: Option<Option<String>>,
    /// Optional misfire policy.
    pub misfire_policy: Option<String>,
    /// Optional start timestamp update. Outer `None` leaves unchanged; inner `None` clears it.
    pub schedule_start_at: Option<Option<String>>,
    /// Optional end timestamp update. Outer `None` leaves unchanged; inner `None` clears it.
    pub schedule_end_at: Option<Option<String>>,
    /// Optional lifecycle calendar update. Outer `None` leaves unchanged; inner `None` clears it.
    pub schedule_calendar_json: Option<Option<String>>,
    /// Optional SDK worker processor binding. Outer `None` leaves unchanged; inner `None` clears it.
    pub processor_name: Option<Option<String>>,
    /// Optional custom plugin processor type.
    pub processor_type: Option<Option<String>>,
    /// Optional managed script binding. Outer `None` leaves unchanged; inner `None` clears it.
    pub script_id: Option<Option<String>>,
    /// Optional enabled flag.
    pub enabled: Option<bool>,
    /// Optional canary target update. Outer `None` leaves unchanged; inner `None` clears it.
    pub canary_job_id: Option<Option<String>>,
    /// Optional canary percentage update.
    pub canary_percent: Option<i32>,
}

/// Job summary returned to management API callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSummary {
    /// Latest immutable version number.
    pub version_number: i64,
    /// Job identifier.
    pub id: String,
    /// Namespace name.
    pub namespace: String,
    /// Application name.
    pub app: String,
    /// Job display name.
    pub name: String,
    /// Schedule type.
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
    /// Optional custom plugin processor type.
    pub processor_type: Option<String>,
    /// Optional managed script binding.
    pub script_id: Option<String>,
    /// Enabled flag.
    pub enabled: bool,
    /// Optional canary target job id for explicit trigger routing.
    pub canary_job_id: Option<String>,
    /// Canary traffic percentage in 0..=100.
    pub canary_percent: i32,
}
