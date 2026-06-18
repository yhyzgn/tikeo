//! Dedicated migration toolkit for moving legacy schedulers and Java workers to Tikeo.
//!
//! The default workflow is intentionally non-destructive: it reads a legacy scheduler export,
//! inspects an optional Java/Spring project, and writes a migration bundle containing Tikeo job
//! drafts, Java dependency guidance, source patches, and review notes. Live API writes require the
//! explicit `apply` command.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::{Column, Row};
use tikeo_core::{MisfirePolicy, ScheduleType};

/// tikeo-migrate command-line entrypoint.
#[derive(Debug, Parser)]
#[command(
    name = "tikeo-migrate",
    version,
    about = "Dedicated migration toolkit for moving legacy schedulers and Java workers to Tikeo"
)]
pub struct Cli {
    /// Command to execute.
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// Execute the selected command.
    ///
    /// # Errors
    ///
    /// Returns an error when migration planning, bundle writing, or API application fails.
    pub async fn run(self) -> Result<()> {
        match self.command {
            Command::Plan(command) => run_plan_command(&command).await,
            Command::Apply(command) => run_apply_command(&command).await,
        }
    }
}

/// Supported tikeo-migrate commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build a complete non-destructive migration bundle.
    Plan(PlanCommand),
    /// Apply ready job drafts from an existing migration bundle to a Tikeo server.
    #[command(name = "apply")]
    Apply(ApplyCommand),
}

/// Source scheduler supported by the migration planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
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

/// Output format for standalone reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MigrationReportFormat {
    /// Machine-readable JSON report.
    Json,
    /// Human-readable Markdown report.
    Markdown,
}

/// Build a complete non-destructive migration bundle.
#[derive(Debug, Clone, clap::Args)]
pub struct PlanCommand {
    /// Source scheduler export format. Auto-detected from export content/file name when omitted.
    #[arg(long, value_enum)]
    pub from: Option<MigrationSource>,
    /// Path to a pre-exported JSON file. Usually unnecessary: when omitted, the CLI first tries to export from the detected legacy database, then falls back to compatible JSON files.
    #[arg(long)]
    pub input: Option<PathBuf>,
    /// Legacy scheduler database URL. Auto-detected from Spring config when omitted. Supports MySQL/PostgreSQL JDBC and native URLs.
    #[arg(long, env = "TIKEO_MIGRATE_LEGACY_DB_URL")]
    pub legacy_db_url: Option<String>,
    /// Legacy scheduler database username when it is not embedded in the URL. Auto-detected from Spring config when omitted.
    #[arg(long, env = "TIKEO_MIGRATE_LEGACY_DB_USER")]
    pub legacy_db_user: Option<String>,
    /// Legacy scheduler database password when it is not embedded in the URL. Auto-detected from Spring config when omitted.
    #[arg(long, env = "TIKEO_MIGRATE_LEGACY_DB_PASSWORD")]
    pub legacy_db_password: Option<String>,
    /// Output directory for the migration bundle.
    #[arg(long, default_value = ".tikeo-migration")]
    pub output_dir: PathBuf,
    /// Optional legacy Java/Spring project root. Defaults to the current directory when it looks like a Java project.
    #[arg(long)]
    pub project: Option<PathBuf>,
    /// Optional standalone report output. The bundle always contains JSON and Markdown reports.
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Standalone report output format for --output.
    #[arg(long, value_enum, default_value = "json")]
    pub format: MigrationReportFormat,
    /// Default Tikeo namespace for generated job drafts.
    #[arg(long, default_value = "default")]
    pub namespace: String,
    /// Default Tikeo app for generated job drafts when source app/group is absent.
    #[arg(long, default_value = "default")]
    pub app: String,
    /// Tikeo dependency version placeholder or concrete version used in generated Java snippets.
    #[arg(long, default_value = "${TIKEO_VERSION}")]
    pub tikeo_version: String,
}

