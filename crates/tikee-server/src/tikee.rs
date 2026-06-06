//! Automatic schedule tick loop for CRON, fixed-rate, fixed-delay, one-shot, and daily-window jobs.

use std::{
    hash::{Hash, Hasher},
    str::FromStr,
    time::Duration,
};

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use tikee_core::{ExecutionMode, InstanceStatus, MisfirePolicy, ScheduleType, TriggerType};

use crate::cluster::SharedClusterCoordinator;
use tikee_storage::{
    CalendarRepository, CalendarSummary, CreateJobInstance, JobInstanceRepository, JobRepository,
    JobSummary, ScheduleCursorRepository,
};
use tracing::{debug, warn};

const TICK_INTERVAL: Duration = Duration::from_secs(1);
const MISFIRE_GRACE: chrono::Duration = chrono::Duration::seconds(5);
const CATCH_UP_LIMIT: usize = 8;

/// Backward-compatible lightweight tick state handle. Durable schedule cursors live in storage.
#[derive(Debug, Default)]
pub struct ScheduleState;

/// Run the automatic tikee tick loop forever.
pub async fn run_tick_loop(
    jobs: JobRepository,
    instances: JobInstanceRepository,
    cluster: SharedClusterCoordinator,
) {
    let state = ScheduleState;
    let mut ticker = tokio::time::interval(TICK_INTERVAL);

    loop {
        ticker.tick().await;
        if let Err(error) =
            tick_once_if_owner(&jobs, &instances, &state, &cluster, Utc::now()).await
        {
            warn!(%error, "schedule tick iteration failed");
        }
    }
}

async fn tick_once_if_owner(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    state: &ScheduleState,
    cluster: &SharedClusterCoordinator,
    now: DateTime<Utc>,
) -> Result<(), tikee_storage::DbErr> {
    let status = cluster.status().await;
    if !status.can_schedule {
        debug!(role = status.role.as_str(), node_id = %status.node_id, "skip schedule tick without cluster ownership");
        return Ok(());
    }
    tick_once(jobs, instances, state, now).await
}

