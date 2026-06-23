//! CLI smoke tests for the dedicated tikeo-migrate binary.

use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn write_sqlite_db(path: &Path, sql: &str) {
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(
            "import sqlite3, sys\nconn = sqlite3.connect(sys.argv[1])\nconn.executescript(sys.stdin.read())\nconn.commit()\nconn.close()",
        )
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("sqlite fixture writer should start: {error}"));
    child
        .stdin
        .as_mut()
        .expect("sqlite fixture writer stdin should be available")
        .write_all(sql.as_bytes())
        .unwrap_or_else(|error| panic!("sqlite fixture sql should be written: {error}"));
    let status = child
        .wait()
        .unwrap_or_else(|error| panic!("sqlite fixture writer should finish: {error}"));
    assert!(status.success());
}

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
    assert!(report.contains("billingSettlementProcessor"));
    let java_plan = fs::read_to_string(output_dir.join("java-project-plan.md"))
        .unwrap_or_else(|error| panic!("java plan should be readable: {error}"));
    assert!(java_plan.contains("tikeo-spring-boot3-starter"));
    assert!(java_plan.contains("billingProcessor"));
    assert!(output_dir.join("data-import-plan.json").exists());
    assert!(output_dir.join("CHECKLIST.md").exists());
    let _ = fs::remove_dir_all(output_dir);
}

