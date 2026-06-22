use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{ApplyCommand, JavaProjectMigrationPlan};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeApplyEvidence {
    pub source_project: String,
    pub output_project: String,
    pub bundle: String,
    pub changed_files: Vec<String>,
    pub skipped_paths: Vec<String>,
    pub warnings: Vec<String>,
}

pub(crate) fn apply_code(command: &ApplyCommand) -> Result<CodeApplyEvidence> {
    let plan_path = command.bundle.join("java-project-plan.json");
    let plan_text = fs::read_to_string(&plan_path)
        .with_context(|| format!("failed to read Java project plan {}", plan_path.display()))?;
    let plan: JavaProjectMigrationPlan = serde_json::from_str(&plan_text)
        .with_context(|| format!("failed to parse Java project plan {}", plan_path.display()))?;
    let source_project = command
        .project
        .clone()
        .unwrap_or_else(|| PathBuf::from(&plan.project_root));
    if !source_project.is_dir() {
        bail!(
            "source project is not a directory: {}",
            source_project.display()
        );
    }
    let output_project = command
        .output_project
        .clone()
        .unwrap_or_else(|| command.bundle.join("migrated-project"));
    if output_project.exists() {
        if command.force {
            fs::remove_dir_all(&output_project).with_context(|| {
                format!(
                    "failed to remove existing output project {}",
                    output_project.display()
                )
            })?;
        } else {
            bail!(
                "output project already exists: {} (pass --force to replace it)",
                output_project.display()
            );
        }
    }
    let mut skipped_paths = Vec::new();
    copy_project(
        &source_project,
        &source_project,
        &output_project,
        &mut skipped_paths,
    )?;
    let mut changed_files = Vec::new();
    let mut warnings = Vec::new();
    apply_dependency(&output_project, &plan, &mut changed_files, &mut warnings)?;
    apply_worker_config(&output_project, &plan, &mut changed_files)?;
    for handler in &plan.handler_candidates {
        let path = output_project.join(&handler.path);
        if !path.exists() {
            warnings.push(format!(
                "handler source file missing in output copy: {}",
                handler.path
            ));
            continue;
        }
        let before = fs::read_to_string(&path)
            .with_context(|| format!("failed to read handler source {}", path.display()))?;
        let after = match handler.framework.as_str() {
            "powerjob" => transform_powerjob_handler(&before, &handler.processor_name),
            "xxl-job" => transform_xxl_handler(&before, &handler.processor_name),
            other => {
                warnings.push(format!(
                    "unsupported handler framework `{other}` for {}",
                    handler.path
                ));
                before.clone()
            }
        };
        if after != before {
            fs::write(&path, after)
                .with_context(|| format!("failed to write handler source {}", path.display()))?;
            push_unique(&mut changed_files, handler.path.clone());
        }
    }
    let evidence = CodeApplyEvidence {
        source_project: source_project.display().to_string(),
        output_project: output_project.display().to_string(),
        bundle: command.bundle.display().to_string(),
        changed_files,
        skipped_paths,
        warnings,
    };
    write_code_apply_outputs(command, &output_project, &evidence)?;
    Ok(evidence)
}

fn copy_project(
    root: &Path,
    current: &Path,
    output: &Path,
    skipped: &mut Vec<String>,
) -> Result<()> {
    fs::create_dir_all(output)
        .with_context(|| format!("failed to create output directory {}", output.display()))?;
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        let source = entry.path();
        let relative = source.strip_prefix(root).unwrap_or(&source);
        if should_skip(relative) {
            skipped.push(relative.display().to_string());
            continue;
        }
        let target = output.join(relative);
        if source.is_dir() {
            copy_project(root, &source, output, skipped)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::copy(&source, &target).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source.display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn should_skip(relative: &Path) -> bool {
    relative.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git" | ".gradle" | ".idea" | ".tikeo-migration" | "build" | "target" | "node_modules"
        )
    })
}

fn apply_dependency(
    project: &Path,
    plan: &JavaProjectMigrationPlan,
    changed: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    match plan.build_system.as_str() {
        "maven" => apply_maven_dependency(project, &plan.recommended_artifact, changed),
        "gradle-kotlin" => apply_gradle_dependency(
            project,
            "build.gradle.kts",
            &plan.recommended_artifact,
            changed,
        ),
        "gradle-groovy" => {
            apply_gradle_dependency(project, "build.gradle", &plan.recommended_artifact, changed)
        }
        other => {
            warnings.push(format!(
                "unsupported build system `{other}`; dependency was not edited automatically"
            ));
            Ok(())
        }
    }
}