async fn tick_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    _state: &ScheduleState,
    now: DateTime<Utc>,
) -> Result<(), tikee_storage::DbErr> {
    let cursors = ScheduleCursorRepository::new(jobs.db());
    for job in jobs.list_enabled_scheduled_jobs().await? {
        let decision = match due_triggers(jobs, &job, instances, &cursors, now).await {
            Ok(decision) => decision,
            Err(error) => {
                warn!(job_id = %job.id, %error, "schedule decision failed");
                continue;
            }
        };

        for trigger in decision.triggers {
            cursors
                .create_pending_once(
                    CreateJobInstance {
                        job_id: job.id.clone(),
                        trigger_type: trigger.trigger_type,
                        execution_mode: ExecutionMode::Single,
                    },
                    trigger.fire_at.to_rfc3339(),
                )
                .await?;
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
struct ScheduleDecision {
    triggers: Vec<ScheduleTrigger>,
}

#[derive(Debug, Clone, Copy)]
struct ScheduleTrigger {
    trigger_type: TriggerType,
    fire_at: DateTime<Utc>,
}

async fn due_triggers(
    jobs: &JobRepository,
    job: &JobSummary,
    instances: &JobInstanceRepository,
    cursors: &ScheduleCursorRepository,
    now: DateTime<Utc>,
) -> Result<ScheduleDecision, ScheduleDecisionError> {
    if !within_lifecycle_window(jobs, job, now).await? {
        return Ok(ScheduleDecision::default());
    }
    let schedule_type = ScheduleType::from_str(&job.schedule_type)
        .map_err(|error| ScheduleDecisionError::InvalidScheduleType(error.to_string()))?;

    match schedule_type {
        ScheduleType::Cron => cron_due(job, cursors, now).await,
        ScheduleType::FixedRate => fixed_rate_due(job, cursors, now).await,
        ScheduleType::FixedDelay => fixed_delay_due(job, instances, cursors, now).await,
        ScheduleType::Once => once_due(job, cursors, now).await,
        ScheduleType::DailyTimeInterval => daily_time_interval_due(job, cursors, now).await,
        ScheduleType::Api => Ok(ScheduleDecision::default()),
    }
}

async fn cron_due(
    job: &JobSummary,
    cursors: &ScheduleCursorRepository,
    now: DateTime<Utc>,
) -> Result<ScheduleDecision, ScheduleDecisionError> {
    let Some(expression) = non_empty_expr(job) else {
        return Ok(ScheduleDecision::default());
    };
    let cron_spec = parse_cron_expression(expression)?;
    if cron_spec
        .excluded_dates
        .contains(&now.format("%Y-%m-%d").to_string())
    {
        return Ok(ScheduleDecision::default());
    }
    let schedule = Schedule::from_str(&cron_spec.expression)
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))?;
    let previous = latest_cursor(cursors, &job.id)
        .await?
        .unwrap_or_else(|| now - chrono::Duration::seconds(1));
    let due_times: Vec<DateTime<Utc>> = if let Some(timezone) = cron_spec.timezone {
        let local_now = timezone.from_utc_datetime(&now.naive_utc());
        let local_previous = timezone.from_utc_datetime(&previous.naive_utc());
        let excluded_dates = cron_spec.excluded_dates;
        schedule
            .after(&local_previous)
            .take(CATCH_UP_LIMIT)
            .take_while(|next| *next <= local_now)
            .filter(|next| !excluded_dates.contains(&next.format("%Y-%m-%d").to_string()))
            .map(|next| next.with_timezone(&Utc))
            .collect()
    } else {
        schedule
            .after(&previous)
            .take(CATCH_UP_LIMIT)
            .take_while(|next| *next <= now)
            .collect()
    };
    Ok(misfire_decision(job, &due_times, TriggerType::Cron, now))
}

#[derive(Debug, Clone)]
struct CronExpressionSpec {
    expression: String,
    timezone: Option<Tz>,
    excluded_dates: Vec<String>,
}

fn parse_cron_expression(expression: &str) -> Result<CronExpressionSpec, ScheduleDecisionError> {
    let mut parts = expression.split(';');
    let cron = parts.next().unwrap_or_default().trim();
    if cron.is_empty() {
        return Err(ScheduleDecisionError::InvalidExpression(
            "cron expression must not be empty".to_owned(),
        ));
    }
    let mut timezone = None;
    let mut excluded_dates = Vec::new();
    for option in parts {
        let Some((key, value)) = option.trim().split_once('=') else {
            return Err(ScheduleDecisionError::InvalidExpression(format!(
                "invalid cron option: {option}"
            )));
        };
        match key.trim() {
            "tz" | "timezone" => {
                timezone = Some(value.trim().parse::<Tz>().map_err(|_| {
                    ScheduleDecisionError::InvalidExpression(format!(
                        "invalid cron timezone: {}",
                        value.trim()
                    ))
                })?);
            }
            "exclude" | "exclude_dates" | "calendar_exclude" => {
                excluded_dates.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(validate_yyyy_mm_dd)
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            _ => {
                return Err(ScheduleDecisionError::InvalidExpression(format!(
                    "unsupported cron option: {}",
                    key.trim()
                )));
            }
        }
    }
    Ok(CronExpressionSpec {
        expression: cron.to_owned(),
        timezone,
        excluded_dates,
    })
}

fn validate_yyyy_mm_dd(value: &str) -> Result<String, ScheduleDecisionError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))
}