/// Apply ready job drafts from a bundle to Tikeo Management API.
#[derive(Debug, Clone, clap::Args)]
pub struct ApplyCommand {
    /// Migration bundle directory created by `tikeo-migrate plan`.
    #[arg(long, default_value = ".tikeo-migration")]
    pub bundle: PathBuf,
    /// Tikeo server endpoint, for example http://127.0.0.1:9090.
    #[arg(long)]
    pub endpoint: String,
    /// SDK API key. Prefer environment/secret injection in real runs.
    #[arg(long, env = "TIKEO_MIGRATION_API_KEY")]
    pub api_key: String,
    /// Also apply jobs marked needs_review. By default only ready jobs are sent.
    #[arg(long)]
    pub include_needs_review: bool,
    /// Do not call the API; write the request list that would be sent.
    #[arg(long)]
    pub dry_run: bool,
    /// Optional output path for apply evidence. Defaults to <bundle>/apply-evidence.json.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

/// Execute the non-destructive plan command.
///
/// # Errors
///
/// Returns an error when inputs cannot be read, parsed, inspected, or written.
pub async fn run_plan_command(command: &PlanCommand) -> Result<()> {
    let bundle = build_migration_bundle(command).await?;
    write_migration_bundle(&bundle, &command.output_dir)?;
    if let Some(output) = &command.output {
        let rendered = match command.format {
            MigrationReportFormat::Json => serde_json::to_string_pretty(&bundle.report)?,
            MigrationReportFormat::Markdown => render_markdown_report(&bundle.report),
        };
        fs::write(output, rendered)
            .with_context(|| format!("failed to write migration report {}", output.display()))?;
    }
    println!(
        "migration bundle written to {}",
        command.output_dir.display()
    );
    Ok(())
}

/// Execute the data-apply command.
///
/// # Errors
///
/// Returns an error when the bundle cannot be read or the Tikeo API rejects a request.
pub async fn run_apply_command(command: &ApplyCommand) -> Result<()> {
    let report_path = command.bundle.join("jobs.tikeo.json");
    let report_text = fs::read_to_string(&report_path).with_context(|| {
        format!(
            "failed to read migration bundle report {}",
            report_path.display()
        )
    })?;
    let report: MigrationReport = serde_json::from_str(&report_text).with_context(|| {
        format!(
            "failed to parse migration bundle report {}",
            report_path.display()
        )
    })?;
    let evidence = apply_data(command, &report).await?;
    let output = command
        .output
        .clone()
        .unwrap_or_else(|| command.bundle.join("apply-evidence.json"));
    fs::write(&output, serde_json::to_string_pretty(&evidence)?)
        .with_context(|| format!("failed to write apply evidence {}", output.display()))?;
    println!("apply evidence written to {}", output.display());
    Ok(())
}

#[derive(Debug, Clone)]
struct ResolvedPlanInputs {
    source: MigrationSource,
    input_origin: String,
    export_json: String,
    project: Option<PathBuf>,
}

async fn resolve_plan_inputs(command: &PlanCommand) -> Result<ResolvedPlanInputs> {
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    resolve_plan_inputs_from(command, &cwd).await
}

async fn resolve_plan_inputs_from(command: &PlanCommand, cwd: &Path) -> Result<ResolvedPlanInputs> {
    let project = command
        .project
        .clone()
        .or_else(|| looks_like_java_project(cwd).then_some(cwd.to_path_buf()));
    if let Some(input) = &command.input {
        let input_text = fs::read_to_string(input)
            .with_context(|| format!("failed to read migration input {}", input.display()))?;
        let source = command
            .from
            .or_else(|| infer_source_from_path(input))
            .or_else(|| infer_source_from_json(&input_text))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "failed to auto-detect source scheduler; pass --from xxl-job or --from powerjob"
                )
            })?;
        return Ok(ResolvedPlanInputs {
            source,
            input_origin: format!("json-file:{}", input.display()),
            export_json: input_text,
            project,
        });
    }

    if let Some(export) =
        export_from_legacy_database(command, project.as_deref().unwrap_or(cwd)).await?
    {
        return Ok(ResolvedPlanInputs {
            source: export.source,
            input_origin: export.origin,
            export_json: export.json,
            project,
        });
    }

    let input = find_export_file(project.as_deref().unwrap_or(cwd))?;
    let input_text = fs::read_to_string(&input)
        .with_context(|| format!("failed to read migration input {}", input.display()))?;
    let source = command
        .from
        .or_else(|| infer_source_from_path(&input))
        .or_else(|| infer_source_from_json(&input_text))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "failed to auto-detect source scheduler; pass --from xxl-job or --from powerjob"
            )
        })?;
    Ok(ResolvedPlanInputs {
        source,
        input_origin: format!("json-file:{}", input.display()),
        export_json: input_text,
        project,
    })
}

#[derive(Debug, Clone)]
struct LegacyDatabaseExport {
    source: MigrationSource,
    origin: String,
    json: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LegacyDbConfig {
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

async fn export_from_legacy_database(
    command: &PlanCommand,
    project_root: &Path,
) -> Result<Option<LegacyDatabaseExport>> {
    let config = resolve_legacy_db_config(command, project_root)?;
    let Some(raw_url) = config.url else {
        return Ok(None);
    };
    let source = command
        .from
        .or_else(|| infer_source_from_project(project_root))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "legacy database URL was detected but source scheduler was not; pass --from xxl-job or --from powerjob"
            )
        })?;
    let database_url = normalize_database_url(
        &raw_url,
        config.username.as_deref(),
        config.password.as_deref(),
    )?;
    let rows = export_legacy_rows(source, &database_url)
        .await
        .with_context(|| {
            format!(
                "failed to export {} jobs from detected legacy database",
                source.as_str()
            )
        })?;
    Ok(Some(LegacyDatabaseExport {
        source,
        origin: format!("legacy-db:{}", redact_database_url(&database_url)),
        json: serde_json::to_string(&json!({"jobs": rows}))?,
    }))
}

fn resolve_legacy_db_config(command: &PlanCommand, project_root: &Path) -> Result<LegacyDbConfig> {
    let mut config = read_legacy_db_config(project_root)?;
    if command.legacy_db_url.is_some() {
        config.url = command.legacy_db_url.clone();
    }
    if command.legacy_db_user.is_some() {
        config.username = command.legacy_db_user.clone();
    }
    if command.legacy_db_password.is_some() {
        config.password = command.legacy_db_password.clone();
    }
    Ok(config)
}

fn read_legacy_db_config(project_root: &Path) -> Result<LegacyDbConfig> {
    let mut config = LegacyDbConfig::default();
    let candidates = [
        "src/main/resources/application.properties",
        "src/main/resources/application.yml",
        "src/main/resources/application.yaml",
        "src/main/resources/bootstrap.properties",
        "src/main/resources/bootstrap.yml",
        "src/main/resources/bootstrap.yaml",
        "application.properties",
        "application.yml",
        "application.yaml",
    ];
    for relative in candidates {
        let path = project_root.join(relative);
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read legacy config {}", path.display()))?;
        merge_legacy_db_config_from_text(&mut config, &text);
    }
    Ok(config)
}

