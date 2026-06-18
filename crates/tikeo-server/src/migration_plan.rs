//! Report-only migration planner for existing scheduler exports.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tikeo_core::{MisfirePolicy, ScheduleType};

/// Source scheduler supported by the migration planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MigrationSource {
    /// XXL-JOB exported job records.
    #[value(name = "xxl-job")]
    XxlJob,
    /// PowerJob exported job records.
    #[value(name = "powerjob", alias = "power-job")]
    PowerJob,
}

impl MigrationSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::XxlJob => "xxl-job",
            Self::PowerJob => "powerjob",
        }
    }
}

/// Output format for migration reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MigrationReportFormat {
    /// Machine-readable JSON report.
    Json,
    /// Human-readable Markdown report.
    Markdown,
}

/// CLI options for report-only scheduler migration planning.
#[derive(Debug, Clone, clap::Args)]
pub struct MigrationCommand {
    /// Source scheduler export format.
    #[arg(long, value_enum)]
    pub from: MigrationSource,
    /// Path to an exported JSON file.
    #[arg(long)]
    pub input: PathBuf,
    /// Optional report output path. Defaults to stdout.
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Report output format.
    #[arg(long, value_enum, default_value = "json")]
    pub format: MigrationReportFormat,
    /// Default Tikeo namespace for generated job drafts.
    #[arg(long, default_value = "default")]
    pub namespace: String,
    /// Default Tikeo app for generated job drafts when source app/group is absent.
    #[arg(long, default_value = "default")]
    pub app: String,
}

/// Execute the report-only migration planner.
///
/// # Errors
///
/// Returns an error when the input cannot be read, parsed, planned, or written.
pub fn run_migration_command(command: &MigrationCommand) -> Result<()> {
    let input = fs::read_to_string(&command.input)
        .with_context(|| format!("failed to read migration input {}", command.input.display()))?;
    let report = plan_migration(
        command.from,
        &input,
        MigrationDefaults {
            namespace: command.namespace.clone(),
            app: command.app.clone(),
        },
    )?;
    let rendered = match command.format {
        MigrationReportFormat::Json => serde_json::to_string_pretty(&report)?,
        MigrationReportFormat::Markdown => render_markdown_report(&report),
    };
    if let Some(output) = &command.output {
        fs::write(output, rendered)
            .with_context(|| format!("failed to write migration report {}", output.display()))?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct MigrationDefaults {
    namespace: String,
    app: String,
}

/// Complete report emitted by the migration planner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    /// Source scheduler family.
    pub source: String,
    /// Planner mode. MVP is intentionally dry-run/report-only.
    pub mode: String,
    /// Summary counters.
    pub summary: MigrationSummary,
    /// Planned jobs.
    pub jobs: Vec<MigrationJobPlan>,
    /// Report-level warnings.
    pub warnings: Vec<String>,
}

/// Migration summary counters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSummary {
    /// Source records inspected.
    pub total: usize,
    /// Jobs planned with no blocking issue.
    pub ready: usize,
    /// Jobs that can be imported only after operator review.
    pub needs_review: usize,
    /// Jobs skipped because required fields were missing.
    pub skipped: usize,
}

/// One planned Tikeo job draft with evidence and warnings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationJobPlan {
    /// Source scheduler job id.
    pub source_id: String,
    /// Source display name.
    pub source_name: String,
    /// Whether this plan is ready, needs review, or skipped.
    pub status: String,
    /// Tikeo create-job draft payload.
    pub tikeo_job: Option<TikeoJobDraft>,
    /// Unsupported or lossy source features.
    pub unsupported_features: Vec<String>,
    /// Per-job warnings.
    pub warnings: Vec<String>,
    /// Original source snapshot fragment used for review.
    pub source_snapshot: Value,
}

/// Tikeo job draft generated from a source scheduler record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TikeoJobDraft {
    /// Target namespace.
    pub namespace: String,
    /// Target app.
    pub app: String,
    /// Job display name.
    pub name: String,
    /// Tikeo schedule type.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Tikeo misfire policy.
    pub misfire_policy: String,
    /// Optional Worker processor binding.
    pub processor_name: Option<String>,
    /// Whether the imported job should be enabled.
    pub enabled: bool,
    /// Retry policy draft.
    pub retry_policy: Value,
    /// Migration metadata for audit/review.
    pub metadata: Value,
}