async fn fixed_rate_due(
    job: &JobSummary,
    cursors: &ScheduleCursorRepository,
    now: DateTime<Utc>,
) -> Result<ScheduleDecision, ScheduleDecisionError> {
    let Some(expression) = non_empty_expr(job) else {
        return Ok(ScheduleDecision::default());
    };
    let spec = parse_fixed_rate_expression(expression)?;
    let rate = spec.interval;
    let jitter = deterministic_jitter(&job.id, spec.jitter);
    let previous = latest_cursor(cursors, &job.id).await?;
    let Some(last) = previous else {
        if jitter > chrono::Duration::zero()
            && now
                .timestamp_millis()
                .rem_euclid(rate.num_milliseconds().max(1))
                < jitter.num_milliseconds()
        {
            return Ok(ScheduleDecision::default());
        }
        return Ok(one_trigger(TriggerType::FixedRate, now));
    };
    let due_after = rate + jitter;
    if now.signed_duration_since(last) < due_after {
        return Ok(ScheduleDecision::default());
    }
    let mut due_times = Vec::new();
    let mut next = last + rate;
    while next <= now && due_times.len() < CATCH_UP_LIMIT {
        due_times.push(next);
        next += rate;
    }
    if due_times.is_empty() {
        due_times.push(now);
    }
    Ok(misfire_decision(
        job,
        &due_times,
        TriggerType::FixedRate,
        now,
    ))
}

#[derive(Debug, Clone, Copy)]
struct FixedRateSpec {
    interval: chrono::Duration,
    jitter: chrono::Duration,
}

fn parse_fixed_rate_expression(expression: &str) -> Result<FixedRateSpec, ScheduleDecisionError> {
    let mut parts = expression.split(';');
    let interval = parse_chrono_duration(parts.next().unwrap_or_default().trim())?;
    let mut jitter = chrono::Duration::zero();
    for option in parts {
        let Some((key, value)) = option.trim().split_once('=') else {
            return Err(ScheduleDecisionError::InvalidExpression(format!(
                "invalid fixed_rate option: {option}"
            )));
        };
        match key.trim() {
            "jitter" => jitter = parse_chrono_duration(value.trim())?,
            _ => {
                return Err(ScheduleDecisionError::InvalidExpression(format!(
                    "unsupported fixed_rate option: {}",
                    key.trim()
                )));
            }
        }
    }
    if jitter < chrono::Duration::zero() || jitter >= interval {
        return Err(ScheduleDecisionError::InvalidExpression(
            "fixed_rate jitter must be non-negative and smaller than interval".to_owned(),
        ));
    }
    Ok(FixedRateSpec { interval, jitter })
}

