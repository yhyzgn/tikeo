/// Minimal job creation input.
#[derive(Debug, Clone)]
pub struct CreateJob {
    /// Namespace name. Defaults to `default` at HTTP boundary.
    pub namespace: String,
    /// Application name. Defaults to `default` at HTTP boundary.
    pub app: String,
    /// Job display name.
    pub name: String,
    /// Schedule type such as `api`, `cron`, `fixed_rate`.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Optional worker processor binding. When absent, dispatch falls back to job id.
    pub processor_name: Option<String>,
    /// Whether the job is enabled.
    pub enabled: bool,
}

/// Job summary returned to management API callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSummary {
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
    /// Optional worker processor binding.
    pub processor_name: Option<String>,
    /// Enabled flag.
    pub enabled: bool,
}