/// Plan a scheduler migration from a JSON export string.
///
/// # Errors
///
/// Returns an error when the JSON document shape is unsupported.
fn plan_migration(
    source: MigrationSource,
    input: &str,
    defaults: MigrationDefaults,
) -> Result<MigrationReport> {
    let root: Value = serde_json::from_str(input).context("migration input must be valid JSON")?;
    let records = extract_records(&root)?;
    let mut jobs = Vec::with_capacity(records.len());
    for record in records {
        jobs.push(match source {
            MigrationSource::XxlJob => plan_xxl_job(record, &defaults),
            MigrationSource::PowerJob => plan_powerjob(record, &defaults),
        });
    }
    let mut summary = MigrationSummary {
        total: jobs.len(),
        ..MigrationSummary::default()
    };
    for job in &jobs {
        match job.status.as_str() {
            "ready" => summary.ready += 1,
            "needs_review" => summary.needs_review += 1,
            _ => summary.skipped += 1,
        }
    }
    let warnings = if jobs.is_empty() {
        vec!["input contained no job records".to_owned()]
    } else {
        Vec::new()
    };
    Ok(MigrationReport {
        source: source.as_str().to_owned(),
        mode: "dry_run_report_only".to_owned(),
        summary,
        jobs,
        warnings,
    })
}

fn extract_records(root: &Value) -> Result<Vec<&Map<String, Value>>> {
    let candidates = [
        root.get("jobs"),
        root.get("data"),
        root.pointer("/data/jobs"),
        root.pointer("/content"),
        Some(root),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Some(array) = candidate.as_array() {
            return Ok(array.iter().filter_map(Value::as_object).collect());
        }
    }
    if let Some(object) = root.as_object() {
        return Ok(vec![object]);
    }
    bail!("migration input must be a JSON object or array of job objects")
}

fn plan_xxl_job(record: &Map<String, Value>, defaults: &MigrationDefaults) -> MigrationJobPlan {
    let source_id = string_field(record, &["id", "jobId"]).unwrap_or_else(|| "unknown".to_owned());
    let source_name = string_field(record, &["jobDesc", "job_desc", "name"])
        .unwrap_or_else(|| format!("xxl-job-{source_id}"));
    let processor_name = string_field(record, &["executorHandler", "executor_handler"]);
    let schedule_type_raw = string_field(record, &["scheduleType", "schedule_type"])
        .unwrap_or_else(|| "NONE".to_owned());
    let schedule_expr = string_field(record, &["scheduleConf", "schedule_conf"]);
    let (schedule_type, schedule_warning) =
        map_xxl_schedule(&schedule_type_raw, schedule_expr.as_deref());
    let mut warnings = Vec::new();
    let mut unsupported_features = Vec::new();
    if let Some(warning) = schedule_warning {
        warnings.push(warning);
    }
    if processor_name.as_deref().unwrap_or_default().is_empty() {
        warnings.push("missing executorHandler; generated draft has no processorName".to_owned());
    }
    collect_if_present(
        record,
        &mut unsupported_features,
        "glueType",
        "XXL-JOB GLUE/script type requires manual script/plugin mapping",
    );
    collect_if_present(
        record,
        &mut unsupported_features,
        "executorRouteStrategy",
        "XXL-JOB route strategy is replaced by Tikeo Worker capability and LASSO scoring",
    );
    collect_if_present(
        record,
        &mut unsupported_features,
        "executorBlockStrategy",
        "XXL-JOB block strategy requires review against Tikeo concurrency policy",
    );
    let retry_count = integer_field(
        record,
        &["executorFailRetryCount", "executor_fail_retry_count"],
    )
    .unwrap_or(0)
    .max(0);
    let enabled = integer_field(record, &["triggerStatus", "trigger_status"]).unwrap_or(1) != 0;
    let status = if schedule_type == ScheduleType::Api.as_str()
        && schedule_type_raw.to_ascii_uppercase() != "NONE"
    {
        "needs_review"
    } else if processor_name.is_none() {
        "needs_review"
    } else if unsupported_features.is_empty() {
        "ready"
    } else {
        "needs_review"
    };
    MigrationJobPlan {
        source_id: source_id.clone(),
        source_name: source_name.clone(),
        status: status.to_owned(),
        tikeo_job: Some(TikeoJobDraft {
            namespace: defaults.namespace.clone(),
            app: string_field(record, &["app", "executorAppName", "executor_app_name"])
                .unwrap_or_else(|| defaults.app.clone()),
            name: source_name,
            schedule_type: schedule_type.to_owned(),
            schedule_expr: if schedule_type == ScheduleType::Api.as_str() {
                None
            } else {
                schedule_expr
            },
            misfire_policy: map_misfire_policy(
                string_field(record, &["misfireStrategy", "misfire_strategy"]).as_deref(),
            ),
            processor_name,
            enabled,
            retry_policy: json!({
                "enabled": retry_count > 0,
                "maxAttempts": retry_count + 1,
                "initialDelaySeconds": 30,
                "backoffMultiplier": 2.0,
                "maxDelaySeconds": 300
            }),
            metadata: json!({
                "migratedFrom": "xxl-job",
                "sourceId": source_id,
                "executorParam": string_field(record, &["executorParam", "executor_param"]),
                "executorTimeoutSeconds": integer_field(record, &["executorTimeout", "executor_timeout"]),
            }),
        }),
        unsupported_features,
        warnings,
        source_snapshot: Value::Object(record.clone()),
    }
}