fn deterministic_jitter(job_id: &str, max_jitter: chrono::Duration) -> chrono::Duration {
    let max = max_jitter.num_milliseconds();
    if max <= 0 {
        return chrono::Duration::zero();
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    job_id.hash(&mut hasher);
    let max = u64::try_from(max).unwrap_or(0);
    if max == 0 {
        return chrono::Duration::zero();
    }
    let jitter = hasher.finish() % max;
    chrono::Duration::milliseconds(i64::try_from(jitter).unwrap_or(i64::MAX))
}

async fn fixed_delay_due(
    job: &JobSummary,
    instances: &JobInstanceRepository,
    cursors: &ScheduleCursorRepository,
    now: DateTime<Utc>,
) -> Result<ScheduleDecision, ScheduleDecisionError> {
    let Some(expression) = non_empty_expr(job) else {
        return Ok(ScheduleDecision::default());
    };
    let delay = parse_chrono_duration(expression)?;
    let latest_cursor = latest_cursor(cursors, &job.id).await?;
    let active_exists = instances
        .list_by_job(&job.id)
        .await?
        .into_iter()
        .any(|instance| {
            matches!(
                instance.status,
                InstanceStatus::Pending | InstanceStatus::Running
            )
        });
    if active_exists {
        return Ok(ScheduleDecision::default());
    }
    let Some(last_terminal) = instances.latest_terminal_by_job(&job.id).await? else {
        return if latest_cursor.is_none() {
            Ok(one_trigger(TriggerType::FixedDelay, now))
        } else {
            Ok(ScheduleDecision::default())
        };
    };
    let completed_at = parse_rfc3339_utc(&last_terminal.updated_at)?;
    if latest_cursor.is_some_and(|cursor| cursor >= completed_at) {
        return Ok(ScheduleDecision::default());
    }
    if now.signed_duration_since(completed_at) >= delay {
        Ok(one_trigger(TriggerType::FixedDelay, now))
    } else {
        Ok(ScheduleDecision::default())
    }
}

async fn once_due(
    job: &JobSummary,
    cursors: &ScheduleCursorRepository,
    now: DateTime<Utc>,
) -> Result<ScheduleDecision, ScheduleDecisionError> {
    if latest_cursor(cursors, &job.id).await?.is_some() {
        return Ok(ScheduleDecision::default());
    }
    let Some(expression) = non_empty_expr(job) else {
        return Ok(ScheduleDecision::default());
    };
    let fire_at = parse_rfc3339_utc(expression)?;
    if fire_at <= now {
        Ok(one_trigger(TriggerType::Once, fire_at))
    } else {
        Ok(ScheduleDecision::default())
    }
}

async fn daily_time_interval_due(
    job: &JobSummary,
    cursors: &ScheduleCursorRepository,
    now: DateTime<Utc>,
) -> Result<ScheduleDecision, ScheduleDecisionError> {
    let Some(expression) = non_empty_expr(job) else {
        return Ok(ScheduleDecision::default());
    };
    let spec = parse_daily_time_interval(expression)?;
    let Some(offset) = chrono::FixedOffset::east_opt(spec.utc_offset_seconds) else {
        return Err(ScheduleDecisionError::InvalidExpression(format!(
            "invalid timezone offset seconds: {}",
            spec.utc_offset_seconds
        )));
    };
    let local_now = now.with_timezone(&offset);
    let minute_of_day =
        i64::from(local_now.time().hour()) * 60 + i64::from(local_now.time().minute());
    if minute_of_day < spec.start_minute || minute_of_day > spec.end_minute {
        return Ok(ScheduleDecision::default());
    }
    let elapsed = minute_of_day - spec.start_minute;
    if elapsed % spec.interval_minutes != 0 {
        return Ok(ScheduleDecision::default());
    }
    let previous = latest_cursor(cursors, &job.id).await?;
    if previous
        .is_some_and(|last| now.signed_duration_since(last).num_minutes() < spec.interval_minutes)
    {
        return Ok(ScheduleDecision::default());
    }
    Ok(one_trigger(TriggerType::DailyTimeInterval, now))
}

#[derive(Debug, Clone, Copy)]
struct DailyTimeIntervalSpec {
    start_minute: i64,
    end_minute: i64,
    interval_minutes: i64,
    utc_offset_seconds: i32,
}

fn parse_daily_time_interval(
    expression: &str,
) -> Result<DailyTimeIntervalSpec, ScheduleDecisionError> {
    let (window_and_interval, tz) = expression.split_once('@').unwrap_or((expression, "+00:00"));
    let (window, interval) = window_and_interval
        .split_once('/')
        .unwrap_or((window_and_interval, "1m"));
    let (start, end) = window.split_once('-').ok_or_else(|| {
        ScheduleDecisionError::InvalidExpression(
            "daily_time_interval must use HH:MM-HH:MM[/interval]@TZ".to_owned(),
        )
    })?;
    let start_minute = parse_hhmm(start)?;
    let end_minute = parse_hhmm(end)?;
    if end_minute < start_minute {
        return Err(ScheduleDecisionError::InvalidExpression(
            "daily_time_interval end must be after start on the same day".to_owned(),
        ));
    }
    let interval = parse_chrono_duration(interval)?.num_minutes().max(1);
    Ok(DailyTimeIntervalSpec {
        start_minute,
        end_minute,
        interval_minutes: interval,
        utc_offset_seconds: parse_timezone_offset_seconds(tz)?,
    })
}

fn parse_hhmm(value: &str) -> Result<i64, ScheduleDecisionError> {
    let (hour, minute) = value.trim().split_once(':').ok_or_else(|| {
        ScheduleDecisionError::InvalidExpression(format!("invalid daily time: {value}"))
    })?;
    let hour: i64 = hour.parse().map_err(|_| {
        ScheduleDecisionError::InvalidExpression(format!("invalid daily hour: {value}"))
    })?;
    let minute: i64 = minute.parse().map_err(|_| {
        ScheduleDecisionError::InvalidExpression(format!("invalid daily minute: {value}"))
    })?;
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) {
        return Err(ScheduleDecisionError::InvalidExpression(format!(
            "invalid daily time range: {value}"
        )));
    }
    Ok(hour * 60 + minute)
}

