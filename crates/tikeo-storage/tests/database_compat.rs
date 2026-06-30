//! Cross-database compatibility smoke tests for tikeo storage.

use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
use tikeo_storage::{
    AppendJobInstanceLog, CreateJob, CreateJobInstance, CreatePlugin, CreateScript,
    JobInstanceLogRepository, JobInstanceRepository, JobRepository, PluginAlertChannelTypeSummary,
    PluginProcessorTypeSummary, PluginRepository, ScopeRepository, ScriptRepository,
    connect_and_migrate,
};

#[tokio::test]
async fn sqlite_database_compatibility_smoke() {
    run_storage_smoke("sqlite::memory:").await;
}

#[tokio::test]
async fn external_database_compatibility_smoke() {
    let urls = external_connection_urls();
    if urls.is_empty() {
        eprintln!(
            "skip external database compatibility smoke: set TIKEO_TEST_CONNECTION_URLS, \
             TIKEO_TEST_POSTGRES_URL, or TIKEO_TEST_MYSQL_URL"
        );
        return;
    }

    for url in urls {
        run_storage_smoke(&url).await;
    }
}

async fn run_storage_smoke(connection_url: &str) {
    let db = connect_and_migrate(connection_url)
        .await
        .unwrap_or_else(|error| {
            panic!("database should connect and migrate: {connection_url}: {error}")
        });

    let scope_repository = ScopeRepository::new(db.clone());
    let namespace = format!("compat-{}", unique_suffix());
    let app = "storage";
    let pool = "default";
    let created_namespace = scope_repository
        .create_namespace(&namespace)
        .await
        .unwrap_or_else(|error| panic!("namespace should create on {connection_url}: {error}"));
    assert_eq!(created_namespace.name, namespace);
    let created_app = scope_repository
        .create_app(&namespace, app)
        .await
        .unwrap_or_else(|error| panic!("app should create on {connection_url}: {error}"));
    assert_eq!(created_app.name, app);
    let created_pool = scope_repository
        .create_worker_pool(&namespace, app, pool)
        .await
        .unwrap_or_else(|error| panic!("worker pool should create on {connection_url}: {error}"));
    assert_eq!(created_pool.name, pool);

    let plugin_repository = PluginRepository::new(db.clone());
    let plugin = plugin_repository
        .create_plugin(CreatePlugin {
            name: format!("compat-plugin-{}", unique_suffix()),
            kind: "compat".to_owned(),
            processor_types: vec![PluginProcessorTypeSummary {
                r#type: "compat.sql".to_owned(),
                label: "Compatibility SQL".to_owned(),
                capability: "sql".to_owned(),
                processor_names: vec!["compat.sql.sync".to_owned()],
                description: Some("cross database json payload".to_owned()),
                artifact_ref: Some("oci://example.invalid/tikeo/compat-plugin:1".to_owned()),
                container_image: Some("example.invalid/tikeo/compat-plugin:1".to_owned()),
                entrypoint: Some(vec!["/plugin".to_owned(), "run".to_owned()]),
                checksum: Some("sha256:compat".to_owned()),
            }],
            alert_channel_types: vec![PluginAlertChannelTypeSummary {
                r#type: "compat.webhook".to_owned(),
                label: "Compatibility Webhook".to_owned(),
                target_kind: "webhook".to_owned(),
                description: Some("cross database json payload".to_owned()),
                template: serde_json::json!({"body":{"text":"{{message}}"}}),
            }],
            enabled: true,
        })
        .await
        .unwrap_or_else(|error| {
            panic!("plugin json payload should persist on {connection_url}: {error}")
        });
    assert_eq!(
        plugin.processor_types[0].entrypoint.as_deref(),
        Some(["/plugin".to_owned(), "run".to_owned()].as_slice())
    );
    assert!(
        plugin_repository
            .resolve_processor_type("compat.sql")
            .await
            .unwrap_or_else(|error| panic!("plugin should resolve on {connection_url}: {error}"))
            .is_some()
    );

    let calendar_repository = tikeo_storage::CalendarRepository::new(db.clone());
    let calendar = calendar_repository
        .upsert(tikeo_storage::UpsertCalendar {
            namespace: namespace.clone(),
            app: app.to_owned(),
            name: format!("compat-calendar-{}", unique_suffix()),
            timezone: "Asia/Shanghai".to_owned(),
            excluded_dates: vec!["2026-10-01".to_owned()],
            holidays: vec!["2026-10-02".to_owned()],
            maintenance_windows: vec![tikeo_storage::CalendarWindowSummary {
                start: "2026-10-01T01:00:00+08:00".to_owned(),
                end: "2026-10-01T02:00:00+08:00".to_owned(),
            }],
            freeze_windows: Vec::new(),
            created_by: "compat-test".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("calendar should persist on {connection_url}: {error}"));
    assert_eq!(calendar.timezone, "Asia/Shanghai");
    let calendars = calendar_repository
        .list(Some(&namespace), Some(app))
        .await
        .unwrap_or_else(|error| panic!("calendars should list on {connection_url}: {error}"));
    assert!(calendars.iter().any(|item| item.id == calendar.id));

    let script_repository = ScriptRepository::new(db.clone());
    let script = script_repository
        .create_script(CreateScript {
            name: format!("compat-script-{}", unique_suffix()),
            language: "shell".to_owned(),
            version: "1.0.0".to_owned(),
            content: "echo compat".to_owned(),
            created_by: "compat-test".to_owned(),
            timeout_seconds: Some(30),
            max_memory_bytes: Some(64 * 1024 * 1024),
            allow_network: false,
            allowed_env_vars: Some(serde_json::json!(["TIKEO_COMPAT"]).to_string()),
            policy_json: Some(
                serde_json::json!({"version":1,"network":{"enabled":false}}).to_string(),
            ),
        })
        .await
        .unwrap_or_else(|error| panic!("script should persist on {connection_url}: {error}"));
    assert_eq!(
        script.allowed_env_vars.as_deref(),
        Some(["TIKEO_COMPAT".to_owned()].as_slice())
    );

    let job_repository = JobRepository::new(db.clone());
    let job = job_repository
        .create_job(CreateJob {
            created_by: Some("compat-test".to_owned()),
            namespace: namespace.clone(),
            app: app.to_owned(),
            name: format!("compat.job.{}", unique_suffix()),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "skip".to_owned(),
            schedule_start_at: Some("2026-06-01T00:00:00+08:00".to_owned()),
            schedule_end_at: Some("2026-06-02T00:00:00+08:00".to_owned()),
            schedule_calendar_json: Some(
                serde_json::json!({
                    "timezone":"Asia/Shanghai",
                    "maintenanceWindows":[{"start":"2026-06-01T01:00:00+08:00","end":"2026-06-01T02:00:00+08:00"}],
                    "freezeWindows":[]
                })
                .to_string(),
            ),
            processor_name: Some("compat.echo".to_owned()),
            processor_type: None,
            worker_pool: None,
                script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            canary_policy: None,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should persist on {connection_url}: {error}"));
    assert_eq!(job.namespace, namespace);
    assert_eq!(job.processor_name.as_deref(), Some("compat.echo"));

    let version = job_repository
        .versions()
        .get_version_by_number(&job.id, 1)
        .await
        .unwrap_or_else(|error| panic!("job version should load on {connection_url}: {error}"))
        .unwrap_or_else(|| panic!("job version should exist on {connection_url}"));
    assert_eq!(version.version_number, 1);
    assert!(version.schedule_calendar_json.is_some());

    let instance_repository = JobInstanceRepository::new(db.clone());
    let instance = instance_repository
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create on {connection_url}: {error}"))
        .unwrap_or_else(|| panic!("instance should exist on {connection_url}"));
    assert_eq!(instance.status, InstanceStatus::Pending);
    assert!(
        instance_repository
            .claim_pending_for_dispatch(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should claim on {connection_url}: {error}"))
    );
    let updated = instance_repository
        .update_status_if_current(
            &instance.id,
            InstanceStatus::Dispatching,
            InstanceStatus::Succeeded,
        )
        .await
        .unwrap_or_else(|error| {
            panic!("instance status should update on {connection_url}: {error}")
        });
    assert!(updated);

    let log_repository = JobInstanceLogRepository::new(db.clone());
    log_repository
        .append(AppendJobInstanceLog {
            instance_id: instance.id.clone(),
            worker_id: "compat-worker".to_owned(),
            level: "info".to_owned(),
            message: "compat log with unicode 时区".to_owned(),
            sequence: 1,
        })
        .await
        .unwrap_or_else(|error| panic!("instance log should append on {connection_url}: {error}"))
        .unwrap_or_else(|| panic!("instance log should exist on {connection_url}"));
    let logs = log_repository
        .list_by_instance(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("instance logs should list on {connection_url}: {error}"));
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].worker_id, "compat-worker");

    connect_and_migrate(connection_url)
        .await
        .unwrap_or_else(|error| {
            panic!("migration rerun should be idempotent on {connection_url}: {error}")
        });

    db.close()
        .await
        .unwrap_or_else(|error| panic!("database should close cleanly: {error}"));
}

fn external_connection_urls() -> Vec<String> {
    let mut urls = Vec::new();
    if let Ok(value) = std::env::var("TIKEO_TEST_CONNECTION_URLS") {
        urls.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
        );
    }
    for name in ["TIKEO_TEST_POSTGRES_URL", "TIKEO_TEST_MYSQL_URL"] {
        if let Ok(value) = std::env::var(name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                urls.push(trimmed.to_owned());
            }
        }
    }
    urls.sort();
    urls.dedup();
    urls
}

fn unique_suffix() -> String {
    uuid::Uuid::now_v7().simple().to_string()
}