fn plan_powerjob(record: &Map<String, Value>, defaults: &MigrationDefaults) -> MigrationJobPlan {
    let source_id =
        string_field(record, &["id", "jobId", "job_id"]).unwrap_or_else(|| "unknown".to_owned());
    let source_name = string_field(record, &["jobName", "job_name", "name"])
        .unwrap_or_else(|| format!("powerjob-{source_id}"));
    let processor_name = string_field(
        record,
        &["processorInfo", "processor_info", "processorName"],
    );
    let schedule_type_raw = string_field(
        record,
        &["timeExpressionType", "time_expression_type", "scheduleType"],
    )
    .unwrap_or_else(|| "API".to_owned());
    let schedule_expr = string_field(
        record,
        &["timeExpression", "time_expression", "scheduleConf"],
    );
    let (schedule_type, schedule_warning) =
        map_powerjob_schedule(&schedule_type_raw, schedule_expr.as_deref());
    let mut warnings = Vec::new();
    let mut unsupported_features = Vec::new();
    if let Some(warning) = schedule_warning {
        warnings.push(warning);
    }
    if processor_name.as_deref().unwrap_or_default().is_empty() {
        warnings.push("missing processorInfo; generated draft has no processorName".to_owned());
    }
    collect_if_present(
        record,
        &mut unsupported_features,
        "executeType",
        "PowerJob executeType/broadcast/map-reduce semantics require workflow or fan-out review",
    );
    collect_if_present(
        record,
        &mut unsupported_features,
        "designatedWorkers",
        "PowerJob designatedWorkers must be translated to Tikeo Worker labels/capabilities",
    );
    collect_if_present(
        record,
        &mut unsupported_features,
        "maxInstanceNum",
        "PowerJob maxInstanceNum requires review against Tikeo concurrency policy",
    );
    let retry_count = integer_field(
        record,
        &["instanceRetryNum", "instance_retry_num", "retryNum"],
    )
    .unwrap_or(0)
    .max(0);
    let enabled = !matches!(integer_field(record, &["status", "enable"]), Some(0));
    let status = if processor_name.is_none() || !unsupported_features.is_empty() {
        "needs_review"
    } else {
        "ready"
    };
    MigrationJobPlan {
        source_id: source_id.clone(),
        source_name: source_name.clone(),
        status: status.to_owned(),
        tikeo_job: Some(TikeoJobDraft {
            namespace: defaults.namespace.clone(),
            app: string_field(record, &["appName", "app_name", "app"])
                .unwrap_or_else(|| defaults.app.clone()),
            name: source_name,
            schedule_type: schedule_type.to_owned(),
            schedule_expr: if schedule_type == ScheduleType::Api.as_str() {
                None
            } else {
                schedule_expr
            },
            misfire_policy: MisfirePolicy::FireOnce.as_str().to_owned(),
            processor_name,
            enabled,
            retry_policy: json!({
                "enabled": retry_count > 0,
                "maxAttempts": retry_count + 1,
                "initialDelaySeconds": 30,
                "backoffMultiplier": 2.0,
                "maxDelaySeconds": 300
            }),
            metadata: json!({
                "migratedFrom": "powerjob",
                "sourceId": source_id,
                "processorType": string_field(record, &["processorType", "processor_type"]),
                "jobParams": string_field(record, &["jobParams", "job_params", "instanceParams"]),
            }),
        }),
        unsupported_features,
        warnings,
        source_snapshot: Value::Object(record.clone()),
    }
}