fn apply_maven_dependency(project: &Path, artifact: &str, changed: &mut Vec<String>) -> Result<()> {
    let path = project.join("pom.xml");
    let mut text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    if text.contains(&format!("<artifactId>{artifact}</artifactId>")) {
        return Ok(());
    }
    let dependency = format!(
        "    <dependency>\n      <groupId>net.tikeo</groupId>\n      <artifactId>{artifact}</artifactId>\n      <version>${{TIKEO_VERSION}}</version>\n    </dependency>\n"
    );
    let search_start = text
        .find("</dependencyManagement>")
        .map_or(0, |index| index + "</dependencyManagement>".len());
    if let Some(offset) = text[search_start..].find("</dependencies>") {
        text.insert_str(search_start + offset, &dependency);
    } else if let Some(index) = text.find("</project>") {
        text.insert_str(
            index,
            &format!("  <dependencies>\n{dependency}  </dependencies>\n"),
        );
    } else {
        bail!("pom.xml does not contain </project>");
    }
    fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))?;
    push_unique(changed, "pom.xml".to_owned());
    Ok(())
}

fn apply_gradle_dependency(
    project: &Path,
    file: &str,
    artifact: &str,
    changed: &mut Vec<String>,
) -> Result<()> {
    let path = project.join(file);
    let mut text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    if text.contains(&format!("net.tikeo:{artifact}:")) {
        return Ok(());
    }
    let line = if file.ends_with(".kts") {
        format!("    implementation(\"net.tikeo:{artifact}:${{TIKEO_VERSION}}\")\n")
    } else {
        format!("    implementation 'net.tikeo:{artifact}:${{TIKEO_VERSION}}'\n")
    };
    if let Some(index) = text.find("dependencies {") {
        let insert_at = text[index..]
            .find('\n')
            .map_or(index + "dependencies {".len(), |offset| index + offset + 1);
        text.insert_str(insert_at, &line);
    } else {
        text.push_str(&format!("\ndependencies {{\n{line}}}\n"));
    }
    fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))?;
    push_unique(changed, file.to_owned());
    Ok(())
}

fn apply_worker_config(
    project: &Path,
    _plan: &JavaProjectMigrationPlan,
    changed: &mut Vec<String>,
) -> Result<()> {
    let resources = project.join("src/main/resources");
    fs::create_dir_all(&resources)
        .with_context(|| format!("failed to create {}", resources.display()))?;
    let app = infer_app_name(project).unwrap_or_else(|| "default".to_owned());
    let targets = config_targets(project, &resources)?;
    for path in targets {
        let relative = path
            .strip_prefix(project)
            .unwrap_or(&path)
            .display()
            .to_string();
        let mut content = if path.exists() {
            fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?
        } else {
            String::new()
        };
        if content.contains("Generated by tikeo-migrate apply") {
            continue;
        }
        let block = if path.extension().and_then(|ext| ext.to_str()) == Some("properties") {
            tikeo_properties_block(&app)
        } else {
            tikeo_yaml_block(&app)
        };
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&block);
        fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
        push_unique(changed, relative);
    }
    Ok(())
}

fn config_targets(project: &Path, resources: &Path) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    if resources.is_dir() {
        collect_config_candidates(resources, &mut candidates)?;
    }
    candidates.sort();
    candidates.dedup();
    let mut legacy = Vec::new();
    for path in &candidates {
        let text = fs::read_to_string(path)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if text.contains("powerjob")
            || text.contains("power-job")
            || text.contains("xxl-job")
            || text.contains("xxl:")
            || text.contains("xxl.job")
        {
            legacy.push(path.clone());
        }
    }
    if !legacy.is_empty() {
        return Ok(legacy);
    }
    for name in [
        "application.yml",
        "application.yaml",
        "application.properties",
    ] {
        let path = resources.join(name);
        if path.exists() {
            return Ok(vec![path]);
        }
    }
    let path = resources.join("application.yml");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let relative = path.strip_prefix(project).unwrap_or(&path).display();
    eprintln!(
        "warning: no legacy scheduler config file found; creating {relative} for Tikeo configuration"
    );
    Ok(vec![path])
}