fn merge_legacy_db_config_from_text(config: &mut LegacyDbConfig, text: &str) {
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let normalized = line.trim_start_matches('-').trim();
        let Some((key, value)) = split_config_line(normalized) else {
            continue;
        };
        let key = key
            .trim()
            .trim_matches(|ch| ch == '"' || ch == '\'')
            .to_ascii_lowercase();
        let value = strip_inline_comment(value.trim())
            .trim()
            .trim_matches(|ch| ch == '"' || ch == '\'')
            .to_owned();
        if value.is_empty() {
            continue;
        }
        match key.as_str() {
            "spring.datasource.url" | "datasource.url" | "jdbc-url" | "url"
                if looks_like_db_url(&value) =>
            {
                config.url.get_or_insert(value);
            }
            "spring.datasource.username" | "datasource.username" | "username" | "user" => {
                config.username.get_or_insert(value);
            }
            "spring.datasource.password" | "datasource.password" | "password" => {
                config.password.get_or_insert(value);
            }
            _ => {}
        }
    }
}

fn split_config_line(line: &str) -> Option<(&str, &str)> {
    if let Some(index) = line.find('=') {
        Some((&line[..index], &line[index + 1..]))
    } else if let Some(index) = line.find(':') {
        Some((&line[..index], &line[index + 1..]))
    } else {
        None
    }
}

fn strip_inline_comment(value: &str) -> &str {
    value.split(" #").next().unwrap_or(value)
}

fn looks_like_db_url(value: &str) -> bool {
    value.starts_with("jdbc:mysql://")
        || value.starts_with("jdbc:postgresql://")
        || value.starts_with("mysql://")
        || value.starts_with("postgres://")
        || value.starts_with("postgresql://")
}

fn infer_source_from_project(project_root: &Path) -> Option<MigrationSource> {
    let mut stack = vec![project_root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(&path).ok()?;
        for entry in entries.flatten() {
            let child = entry.path();
            let name = child
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if child.is_dir() {
                if matches!(
                    name,
                    "build" | "target" | ".gradle" | ".git" | "node_modules"
                ) {
                    continue;
                }
                stack.push(child);
            } else if matches!(
                child.extension().and_then(|ext| ext.to_str()),
                Some("java" | "xml" | "gradle" | "kts" | "properties" | "yml" | "yaml")
            ) {
                let text = fs::read_to_string(&child).ok()?;
                let lower = text.to_ascii_lowercase();
                if lower.contains("xxl-job")
                    || lower.contains("xxljob")
                    || lower.contains("@xxljob")
                {
                    return Some(MigrationSource::XxlJob);
                }
                if lower.contains("powerjob")
                    || lower.contains("tech.powerjob")
                    || lower.contains("basicprocessor")
                {
                    return Some(MigrationSource::PowerJob);
                }
            }
        }
    }
    None
}

fn normalize_database_url(
    raw: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    let mut url = raw.trim().to_owned();
    if let Some(rest) = url.strip_prefix("jdbc:mysql://") {
        url = format!("mysql://{rest}");
    } else if let Some(rest) = url.strip_prefix("jdbc:postgresql://") {
        url = format!("postgres://{rest}");
    } else if let Some(rest) = url.strip_prefix("postgresql://") {
        url = format!("postgres://{rest}");
    }
    if !(url.starts_with("mysql://") || url.starts_with("postgres://")) {
        bail!("unsupported legacy database URL; only MySQL/PostgreSQL URLs can be auto-exported")
    }
    inject_database_credentials(&url, username, password)
}

fn inject_database_credentials(
    url: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    let Some(username) = username.filter(|value| !value.is_empty()) else {
        return Ok(url.to_owned());
    };
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| anyhow::anyhow!("invalid database URL"))?;
    if rest.contains('@') {
        return Ok(url.to_owned());
    }
    let encoded_user = percent_encode_credential(username);
    let encoded_password = password.map(percent_encode_credential).unwrap_or_default();
    Ok(if password.is_some() {
        format!("{scheme}://{encoded_user}:{encoded_password}@{rest}")
    } else {
        format!("{scheme}://{encoded_user}@{rest}")
    })
}

fn percent_encode_credential(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

fn redact_database_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return "<redacted>".to_owned();
    };
    if let Some((_, host)) = rest.split_once('@') {
        format!("{scheme}://***:***@{host}")
    } else {
        url.to_owned()
    }
}

async fn export_legacy_rows(source: MigrationSource, database_url: &str) -> Result<Vec<Value>> {
    sqlx::any::install_default_drivers();
    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await?;
    let queries = match source {
        MigrationSource::XxlJob => [
            "select * from xxl_job_info order by id",
            "select * from XXL_JOB_INFO order by id",
            "select * from job_info order by id",
        ],
        MigrationSource::PowerJob => [
            "select * from pj_job_info order by id",
            "select * from job_info order by id",
            "select * from powerjob_job_info order by id",
        ],
    };
    let mut last_error = None;
    for query in queries {
        match sqlx::query(query).fetch_all(&pool).await {
            Ok(rows) => {
                return Ok(rows.iter().map(row_to_json).collect());
            }
            Err(error) => last_error = Some(error),
        }
    }
    if let Some(error) = last_error {
        bail!(error)
    }
    Ok(Vec::new())
}

fn row_to_json(row: &sqlx::any::AnyRow) -> Value {
    let mut object = Map::new();
    for column in row.columns() {
        let name = column.name();
        let value = row
            .try_get::<String, _>(name)
            .map(Value::String)
            .or_else(|_| row.try_get::<i64, _>(name).map(|value| json!(value)))
            .or_else(|_| row.try_get::<i32, _>(name).map(|value| json!(value)))
            .or_else(|_| row.try_get::<bool, _>(name).map(|value| json!(value)))
            .or_else(|_| row.try_get::<f64, _>(name).map(|value| json!(value)))
            .unwrap_or(Value::Null);
        object.insert(to_camelish(name), value);
    }
    Value::Object(object)
}

fn to_camelish(name: &str) -> String {
    let mut output = String::new();
    let mut uppercase_next = false;
    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            uppercase_next = true;
        } else if uppercase_next {
            output.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            output.push(ch);
        }
    }
    output
}