fn map_xxl_schedule(raw: &str, expr: Option<&str>) -> (&'static str, Option<String>) {
    match raw.trim().to_ascii_uppercase().as_str() {
        "CRON" => (ScheduleType::Cron.as_str(), None),
        "FIX_RATE" | "FIXED_RATE" => (ScheduleType::FixedRate.as_str(), None),
        "NONE" | "API" | "MANUAL" => (ScheduleType::Api.as_str(), None),
        other => (
            ScheduleType::Api.as_str(),
            Some(format!(
                "unsupported XXL-JOB scheduleType '{other}' mapped to api for manual review; original expression={expr:?}"
            )),
        ),
    }
}

fn map_powerjob_schedule(raw: &str, expr: Option<&str>) -> (&'static str, Option<String>) {
    match raw.trim().to_ascii_uppercase().as_str() {
        "2" | "CRON" => (ScheduleType::Cron.as_str(), None),
        "3" | "FIXED_RATE" | "FIX_RATE" | "FIX_RATE_SECONDS" => {
            (ScheduleType::FixedRate.as_str(), None)
        }
        "4" | "FIXED_DELAY" | "FIX_DELAY" => (ScheduleType::FixedDelay.as_str(), None),
        "1" | "API" | "NONE" => (ScheduleType::Api.as_str(), None),
        other => (
            ScheduleType::Api.as_str(),
            Some(format!(
                "unsupported PowerJob timeExpressionType '{other}' mapped to api for manual review; original expression={expr:?}"
            )),
        ),
    }
}

fn map_misfire_policy(raw: Option<&str>) -> String {
    match raw.unwrap_or_default().trim().to_ascii_uppercase().as_str() {
        "DO_NOTHING" | "IGNORE" => MisfirePolicy::DoNothing.as_str().to_owned(),
        _ => MisfirePolicy::FireOnce.as_str().to_owned(),
    }
}

fn collect_if_present(
    record: &Map<String, Value>,
    features: &mut Vec<String>,
    field: &str,
    message: &str,
) {
    if record.get(field).is_some_and(|value| !value.is_null()) {
        features.push(message.to_owned());
    }
}

fn string_field(record: &Map<String, Value>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        record.get(*name).and_then(|value| match value {
            Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        })
    })
}

fn integer_field(record: &Map<String, Value>, names: &[&str]) -> Option<i64> {
    names.iter().find_map(|name| {
        record.get(*name).and_then(|value| match value {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse::<i64>().ok(),
            Value::Bool(value) => Some(i64::from(*value)),
            _ => None,
        })
    })
}