fn collect_config_candidates(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_config_candidates(&path, output)?;
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        let supported =
            lower.ends_with(".yml") || lower.ends_with(".yaml") || lower.ends_with(".properties");
        if supported && (lower.starts_with("application") || lower.starts_with("bootstrap")) {
            output.push(path);
        }
    }
    Ok(())
}

fn tikeo_yaml_block(app: &str) -> String {
    format!(
        r#"
# Generated by tikeo-migrate apply. Review values before enabling production traffic.
# Legacy scheduler workers are disabled in the migrated profile.
powerjob:
  worker:
    enabled: false
xxl:
  job:
    executor:
      enabled: false

tikeo:
  worker:
    enabled: ${{TIKEO_WORKER_ENABLED:true}}
    auto-startup: ${{TIKEO_WORKER_AUTO_STARTUP:true}}
    endpoint: ${{TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}}
    dry-run: ${{TIKEO_WORKER_DRY_RUN:false}}
    heartbeat-interval-millis: ${{TIKEO_WORKER_HEARTBEAT_INTERVAL_MILLIS:10000}}
    client-instance-id: ${{TIKEO_WORKER_CLIENT_INSTANCE_ID:}}
    state-dir: ${{TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}}
    namespace: ${{TIKEO_NAMESPACE:default}}
    app: ${{TIKEO_APP:{app}}}
    cluster: ${{TIKEO_WORKER_CLUSTER:default}}
    region: ${{TIKEO_WORKER_REGION:default}}
    capabilities:
      - ${{TIKEO_WORKER_CAPABILITY_0:java}}
      - ${{TIKEO_WORKER_CAPABILITY_1:spring-boot}}
    labels:
      migrated-from: ${{TIKEO_WORKER_LABEL_MIGRATED_FROM:legacy-scheduler}}
    election:
      enabled: ${{TIKEO_WORKER_ELECTION_ENABLED:true}}
      domain: ${{TIKEO_WORKER_ELECTION_DOMAIN:}}
      priority: ${{TIKEO_WORKER_ELECTION_PRIORITY:100}}
    wasm:
      auto-install: ${{TIKEO_WORKER_WASM_AUTO_INSTALL:true}}
      install-version: ${{TIKEO_WORKER_WASM_INSTALL_VERSION:latest}}
      install-dir: ${{TIKEO_WORKER_WASM_INSTALL_DIR:}}
      installer-url: ${{TIKEO_WORKER_WASM_INSTALLER_URL:https://wasmtime.dev/install.sh}}
      install-timeout-millis: ${{TIKEO_WORKER_WASM_INSTALL_TIMEOUT_MILLIS:120000}}
    scripts:
      enabled: ${{TIKEO_WORKER_SCRIPTS_ENABLED:true}}
      container-enabled: ${{TIKEO_WORKER_SCRIPTS_CONTAINER_ENABLED:false}}
      availability-check: ${{TIKEO_WORKER_SCRIPTS_AVAILABILITY_CHECK:true}}
      runtime-command: ${{TIKEO_WORKER_SCRIPTS_RUNTIME_COMMAND:}}
      # Optional extra container-runtime arguments; uncomment and add entries when needed.
      runtime-args: []
      auto-install-tools: ${{TIKEO_WORKER_SCRIPTS_AUTO_INSTALL_TOOLS:true}}
      srt-install-version: ${{TIKEO_WORKER_SCRIPTS_SRT_INSTALL_VERSION:latest}}
      srt-install-dir: ${{TIKEO_WORKER_SCRIPTS_SRT_INSTALL_DIR:}}
      ripgrep-install-version: ${{TIKEO_WORKER_SCRIPTS_RIPGREP_INSTALL_VERSION:latest}}
      ripgrep-install-dir: ${{TIKEO_WORKER_SCRIPTS_RIPGREP_INSTALL_DIR:}}
      deno-install-version: ${{TIKEO_WORKER_SCRIPTS_DENO_INSTALL_VERSION:latest}}
      deno-install-dir: ${{TIKEO_WORKER_SCRIPTS_DENO_INSTALL_DIR:}}
      deno-installer-url: ${{TIKEO_WORKER_SCRIPTS_DENO_INSTALLER_URL:https://deno.land/install.sh}}
      rhai-install-version: ${{TIKEO_WORKER_SCRIPTS_RHAI_INSTALL_VERSION:}}
      rhai-install-dir: ${{TIKEO_WORKER_SCRIPTS_RHAI_INSTALL_DIR:}}
      power-shell-install-version: ${{TIKEO_WORKER_SCRIPTS_POWER_SHELL_INSTALL_VERSION:7.5.4}}
      power-shell-install-dir: ${{TIKEO_WORKER_SCRIPTS_POWER_SHELL_INSTALL_DIR:}}
      wasmedge-auto-install: ${{TIKEO_WORKER_SCRIPTS_WASMEDGE_AUTO_INSTALL:false}}
      wasmedge-install-version: ${{TIKEO_WORKER_SCRIPTS_WASMEDGE_INSTALL_VERSION:latest}}
      wasmedge-install-dir: ${{TIKEO_WORKER_SCRIPTS_WASMEDGE_INSTALL_DIR:}}
      wasmedge-installer-url: ${{TIKEO_WORKER_SCRIPTS_WASMEDGE_INSTALLER_URL:https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh}}
      v8-install-version: ${{TIKEO_WORKER_SCRIPTS_V8_INSTALL_VERSION:latest}}
      v8-install-dir: ${{TIKEO_WORKER_SCRIPTS_V8_INSTALL_DIR:}}
      tool-install-timeout-millis: ${{TIKEO_WORKER_SCRIPTS_TOOL_INSTALL_TIMEOUT_MILLIS:120000}}
      images:
        shell: ${{TIKEO_WORKER_SCRIPTS_IMAGE_SHELL:}}
        python: ${{TIKEO_WORKER_SCRIPTS_IMAGE_PYTHON:}}
        js: ${{TIKEO_WORKER_SCRIPTS_IMAGE_JS:}}
        ts: ${{TIKEO_WORKER_SCRIPTS_IMAGE_TS:}}
        powershell: ${{TIKEO_WORKER_SCRIPTS_IMAGE_POWERSHELL:}}
        php: ${{TIKEO_WORKER_SCRIPTS_IMAGE_PHP:}}
        groovy: ${{TIKEO_WORKER_SCRIPTS_IMAGE_GROOVY:}}
        rhai: ${{TIKEO_WORKER_SCRIPTS_IMAGE_RHAI:}}
  management:
    enabled: ${{TIKEO_MANAGEMENT_ENABLED:false}}
    endpoint: ${{TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}}
    api-key: ${{TIKEO_MANAGEMENT_API_KEY:}}
    namespace: ${{TIKEO_NAMESPACE:default}}
    app: ${{TIKEO_APP:{app}}}
"#
    )
}

fn tikeo_properties_block(app: &str) -> String {
    format!(
        r#"
# Generated by tikeo-migrate apply. Review values before enabling production traffic.
# Legacy scheduler workers are disabled in the migrated profile.
powerjob.worker.enabled=false
xxl.job.executor.enabled=false

tikeo.worker.enabled=${{TIKEO_WORKER_ENABLED:true}}
tikeo.worker.auto-startup=${{TIKEO_WORKER_AUTO_STARTUP:true}}
tikeo.worker.endpoint=${{TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}}
tikeo.worker.dry-run=${{TIKEO_WORKER_DRY_RUN:false}}
tikeo.worker.heartbeat-interval-millis=${{TIKEO_WORKER_HEARTBEAT_INTERVAL_MILLIS:10000}}
tikeo.worker.client-instance-id=${{TIKEO_WORKER_CLIENT_INSTANCE_ID:}}
tikeo.worker.state-dir=${{TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}}
tikeo.worker.namespace=${{TIKEO_NAMESPACE:default}}
tikeo.worker.app=${{TIKEO_APP:{app}}}
tikeo.worker.cluster=${{TIKEO_WORKER_CLUSTER:default}}
tikeo.worker.region=${{TIKEO_WORKER_REGION:default}}
tikeo.worker.capabilities[0]=${{TIKEO_WORKER_CAPABILITY_0:java}}
tikeo.worker.capabilities[1]=${{TIKEO_WORKER_CAPABILITY_1:spring-boot}}
tikeo.worker.labels.migrated-from=${{TIKEO_WORKER_LABEL_MIGRATED_FROM:legacy-scheduler}}
tikeo.worker.election.enabled=${{TIKEO_WORKER_ELECTION_ENABLED:true}}
tikeo.worker.election.domain=${{TIKEO_WORKER_ELECTION_DOMAIN:}}
tikeo.worker.election.priority=${{TIKEO_WORKER_ELECTION_PRIORITY:100}}
tikeo.worker.wasm.auto-install=${{TIKEO_WORKER_WASM_AUTO_INSTALL:true}}
tikeo.worker.wasm.install-version=${{TIKEO_WORKER_WASM_INSTALL_VERSION:latest}}
tikeo.worker.wasm.install-dir=${{TIKEO_WORKER_WASM_INSTALL_DIR:}}
tikeo.worker.wasm.installer-url=${{TIKEO_WORKER_WASM_INSTALLER_URL:https://wasmtime.dev/install.sh}}
tikeo.worker.wasm.install-timeout-millis=${{TIKEO_WORKER_WASM_INSTALL_TIMEOUT_MILLIS:120000}}
tikeo.worker.scripts.enabled=${{TIKEO_WORKER_SCRIPTS_ENABLED:true}}
tikeo.worker.scripts.container-enabled=${{TIKEO_WORKER_SCRIPTS_CONTAINER_ENABLED:false}}
tikeo.worker.scripts.availability-check=${{TIKEO_WORKER_SCRIPTS_AVAILABILITY_CHECK:true}}
tikeo.worker.scripts.runtime-command=${{TIKEO_WORKER_SCRIPTS_RUNTIME_COMMAND:}}
# Optional extra container-runtime arguments; uncomment and add entries when needed.
# tikeo.worker.scripts.runtime-args[0]=${{TIKEO_WORKER_SCRIPTS_RUNTIME_ARG_0:}}
tikeo.worker.scripts.auto-install-tools=${{TIKEO_WORKER_SCRIPTS_AUTO_INSTALL_TOOLS:true}}
tikeo.worker.scripts.srt-install-version=${{TIKEO_WORKER_SCRIPTS_SRT_INSTALL_VERSION:latest}}
tikeo.worker.scripts.srt-install-dir=${{TIKEO_WORKER_SCRIPTS_SRT_INSTALL_DIR:}}
tikeo.worker.scripts.ripgrep-install-version=${{TIKEO_WORKER_SCRIPTS_RIPGREP_INSTALL_VERSION:latest}}
tikeo.worker.scripts.ripgrep-install-dir=${{TIKEO_WORKER_SCRIPTS_RIPGREP_INSTALL_DIR:}}
tikeo.worker.scripts.deno-install-version=${{TIKEO_WORKER_SCRIPTS_DENO_INSTALL_VERSION:latest}}
tikeo.worker.scripts.deno-install-dir=${{TIKEO_WORKER_SCRIPTS_DENO_INSTALL_DIR:}}
tikeo.worker.scripts.deno-installer-url=${{TIKEO_WORKER_SCRIPTS_DENO_INSTALLER_URL:https://deno.land/install.sh}}
tikeo.worker.scripts.rhai-install-version=${{TIKEO_WORKER_SCRIPTS_RHAI_INSTALL_VERSION:}}
tikeo.worker.scripts.rhai-install-dir=${{TIKEO_WORKER_SCRIPTS_RHAI_INSTALL_DIR:}}
tikeo.worker.scripts.power-shell-install-version=${{TIKEO_WORKER_SCRIPTS_POWER_SHELL_INSTALL_VERSION:7.5.4}}
tikeo.worker.scripts.power-shell-install-dir=${{TIKEO_WORKER_SCRIPTS_POWER_SHELL_INSTALL_DIR:}}
tikeo.worker.scripts.wasmedge-auto-install=${{TIKEO_WORKER_SCRIPTS_WASMEDGE_AUTO_INSTALL:false}}
tikeo.worker.scripts.wasmedge-install-version=${{TIKEO_WORKER_SCRIPTS_WASMEDGE_INSTALL_VERSION:latest}}
tikeo.worker.scripts.wasmedge-install-dir=${{TIKEO_WORKER_SCRIPTS_WASMEDGE_INSTALL_DIR:}}
tikeo.worker.scripts.wasmedge-installer-url=${{TIKEO_WORKER_SCRIPTS_WASMEDGE_INSTALLER_URL:https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh}}
tikeo.worker.scripts.v8-install-version=${{TIKEO_WORKER_SCRIPTS_V8_INSTALL_VERSION:latest}}
tikeo.worker.scripts.v8-install-dir=${{TIKEO_WORKER_SCRIPTS_V8_INSTALL_DIR:}}
tikeo.worker.scripts.tool-install-timeout-millis=${{TIKEO_WORKER_SCRIPTS_TOOL_INSTALL_TIMEOUT_MILLIS:120000}}
tikeo.worker.scripts.images.shell=${{TIKEO_WORKER_SCRIPTS_IMAGE_SHELL:}}
tikeo.worker.scripts.images.python=${{TIKEO_WORKER_SCRIPTS_IMAGE_PYTHON:}}
tikeo.worker.scripts.images.js=${{TIKEO_WORKER_SCRIPTS_IMAGE_JS:}}
tikeo.worker.scripts.images.ts=${{TIKEO_WORKER_SCRIPTS_IMAGE_TS:}}
tikeo.worker.scripts.images.powershell=${{TIKEO_WORKER_SCRIPTS_IMAGE_POWERSHELL:}}
tikeo.worker.scripts.images.php=${{TIKEO_WORKER_SCRIPTS_IMAGE_PHP:}}
tikeo.worker.scripts.images.groovy=${{TIKEO_WORKER_SCRIPTS_IMAGE_GROOVY:}}
tikeo.worker.scripts.images.rhai=${{TIKEO_WORKER_SCRIPTS_IMAGE_RHAI:}}
tikeo.management.enabled=${{TIKEO_MANAGEMENT_ENABLED:false}}
tikeo.management.endpoint=${{TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}}
tikeo.management.api-key=${{TIKEO_MANAGEMENT_API_KEY:}}
tikeo.management.namespace=${{TIKEO_NAMESPACE:default}}
tikeo.management.app=${{TIKEO_APP:{app}}}
"#
    )
}

fn infer_app_name(project: &Path) -> Option<String> {
    let text = fs::read_to_string(project.join("pom.xml")).ok()?;
    let parent_start = text.find("<parent>");
    let parent_end = text
        .find("</parent>")
        .map(|index| index + "</parent>".len());
    let simplified = match (parent_start, parent_end) {
        (Some(start), Some(end)) if start < end => format!("{}{}", &text[..start], &text[end..]),
        _ => text,
    };
    let start = simplified.find("<artifactId>")? + "<artifactId>".len();
    let end = simplified[start..].find("</artifactId>")? + start;
    Some(simplified[start..end].trim().to_owned()).filter(|value| !value.is_empty())
}

fn transform_powerjob_handler(content: &str, processor_name: &str) -> String {
    let mut output = content.to_owned();
    output = output.replace(
        "import tech.powerjob.worker.core.processor.ProcessResult;\n",
        "import net.tikeo.processor.TaskOutcome;\n",
    );
    output = output.replace(
        "import tech.powerjob.worker.core.processor.TaskContext;\n",
        "import net.tikeo.processor.TaskContext;\n",
    );
    output = output.replace(
        "import tech.powerjob.worker.core.processor.sdk.BasicProcessor;\n",
        "import net.tikeo.processor.TikeoProcessor;\n",
    );
    if output.contains("getJobParams()")
        && !output.contains("import java.nio.charset.StandardCharsets;")
    {
        output = insert_import(output, "import java.nio.charset.StandardCharsets;\n");
    }
    output = output.replace(" implements BasicProcessor", "");
    output = output.replace(
        "ProcessResult process(TaskContext context)",
        "TaskOutcome process(TaskContext context)",
    );
    output = output.replace(
        "public ProcessResult process(TaskContext context)",
        "public TaskOutcome process(TaskContext context)",
    );
    output = output.replace("new ProcessResult(true,", "new TaskOutcome(true,");
    output = output.replace("new ProcessResult(false,", "new TaskOutcome(false,");
    output = output.replace(
        "return new ProcessResult(true);",
        "return TaskOutcome.succeeded();",
    );
    output = output.replace(
        "return new ProcessResult(false);",
        "return TaskOutcome.failed(\"\");",
    );
    output = output.replace(
        "context.getJobParams()",
        "new String(context.payload(), StandardCharsets.UTF_8)",
    );
    output = remove_override_before_process(output);
    add_annotation_before_process(output, processor_name)
}

fn transform_xxl_handler(content: &str, processor_name: &str) -> String {
    let mut output = content.to_owned();
    output = output.replace(
        "import com.xxl.job.core.handler.annotation.XxlJob;",
        "import net.tikeo.processor.TikeoProcessor;",
    );
    let mut lines = Vec::new();
    for line in output.lines() {
        if line.contains("@XxlJob(") {
            lines.push(format!("    @TikeoProcessor(\"{processor_name}\")"));
        } else {
            lines.push(line.to_owned());
        }
    }
    let mut transformed = lines.join("\n");
    if output.ends_with('\n') {
        transformed.push('\n');
    }
    transformed
}

fn remove_override_before_process(content: String) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim() == "@Override"
            && lines
                .get(index + 1)
                .is_some_and(|next| next.contains(" process(TaskContext context)"))
        {
            index += 1;
            continue;
        }
        output.push(lines[index].to_owned());
        index += 1;
    }
    let mut result = output.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn add_annotation_before_process(content: String, processor_name: &str) -> String {
    if content.contains("@TikeoProcessor(") {
        return content;
    }
    let mut lines = Vec::new();
    let mut inserted = false;
    for line in content.lines() {
        if !inserted && line.contains(" process(TaskContext context)") {
            let indent = line
                .chars()
                .take_while(|ch| ch.is_whitespace())
                .collect::<String>();
            lines.push(format!("{indent}@TikeoProcessor(\"{processor_name}\")"));
            inserted = true;
        }
        lines.push(line.to_owned());
    }
    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn insert_import(content: String, import_line: &str) -> String {
    if let Some(index) = content.rfind("import ") {
        let insert_at = content[index..]
            .find('\n')
            .map_or(content.len(), |offset| index + offset + 1);
        let mut output = content;
        output.insert_str(insert_at, import_line);
        output
    } else if let Some(index) = content.find(';') {
        let mut output = content;
        output.insert_str(index + 1, &format!("\n\n{import_line}"));
        output
    } else {
        format!("{import_line}{content}")
    }
}

fn write_code_apply_outputs(
    command: &ApplyCommand,
    output_project: &Path,
    evidence: &CodeApplyEvidence,
) -> Result<()> {
    let evidence_path = command
        .output
        .clone()
        .unwrap_or_else(|| command.bundle.join("code-apply-evidence.json"));
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&evidence_path, serde_json::to_string_pretty(evidence)?)
        .with_context(|| format!("failed to write {}", evidence_path.display()))?;
    let report = render_code_apply_report(evidence);
    fs::write(
        output_project.join("CODE_MIGRATION_REPORT.md"),
        report.clone(),
    )
    .with_context(|| "failed to write CODE_MIGRATION_REPORT.md".to_owned())?;
    fs::write(command.bundle.join("CODE_MIGRATION_REPORT.md"), report)
        .with_context(|| "failed to write bundle CODE_MIGRATION_REPORT.md".to_owned())?;
    Ok(())
}

fn render_code_apply_report(evidence: &CodeApplyEvidence) -> String {
    let mut output = format!(
        "# Tikeo code migration report\n\n- Source project: `{}`\n- Output project: `{}`\n- Bundle: `{}`\n\n## Changed files\n\n",
        evidence.source_project, evidence.output_project, evidence.bundle
    );
    for file in &evidence.changed_files {
        output.push_str(&format!("- `{file}`\n"));
    }
    if evidence.changed_files.is_empty() {
        output.push_str("- No files changed.\n");
    }
    output.push_str("\n## Skipped copied paths\n\n");
    for file in &evidence.skipped_paths {
        output.push_str(&format!("- `{file}`\n"));
    }
    if evidence.skipped_paths.is_empty() {
        output.push_str("- None.\n");
    }
    output.push_str("\n## Warnings\n\n");
    for warning in &evidence.warnings {
        output.push_str(&format!("- {warning}\n"));
    }
    if evidence.warnings.is_empty() {
        output.push_str("- None.\n");
    }
    output
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}
