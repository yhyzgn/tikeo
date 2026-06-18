//! CLI smoke tests for report-only scheduler migration planning.

use std::{fs, process::Command};

#[test]
fn migration_cli_writes_json_report_for_xxl_job_fixture() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let output_path = std::env::temp_dir().join(format!(
        "tikeo-migration-report-{}.json",
        std::process::id()
    ));
    let status = Command::new(&binary)
        .args([
            "migrate",
            "--from",
            "xxl-job",
            "--input",
            "crates/tikeo-server/tests/fixtures/migration/xxl-job-export.json",
            "--output",
        ])
        .arg(&output_path)
        .args(["--namespace", "ops", "--app", "fallback"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("migration CLI should run: {error}"));

    assert!(status.success());
    let report = fs::read_to_string(&output_path)
        .unwrap_or_else(|error| panic!("report should be readable: {error}"));
    assert!(report.contains(r#""source": "xxl-job""#));
    assert!(report.contains(r#""mode": "dry_run_report_only""#));
    assert!(report.contains(r#""total": 2"#));
    assert!(report.contains(r#""namespace": "ops""#));
    assert!(report.contains(r#""scheduleType": "cron""#));
    assert!(report.contains("XXL-JOB route strategy"));
    assert!(report.contains("XXL-JOB block strategy"));
    let _ = fs::remove_file(output_path);
}

#[test]
fn migration_cli_renders_powerjob_markdown_to_stdout() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let output = Command::new(&binary)
        .args([
            "migrate",
            "--from",
            "powerjob",
            "--input",
            "crates/tikeo-server/tests/fixtures/migration/powerjob-export.json",
            "--format",
            "markdown",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|error| panic!("migration CLI should run: {error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("stdout should be utf8: {error}"));
    assert!(stdout.contains("Tikeo migration report"));
    assert!(stdout.contains("etl broadcast fanout"));
    assert!(stdout.contains("needs_review"));
}