#[test]
fn plan_command_normalizes_pascal_case_xxl_handlers_to_lower_camel() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let output_dir = std::env::temp_dir().join(format!(
        "tikeo-migrate-xxl-lower-camel-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&output_dir);
    let input = output_dir.join("xxl.json");
    fs::create_dir_all(&output_dir)
        .unwrap_or_else(|error| panic!("output dir should be created: {error}"));
    fs::write(
        &input,
        r#"{"jobs":[{"id":9,"jobDesc":"Pascal handler","scheduleType":"NONE","executorHandler":"BillingProcessor","triggerStatus":1}]}"#,
    )
    .unwrap_or_else(|error| panic!("xxl export should be written: {error}"));

    let status = Command::new(&binary)
        .args(["plan", "--from", "xxl-job", "--input"])
        .arg(&input)
        .args(["--output-dir"])
        .arg(output_dir.join("bundle"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("migration CLI should run: {error}"));

    assert!(status.success());
    let report = fs::read_to_string(output_dir.join("bundle/jobs.tikeo.json"))
        .unwrap_or_else(|error| panic!("report should be readable: {error}"));
    assert!(report.contains(r#""processorName": "billingProcessor""#));
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
fn plan_command_falls_back_to_code_only_powerjob_handlers_without_scheduler_export() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let project_dir = std::env::temp_dir().join(format!(
        "tikeo-migrate-code-only-powerjob-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(project_dir.join("src/main/java/com/example/jobs"))
        .unwrap_or_else(|error| panic!("project dir should be created: {error}"));
    fs::write(
        project_dir.join("pom.xml"),
        r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <parent><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-parent</artifactId><version>3.5.8</version></parent>
  <groupId>com.example</groupId>
  <artifactId>ivs</artifactId>
  <properties>
    <java.version>21</java.version>
    <powerjob.version>4.3.2</powerjob.version>
  </properties>
  <dependencyManagement>
    <dependencies>
      <dependency><groupId>tech.powerjob</groupId><artifactId>powerjob-worker-spring-boot-starter</artifactId><version>${powerjob.version}</version></dependency>
    </dependencies>
  </dependencyManagement>
  <dependencies>
    <dependency><groupId>tech.powerjob</groupId><artifactId>powerjob-worker-spring-boot-starter</artifactId></dependency>
  </dependencies>
</project>
"#,
    )
    .unwrap_or_else(|error| panic!("pom should be written: {error}"));
    fs::write(
        project_dir.join("src/main/java/com/example/jobs/OutboxPublishProcessor.java"),
        "package com.example.jobs;
import tech.powerjob.worker.core.processor.sdk.BasicProcessor;
import tech.powerjob.worker.core.processor.ProcessResult;
import tech.powerjob.worker.core.processor.TaskContext;
public class OutboxPublishProcessor implements BasicProcessor { public ProcessResult process(TaskContext context) { return new ProcessResult(true); } }
",
    )
    .unwrap_or_else(|error| panic!("processor should be written: {error}"));

    let status = Command::new(&binary)
        .arg("plan")
        .current_dir(&project_dir)
        .status()
        .unwrap_or_else(|error| panic!("code-only migration CLI should run: {error}"));

    assert!(status.success());
    let output_dir = project_dir.join(".tikeo-migration");
    let manifest = fs::read_to_string(output_dir.join("manifest.json"))
        .unwrap_or_else(|error| panic!("manifest should be readable: {error}"));
    assert!(manifest.contains("code-only:"));
    assert!(manifest.contains("OutboxPublishProcessor"));
    let report = fs::read_to_string(output_dir.join("jobs.tikeo.json"))
        .unwrap_or_else(|error| panic!("report should be readable: {error}"));
    assert!(report.contains(r#""source": "powerjob""#));
    assert!(report.contains(r#""total": 1"#));
    assert!(report.contains(r#""needsReview": 1"#));
    assert!(report.contains("Generated from Java handler code only"));
    assert!(report.contains(r#""app": "ivs""#));
    let java_plan = fs::read_to_string(output_dir.join("java-project-plan.md"))
        .unwrap_or_else(|error| panic!("java plan should be readable: {error}"));
    assert!(java_plan.contains("tikeo-spring-boot3-starter"));
    assert!(java_plan.contains("outboxPublishProcessor"));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn plan_command_auto_exports_xxl_job_from_legacy_sqlite_fixture() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let project_dir = std::env::temp_dir().join(format!(
        "tikeo-migrate-auto-xxl-sqlite-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(project_dir.join("src/main/java/com/example"))
        .unwrap_or_else(|error| panic!("project dir should be created: {error}"));
    fs::create_dir_all(project_dir.join("src/main/resources"))
        .unwrap_or_else(|error| panic!("resources dir should be created: {error}"));
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
    let db_path = project_dir.join("legacy-xxl-job.db");
    write_sqlite_db(
        &db_path,
        r#"
create table xxl_job_info (
  id integer primary key,
  job_desc text,
  executor_app_name text,
  schedule_type text,
  schedule_conf text,
  executor_handler text,
  executor_fail_retry_count integer,
  trigger_status integer,
  executor_route_strategy text
);
insert into xxl_job_info values (1001, 'nightly billing', 'billing', 'CRON', '0 0 2 * * ?', 'billingProcessor', 2, 1, null);
"#,
    );
    fs::write(
        project_dir.join("src/main/resources/application.properties"),
        format!("spring.datasource.url=sqlite:{}\n", db_path.display()),
    )
    .unwrap_or_else(|error| panic!("application.properties should be written: {error}"));

    let status = Command::new(&binary)
        .arg("plan")
        .current_dir(&project_dir)
        .status()
        .unwrap_or_else(|error| panic!("auto DB migration CLI should run: {error}"));

    assert!(status.success());
    let output_dir = project_dir.join(".tikeo-migration");
    let manifest = fs::read_to_string(output_dir.join("manifest.json"))
        .unwrap_or_else(|error| panic!("manifest should be readable: {error}"));
    assert!(manifest.contains("legacy-db:sqlite:"));
    assert!(manifest.contains("nightly billing"));
    assert!(manifest.contains("billingProcessor"));
    let report = fs::read_to_string(output_dir.join("jobs.tikeo.json"))
        .unwrap_or_else(|error| panic!("report should be readable: {error}"));
    assert!(report.contains(r#""source": "xxl-job""#));
    assert!(report.contains(r#""total": 1"#));
    let java_plan = fs::read_to_string(output_dir.join("java-project-plan.md"))
        .unwrap_or_else(|error| panic!("java plan should be readable: {error}"));
    assert!(java_plan.contains("tikeo-spring-boot3-starter"));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn plan_command_auto_exports_powerjob_from_explicit_legacy_sqlite_fixture() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let project_dir = std::env::temp_dir().join(format!(
        "tikeo-migrate-auto-powerjob-sqlite-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(&project_dir)
        .unwrap_or_else(|error| panic!("project dir should be created: {error}"));
    let db_path = project_dir.join("legacy-powerjob.db");
    write_sqlite_db(
        &db_path,
        r#"
create table pj_job_info (
  id integer primary key,
  job_name text,
  app_name text,
  time_expression_type integer,
  time_expression text,
  processor_info text,
  instance_retry_num integer,
  execute_type text,
  max_instance_num integer
);
insert into pj_job_info values (2001, 'etl fanout', 'data-platform', 4, 'PT30S', 'etlProcessor', 1, 'BROADCAST', 4);
"#,
    );

    let status = Command::new(&binary)
        .args(["plan", "--from", "powerjob", "--legacy-db-url"])
        .arg(format!("sqlite:{}", db_path.display()))
        .current_dir(&project_dir)
        .status()
        .unwrap_or_else(|error| panic!("explicit auto DB migration CLI should run: {error}"));

    assert!(status.success());
    let output_dir = project_dir.join(".tikeo-migration");
    let report = fs::read_to_string(output_dir.join("jobs.tikeo.json"))
        .unwrap_or_else(|error| panic!("report should be readable: {error}"));
    assert!(report.contains(r#""source": "powerjob""#));
    assert!(report.contains("etl fanout"));
    assert!(report.contains("etlProcessor"));
    assert!(report.contains("needs_review"));
    let manifest = fs::read_to_string(output_dir.join("manifest.json"))
        .unwrap_or_else(|error| panic!("manifest should be readable: {error}"));
    assert!(manifest.contains("legacy-db:sqlite:"));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn apply_command_transforms_powerjob_worker_in_place() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let project_dir = std::env::temp_dir().join(format!(
        "tikeo-migrate-apply-powerjob-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(project_dir.join("src/main/java/com/example/jobs"))
        .unwrap_or_else(|error| panic!("project dir should be created: {error}"));
    fs::create_dir_all(project_dir.join("src/main/resources"))
        .unwrap_or_else(|error| panic!("resources dir should be created: {error}"));
    fs::write(
        project_dir.join("pom.xml"),
        r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <parent><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-parent</artifactId><version>3.5.8</version></parent>
  <groupId>com.example</groupId>
  <artifactId>ivs</artifactId>
  <properties>
    <java.version>21</java.version>
    <powerjob.version>4.3.2</powerjob.version>
  </properties>
  <dependencyManagement>
    <dependencies>
      <dependency><groupId>tech.powerjob</groupId><artifactId>powerjob-worker-spring-boot-starter</artifactId><version>${powerjob.version}</version></dependency>
    </dependencies>
  </dependencyManagement>
  <dependencies>
    <dependency><groupId>tech.powerjob</groupId><artifactId>powerjob-worker-spring-boot-starter</artifactId></dependency>
  </dependencies>
</project>
"#,
    )
    .unwrap_or_else(|error| panic!("pom should be written: {error}"));
    fs::write(
        project_dir.join("src/main/java/com/example/jobs/OutboxPublishProcessor.java"),
        "package com.example.jobs;\nimport org.springframework.stereotype.Component;\nimport tech.powerjob.worker.core.processor.ProcessResult;\nimport tech.powerjob.worker.core.processor.TaskContext;\nimport tech.powerjob.worker.core.processor.sdk.BasicProcessor;\n@Component\npublic class OutboxPublishProcessor implements BasicProcessor {\n  @Override\n  public ProcessResult process(TaskContext context) { return new ProcessResult(true, \"ok\"); }\n}\n",
    )
    .unwrap_or_else(|error| panic!("processor should be written: {error}"));

    fs::write(
        project_dir.join("src/main/resources/application-dev.yml"),
        "spring:
  application:
    name: ivs
powerjob:
  worker:
    enabled: true
    app-name: ivs
    server-address: 127.0.0.1:7700
xxl:
  job:
    admin-addresses: http://127.0.0.1:8080/xxl-job-admin
xxl.job.executor.enabled: true
",
    )
    .unwrap_or_else(|error| panic!("legacy config should be written: {error}"));

    let plan_status = Command::new(&binary)
        .arg("plan")
        .current_dir(&project_dir)
        .status()
        .unwrap_or_else(|error| panic!("code-only plan should run: {error}"));
    assert!(plan_status.success());

    let apply_status = Command::new(&binary)
        .args(["apply", "--bundle"])
        .arg(project_dir.join(".tikeo-migration"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("apply local code migration should run: {error}"));
    assert!(apply_status.success());

    let migrated = fs::read_to_string(
        project_dir.join("src/main/java/com/example/jobs/OutboxPublishProcessor.java"),
    )
    .unwrap_or_else(|error| panic!("migrated source should be readable: {error}"));
    assert!(migrated.contains("import net.tikeo.processor.TikeoProcessor;"));
    assert!(migrated.contains("import net.tikeo.processor.TaskOutcome;"));
    assert!(migrated.contains("@TikeoProcessor(\"outboxPublishProcessor\")"));
    assert!(!migrated.contains("implements BasicProcessor"));
    assert!(migrated.contains("TaskOutcome process(TaskContext context)"));
    let pom = fs::read_to_string(project_dir.join("pom.xml"))
        .unwrap_or_else(|error| panic!("migrated pom should be readable: {error}"));
    assert!(pom.contains("<artifactId>tikeo-spring-boot3-starter</artifactId>"));
    assert!(!pom.contains("${TIKEO_VERSION}"));
    assert!(pom.contains("<tikeo.version>0.3.10</tikeo.version>"));
    assert!(pom.contains("<version>${tikeo.version}</version>"));
    assert!(!pom.contains("tech.powerjob"));
    assert!(!pom.contains("powerjob.version"));
    let migrated_config =
        fs::read_to_string(project_dir.join("src/main/resources/application-dev.yml"))
            .unwrap_or_else(|error| panic!("migrated config should be readable: {error}"));
    assert!(migrated_config.contains("Generated by tikeo-migrate apply"));
    assert!(!migrated_config.contains("powerjob:"));
    assert!(!migrated_config.contains("POWERJOB_ENABLED"));
    assert!(!migrated_config.contains("xxl:"));
    assert!(!migrated_config.contains("xxl.job"));
    assert!(migrated_config.contains("tikeo:"));
    assert!(migrated_config.contains("endpoint: ${TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}"));
    assert!(migrated_config.contains("app: ${TIKEO_APP:ivs}"));
    assert!(migrated_config.contains("state-dir: ${TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}"));
    assert!(migrated_config.contains("api-key: ${TIKEO_MANAGEMENT_API_KEY:}"));
    assert!(!migrated_config.contains("heartbeat-interval-millis"));
    assert!(!migrated_config.contains("power-shell-install-version"));
    assert!(!migrated_config.contains("images:"));
    assert!(
        !project_dir
            .join("src/main/resources/application-tikeo-migration.yml")
            .exists()
    );
    assert!(project_dir.join("CODE_MIGRATION_REPORT.md").exists());
    let code_report = fs::read_to_string(project_dir.join("CODE_MIGRATION_REPORT.md"))
        .unwrap_or_else(|error| panic!("code migration report should be readable: {error}"));
    assert!(code_report.contains("Target project (in-place)"));
    assert!(code_report.contains("## Migration result checklist"));
    assert!(code_report.contains("| `pom.xml` | dependency | migrated |"));
    assert!(
        code_report
            .contains("| `src/main/resources/application-dev.yml` | configuration | migrated |")
    );
    assert!(code_report.contains("OutboxPublishProcessor.java` | executor | migrated"));
    assert!(code_report.contains("## Data import summary"));
    assert!(code_report.contains("## Semantic review items"));
    assert!(code_report.contains("Generated from Java handler code only"));
    assert!(code_report.contains("Review every needs_review job"));
    assert!(code_report.contains("tikeo-migrate apply never calls the server"));
    let evidence =
        fs::read_to_string(project_dir.join(".tikeo-migration/code-apply-evidence.json"))
            .unwrap_or_else(|error| panic!("code evidence should be readable: {error}"));
    assert!(evidence.contains("OutboxPublishProcessor.java"));
    assert!(evidence.contains("outboxPublishProcessor"));
    assert!(evidence.contains(r#""dataImportSummary""#));
    assert!(evidence.contains(r#""semanticReviewItems""#));
    assert!(evidence.contains(r#""nextActions""#));
    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn plan_command_reports_known_tables_for_legacy_db_export_failure() {
    let binary = std::env::var("CARGO_BIN_EXE_tikeo-migrate")
        .unwrap_or_else(|error| panic!("binary path should exist: {error}"));
    let project_dir =
        std::env::temp_dir().join(format!("tikeo-migrate-diagnostics-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(&project_dir)
        .unwrap_or_else(|error| panic!("project dir should be created: {error}"));
    let db_path = project_dir.join("empty-legacy.db");
    write_sqlite_db(
        &db_path,
        "create table unrelated_table (id integer primary key);",
    );

    let output = Command::new(&binary)
        .args(["plan", "--from", "xxl-job", "--legacy-db-url"])
        .arg(format!("sqlite:{}", db_path.display()))
        .current_dir(&project_dir)
        .output()
        .unwrap_or_else(|error| panic!("diagnostic migration CLI should run: {error}"));

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to export xxl-job jobs from detected legacy database"));
    assert!(stderr.contains("grant read-only SELECT on known tables"));
    assert!(stderr.contains("xxl_job_info"));
    assert!(stderr.contains("job_info"));
    let _ = fs::remove_dir_all(project_dir);
}