fn looks_like_java_project(path: &Path) -> bool {
    path.join("pom.xml").exists()
        || path.join("build.gradle").exists()
        || path.join("build.gradle.kts").exists()
}

fn find_export_file(root: &Path) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    let names = [
        "tikeo-migration.json",
        "xxl-job-export.json",
        "xxljob-export.json",
        "powerjob-export.json",
        "power-job-export.json",
        "jobs-export.json",
    ];
    for name in names {
        let path = root.join(name);
        if path.exists() {
            candidates.push(path);
        }
    }
    for dir in [
        root.to_path_buf(),
        root.join("export"),
        root.join("exports"),
        root.join("migration"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        for entry in
            fs::read_dir(&dir).with_context(|| format!("failed to scan {}", dir.display()))?
        {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        let lower = name.to_ascii_lowercase();
                        lower.contains("xxl") || lower.contains("powerjob") || lower.contains("job")
                    })
            {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    let detectable = candidates
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|text| infer_source_from_json(&text))
                .is_some()
                || infer_source_from_path(path).is_some()
        })
        .collect::<Vec<_>>();
    match detectable.as_slice() {
        [single] => Ok(single.clone()),
        [] => bail!(
            "failed to auto-detect legacy export JSON under {}; pass --input",
            root.display()
        ),
        many => bail!(
            "multiple possible legacy export JSON files found ({}); pass --input",
            many.iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn infer_source_from_path(path: &Path) -> Option<MigrationSource> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.contains("xxl") {
        Some(MigrationSource::XxlJob)
    } else if name.contains("powerjob") || name.contains("power-job") {
        Some(MigrationSource::PowerJob)
    } else {
        None
    }
}

fn infer_source_from_json(input: &str) -> Option<MigrationSource> {
    let lower = input.to_ascii_lowercase();
    if lower.contains("executorhandler")
        || lower.contains("executor_handler")
        || lower.contains("jobdesc")
        || (lower.contains("scheduletype") && lower.contains("scheduleconf"))
    {
        Some(MigrationSource::XxlJob)
    } else if lower.contains("processorinfo")
        || lower.contains("timeexpressiontype")
        || lower.contains("instanceretrynum")
        || lower.contains("executetype")
    {
        Some(MigrationSource::PowerJob)
    } else {
        None
    }
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
    /// Planner mode.
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
    /// Optional plugin processor type.
    pub processor_type: Option<String>,
    /// Optional script id.
    pub script_id: Option<String>,
    /// Whether the imported job should be enabled.
    pub enabled: bool,
    /// Retry policy draft.
    pub retry_policy: Value,
    /// Migration metadata for audit/review.
    pub metadata: Value,
}

/// Complete non-destructive migration bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationBundle {
    /// Source scheduler family.
    pub source: String,
    /// Data migration report.
    pub report: MigrationReport,
    /// Ready-to-review data import plan.
    pub data_import: DataImportPlan,
    /// Optional Java project migration plan.
    pub java_project: Option<JavaProjectMigrationPlan>,
    /// Ordered checklist for humans/operators.
    pub checklist: Vec<String>,
}

/// Data import plan extracted from the report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataImportPlan {
    /// Ready drafts that can be applied by default.
    pub ready: Vec<TikeoJobDraft>,
    /// Drafts that require human review before application.
    pub needs_review: Vec<TikeoJobDraft>,
    /// Jobs skipped because no draft was safe to create.
    pub skipped: Vec<String>,
}

/// Java/Spring project migration plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaProjectMigrationPlan {
    /// Project root inspected.
    pub project_root: String,
    /// Detected build system.
    pub build_system: String,
    /// Detected Spring Boot major version when known.
    pub spring_boot_major: Option<u8>,
    /// Tikeo artifact that should be added.
    pub recommended_artifact: String,
    /// Dependency snippet for the detected build system.
    pub dependency_snippet: String,
    /// Files with generated patches.
    pub patches: Vec<ProjectPatch>,
    /// Handler candidates discovered in source.
    pub handler_candidates: Vec<HandlerCandidate>,
    /// Review notes and limitations.
    pub review_notes: Vec<String>,
}

/// One generated project patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatch {
    /// Relative project path.
    pub path: String,
    /// Patch kind.
    pub kind: String,
    /// Unified diff or insertion guidance.
    pub diff: String,
}

/// Legacy handler candidate discovered from Java source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandlerCandidate {
    /// Relative source file path.
    pub path: String,
    /// Legacy framework detected.
    pub framework: String,
    /// Processor name to use in Tikeo.
    pub processor_name: String,
    /// Method name when detected.
    pub method_name: Option<String>,
}

/// Evidence for applying job drafts to Tikeo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyEvidence {
    /// Whether this run avoided live API calls.
    pub dry_run: bool,
    /// Requests attempted or planned.
    pub requests: Vec<ApplyRequestEvidence>,
}

/// Evidence for one apply request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequestEvidence {
    /// Job name.
    pub name: String,
    /// Request status.
    pub status: String,
    /// HTTP status code when live API was called.
    pub http_status: Option<u16>,
    /// Response or error body.
    pub response: Option<String>,
}