fn parse_timezone_offset_seconds(value: &str) -> Result<i32, ScheduleDecisionError> {
    let value = value.trim();
    let value = match value {
        "UTC" | "Etc/UTC" | "Z" => "+00:00",
        "Asia/Shanghai" | "PRC" => "+08:00",
        other => other,
    };
    let sign = if let Some(rest) = value.strip_prefix('+') {
        (1, rest)
    } else if let Some(rest) = value.strip_prefix('-') {
        (-1, rest)
    } else {
        return Err(ScheduleDecisionError::InvalidExpression(format!(
            "unsupported timezone offset: {value}"
        )));
    };
    let (hours, minutes) = sign.1.split_once(':').ok_or_else(|| {
        ScheduleDecisionError::InvalidExpression(format!("invalid timezone offset: {value}"))
    })?;
    let hours: i32 = hours.parse().map_err(|_| {
        ScheduleDecisionError::InvalidExpression(format!("invalid timezone hour: {value}"))
    })?;
    let minutes: i32 = minutes.parse().map_err(|_| {
        ScheduleDecisionError::InvalidExpression(format!("invalid timezone minute: {value}"))
    })?;
    Ok(sign.0 * (hours * 3600 + minutes * 60))
}

fn one_trigger(trigger_type: TriggerType, fire_at: DateTime<Utc>) -> ScheduleDecision {
    ScheduleDecision {
        triggers: vec![ScheduleTrigger {
            trigger_type,
            fire_at,
        }],
    }
}

fn misfire_decision(
    job: &JobSummary,
    due_times: &[DateTime<Utc>],
    trigger_type: TriggerType,
    now: DateTime<Utc>,
) -> ScheduleDecision {
    let Some(latest) = due_times.last().copied() else {
        return ScheduleDecision::default();
    };
    let misfired = due_times.len() > 1 || now.signed_duration_since(latest) > MISFIRE_GRACE;
    let policy = job
        .misfire_policy
        .parse::<MisfirePolicy>()
        .unwrap_or_default();
    let count = if misfired {
        match policy {
            MisfirePolicy::DoNothing | MisfirePolicy::Reschedule => 0,
            MisfirePolicy::FireOnce | MisfirePolicy::LatestOnly => 1,
            MisfirePolicy::CatchUpLimited => due_times.len().min(CATCH_UP_LIMIT),
        }
    } else {
        1
    };
    let selected: Vec<DateTime<Utc>> =
        if misfired && matches!(policy, MisfirePolicy::FireOnce | MisfirePolicy::LatestOnly) {
            due_times.last().copied().into_iter().collect()
        } else {
            due_times.iter().copied().take(count).collect()
        };
    ScheduleDecision {
        triggers: selected
            .into_iter()
            .map(|fire_at| ScheduleTrigger {
                trigger_type,
                fire_at,
            })
            .collect(),
    }
}

async fn latest_cursor(
    cursors: &ScheduleCursorRepository,
    job_id: &str,
) -> Result<Option<DateTime<Utc>>, ScheduleDecisionError> {
    cursors
        .latest_fire_at(job_id)
        .await?
        .map(|value| parse_rfc3339_utc(&value))
        .transpose()
}

