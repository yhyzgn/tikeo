//! Automatic schedule tick loop for CRON and fixed-rate jobs.

use std::{collections::HashMap, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use cron::Schedule;
use scheduler_core::{ExecutionMode, ScheduleType, TriggerType};
use scheduler_storage::{CreateJobInstance, JobInstanceRepository, JobRepository, JobSummary};
use tokio::sync::Mutex;
use tracing::warn;

const TICK_INTERVAL: Duration = Duration::from_secs(1);

/// Shared in-memory schedule cursor state.
#[derive(Debug, Default)]
pub struct ScheduleState {
    last_triggered_at: Mutex<HashMap<String, DateTime<Utc>>>,
}

/// Run the automatic scheduler tick loop forever.
pub async fn run_tick_loop(jobs: JobRepository, instances: JobInstanceRepository) {
    let state = ScheduleState::default();
    let mut ticker = tokio::time::interval(TICK_INTERVAL);

    loop {
        ticker.tick().await;
        if let Err(error) = tick_once(&jobs, &instances, &state, Utc::now()).await {
            warn!(%error, "schedule tick iteration failed");
        }
    }
}

async fn tick_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    state: &ScheduleState,
    now: DateTime<Utc>,
) -> Result<(), scheduler_storage::DbErr> {
    for job in jobs.list_enabled_scheduled_jobs().await? {
        let Ok(Some(trigger_type)) = due_trigger(&job, state, now).await else {
            continue;
        };

        instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type,
                execution_mode: ExecutionMode::Single,
            })
            .await?;
        state.last_triggered_at.lock().await.insert(job.id, now);
    }

    Ok(())
}

async fn due_trigger(
    job: &JobSummary,
    state: &ScheduleState,
    now: DateTime<Utc>,
) -> Result<Option<TriggerType>, ScheduleDecisionError> {
    let schedule_type = ScheduleType::from_str(&job.schedule_type)
        .map_err(|error| ScheduleDecisionError::InvalidScheduleType(error.to_string()))?;

    match schedule_type {
        ScheduleType::Cron => cron_due(job, state, now).await,
        ScheduleType::FixedRate => fixed_rate_due(job, state, now).await,
        ScheduleType::Api | ScheduleType::FixedDelay => Ok(None),
    }
}

async fn cron_due(
    job: &JobSummary,
    state: &ScheduleState,
    now: DateTime<Utc>,
) -> Result<Option<TriggerType>, ScheduleDecisionError> {
    let Some(expression) = non_empty_expr(job) else {
        return Ok(None);
    };
    let schedule = Schedule::from_str(expression)
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))?;
    let previous = state
        .last_triggered_at
        .lock()
        .await
        .get(&job.id)
        .copied()
        .unwrap_or_else(|| now - chrono::Duration::seconds(1));
    let due = schedule
        .after(&previous)
        .next()
        .is_some_and(|next| next <= now);

    Ok(due.then_some(TriggerType::Cron))
}

async fn fixed_rate_due(
    job: &JobSummary,
    state: &ScheduleState,
    now: DateTime<Utc>,
) -> Result<Option<TriggerType>, ScheduleDecisionError> {
    let Some(expression) = non_empty_expr(job) else {
        return Ok(None);
    };
    let duration = humantime::parse_duration(expression)
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))?;
    let Ok(rate) = chrono::Duration::from_std(duration) else {
        return Ok(None);
    };
    let previous = state.last_triggered_at.lock().await.get(&job.id).copied();
    let due = previous.is_none_or(|last| now.signed_duration_since(last) >= rate);

    Ok(due.then_some(TriggerType::FixedRate))
}

fn non_empty_expr(job: &JobSummary) -> Option<&str> {
    job.schedule_expr
        .as_deref()
        .map(str::trim)
        .filter(|expr| !expr.is_empty())
}

#[derive(Debug, thiserror::Error)]
enum ScheduleDecisionError {
    #[error("invalid schedule type: {0}")]
    InvalidScheduleType(String),
    #[error("invalid schedule expression: {0}")]
    InvalidExpression(String),
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use scheduler_core::{InstanceStatus, TriggerType};
    use scheduler_storage::{CreateJob, JobInstanceRepository, JobRepository, connect_and_migrate};

    use super::{ScheduleState, tick_once};

    #[tokio::test]
    async fn fixed_rate_tick_creates_one_pending_instance_when_due() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "fixed".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState::default();
        let now = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once(&jobs, &instances, &state, now)
            .await
            .unwrap_or_else(|error| panic!("tick should run: {error}"));
        tick_once(&jobs, &instances, &state, now)
            .await
            .unwrap_or_else(|error| panic!("same tick should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, InstanceStatus::Pending);
        assert_eq!(listed[0].trigger_type, TriggerType::FixedRate);
    }

    #[tokio::test]
    async fn cron_tick_creates_pending_instance_when_expression_is_due() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cron".to_owned(),
                schedule_type: "cron".to_owned(),
                schedule_expr: Some("0/1 * * * * * *".to_owned()),
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState::default();
        let now = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 1)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once(&jobs, &instances, &state, now)
            .await
            .unwrap_or_else(|error| panic!("tick should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trigger_type, TriggerType::Cron);
    }

    #[tokio::test]
    async fn disabled_scheduled_job_does_not_trigger() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "disabled".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                enabled: false,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState::default();
        let now = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once(&jobs, &instances, &state, now)
            .await
            .unwrap_or_else(|error| panic!("tick should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert!(listed.is_empty());
    }

    async fn repositories() -> (JobRepository, JobInstanceRepository) {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        (
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db),
        )
    }
}