/// Build the full migration bundle in memory.
///
/// # Errors
///
/// Returns an error when the legacy export or optional project cannot be read.
pub async fn build_migration_bundle(command: &PlanCommand) -> Result<MigrationBundle> {
    let resolved = resolve_plan_inputs(command).await?;
    let report = plan_migration(
        resolved.source,
        &resolved.export_json,
        MigrationDefaults {
            namespace: command.namespace.clone(),
            app: command.app.clone(),
        },
    )?;
    let data_import = build_data_import_plan(&report);
    let java_project = resolved
        .project
        .as_ref()
        .map(|project| scan_java_project(project, resolved.source, &command.tikeo_version))
        .transpose()?;
    let mut checklist = vec![
        format!("Input source captured as `{}`; keep the generated manifest as the immutable audit source for this migration run.", resolved.input_origin),
        "Review jobs.needsReview before applying them; ready jobs are the only default apply set.".to_owned(),
        "Apply Java project patches in a branch, run unit tests, then start a Worker against a staging Tikeo Server.".to_owned(),
        "Run tikeo-migrate apply against staging, trigger one migrated job, and compare logs/results with the legacy scheduler.".to_owned(),
        "Keep legacy and Tikeo in dual-run until instance outcomes and operator evidence match.".to_owned(),
    ];
    if java_project.is_none() {
        checklist.push("No Java project was scanned; code migration remains manual until --project is provided.".to_owned());
    }
    Ok(MigrationBundle {
        source: resolved.source.as_str().to_owned(),
        report,
        data_import,
        java_project,
        checklist,
    })
}

fn build_data_import_plan(report: &MigrationReport) -> DataImportPlan {
    let mut ready = Vec::new();
    let mut needs_review = Vec::new();
    let mut skipped = Vec::new();
    for job in &report.jobs {
        match (job.status.as_str(), job.tikeo_job.clone()) {
            ("ready", Some(draft)) => ready.push(draft),
            ("needs_review", Some(draft)) => needs_review.push(draft),
            _ => skipped.push(job.source_id.clone()),
        }
    }
    DataImportPlan {
        ready,
        needs_review,
        skipped,
    }
}

fn write_migration_bundle(bundle: &MigrationBundle, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir.join("java-patches")).with_context(|| {
        format!(
            "failed to create migration bundle directory {}",
            output_dir.display()
        )
    })?;
    fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_string_pretty(bundle)?,
    )?;
    fs::write(
        output_dir.join("jobs.tikeo.json"),
        serde_json::to_string_pretty(&bundle.report)?,
    )?;
    fs::write(
        output_dir.join("jobs.tikeo.md"),
        render_markdown_report(&bundle.report),
    )?;
    fs::write(
        output_dir.join("data-import-plan.json"),
        serde_json::to_string_pretty(&bundle.data_import)?,
    )?;
    fs::write(output_dir.join("CHECKLIST.md"), render_checklist(bundle))?;
    if let Some(project) = &bundle.java_project {
        fs::write(
            output_dir.join("java-project-plan.json"),
            serde_json::to_string_pretty(project)?,
        )?;
        fs::write(
            output_dir.join("java-project-plan.md"),
            render_java_project_plan(project),
        )?;
        for patch in &project.patches {
            let safe_name = patch.path.replace(['/', '\\'], "__");
            fs::write(
                output_dir
                    .join("java-patches")
                    .join(format!("{safe_name}.patch")),
                &patch.diff,
            )?;
        }
    }
    Ok(())
}

fn render_checklist(bundle: &MigrationBundle) -> String {
    let mut output = format!(
        "# Tikeo migration checklist\n\nSource: `{}`\n\n",
        bundle.source
    );
    for (index, item) in bundle.checklist.iter().enumerate() {
        output.push_str(&format!("{}. {item}\n", index + 1));
    }
    output
}

fn scan_java_project(
    project: &Path,
    source: MigrationSource,
    tikeo_version: &str,
) -> Result<JavaProjectMigrationPlan> {
    if !project.is_dir() {
        bail!(
            "Java project path is not a directory: {}",
            project.display()
        );
    }
    let build_system = detect_build_system(project);
    let spring_boot_major = detect_spring_boot_major(project)?;
    let recommended_artifact = recommended_artifact(spring_boot_major).to_owned();
    let dependency_snippet =
        dependency_snippet(&build_system, &recommended_artifact, tikeo_version);
    let mut handler_candidates = Vec::new();
    let mut patches = Vec::new();
    collect_java_handlers(
        project,
        project,
        source,
        &mut handler_candidates,
        &mut patches,
    )?;
    if let Some(patch) =
        dependency_patch(project, &build_system, &recommended_artifact, tikeo_version)?
    {
        patches.insert(0, patch);
    }
    let mut review_notes = vec![
        "Generated patches are review-first and should be applied on a branch.".to_owned(),
        "Tikeo @TikeoProcessor methods support TaskContext, String, byte[], void, String, boolean, and TaskOutcome shapes; complex legacy signatures need manual adapters.".to_owned(),
    ];
    if handler_candidates.is_empty() {
        review_notes.push("No legacy Java handler annotations/interfaces were detected; code migration may require manual mapping.".to_owned());
    }
    Ok(JavaProjectMigrationPlan {
        project_root: project.display().to_string(),
        build_system,
        spring_boot_major,
        recommended_artifact,
        dependency_snippet,
        patches,
        handler_candidates,
        review_notes,
    })
}

fn detect_build_system(project: &Path) -> String {
    if project.join("pom.xml").exists() {
        "maven".to_owned()
    } else if project.join("build.gradle.kts").exists() {
        "gradle-kotlin".to_owned()
    } else if project.join("build.gradle").exists() {
        "gradle-groovy".to_owned()
    } else {
        "unknown".to_owned()
    }
}

fn detect_spring_boot_major(project: &Path) -> Result<Option<u8>> {
    let mut content = String::new();
    for file in ["pom.xml", "build.gradle.kts", "build.gradle"] {
        let path = project.join(file);
        if path.exists() {
            content.push_str(
                &fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?,
            );
        }
    }
    for major in [4_u8, 3, 2] {
        if content.contains(&format!(
            "spring-boot-starter-parent</artifactId>\n        <version>{major}."
        )) || content.contains(&format!(
            "spring-boot-starter-parent</artifactId><version>{major}."
        )) || content.contains(&format!("org.springframework.boot:spring-boot"))
            && content.contains(&format!("{major}."))
            || content.contains(&format!(
                "id(\"org.springframework.boot\") version \"{major}."
            ))
            || content.contains(&format!("id 'org.springframework.boot' version '{major}."))
            || content.contains(&format!("springBootVersion = \"{major}."))
            || content.contains(&format!("springBootVersion='{major}."))
        {
            return Ok(Some(major));
        }
    }
    Ok(None)
}