async fn within_lifecycle_window(
    jobs: &JobRepository,
    job: &JobSummary,
    now: DateTime<Utc>,
) -> Result<bool, ScheduleDecisionError> {
    if let Some(start) = job
        .schedule_start_at
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        && now < parse_rfc3339_utc(start)?
    {
        return Ok(false);
    }
    if let Some(end) = job
        .schedule_end_at
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        && now >= parse_rfc3339_utc(end)?
    {
        return Ok(false);
    }
    if schedule_calendar_blocks(jobs, job, now).await? {
        return Ok(false);
    }
    Ok(true)
}

async fn schedule_calendar_blocks(
    jobs: &JobRepository,
    job: &JobSummary,
    now: DateTime<Utc>,
) -> Result<bool, ScheduleDecisionError> {
    let Some(calendar) = resolve_schedule_calendar(jobs, job).await? else {
        return Ok(false);
    };
    if date_list_contains(&calendar, "excludedDates", now)
        || date_list_contains(&calendar, "holidays", now)
    {
        return Ok(true);
    }
    for key in ["maintenanceWindows", "freezeWindows"] {
        if time_windows_contain(&calendar, key, now)? {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn resolve_schedule_calendar(
    jobs: &JobRepository,
    job: &JobSummary,
) -> Result<Option<serde_json::Value>, ScheduleDecisionError> {
    let Some(raw) = job
        .schedule_calendar_json
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let calendar = serde_json::from_str::<serde_json::Value>(raw)
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))?;
    let Some(name) = calendar
        .get("calendarRef")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(Some(calendar));
    };
    let Some(stored) = CalendarRepository::new(jobs.db())
        .get_by_name(&job.namespace, &job.app, name)
        .await?
    else {
        return Err(ScheduleDecisionError::InvalidExpression(format!(
            "calendarRef not found: {}/{}/{}",
            job.namespace, job.app, name
        )));
    };
    Ok(Some(calendar_summary_to_value(&stored)))
}

fn calendar_summary_to_value(calendar: &CalendarSummary) -> serde_json::Value {
    serde_json::json!({
        "timezone": calendar.timezone,
        "excludedDates": calendar.excluded_dates,
        "holidays": calendar.holidays,
        "maintenanceWindows": calendar.maintenance_windows,
        "freezeWindows": calendar.freeze_windows,
    })
}

fn date_list_contains(calendar: &serde_json::Value, key: &str, now: DateTime<Utc>) -> bool {
    let today = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());
    calendar
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|dates| {
            dates
                .iter()
                .filter_map(serde_json::Value::as_str)
                .any(|date| date == today)
        })
}

fn time_windows_contain(
    calendar: &serde_json::Value,
    key: &str,
    now: DateTime<Utc>,
) -> Result<bool, ScheduleDecisionError> {
    let Some(windows) = calendar.get(key).and_then(serde_json::Value::as_array) else {
        return Ok(false);
    };
    for window in windows {
        let Some(start) = window.get("start").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(end) = window.get("end").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if now >= parse_rfc3339_utc(start)? && now < parse_rfc3339_utc(end)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn parse_chrono_duration(expression: &str) -> Result<chrono::Duration, ScheduleDecisionError> {
    let duration = humantime::parse_duration(expression)
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))?;
    chrono::Duration::from_std(duration)
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))
}

fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>, ScheduleDecisionError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| ScheduleDecisionError::InvalidExpression(error.to_string()))
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
    #[error("storage error: {0}")]
    Storage(#[from] tikee_storage::DbErr),
}