fn render_markdown_report(report: &MigrationReport) -> String {
    let mut output = format!(
        "# Tikeo migration report\n\n- Source: `{}`\n- Mode: `{}`\n- Total: {}\n- Ready: {}\n- Needs review: {}\n- Skipped: {}\n\n",
        report.source,
        report.mode,
        report.summary.total,
        report.summary.ready,
        report.summary.needs_review,
        report.summary.skipped
    );
    output.push_str("| Source ID | Name | Status | Tikeo schedule | Processor | Notes |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for job in &report.jobs {
        let (schedule, processor) = job.tikeo_job.as_ref().map_or(("-", "-"), |draft| {
            (
                draft.schedule_type.as_str(),
                draft.processor_name.as_deref().unwrap_or("-"),
            )
        });
        let notes = job
            .unsupported_features
            .iter()
            .chain(job.warnings.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("; ")
            .replace('|', "\\|");
        output.push_str(&format!(
            "| `{}` | {} | {} | `{}` | `{}` | {} |\n",
            job.source_id,
            job.source_name.replace('|', "\\|"),
            job.status,
            schedule,
            processor,
            notes
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_xxl_job_export_into_tikeo_drafts_with_review_flags() {
        let input = r#"{
          "jobs": [
            {"id": 7, "jobDesc": "nightly billing", "scheduleType": "CRON", "scheduleConf": "0 0 2 * * ?", "executorHandler": "billingProcessor", "executorFailRetryCount": 2, "triggerStatus": 1, "executorRouteStrategy": "ROUND"},
            {"id": 8, "jobDesc": "manual cleanup", "scheduleType": "NONE", "executorHandler": "cleanupProcessor", "triggerStatus": 0}
          ]
        }"#;

        let report = plan_migration(
            MigrationSource::XxlJob,
            input,
            MigrationDefaults {
                namespace: "ops".to_owned(),
                app: "billing".to_owned(),
            },
        )
        .unwrap_or_else(|error| panic!("report should plan: {error}"));

        assert_eq!(report.source, "xxl-job");
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.ready, 1);
        assert_eq!(report.summary.needs_review, 1);
        let cron_job = report
            .jobs
            .first()
            .unwrap_or_else(|| panic!("job should exist"));
        assert_eq!(cron_job.status, "needs_review");
        assert_eq!(
            cron_job
                .tikeo_job
                .as_ref()
                .unwrap_or_else(|| panic!("draft"))
                .schedule_type,
            "cron"
        );
        assert_eq!(
            cron_job
                .tikeo_job
                .as_ref()
                .unwrap_or_else(|| panic!("draft"))
                .processor_name
                .as_deref(),
            Some("billingProcessor")
        );
        assert!(
            cron_job
                .unsupported_features
                .iter()
                .any(|item| item.contains("route strategy"))
        );
        let manual = &report.jobs[1];
        assert_eq!(manual.status, "ready");
        assert_eq!(
            manual
                .tikeo_job
                .as_ref()
                .unwrap_or_else(|| panic!("draft"))
                .schedule_type,
            "api"
        );
        assert!(
            !manual
                .tikeo_job
                .as_ref()
                .unwrap_or_else(|| panic!("draft"))
                .enabled
        );
    }

    #[test]
    fn plans_powerjob_export_and_renders_markdown() {
        let input = r#"[
          {"id": 42, "jobName": "etl fanout", "appName": "data", "timeExpressionType": 4, "timeExpression": "PT30S", "processorInfo": "etlProcessor", "instanceRetryNum": 1, "executeType": "BROADCAST"}
        ]"#;

        let report = plan_migration(
            MigrationSource::PowerJob,
            input,
            MigrationDefaults {
                namespace: "default".to_owned(),
                app: "fallback".to_owned(),
            },
        )
        .unwrap_or_else(|error| panic!("report should plan: {error}"));

        assert_eq!(report.summary.needs_review, 1);
        let draft = report.jobs[0]
            .tikeo_job
            .as_ref()
            .unwrap_or_else(|| panic!("draft"));
        assert_eq!(draft.app, "data");
        assert_eq!(draft.schedule_type, "fixed_delay");
        assert_eq!(draft.schedule_expr.as_deref(), Some("PT30S"));
        assert!(
            report.jobs[0]
                .unsupported_features
                .iter()
                .any(|item| item.contains("executeType"))
        );
        let markdown = render_markdown_report(&report);
        assert!(markdown.contains("Tikeo migration report"));
        assert!(markdown.contains("etl fanout"));
    }
}