fn recommended_artifact(spring_boot_major: Option<u8>) -> &'static str {
    match spring_boot_major {
        Some(2) => "tikeo-spring-boot2-starter",
        Some(3) => "tikeo-spring-boot3-starter",
        _ => "tikeo-spring-boot-starter",
    }
}

fn dependency_snippet(build_system: &str, artifact: &str, version: &str) -> String {
    match build_system {
        "maven" => format!(
            "<dependency>\n  <groupId>net.tikeo</groupId>\n  <artifactId>{artifact}</artifactId>\n  <version>{version}</version>\n</dependency>"
        ),
        "gradle-kotlin" => format!("implementation(\"net.tikeo:{artifact}:{version}\")"),
        "gradle-groovy" => format!("implementation 'net.tikeo:{artifact}:{version}'"),
        _ => format!("Add dependency net.tikeo:{artifact}:{version}"),
    }
}

fn dependency_patch(
    project: &Path,
    build_system: &str,
    artifact: &str,
    version: &str,
) -> Result<Option<ProjectPatch>> {
    let (relative, marker, insertion) = match build_system {
        "maven" => (
            "pom.xml",
            "</dependencies>",
            format!(
                "  {}\n",
                indent(&dependency_snippet(build_system, artifact, version), 2)
            ),
        ),
        "gradle-kotlin" => (
            "build.gradle.kts",
            "dependencies {",
            format!(
                "    {}\n",
                dependency_snippet(build_system, artifact, version)
            ),
        ),
        "gradle-groovy" => (
            "build.gradle",
            "dependencies {",
            format!(
                "    {}\n",
                dependency_snippet(build_system, artifact, version)
            ),
        ),
        _ => return Ok(None),
    };
    let path = project.join(relative);
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    if content.contains(&format!("net.tikeo:{artifact}"))
        || content.contains(&format!("<artifactId>{artifact}</artifactId>"))
    {
        return Ok(None);
    }
    let diff = format!(
        "--- a/{relative}\n+++ b/{relative}\n@@\n {marker}\n+{}",
        insertion.replace('\n', "\n+").trim_end_matches('+')
    );
    Ok(Some(ProjectPatch {
        path: relative.to_owned(),
        kind: "dependency".to_owned(),
        diff,
    }))
}

fn indent(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_java_handlers(
    root: &Path,
    directory: &Path,
    source: MigrationSource,
    handlers: &mut Vec<HandlerCandidate>,
    patches: &mut Vec<ProjectPatch>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| matches!(name, "build" | "target" | ".gradle" | ".git"))
            {
                continue;
            }
            collect_java_handlers(root, &path, source, handlers, patches)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("java") {
            inspect_java_file(root, &path, source, handlers, patches)?;
        }
    }
    Ok(())
}

fn inspect_java_file(
    root: &Path,
    path: &Path,
    source: MigrationSource,
    handlers: &mut Vec<HandlerCandidate>,
    patches: &mut Vec<ProjectPatch>,
) -> Result<()> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    let candidates = match source {
        MigrationSource::XxlJob => detect_xxl_handlers(&content),
        MigrationSource::PowerJob => detect_powerjob_handlers(&content),
    };
    if candidates.is_empty() {
        return Ok(());
    }
    for (processor_name, method_name) in &candidates {
        handlers.push(HandlerCandidate {
            path: relative.clone(),
            framework: source.as_str().to_owned(),
            processor_name: processor_name.clone(),
            method_name: method_name.clone(),
        });
    }
    patches.push(ProjectPatch {
        path: relative,
        kind: "java-processor-annotations".to_owned(),
        diff: java_patch_guidance(&content, &candidates),
    });
    Ok(())
}

fn detect_xxl_handlers(content: &str) -> Vec<(String, Option<String>)> {
    let mut handlers = Vec::new();
    for line in content.lines() {
        if let Some(start) = line.find("@XxlJob(") {
            let rest = &line[start..];
            if let Some(name) = first_quoted(rest) {
                handlers.push((name, None));
            }
        }
        if line.contains("extends IJobHandler") || line.contains("implements IJobHandler") {
            handlers.push((
                "TODO_from_xxl_class_handler".to_owned(),
                Some("execute".to_owned()),
            ));
        }
    }
    handlers
}

fn detect_powerjob_handlers(content: &str) -> Vec<(String, Option<String>)> {
    let mut handlers = Vec::new();
    if content.contains("BasicProcessor")
        || content.contains("MapProcessor")
        || content.contains("BroadcastProcessor")
        || content.contains("ProcessorBean")
    {
        let name =
            class_name(content).map_or_else(|| "TODO_powerjob_processor".to_owned(), |name| name);
        handlers.push((name, Some("process".to_owned())));
    }
    handlers
}

fn first_quoted(value: &str) -> Option<String> {
    let start = value.find('"')? + 1;
    let end = value[start..].find('"')? + start;
    Some(value[start..end].to_owned())
}

fn class_name(content: &str) -> Option<String> {
    for token in [" class ", " record "] {
        if let Some(index) = content.find(token) {
            let rest = &content[index + token.len()..];
            let name = rest
                .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .next()?;
            if !name.is_empty() {
                return Some(name.to_owned());
            }
        }
    }
    None
}