#[cfg(test)]
mod tests {
    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};
    use chrono::{TimeZone, Utc};
    use tikee_core::{InstanceStatus, TriggerType};
    use tikee_storage::{CreateJob, JobInstanceRepository, JobRepository, connect_and_migrate};

    use super::{ScheduleState, tick_once, tick_once_if_owner};

    #[tokio::test]
    async fn fixed_rate_tick_creates_one_pending_instance_when_due() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "fixed".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
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
    async fn fixed_rate_cursor_survives_tick_loop_restart_without_duplicate_trigger() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "fixed-persisted-cursor".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let now = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once(&jobs, &instances, &ScheduleState, now)
            .await
            .unwrap_or_else(|error| panic!("first tick should run: {error}"));
        tick_once(&jobs, &instances, &ScheduleState, now)
            .await
            .unwrap_or_else(|error| panic!("restart tick should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(
            listed.len(),
            1,
            "persisted schedule cursor must prevent duplicate triggers after restart"
        );
    }

    #[tokio::test]
    async fn lifecycle_window_blocks_calendar_windows() {
        let (jobs, instances) = repositories().await;
        let now = Utc
            .with_ymd_and_hms(2026, 5, 29, 10, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));
        let calendar = serde_json::json!({
            "maintenanceWindows": [{
                "start": "2026-05-29T09:00:00Z",
                "end": "2026-05-29T11:00:00Z"
            }],
            "freezeWindows": [{
                "start": "2026-12-24T00:00:00Z",
                "end": "2026-12-26T00:00:00Z"
            }],
            "excludedDates": ["2026-06-01"]
        });
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "calendar-blocked".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: Some(calendar.to_string()),
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));

        tick_once(&jobs, &instances, &ScheduleState, now)
            .await
            .unwrap_or_else(|error| panic!("tick should run: {error}"));
        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert!(listed.is_empty());
    }

    #[tokio::test]
    async fn lifecycle_window_resolves_centralized_calendar_ref() {
        let (jobs, instances) = repositories().await;
        tikee_storage::CalendarRepository::new(jobs.db())
            .upsert(tikee_storage::UpsertCalendar {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cn-maintenance".to_owned(),
                timezone: "Asia/Shanghai".to_owned(),
                excluded_dates: vec!["2026-05-29".to_owned()],
                holidays: Vec::new(),
                maintenance_windows: Vec::new(),
                freeze_windows: Vec::new(),
                created_by: "test".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("calendar should upsert: {error}"));
        let now = Utc
            .with_ymd_and_hms(2026, 5, 29, 10, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "calendar-ref-blocked".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: Some(
                    serde_json::json!({"calendarRef":"cn-maintenance"}).to_string(),
                ),
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));

        tick_once(&jobs, &instances, &ScheduleState, now)
            .await
            .unwrap_or_else(|error| panic!("tick should run: {error}"));
        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert!(listed.is_empty());
    }

    #[test]
    fn fixed_rate_expression_supports_jitter_option() {
        let spec = super::parse_fixed_rate_expression("30s;jitter=5s")
            .unwrap_or_else(|error| panic!("fixed_rate expression should parse: {error}"));
        assert_eq!(spec.interval.num_seconds(), 30);
        assert_eq!(spec.jitter.num_seconds(), 5);
        let jitter = super::deterministic_jitter("job-fixed", spec.jitter);
        assert!(jitter >= chrono::Duration::zero());
        assert!(jitter < spec.jitter);
    }

    #[test]
    fn fixed_rate_expression_rejects_jitter_not_smaller_than_interval() {
        let Err(error) = super::parse_fixed_rate_expression("30s;jitter=30s") else {
            panic!("jitter equal to interval must be rejected");
        };
        assert!(error.to_string().contains("jitter"));
    }

    #[tokio::test]
    async fn fixed_rate_latest_only_misfire_keeps_one_instance() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "fixed-latest-only".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "latest_only".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let first = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));
        let later = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 10)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once(&jobs, &instances, &state, first)
            .await
            .unwrap_or_else(|error| panic!("first tick should run: {error}"));
        tick_once(&jobs, &instances, &state, later)
            .await
            .unwrap_or_else(|error| panic!("second tick should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(listed.len(), 2);
        assert!(
            listed
                .iter()
                .all(|item| item.trigger_type == TriggerType::FixedRate)
        );
    }

    #[tokio::test]
    async fn cron_tick_creates_pending_instance_when_expression_is_due() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cron".to_owned(),
                schedule_type: "cron".to_owned(),
                schedule_expr: Some("0/1 * * * * * *".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
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
    async fn cron_tick_uses_iana_timezone_option() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cron-tz".to_owned(),
                schedule_type: "cron".to_owned(),
                schedule_expr: Some("0 30 9 * * * *;tz=Asia/Shanghai".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let now = Utc
            .with_ymd_and_hms(2026, 5, 29, 1, 30, 0)
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
    async fn cron_tick_skips_excluded_calendar_date() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cron-exclude".to_owned(),
                schedule_type: "cron".to_owned(),
                schedule_expr: Some(
                    "0 30 9 * * * *;tz=Asia/Shanghai;exclude=2026-05-29".to_owned(),
                ),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let now = Utc
            .with_ymd_and_hms(2026, 5, 29, 1, 30, 0)
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

    #[tokio::test]
    async fn disabled_scheduled_job_does_not_trigger() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "disabled".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: false,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
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

    #[tokio::test]
    async fn follower_tick_does_not_create_instances() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "follower-skip".to_owned(),
                schedule_type: "fixed_rate".to_owned(),
                schedule_expr: Some("1s".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let follower = StaticCoordinator::shared(ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Follower,
            node_id: "node-b".to_owned(),
            nodes: 3,
            can_schedule: false,
            leader_fencing_token: None,
            detail: "test follower".to_owned(),
        });
        let now = Utc
            .with_ymd_and_hms(2026, 5, 19, 1, 0, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once_if_owner(&jobs, &instances, &state, &follower, now)
            .await
            .unwrap_or_else(|error| panic!("tick gate should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert!(listed.is_empty());
    }

    #[tokio::test]
    async fn daily_time_interval_tick_creates_instance_inside_aligned_window() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "daily-window".to_owned(),
                schedule_type: "daily_time_interval".to_owned(),
                schedule_expr: Some("09:00-18:00/30m@Asia/Shanghai".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let now = Utc
            .with_ymd_and_hms(2026, 5, 29, 1, 30, 0)
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
        assert_eq!(listed[0].trigger_type, TriggerType::DailyTimeInterval);
    }

    #[tokio::test]
    async fn daily_time_interval_tick_skips_outside_window() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "daily-window-outside".to_owned(),
                schedule_type: "daily_time_interval".to_owned(),
                schedule_expr: Some("09:00-18:00/30m@+08:00".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let now = Utc
            .with_ymd_and_hms(2026, 5, 29, 0, 30, 0)
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

    #[tokio::test]
    async fn daily_time_interval_tick_does_not_repeat_within_same_interval() {
        let (jobs, instances) = repositories().await;
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "daily-window-no-repeat".to_owned(),
                schedule_type: "daily_time_interval".to_owned(),
                schedule_expr: Some("09:00-18:00/30m@Asia/Shanghai".to_owned()),
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let state = ScheduleState;
        let first = Utc
            .with_ymd_and_hms(2026, 5, 29, 1, 30, 0)
            .single()
            .unwrap_or_else(|| panic!("valid time"));
        let same_interval = Utc
            .with_ymd_and_hms(2026, 5, 29, 1, 30, 30)
            .single()
            .unwrap_or_else(|| panic!("valid time"));

        tick_once(&jobs, &instances, &state, first)
            .await
            .unwrap_or_else(|error| panic!("first tick should run: {error}"));
        tick_once(&jobs, &instances, &state, same_interval)
            .await
            .unwrap_or_else(|error| panic!("second tick should run: {error}"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));
        assert_eq!(listed.len(), 1);
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
