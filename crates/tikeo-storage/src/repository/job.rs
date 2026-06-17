use serde::{Deserialize, Serialize};

/// Structured failure retry strategy for job execution.
///
/// `max_attempts` counts the initial execution plus retries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobRetryPolicy {
    /// Whether failed runtime execution should be retried.
    pub enabled: bool,
    /// Total attempts including the first execution.
    pub max_attempts: i32,
    /// Delay before the first retry.
    pub initial_delay_seconds: i64,
    /// Integer exponential backoff multiplier.
    pub backoff_multiplier: i32,
    /// Upper cap for any retry delay.
    pub max_delay_seconds: i64,
}

impl Default for JobRetryPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            initial_delay_seconds: 5,
            backoff_multiplier: 2,
            max_delay_seconds: 60,
        }
    }
}

impl JobRetryPolicy {
    /// Return a normalized policy with production-safe bounds.
    #[must_use]
    pub fn normalized(self) -> Self {
        let initial_delay_seconds = self.initial_delay_seconds.clamp(0, 86_400);
        let max_delay_seconds = self.max_delay_seconds.clamp(initial_delay_seconds, 86_400);
        Self {
            enabled: self.enabled,
            max_attempts: self.max_attempts.clamp(1, 10),
            initial_delay_seconds,
            backoff_multiplier: self.backoff_multiplier.clamp(1, 10),
            max_delay_seconds,
        }
    }

    /// Parse persisted JSON or return the default policy.
    #[must_use]
    pub fn from_json(value: Option<&str>) -> Self {
        value
            .and_then(|raw| serde_json::from_str::<Self>(raw).ok())
            .unwrap_or_default()
            .normalized()
    }

    /// Serialize normalized policy JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.clone().normalized()).unwrap_or_else(|_| Self::default_json())
    }

    /// Default policy JSON used by migrations.
    #[must_use]
    pub fn default_json() -> String {
        serde_json::to_string(&Self::default()).unwrap_or_else(|_| {
            r#"{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}"#
                .to_owned()
        })
    }

    /// Whether another retry may be scheduled after `completed_attempt` failed.
    #[must_use]
    pub fn allows_retry_after_attempt(&self, completed_attempt: i32) -> bool {
        let policy = self.clone().normalized();
        policy.enabled && completed_attempt > 0 && completed_attempt < policy.max_attempts
    }

    /// Compute retry delay after `completed_attempt` failed.
    #[must_use]
    pub fn delay_after_attempt_seconds(&self, completed_attempt: i32) -> i64 {
        let policy = self.clone().normalized();
        let exponent = completed_attempt
            .saturating_sub(1)
            .clamp(0, 9)
            .cast_unsigned();
        let multiplier = i64::from(policy.backoff_multiplier).saturating_pow(exponent);
        policy
            .initial_delay_seconds
            .saturating_mul(multiplier)
            .min(policy.max_delay_seconds)
    }
}

/// Canary metrics gate and rollback policy for one stable job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobCanaryPolicy {
    /// Whether trigger-time canary metrics gates are evaluated.
    pub metrics_gate_enabled: bool,
    /// Minimum terminal canary samples required before the gate may decide.
    pub minimum_samples: u64,
    /// Number of most recent canary instances inspected.
    pub evaluation_window: u64,
    /// Maximum allowed failure rate from 0.0 to 1.0.
    pub max_failure_rate: f64,
    /// Whether a failing gate should automatically set canary traffic to 0%.
    pub auto_rollback: bool,
}

impl Default for JobCanaryPolicy {
    fn default() -> Self {
        Self {
            metrics_gate_enabled: false,
            minimum_samples: 5,
            evaluation_window: 20,
            max_failure_rate: 0.5,
            auto_rollback: true,
        }
    }
}

impl JobCanaryPolicy {
    /// Return a policy bounded for production-safe trigger-time evaluation.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            metrics_gate_enabled: self.metrics_gate_enabled,
            minimum_samples: self.minimum_samples.clamp(1, 1_000),
            evaluation_window: self.evaluation_window.clamp(1, 10_000),
            max_failure_rate: self.max_failure_rate.clamp(0.0, 1.0),
            auto_rollback: self.auto_rollback,
        }
    }

    /// Parse persisted JSON or return the default policy.
    #[must_use]
    pub fn from_json(value: Option<&str>) -> Self {
        value
            .and_then(|raw| serde_json::from_str::<Self>(raw).ok())
            .unwrap_or_default()
            .normalized()
    }

    /// Serialize normalized policy JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.clone().normalized()).unwrap_or_else(|_| Self::default_json())
    }

    /// Default policy JSON used by migrations.
    #[must_use]
    pub fn default_json() -> String {
        serde_json::to_string(&Self::default()).unwrap_or_else(|_| {
            r#"{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}"#
                .to_owned()
        })
    }
}

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
    /// Canary metrics gate and rollback policy.
    pub canary_policy: Option<JobCanaryPolicy>,
    /// Failure retry policy.
    pub retry_policy: Option<JobRetryPolicy>,
}

/// Minimal job update input. `None` leaves the field unchanged.
#[derive(Debug, Clone, Default)]
pub struct UpdateJob {
    /// Actor creating the update version snapshot.
    pub updated_by: Option<String>,
    /// Optional namespace move. `None` leaves the current namespace unchanged.
    pub namespace: Option<String>,
    /// Optional application move within the target/current namespace. `None` preserves app name.
    pub app: Option<String>,
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
    /// Optional canary policy update.
    pub canary_policy: Option<JobCanaryPolicy>,
    /// Optional failure retry policy update.
    pub retry_policy: Option<JobRetryPolicy>,
}

/// Job summary returned to management API callers.
#[derive(Debug, Clone, PartialEq)]
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
    /// Canary metrics gate and rollback policy.
    pub canary_policy: JobCanaryPolicy,
    /// Failure retry policy.
    pub retry_policy: JobRetryPolicy,
}
