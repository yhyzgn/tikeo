use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApplyCommand, JavaProjectMigrationPlan};

const DEFAULT_TIKEO_VERSION: &str = "0.3.10";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeApplyEvidence {
    pub source_project: String,
    pub target_project: String,
    pub bundle: String,
    pub changed_files: Vec<String>,
    pub skipped_paths: Vec<String>,
    pub data_import_summary: Option<CodeApplyDataImportSummary>,
    pub semantic_review_items: Vec<CodeApplySemanticReviewItem>,
    pub next_actions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeApplyDataImportSummary {
    pub source: Option<String>,
    pub mode: Option<String>,
    pub total: usize,
    pub ready: usize,
    pub needs_review: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeApplySemanticReviewItem {
    pub source_id: String,
    pub source_name: String,
    pub status: String,
    pub processor_name: Option<String>,
    pub unsupported_features: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleMigrationReport {
    source: Option<String>,
    mode: Option<String>,
    summary: Option<BundleMigrationSummary>,
    #[serde(default)]
    jobs: Vec<BundleJobPlan>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleMigrationSummary {
    total: usize,
    ready: usize,
    needs_review: usize,
    skipped: usize,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleDataImportPlan {
    #[serde(default)]
    ready: Vec<Value>,
    #[serde(default)]
    needs_review: Vec<Value>,
    #[serde(default)]
    skipped: Vec<Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleJobPlan {
    #[serde(default)]
    source_id: String,
    #[serde(default)]
    source_name: String,
    #[serde(default)]
    status: String,
    tikeo_job: Option<BundleTikeoJobDraft>,
    #[serde(default)]
    unsupported_features: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleTikeoJobDraft {
    processor_name: Option<String>,
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
    let target_project = source_project.clone();
    let skipped_paths = Vec::new();
    let mut changed_files = Vec::new();
    let mut warnings = Vec::new();
    apply_dependency(&target_project, &plan, &mut changed_files, &mut warnings)?;
    apply_worker_config(&target_project, &plan, &mut changed_files)?;
    for handler in &plan.handler_candidates {
        let path = target_project.join(&handler.path);
        if !path.exists() {
            warnings.push(format!(
                "handler source file missing in target project: {}",
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
    let (data_import_summary, semantic_review_items) =
        read_bundle_review_context(&command.bundle, &mut warnings);
    let evidence = CodeApplyEvidence {
        source_project: source_project.display().to_string(),
        target_project: target_project.display().to_string(),
        bundle: command.bundle.display().to_string(),
        changed_files,
        skipped_paths,
        data_import_summary,
        semantic_review_items,
        next_actions: code_apply_next_actions(),
        warnings,
    };
    write_code_apply_outputs(command, &target_project, &evidence)?;
    Ok(evidence)
}

fn apply_dependency(
    project: &Path,
    plan: &JavaProjectMigrationPlan,
    changed: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    match plan.build_system.as_str() {
        "maven" => apply_maven_dependency(project, plan, changed),
        "gradle-kotlin" => apply_gradle_dependency(
            project,
            "build.gradle.kts",
            &plan.recommended_artifact,
            &tikeo_version_from_plan(plan),
            changed,
        ),
        "gradle-groovy" => apply_gradle_dependency(
            project,
            "build.gradle",
            &plan.recommended_artifact,
            &tikeo_version_from_plan(plan),
            changed,
        ),
        other => {
            warnings.push(format!(
                "unsupported build system `{other}`; dependency was not edited automatically"
            ));
            Ok(())
        }
    }
}

fn apply_maven_dependency(
    project: &Path,
    plan: &JavaProjectMigrationPlan,
    changed: &mut Vec<String>,
) -> Result<()> {
    let artifact = &plan.recommended_artifact;
    let version = tikeo_version_from_plan(plan);
    let path = project.join("pom.xml");
    let mut text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let before = text.clone();
    text = remove_legacy_scheduler_dependencies(&text);
    text = remove_legacy_scheduler_version_properties(&text);
    text = ensure_maven_tikeo_version_property(&text, &version);
    if !text.contains(&format!("<artifactId>{artifact}</artifactId>")) {
        if has_dependency_management(&text) {
            text = ensure_maven_managed_dependency(&text, artifact)?;
            text = ensure_maven_direct_dependency(&text, artifact, false)?;
        } else {
            text = ensure_maven_direct_dependency(&text, artifact, true)?;
        }
    }
    text = trim_whitespace_only_lines(&text);
    if text == before {
        return Ok(());
    }
    fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))?;
    push_unique(changed, "pom.xml".to_owned());
    Ok(())
}

fn trim_whitespace_only_lines(text: &str) -> String {
    let mut output = text
        .lines()
        .map(|line| if line.trim().is_empty() { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n");
    if text.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn has_dependency_management(text: &str) -> bool {
    text.contains("<dependencyManagement>")
}

fn tikeo_version_from_plan(plan: &JavaProjectMigrationPlan) -> String {
    let snippet = plan.dependency_snippet.trim();
    let version = if let Some(start) = snippet.find("<version>") {
        let value_start = start + "<version>".len();
        snippet[value_start..]
            .find("</version>")
            .map(|end| snippet[value_start..value_start + end].trim().to_owned())
    } else if let Some(start) = snippet.find(&format!("net.tikeo:{}:", plan.recommended_artifact)) {
        let value_start = start + format!("net.tikeo:{}:", plan.recommended_artifact).len();
        let tail = &snippet[value_start..];
        let value = tail
            .trim_end_matches(')')
            .trim_end_matches('"')
            .trim_end_matches('\'')
            .trim()
            .to_owned();
        (!value.is_empty()).then_some(value)
    } else {
        None
    }
    .unwrap_or_else(|| DEFAULT_TIKEO_VERSION.to_owned());

    if version.contains("${TIKEO_VERSION}") || version.contains("<") || version.contains(">") {
        DEFAULT_TIKEO_VERSION.to_owned()
    } else {
        version
    }
}

fn remove_legacy_scheduler_dependencies(text: &str) -> String {
    let mut output = text.to_owned();
    for marker in [
        "<groupId>tech.powerjob</groupId>",
        "<groupId>com.xuxueli</groupId>",
        "<artifactId>xxl-job-core</artifactId>",
    ] {
        output = remove_dependency_blocks_containing(&output, marker);
    }
    output
}

fn remove_dependency_blocks_containing(text: &str, marker: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("<dependency>") {
        let start = cursor + relative_start;
        let Some(relative_end) = text[start..].find("</dependency>") else {
            break;
        };
        let end = start + relative_end + "</dependency>".len();
        let block = &text[start..end];
        if block.contains(marker) {
            output.push_str(&text[cursor..start]);
            cursor = consume_following_blank_line(text, end);
        } else {
            output.push_str(&text[cursor..end]);
            cursor = end;
        }
    }
    output.push_str(&text[cursor..]);
    output
}

fn consume_following_blank_line(text: &str, index: usize) -> usize {
    let bytes = text.as_bytes();
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'\r') {
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'\n') {
        let next = cursor + 1;
        if text[next..]
            .lines()
            .next()
            .is_some_and(|line| line.trim().is_empty())
        {
            return next + text[next..].find('\n').map_or(0, |offset| offset + 1);
        }
    }
    index
}

fn remove_legacy_scheduler_version_properties(text: &str) -> String {
    text.lines()
        .filter(|line| !line.contains("<powerjob.version>") && !line.contains("<xxl-job.version>"))
        .collect::<Vec<_>>()
        .join("\n")
        + if text.ends_with('\n') { "\n" } else { "" }
}

fn ensure_maven_tikeo_version_property(text: &str, version: &str) -> String {
    if text.contains("<tikeo.version>") {
        return text.to_owned();
    }
    if let Some(index) = text.find("</properties>") {
        let mut output = text.to_owned();
        let (insert_at, closing_indent) = line_start_and_indent_at(text, index);
        let indent = format!("{closing_indent}    ");
        output.insert_str(
            insert_at,
            &format!(
                "{indent}<!-- tikeo-migrate default: replace with the release badge version when upgrading. -->\n{indent}<tikeo.version>{version}</tikeo.version>\n"
            ),
        );
        output
    } else {
        text.to_owned()
    }
}

fn line_start_and_indent_at(text: &str, index: usize) -> (usize, String) {
    let line_start = text[..index].rfind('\n').map_or(0, |position| position + 1);
    let line_prefix = &text[line_start..index];
    let indent = line_prefix
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    if indent.is_empty() {
        (line_start, "    ".to_owned())
    } else {
        (line_start, indent)
    }
}

fn ensure_maven_managed_dependency(text: &str, artifact: &str) -> Result<String> {
    if text.contains(&format!("<artifactId>{artifact}</artifactId>")) {
        return Ok(text.to_owned());
    }
    let dependency_management_end = text
        .find("</dependencyManagement>")
        .context("pom.xml contains <dependencyManagement> but no </dependencyManagement>")?;
    let search = &text[..dependency_management_end];
    let dependencies_end = search
        .rfind("</dependencies>")
        .context("dependencyManagement does not contain </dependencies>")?;
    let (insert_at, closing_indent) = line_start_and_indent_at(text, dependencies_end);
    let indent = format!("{closing_indent}    ");
    let child = format!("{indent}    ");
    let managed = format!(
        "{indent}<dependency>\n{child}<groupId>net.tikeo</groupId>\n{child}<artifactId>{artifact}</artifactId>\n{child}<version>${{tikeo.version}}</version>\n{indent}</dependency>\n"
    );
    let mut output = text.to_owned();
    output.insert_str(insert_at, &managed);
    Ok(output)
}

fn ensure_maven_direct_dependency(
    text: &str,
    artifact: &str,
    include_version: bool,
) -> Result<String> {
    let dependency_management_end = text
        .find("</dependencyManagement>")
        .map_or(0, |index| index + "</dependencyManagement>".len());
    let Some(relative_end) = text[dependency_management_end..].find("</dependencies>") else {
        let project_end = text
            .find("</project>")
            .context("pom.xml does not contain </project>")?;
        let dependency = direct_dependency_block(artifact, include_version, "        ");
        let mut output = text.to_owned();
        output.insert_str(
            project_end,
            &format!("    <dependencies>\n{dependency}    </dependencies>\n"),
        );
        return Ok(output);
    };
    let dependencies_end = dependency_management_end + relative_end;
    let (insert_at, closing_indent) = line_start_and_indent_at(text, dependencies_end);
    let indent = format!("{closing_indent}    ");
    let dependency = direct_dependency_block(artifact, include_version, &indent);
    let mut output = text.to_owned();
    output.insert_str(insert_at, &dependency);
    Ok(output)
}

fn direct_dependency_block(artifact: &str, include_version: bool, base: &str) -> String {
    let child = format!("{base}    ");
    let version = if include_version {
        format!("{child}<version>${{tikeo.version}}</version>\n")
    } else {
        String::new()
    };
    format!(
        "{base}<dependency>\n{child}<groupId>net.tikeo</groupId>\n{child}<artifactId>{artifact}</artifactId>\n{version}{base}</dependency>\n"
    )
}

fn apply_gradle_dependency(
    project: &Path,
    file: &str,
    artifact: &str,
    version: &str,
    changed: &mut Vec<String>,
) -> Result<()> {
    let path = project.join(file);
    let mut text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    if text.contains(&format!("net.tikeo:{artifact}:")) {
        return Ok(());
    }
    let line = if file.ends_with(".kts") {
        format!("    implementation(\"net.tikeo:{artifact}:{version}\")\n")
    } else {
        format!("    implementation 'net.tikeo:{artifact}:{version}'\n")
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
        let relative = markdown_path(path.strip_prefix(project).unwrap_or(&path));
        let mut content = if path.exists() {
            fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?
        } else {
            String::new()
        };
        if content.contains("Generated by tikeo-migrate apply") {
            continue;
        }
        let is_properties = path.extension().and_then(|ext| ext.to_str()) == Some("properties");
        content = if is_properties {
            remove_legacy_scheduler_properties(content)
        } else {
            remove_legacy_scheduler_yaml(content)
        };
        let block = if is_properties {
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

fn remove_legacy_scheduler_properties(content: String) -> String {
    let mut output = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("powerjob.")
            || lower.starts_with("power-job.")
            || lower.starts_with("xxl.job.")
            || lower.starts_with("xxl-job.")
            || lower.starts_with("xxl.")
        {
            continue;
        }
        output.push(line.to_owned());
    }
    let mut result = output.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn remove_legacy_scheduler_yaml(content: String) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        let indent = leading_whitespace_count(line);
        if indent == 0 && is_legacy_scheduler_yaml_root(trimmed) {
            index += 1;
            while index < lines.len() {
                let next = lines[index];
                let next_trimmed = next.trim();
                let next_indent = leading_whitespace_count(next);
                if !next_trimmed.is_empty() && next_indent == 0 {
                    break;
                }
                index += 1;
            }
            continue;
        }
        if indent == 0 && is_legacy_scheduler_flat_yaml_key(trimmed) {
            index += 1;
            continue;
        }
        output.push(line.to_owned());
        index += 1;
    }
    let mut result = output.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn leading_whitespace_count(line: &str) -> usize {
    line.chars()
        .take_while(|character| character.is_whitespace())
        .count()
}

fn is_legacy_scheduler_yaml_root(trimmed: &str) -> bool {
    let key = trimmed
        .split_once(':')
        .map_or(trimmed, |(key, _)| key)
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase();
    matches!(
        key.as_str(),
        "powerjob" | "power-job" | "xxl" | "xxl-job" | "xxl_job"
    )
}

fn is_legacy_scheduler_flat_yaml_key(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("powerjob.")
        || lower.starts_with("power-job.")
        || lower.starts_with("xxl.job.")
        || lower.starts_with("xxl-job.")
        || lower.starts_with("xxl.")
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
    let relative = markdown_path(path.strip_prefix(project).unwrap_or(&path));
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
# Legacy scheduler powerjob/xxl config blocks are removed from this file during migration.
# This is intentionally minimal: keep advanced tuning in deployment docs or your ops overlay.

tikeo:
  worker:
    enabled: ${{TIKEO_WORKER_ENABLED:true}}
    endpoint: ${{TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}}
    namespace: ${{TIKEO_NAMESPACE:default}}
    app: ${{TIKEO_APP:{app}}}
    cluster: ${{TIKEO_WORKER_CLUSTER:default}}
    capabilities:
      - ${{TIKEO_WORKER_CAPABILITY_0:java}}
      - ${{TIKEO_WORKER_CAPABILITY_1:spring-boot}}
    labels:
      migrated-from: legacy-scheduler
    # Recommended when generated Worker identity should survive restarts.
    state-dir: ${{TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}}
  # Optional: enable only if this service uses the Tikeo Management SDK/API.
  management:
    enabled: ${{TIKEO_MANAGEMENT_ENABLED:false}}
    endpoint: ${{TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}}
    api-key: ${{TIKEO_MANAGEMENT_API_KEY:}}
"#
    )
}

fn tikeo_properties_block(app: &str) -> String {
    format!(
        r#"
# Generated by tikeo-migrate apply. Review values before enabling production traffic.
# Legacy scheduler powerjob/xxl config keys are removed from this file during migration.
# This is intentionally minimal: keep advanced tuning in deployment docs or your ops overlay.

tikeo.worker.enabled=${{TIKEO_WORKER_ENABLED:true}}
tikeo.worker.endpoint=${{TIKEO_WORKER_ENDPOINT:http://127.0.0.1:9998}}
tikeo.worker.namespace=${{TIKEO_NAMESPACE:default}}
tikeo.worker.app=${{TIKEO_APP:{app}}}
tikeo.worker.cluster=${{TIKEO_WORKER_CLUSTER:default}}
tikeo.worker.capabilities[0]=${{TIKEO_WORKER_CAPABILITY_0:java}}
tikeo.worker.capabilities[1]=${{TIKEO_WORKER_CAPABILITY_1:spring-boot}}
tikeo.worker.labels.migrated-from=legacy-scheduler
# Recommended when generated Worker identity should survive restarts.
tikeo.worker.state-dir=${{TIKEO_WORKER_STATE_DIR:~/.tikeo/workers}}
# Optional: enable only if this service uses the Tikeo Management SDK/API.
tikeo.management.enabled=${{TIKEO_MANAGEMENT_ENABLED:false}}
tikeo.management.endpoint=${{TIKEO_MANAGEMENT_ENDPOINT:http://127.0.0.1:9090}}
tikeo.management.api-key=${{TIKEO_MANAGEMENT_API_KEY:}}
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
    let processor_name = lower_camel_processor_name(processor_name);
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
    let output = add_annotation_before_process(output, &processor_name);
    ensure_javadoc_before_tikeo_processor(output)
}

fn transform_xxl_handler(content: &str, processor_name: &str) -> String {
    let processor_name = lower_camel_processor_name(processor_name);
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

fn lower_camel_processor_name(value: &str) -> String {
    let mut chars = value.chars().collect::<Vec<_>>();
    if chars
        .first()
        .is_none_or(|first| !first.is_ascii_uppercase())
    {
        return value.to_owned();
    }
    let mut uppercase_prefix = 0;
    for character in &chars {
        if character.is_ascii_uppercase() {
            uppercase_prefix += 1;
        } else {
            break;
        }
    }
    let chars_to_lower = if uppercase_prefix > 1
        && chars
            .get(uppercase_prefix)
            .is_some_and(|character| character.is_ascii_lowercase())
    {
        uppercase_prefix - 1
    } else {
        1
    };
    for character in chars.iter_mut().take(chars_to_lower) {
        character.make_ascii_lowercase();
    }
    chars.into_iter().collect()
}

fn ensure_javadoc_before_tikeo_processor(content: String) -> String {
    let input_lines = content.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    for (index, line) in input_lines.iter().enumerate() {
        if line.trim_start().starts_with("@TikeoProcessor(")
            && !previous_significant_line_is_javadoc_end(&input_lines, index)
        {
            let indent = line
                .chars()
                .take_while(|character| character.is_whitespace())
                .collect::<String>();
            output.push(format!("{indent}/**"));
            output.push(format!("{indent} * Tikeo 任务执行入口。"));
            output.push(format!("{indent} *"));
            output.push(format!("{indent} * @param context 任务上下文"));
            output.push(format!("{indent} * @return 执行结果"));
            output.push(format!("{indent} */"));
        }
        output.push((*line).to_owned());
    }
    let mut result = output.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn previous_significant_line_is_javadoc_end(lines: &[&str], index: usize) -> bool {
    lines[..index]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim() == "*/")
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

fn read_bundle_review_context(
    bundle: &Path,
    warnings: &mut Vec<String>,
) -> (
    Option<CodeApplyDataImportSummary>,
    Vec<CodeApplySemanticReviewItem>,
) {
    let report_path = bundle.join("jobs.tikeo.json");
    let report = match fs::read_to_string(&report_path) {
        Ok(text) => match serde_json::from_str::<BundleMigrationReport>(&text) {
            Ok(report) => Some(report),
            Err(error) => {
                warnings.push(format!(
                    "failed to parse {}; semantic migration review summary was not embedded: {error}",
                    report_path.display()
                ));
                None
            }
        },
        Err(error) => {
            warnings.push(format!(
                "failed to read {}; semantic migration review summary was not embedded: {error}",
                report_path.display()
            ));
            None
        }
    };

    let data_path = bundle.join("data-import-plan.json");
    let data_plan = match fs::read_to_string(&data_path) {
        Ok(text) => match serde_json::from_str::<BundleDataImportPlan>(&text) {
            Ok(plan) => Some(plan),
            Err(error) => {
                warnings.push(format!(
                    "failed to parse {}; data import summary fell back to jobs.tikeo.json: {error}",
                    data_path.display()
                ));
                None
            }
        },
        Err(error) => {
            warnings.push(format!(
                "failed to read {}; data import summary fell back to jobs.tikeo.json: {error}",
                data_path.display()
            ));
            None
        }
    };

    let data_import_summary = data_plan
        .as_ref()
        .map(|plan| CodeApplyDataImportSummary {
            source: report.as_ref().and_then(|report| report.source.clone()),
            mode: report.as_ref().and_then(|report| report.mode.clone()),
            total: plan.ready.len() + plan.needs_review.len() + plan.skipped.len(),
            ready: plan.ready.len(),
            needs_review: plan.needs_review.len(),
            skipped: plan.skipped.len(),
        })
        .or_else(|| {
            report.as_ref().and_then(|report| {
                report
                    .summary
                    .as_ref()
                    .map(|summary| CodeApplyDataImportSummary {
                        source: report.source.clone(),
                        mode: report.mode.clone(),
                        total: summary.total,
                        ready: summary.ready,
                        needs_review: summary.needs_review,
                        skipped: summary.skipped,
                    })
            })
        });

    let semantic_review_items = report
        .map(|report| {
            report
                .jobs
                .into_iter()
                .filter(|job| {
                    job.status != "ready"
                        || !job.unsupported_features.is_empty()
                        || !job.warnings.is_empty()
                })
                .map(|job| CodeApplySemanticReviewItem {
                    source_id: if job.source_id.is_empty() {
                        "unknown".to_owned()
                    } else {
                        job.source_id
                    },
                    source_name: if job.source_name.is_empty() {
                        "unknown".to_owned()
                    } else {
                        job.source_name
                    },
                    status: if job.status.is_empty() {
                        "unknown".to_owned()
                    } else {
                        job.status
                    },
                    processor_name: job.tikeo_job.and_then(|draft| draft.processor_name),
                    unsupported_features: job.unsupported_features,
                    warnings: job.warnings,
                })
                .collect()
        })
        .unwrap_or_default();

    (data_import_summary, semantic_review_items)
}

fn code_apply_next_actions() -> Vec<String> {
    vec![
        "Review <tikeo.version>, upgrade it to the release badge version when needed, and compile the project on the migration branch.".to_owned(),
        "Fill tikeo.worker.endpoint, tikeo.management.endpoint, and API-key placeholders for staging before starting traffic.".to_owned(),
        "Review every needs_review job and unsupported feature in jobs.tikeo.json/data-import-plan.json before import.".to_owned(),
        "Import only reviewed ready jobs through the console, Management API, or GitOps; tikeo-migrate apply never calls the server.".to_owned(),
        "Trigger at least one migrated job in staging and compare payload, logs, retries, and result semantics with the legacy scheduler.".to_owned(),
    ]
}

fn write_code_apply_outputs(
    command: &ApplyCommand,
    target_project: &Path,
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
        target_project.join("CODE_MIGRATION_REPORT.md"),
        report.clone(),
    )
    .with_context(|| "failed to write CODE_MIGRATION_REPORT.md".to_owned())?;
    fs::write(command.bundle.join("CODE_MIGRATION_REPORT.md"), report)
        .with_context(|| "failed to write bundle CODE_MIGRATION_REPORT.md".to_owned())?;
    Ok(())
}

fn markdown_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn render_code_apply_report(evidence: &CodeApplyEvidence) -> String {
    let mut output = format!(
        "# Tikeo code migration report\n\n- Source project: `{}`\n- Target project (in-place): `{}`\n- Bundle: `{}`\n\n## Migration result checklist\n\n",
        evidence.source_project, evidence.target_project, evidence.bundle
    );
    if evidence.changed_files.is_empty() {
        output.push_str("- No files changed.\n");
    } else {
        output.push_str("| File | Migration type | Status |\n");
        output.push_str("| --- | --- | --- |\n");
        for file in &evidence.changed_files {
            output.push_str(&format!(
                "| `{file}` | {} | migrated |\n",
                migration_type_for_file(file)
            ));
        }
    }
    output.push_str("\n## Data import summary\n\n");
    if let Some(summary) = &evidence.data_import_summary {
        output.push_str("| Source | Mode | Total | Ready | Needs review | Skipped |\n");
        output.push_str("| --- | --- | ---: | ---: | ---: | ---: |\n");
        output.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} | {} |\n",
            summary.source.as_deref().unwrap_or("unknown"),
            summary.mode.as_deref().unwrap_or("unknown"),
            summary.total,
            summary.ready,
            summary.needs_review,
            summary.skipped
        ));
    } else {
        output.push_str(
            "- No `jobs.tikeo.json` or `data-import-plan.json` summary was available in the bundle.\n",
        );
    }

    output.push_str("\n## Semantic review items\n\n");
    if evidence.semantic_review_items.is_empty() {
        output.push_str(
            "- None. All planned jobs are marked ready and no unsupported features were reported.\n",
        );
    } else {
        output.push_str("| Source ID | Name | Status | Processor | Review reason |\n");
        output.push_str("| --- | --- | --- | --- | --- |\n");
        for item in &evidence.semantic_review_items {
            let mut reasons = item.unsupported_features.clone();
            reasons.extend(item.warnings.clone());
            let reason = reasons.join("; ").replace('|', "\\|");
            output.push_str(&format!(
                "| `{}` | {} | `{}` | `{}` | {} |\n",
                item.source_id,
                item.source_name.replace('|', "\\|"),
                item.status,
                item.processor_name.as_deref().unwrap_or("-"),
                if reason.is_empty() {
                    "manual review"
                } else {
                    reason.as_str()
                }
            ));
        }
    }

    output.push_str("\n## Next manual actions\n\n");
    for action in &evidence.next_actions {
        output.push_str(&format!("- {action}\n"));
    }

    output.push_str("\n## Skipped paths\n\n");
    for file in &evidence.skipped_paths {
        output.push_str(&format!("- `{file}`\n"));
    }
    if evidence.skipped_paths.is_empty() {
        output.push_str("- None. `apply` is in-place and does not copy the project.\n");
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

fn migration_type_for_file(file: &str) -> &'static str {
    if file == "pom.xml" || file.ends_with("build.gradle") || file.ends_with("build.gradle.kts") {
        "dependency"
    } else if file.ends_with(".yml") || file.ends_with(".yaml") || file.ends_with(".properties") {
        "configuration"
    } else if file.ends_with(".java") {
        "executor"
    } else {
        "other"
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}
