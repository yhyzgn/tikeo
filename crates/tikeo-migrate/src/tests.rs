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
        &MigrationDefaults {
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
    let normalized = normalize_connection_url(
        config.url.as_deref().unwrap_or_default(),
        config.username.as_deref(),
        config.password.as_deref(),
    )
    .unwrap_or_else(|error| panic!("url should normalize: {error}"));
    assert_eq!(normalized, "mysql://xxl:s3%20cr3t@127.0.0.1:3306/xxl_job");
}

#[test]
fn normalizes_sqlite_paths_without_rewriting_windows_drive_letters() {
    assert_eq!(
        normalize_connection_url("jdbc:sqlite:/tmp/legacy.db", None, None)
            .unwrap_or_else(|error| panic!("sqlite url should normalize: {error}")),
        "sqlite:/tmp/legacy.db"
    );
    assert_eq!(
        normalize_connection_url("sqlite:///tmp/legacy.db", None, None)
            .unwrap_or_else(|error| panic!("sqlite url should normalize: {error}")),
        "sqlite:///tmp/legacy.db"
    );
    assert_eq!(
        normalize_connection_url(
            r"sqlite:C:\legacy\xxl-job.db",
            Some("ignored"),
            Some("ignored")
        )
        .unwrap_or_else(|error| panic!("windows sqlite url should normalize: {error}")),
        r"sqlite:C:\legacy\xxl-job.db"
    );
    assert_eq!(
        redact_connection_url(r"sqlite:C:\legacy\xxl-job.db"),
        r"sqlite:C:\legacy\xxl-job.db"
    );
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
        .unwrap_or_else(|error| panic!("inputs should resolve from project convention: {error}"));

    assert_eq!(resolved.source, MigrationSource::XxlJob);
    assert!(resolved.input_origin.contains("xxl-job-export.json"));
    assert_eq!(resolved.project.as_deref(), Some(project.path()));
}

#[test]
fn plans_powerjob_export_with_review_flags() {
    let input = r#"[{"id":42,"jobName":"etl fanout","appName":"data","timeExpressionType":4,"timeExpression":"PT30S","processorInfo":"etlProcessor","instanceRetryNum":1,"executeType":"BROADCAST"}]"#;
    let report = plan_migration(
        MigrationSource::PowerJob,
        input,
        &MigrationDefaults {
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