fn java_patch_guidance(content: &str, candidates: &[(String, Option<String>)]) -> String {
    let mut diff = String::new();
    diff.push_str("--- legacy.java\n+++ tikeo-migrated.java\n@@\n");
    if !content.contains("net.tikeo.processor.TikeoProcessor") {
        diff.push_str("+import net.tikeo.processor.TikeoProcessor;\n");
    }
    diff.push_str("@@\n");
    for (processor_name, method_name) in candidates {
        diff.push_str(&format!(
            "+// Add @TikeoProcessor(\"{processor_name}\") to the migrated handler{}; adapt parameters to String, byte[], or TaskContext when needed.\n",
            method_name.as_ref().map_or(String::new(), |name| format!(" method `{name}`"))
        ));
    }
    diff
}

fn render_java_project_plan(project: &JavaProjectMigrationPlan) -> String {
    let mut output = format!(
        "# Java project migration plan\n\n- Project: `{}`\n- Build system: `{}`\n- Spring Boot major: `{}`\n- Recommended artifact: `net.tikeo:{}`\n\n## Dependency\n\n```\n{}\n```\n\n",
        project.project_root,
        project.build_system,
        project
            .spring_boot_major
            .map_or_else(|| "unknown".to_owned(), |major| major.to_string()),
        project.recommended_artifact,
        project.dependency_snippet
    );
    output.push_str("## Handler candidates\n\n");
    for handler in &project.handler_candidates {
        output.push_str(&format!(
            "- `{}` → `{}` ({})\n",
            handler.path, handler.processor_name, handler.framework
        ));
    }
    if project.handler_candidates.is_empty() {
        output.push_str("- No legacy handlers detected.\n");
    }
    output.push_str("\n## Review notes\n\n");
    for note in &project.review_notes {
        output.push_str(&format!("- {note}\n"));
    }
    output
}

