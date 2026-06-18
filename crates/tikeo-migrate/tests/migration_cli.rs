//! CLI smoke tests for the dedicated tikeo-migrate binary.

use std::{fs, process::Command};

#[test]
fn plan_command_writes_complete_bundle_for_xxl_job_and_java_project() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let output_dir =
        std::env::temp_dir().join(format!("tikeo-migrate-bundle-{}", std::process::id()));
    let _ = fs::remove_dir_all(&output_dir);
    let status = Command::new(&binary)
        .args([
            "plan",
            "--from",
            "xxl-job",
            "--input",
            "tests/fixtures/migration/xxl-job-export.json",
            "--output-dir",
        ])
        .arg(&output_dir)
        .args([
            "--project",
            "tests/fixtures/java-springboot3",
            "--namespace",
            "ops",
            "--app",
            "fallback",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("migration CLI should run: {error}"));

    assert!(status.success());
    let report = fs::read_to_string(output_dir.join("jobs.tikeo.json"))
        .unwrap_or_else(|error| panic!("report should be readable: {error}"));
    assert!(report.contains(r#""source": "xxl-job""#));
    assert!(report.contains(r#""mode": "dry_run_report_only""#));
    assert!(report.contains(r#""total": 2"#));
    assert!(report.contains(r#""namespace": "ops""#));
    assert!(report.contains(r#""scheduleType": "cron""#));
    let java_plan = fs::read_to_string(output_dir.join("java-project-plan.md"))
        .unwrap_or_else(|error| panic!("java plan should be readable: {error}"));
    assert!(java_plan.contains("tikeo-spring-boot3-starter"));
    assert!(java_plan.contains("billingProcessor"));
    assert!(output_dir.join("data-import-plan.json").exists());
    assert!(output_dir.join("CHECKLIST.md").exists());
    let _ = fs::remove_dir_all(output_dir);
}

#[test]
fn plan_command_uses_project_root_convention_without_manual_params() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let project_dir =
        std::env::temp_dir().join(format!("tikeo-migrate-zero-param-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(project_dir.join("src/main/java/com/example"))
        .unwrap_or_else(|error| panic!("project dir should be created: {error}"));
    fs::write(
        project_dir.join("build.gradle.kts"),
        "plugins { id(\"org.springframework.boot\") version \"3.5.8\" }\ndependencies { implementation(\"com.xuxueli:xxl-job-core:2.4.1\") }\n",
    )
    .unwrap_or_else(|error| panic!("gradle file should be written: {error}"));
    fs::write(
        project_dir.join("src/main/java/com/example/BillingJob.java"),
        "package com.example;\nimport com.xxl.job.core.handler.annotation.XxlJob;\nclass BillingJob {\n  @XxlJob(\"billingProcessor\")\n  public void execute() {}\n}\n",
    )
    .unwrap_or_else(|error| panic!("java file should be written: {error}"));
    fs::write(
        project_dir.join("xxl-job-export.json"),
        r#"{"jobs":[{"id":7,"jobDesc":"nightly billing","scheduleType":"CRON","scheduleConf":"0 0 2 * * ?","executorHandler":"billingProcessor","triggerStatus":1}]}"#,
    )
    .unwrap_or_else(|error| panic!("export file should be written: {error}"));

    let status = Command::new(&binary)
        .arg("plan")
        .current_dir(&project_dir)
        .status()
        .unwrap_or_else(|error| panic!("zero-param migration CLI should run: {error}"));

    assert!(status.success());
    let output_dir = project_dir.join(".tikeo-migration");
    assert!(output_dir.join("manifest.json").exists());
    assert!(output_dir.join("jobs.tikeo.json").exists());
    assert!(output_dir.join("java-project-plan.md").exists());
    let java_plan = fs::read_to_string(output_dir.join("java-project-plan.md"))
        .unwrap_or_else(|error| panic!("java plan should be readable: {error}"));
    assert!(java_plan.contains("tikeo-spring-boot3-starter"));
    assert!(java_plan.contains("billingProcessor"));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn apply_command_dry_run_reads_bundle_and_writes_evidence() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let output_dir =
        std::env::temp_dir().join(format!("tikeo-migrate-apply-{}", std::process::id()));
    let _ = fs::remove_dir_all(&output_dir);
    let plan_status = Command::new(&binary)
        .args([
            "plan",
            "--from",
            "powerjob",
            "--input",
            "tests/fixtures/migration/powerjob-export.json",
            "--output-dir",
        ])
        .arg(&output_dir)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("plan should run: {error}"));
    assert!(plan_status.success());

    let apply_status = Command::new(&binary)
        .args(["apply", "--bundle"])
        .arg(&output_dir)
        .args([
            "--endpoint",
            "http://127.0.0.1:9090",
            "--api-key",
            "dry-run-key",
            "--include-needs-review",
            "--dry-run",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("apply dry-run should run: {error}"));
    assert!(apply_status.success());
    let evidence = fs::read_to_string(output_dir.join("apply-evidence.json"))
        .unwrap_or_else(|error| panic!("evidence should be readable: {error}"));
    assert!(evidence.contains(r#""dryRun": true"#));
    assert!(evidence.contains(r#""status": "planned""#));
    let _ = fs::remove_dir_all(output_dir);
}
