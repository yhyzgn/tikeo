use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{CodeApplyCommand, JavaProjectMigrationPlan};

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

pub(crate) fn apply_code(command: &CodeApplyCommand) -> Result<CodeApplyEvidence> {
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
    plan: &JavaProjectMigrationPlan,
    changed: &mut Vec<String>,
) -> Result<()> {
    let resources = project.join("src/main/resources");
    fs::create_dir_all(&resources)
        .with_context(|| format!("failed to create {}", resources.display()))?;
    let path = resources.join("application-tikeo-migration.yml");
    let app = infer_app_name(project).unwrap_or_else(|| "default".to_owned());
    let content = format!(
        "# Generated by tikeo-migrate apply-code. Import or merge this profile after review.\n\n# Legacy scheduler workers are disabled in the migrated profile.\npowerjob:\n  worker:\n    enabled: false\nxxl:\n  job:\n    executor:\n      enabled: false\n\ntikeo:\n  worker:\n    enabled: true\n    endpoint: ${{TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}}\n    namespace: ${{TIKEO_NAMESPACE:default}}\n    app: ${{TIKEO_APP:{app}}}\n    state-dir: ${{TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}}\n    scripts:\n      auto-install-tools: false\n    wasm:\n      auto-install: false\n\n# Recommended dependency artifact: net.tikeo:{}:${{TIKEO_VERSION}}\n",
        plan.recommended_artifact
    );
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    push_unique(
        changed,
        "src/main/resources/application-tikeo-migration.yml".to_owned(),
    );
    Ok(())
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
    command: &CodeApplyCommand,
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
