use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::{
    HandlerCandidate, MigrationSource, detect_powerjob_handlers, detect_xxl_handlers, first_quoted,
};

pub(crate) fn build_code_only_export_json(
    project_root: &Path,
    source: MigrationSource,
) -> Result<String> {
    let handlers = scan_code_only_handlers(project_root, source)?;
    if handlers.is_empty() {
        bail!(
            "failed to auto-detect legacy scheduler input under {}. No legacy scheduler DB/export was found and no Java handler candidates were detected for {}",
            project_root.display(),
            source.as_str()
        );
    }
    let jobs = handlers
        .into_iter()
        .enumerate()
        .map(|(index, handler)| code_only_job(project_root, source, index, handler))
        .collect::<Vec<_>>();
    Ok(serde_json::to_string(&json!({ "jobs": jobs }))?)
}

fn code_only_job(
    project_root: &Path,
    source: MigrationSource,
    index: usize,
    handler: HandlerCandidate,
) -> Value {
    match source {
        MigrationSource::XxlJob => json!({
            "id": format!("code-only-{}", index + 1),
            "jobDesc": handler.processor_name,
            "executorAppName": infer_project_app_name(project_root).unwrap_or_else(|| "default".to_owned()),
            "scheduleType": "API",
            "executorHandler": handler.processor_name,
            "triggerStatus": 0,
            "codeOnly": true,
            "sourcePath": handler.path,
            "migrationNote": "Generated from legacy Java handler source because no scheduler DB/export was available. Review schedule, routing, retry, and enablement before applying."
        }),
        MigrationSource::PowerJob => json!({
            "id": format!("code-only-{}", index + 1),
            "jobName": handler.processor_name,
            "appName": infer_project_app_name(project_root).unwrap_or_else(|| "default".to_owned()),
            "timeExpressionType": "API",
            "processorInfo": handler.processor_name,
            "instanceRetryNum": 0,
            "status": 0,
            "codeOnly": true,
            "sourcePath": handler.path,
            "migrationNote": "Generated from legacy Java handler source because no scheduler DB/export was available. Review schedule, routing, retry, and enablement before applying."
        }),
    }
}

fn scan_code_only_handlers(
    project_root: &Path,
    source: MigrationSource,
) -> Result<Vec<HandlerCandidate>> {
    let mut handlers = Vec::new();
    collect_code_only_handlers(project_root, project_root, source, &mut handlers)?;
    handlers.sort_by(|left, right| {
        left.processor_name
            .cmp(&right.processor_name)
            .then_with(|| left.path.cmp(&right.path))
    });
    handlers.dedup_by(|left, right| {
        left.processor_name == right.processor_name && left.path == right.path
    });
    Ok(handlers)
}

fn collect_code_only_handlers(
    root: &Path,
    directory: &Path,
    source: MigrationSource,
    handlers: &mut Vec<HandlerCandidate>,
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
                .is_some_and(|name| {
                    matches!(name, "build" | "target" | ".gradle" | ".git" | ".idea")
                })
            {
                continue;
            }
            collect_code_only_handlers(root, &path, source, handlers)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("java") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let candidates = match source {
                MigrationSource::XxlJob => detect_xxl_handlers(&content),
                MigrationSource::PowerJob => detect_powerjob_handlers(&content),
            };
            for (processor_name, method_name) in candidates {
                handlers.push(HandlerCandidate {
                    path: relative.clone(),
                    framework: source.as_str().to_owned(),
                    processor_name,
                    method_name,
                });
            }
        }
    }
    Ok(())
}

fn infer_project_app_name(project_root: &Path) -> Option<String> {
    for relative in ["pom.xml", "build.gradle.kts", "build.gradle"] {
        let path = project_root.join(relative);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(path).ok()?;
        if relative == "pom.xml" {
            if let Some(name) = maven_project_artifact_id(&content) {
                return Some(name);
            }
        }
        for prefix in ["rootProject.name", "archivesBaseName"] {
            for line in content.lines() {
                if line.trim_start().starts_with(prefix) {
                    if let Some(value) = first_quoted(line).or_else(|| {
                        line.split('=')
                            .nth(1)
                            .map(|value| value.trim().trim_matches(['"', '\'']).to_owned())
                    }) && !value.is_empty()
                    {
                        return Some(value);
                    }
                }
            }
        }
    }
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn maven_project_artifact_id(content: &str) -> Option<String> {
    let without_parent =
        if let (Some(start), Some(end)) = (content.find("<parent>"), content.find("</parent>")) {
            let after_parent = end + "</parent>".len();
            format!("{}{}", &content[..start], &content[after_parent..])
        } else {
            content.to_owned()
        };
    first_xml_text_after(&without_parent, "<artifactId>")
}

fn first_xml_text_after(content: &str, tag: &str) -> Option<String> {
    let start = content.find(tag)? + tag.len();
    let end = content[start..].find("</")? + start;
    let value = content[start..end].trim();
    (!value.is_empty()).then(|| value.to_owned())
}