async fn apply_data(command: &ApplyCommand, report: &MigrationReport) -> Result<ApplyEvidence> {
    let mut drafts = Vec::new();
    for job in &report.jobs {
        if job.status == "ready" || (command.include_needs_review && job.status == "needs_review") {
            if let Some(draft) = &job.tikeo_job {
                drafts.push(draft.clone());
            }
        }
    }
    let mut requests = Vec::new();
    if command.dry_run {
        for draft in drafts {
            requests.push(ApplyRequestEvidence {
                name: draft.name,
                status: "planned".to_owned(),
                http_status: None,
                response: None,
            });
        }
        return Ok(ApplyEvidence {
            dry_run: true,
            requests,
        });
    }
    let client = reqwest::Client::new();
    let endpoint = command.endpoint.trim_end_matches('/');
    for draft in drafts {
        let response = client
            .post(format!("{endpoint}/api/v1/jobs"))
            .header("x-tikeo-api-key", &command.api_key)
            .json(&draft)
            .send()
            .await
            .with_context(|| format!("failed to apply job {}", draft.name))?;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        requests.push(ApplyRequestEvidence {
            name: draft.name,
            status: if (200..300).contains(&status) {
                "applied"
            } else {
                "failed"
            }
            .to_owned(),
            http_status: Some(status),
            response: Some(body),
        });
    }
    Ok(ApplyEvidence {
        dry_run: false,
        requests,
    })
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
    collect_if_present(
        record,
        &mut unsupported_features,
        "childJobId",
        "XXL-JOB childJobId should be migrated to a Tikeo Workflow edge set",
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
            processor_type: None,
            script_id: None,
            enabled,
            retry_policy: json!({"enabled": retry_count > 0, "maxAttempts": retry_count + 1, "initialDelaySeconds": 30, "backoffMultiplier": 2.0, "maxDelaySeconds": 300}),
            metadata: json!({"migratedFrom": "xxl-job", "sourceId": source_id, "executorParam": string_field(record, &["executorParam", "executor_param"]), "executorTimeoutSeconds": integer_field(record, &["executorTimeout", "executor_timeout"])}),
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
            processor_type: None,
            script_id: None,
            enabled,
            retry_policy: json!({"enabled": retry_count > 0, "maxAttempts": retry_count + 1, "initialDelaySeconds": 30, "backoffMultiplier": 2.0, "maxDelaySeconds": 300}),
            metadata: json!({"migratedFrom": "powerjob", "sourceId": source_id, "processorType": string_field(record, &["processorType", "processor_type"]), "jobParams": string_field(record, &["jobParams", "job_params", "instanceParams"])}),
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

    fn fixture_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        fs::write(dir.path().join("build.gradle.kts"), "plugins { id(\"org.springframework.boot\") version \"3.5.8\" }\ndependencies {\n    implementation(\"com.xuxueli:xxl-job-core:2.4.1\")\n}\n").unwrap_or_else(|error| panic!("write gradle: {error}"));
        let source_dir = dir.path().join("src/main/java/com/example");
        fs::create_dir_all(&source_dir).unwrap_or_else(|error| panic!("mkdir: {error}"));
        fs::write(source_dir.join("BillingJob.java"), "package com.example;\nimport com.xxl.job.core.handler.annotation.XxlJob;\nclass BillingJob {\n  @XxlJob(\"billingProcessor\")\n  public void execute() {}\n}\n").unwrap_or_else(|error| panic!("write java: {error}"));
        dir
    }

    #[test]
    fn plans_xxl_job_export_into_tikeo_drafts_with_review_flags() {
        let input = r#"{"jobs":[{"id":7,"jobDesc":"nightly billing","scheduleType":"CRON","scheduleConf":"0 0 2 * * ?","executorHandler":"billingProcessor","executorFailRetryCount":2,"triggerStatus":1,"executorRouteStrategy":"ROUND"},{"id":8,"jobDesc":"manual cleanup","scheduleType":"NONE","executorHandler":"cleanupProcessor","triggerStatus":0}]}"#;
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
    fn builds_complete_bundle_with_java_project_patches() {
        let project = fixture_project();
        let input = project.path().join("xxl-export.json");
        fs::write(&input, r#"{"jobs":[{"id":7,"jobDesc":"nightly billing","scheduleType":"CRON","scheduleConf":"0 0 2 * * ?","executorHandler":"billingProcessor","triggerStatus":1}]}"#).unwrap_or_else(|error| panic!("write input: {error}"));
        let command = PlanCommand {
            from: Some(MigrationSource::XxlJob),
            input: Some(input),
            legacy_db_url: None,
            legacy_db_user: None,
            legacy_db_password: None,
            output_dir: project.path().join("bundle"),
            project: Some(project.path().to_path_buf()),
            output: None,
            format: MigrationReportFormat::Json,
            namespace: "ops".to_owned(),
            app: "billing".to_owned(),
            tikeo_version: "${TIKEO_VERSION}".to_owned(),
        };
        let bundle = tokio::runtime::Runtime::new()
            .unwrap_or_else(|error| panic!("runtime: {error}"))
            .block_on(build_migration_bundle(&command))
            .unwrap_or_else(|error| panic!("bundle should build: {error}"));
        let java = bundle
            .java_project
            .as_ref()
            .unwrap_or_else(|| panic!("java plan"));
        assert_eq!(java.spring_boot_major, Some(3));
        assert_eq!(java.recommended_artifact, "tikeo-spring-boot3-starter");
        assert!(
            java.handler_candidates
                .iter()
                .any(|handler| handler.processor_name == "billingProcessor")
        );
        assert!(java.patches.iter().any(|patch| patch.kind == "dependency"));
        write_migration_bundle(&bundle, &command.output_dir)
            .unwrap_or_else(|error| panic!("bundle writes: {error}"));
        assert!(command.output_dir.join("manifest.json").exists());
        assert!(command.output_dir.join("java-project-plan.md").exists());
    }

    #[test]
    fn detects_legacy_database_config_from_spring_properties() {
        let project = fixture_project();
        let resources = project.path().join("src/main/resources");
        fs::create_dir_all(&resources).unwrap_or_else(|error| panic!("resources: {error}"));
        fs::write(
            resources.join("application.properties"),
            "spring.datasource.url=jdbc:mysql://127.0.0.1:3306/xxl_job\nspring.datasource.username=xxl\nspring.datasource.password=s3 cr3t\n",
        )
        .unwrap_or_else(|error| panic!("write properties: {error}"));

        let config = read_legacy_db_config(project.path())
            .unwrap_or_else(|error| panic!("db config should parse: {error}"));
        assert_eq!(
            config.url.as_deref(),
            Some("jdbc:mysql://127.0.0.1:3306/xxl_job")
        );
        assert_eq!(config.username.as_deref(), Some("xxl"));
        assert_eq!(config.password.as_deref(), Some("s3 cr3t"));
        assert_eq!(
            infer_source_from_project(project.path()),
            Some(MigrationSource::XxlJob)
        );
        let normalized = normalize_database_url(
            config.url.as_deref().unwrap_or_default(),
            config.username.as_deref(),
            config.password.as_deref(),
        )
        .unwrap_or_else(|error| panic!("url should normalize: {error}"));
        assert_eq!(normalized, "mysql://xxl:s3%20cr3t@127.0.0.1:3306/xxl_job");
    }

    #[test]
    fn resolves_zero_parameter_project_root_convention() {
        let project = fixture_project();
        fs::write(project.path().join("xxl-job-export.json"), r#"{"jobs":[{"id":7,"jobDesc":"nightly billing","scheduleType":"CRON","scheduleConf":"0 0 2 * * ?","executorHandler":"billingProcessor","triggerStatus":1}]}"#)
            .unwrap_or_else(|error| panic!("write input: {error}"));
        let command = PlanCommand {
            from: None,
            input: None,
            legacy_db_url: None,
            legacy_db_user: None,
            legacy_db_password: None,
            output_dir: PathBuf::from(".tikeo-migration"),
            project: None,
            output: None,
            format: MigrationReportFormat::Json,
            namespace: "ops".to_owned(),
            app: "billing".to_owned(),
            tikeo_version: "${TIKEO_VERSION}".to_owned(),
        };

        let resolved = tokio::runtime::Runtime::new()
            .unwrap_or_else(|error| panic!("runtime: {error}"))
            .block_on(resolve_plan_inputs_from(&command, project.path()))
            .unwrap_or_else(|error| {
                panic!("inputs should resolve from project convention: {error}")
            });

        assert_eq!(resolved.source, MigrationSource::XxlJob);
        assert!(resolved.input_origin.contains("xxl-job-export.json"));
        assert_eq!(resolved.project.as_deref(), Some(project.path()));
    }

    #[test]
    fn plans_powerjob_export_and_apply_dry_run() {
        let input = r#"[{"id":42,"jobName":"etl fanout","appName":"data","timeExpressionType":4,"timeExpression":"PT30S","processorInfo":"etlProcessor","instanceRetryNum":1,"executeType":"BROADCAST"}]"#;
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
        let runtime =
            tokio::runtime::Runtime::new().unwrap_or_else(|error| panic!("runtime: {error}"));
        let evidence = runtime
            .block_on(apply_data(
                &ApplyCommand {
                    bundle: PathBuf::new(),
                    endpoint: "http://127.0.0.1:9090".to_owned(),
                    api_key: "dry".to_owned(),
                    include_needs_review: true,
                    dry_run: true,
                    output: None,
                },
                &report,
            ))
            .unwrap_or_else(|error| panic!("dry run: {error}"));
        assert_eq!(evidence.requests.len(), 1);
        assert_eq!(evidence.requests[0].status, "planned");
        let markdown = render_markdown_report(&report);
        assert!(markdown.contains("Tikeo migration report"));
        assert!(markdown.contains("etl fanout"));
    }
}
