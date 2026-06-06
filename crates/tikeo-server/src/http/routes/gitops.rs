#![allow(missing_docs)]

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use sha2::{Digest, Sha256};

use crate::http::{
    AppState, auth,
    dto::{
        AlertRuleSummary, ApiResponse, GitOpsDiffApiResponse, GitOpsDiffChange, GitOpsDiffRequest,
        GitOpsDiffResponse, GitOpsExportQuery, GitOpsManifest, GitOpsManifestApiResponse,
        GitOpsManifestResponse, GitOpsMetadata, GitOpsResource, GitOpsScope,
    },
    error::ApiError,
    routes::common::audit,
};

#[utoipa::path(
    get,
    path = "/api/v1/gitops/manifest",
    tag = "gitops",
    params(GitOpsExportQuery),
    responses((status = 200, description = "Declarative GitOps manifest", body = GitOpsManifestApiResponse))
)]
/// Export the current scope as a declarative `GitOps` manifest.
///
/// # Errors
///
/// Returns authorization, validation, serialization, or storage errors.
pub async fn export_gitops_manifest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<GitOpsExportQuery>,
) -> Result<Json<ApiResponse<GitOpsManifestResponse>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "read").await?;
    audit(
        &state,
        &principal.username,
        "gitops_manifest_export",
        "tenants",
        &query.namespace.clone().unwrap_or_else(|| "all".to_owned()),
        Some(format!("app={:?}", query.app)),
        &headers,
    )
    .await;
    let manifest = build_manifest(&state, query.namespace.clone(), query.app.clone()).await?;
    let format = query.format.unwrap_or_else(|| "json".to_owned());
    if !matches!(format.as_str(), "json" | "yaml") {
        return Err(ApiError::bad_request(
            "gitops manifest format must be json or yaml",
        ));
    }
    let canonical_json = canonical_manifest_json(&manifest)?;
    let manifest_yaml = if format == "yaml" {
        Some(json_to_yaml(
            &serde_json::to_value(&manifest)
                .map_err(|error| ApiError::bad_request(error.to_string()))?,
        ))
    } else {
        None
    };
    Ok(Json(ApiResponse::success(GitOpsManifestResponse {
        manifest,
        format,
        manifest_yaml,
        checksum: sha256_hex(canonical_json.as_bytes()),
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/gitops/diff",
    tag = "gitops",
    request_body = GitOpsDiffRequest,
    responses((status = 200, description = "GitOps manifest drift diff", body = GitOpsDiffApiResponse))
)]
/// Compare a desired `GitOps` manifest against current server state.
///
/// # Errors
///
/// Returns authorization, validation, serialization, or storage errors.
pub async fn diff_gitops_manifest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<GitOpsDiffRequest>,
) -> Result<Json<ApiResponse<GitOpsDiffResponse>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "read").await?;
    let scope = request.manifest.scope.clone();
    audit(
        &state,
        &principal.username,
        "gitops_manifest_diff",
        "tenants",
        &scope.namespace.clone().unwrap_or_else(|| "all".to_owned()),
        Some(format!("app={:?}", scope.app)),
        &headers,
    )
    .await;
    let current = build_manifest(&state, scope.namespace.clone(), scope.app.clone()).await?;
    let changes = diff_resources(&current.resources, &request.manifest.resources)?;
    let summary = summarize_changes(&changes);
    Ok(Json(ApiResponse::success(GitOpsDiffResponse {
        current_checksum: sha256_hex(canonical_manifest_json(&current)?.as_bytes()),
        desired_checksum: sha256_hex(canonical_manifest_json(&request.manifest)?.as_bytes()),
        summary,
        changes,
    })))
}

async fn build_manifest(
    state: &AppState,
    namespace: Option<String>,
    app: Option<String>,
) -> Result<GitOpsManifest, ApiError> {
    let scope = GitOpsScope { namespace, app };
    let mut resources = Vec::new();

    for job in state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        if scope
            .namespace
            .as_ref()
            .is_some_and(|value| value != &job.namespace)
            || scope.app.as_ref().is_some_and(|value| value != &job.app)
        {
            continue;
        }
        resources.push(resource(
            "Job",
            GitOpsMetadata {
                id: Some(job.id.clone()),
                name: job.name.clone(),
                namespace: Some(job.namespace.clone()),
                app: Some(job.app.clone()),
            },
            serde_json::json!({
                "scheduleType": job.schedule_type,
                "scheduleExpr": job.schedule_expr,
                "misfirePolicy": job.misfire_policy,
                "scheduleStartAt": job.schedule_start_at,
                "scheduleEndAt": job.schedule_end_at,
                "scheduleCalendar": job.schedule_calendar_json.and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok()),
                "processorName": job.processor_name,
                "processorType": job.processor_type,
                "scriptId": job.script_id,
                "enabled": job.enabled,
                "canaryJobId": job.canary_job_id,
                "canaryPercent": job.canary_percent,
            }),
        ));
    }

    for workflow in state
        .workflows
        .list_workflows()
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        resources.push(resource(
            "Workflow",
            GitOpsMetadata {
                id: Some(workflow.id),
                name: workflow.name,
                namespace: None,
                app: None,
            },
            serde_json::json!({ "definition": workflow.definition, "status": workflow.status }),
        ));
    }

    for script in state
        .scripts
        .list_scripts()
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        resources.push(resource(
            "Script",
            GitOpsMetadata {
                id: Some(script.id),
                name: script.name,
                namespace: None,
                app: None,
            },
            serde_json::json!({
                "language": script.language,
                "version": script.version,
                "contentSha256": script.content_sha256,
                "status": script.status,
                "releasedVersionId": script.released_version_id,
                "releasedVersionNumber": script.released_version_number,
                "timeoutSeconds": script.timeout_seconds,
                "maxMemoryBytes": script.max_memory_bytes,
                "allowNetwork": script.allow_network,
                "allowedEnvVars": script.allowed_env_vars,
                "policy": script.policy,
            }),
        ));
    }

    for plugin in state
        .plugins
        .list_plugins()
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        resources.push(resource(
            "Plugin",
            GitOpsMetadata {
                id: Some(plugin.id),
                name: plugin.name,
                namespace: None,
                app: None,
            },
            serde_json::json!({
                "kind": plugin.kind,
                "processorTypes": plugin.processor_types,
                "alertChannelTypes": plugin.alert_channel_types,
                "enabled": plugin.enabled,
            }),
        ));
    }

    for alert in state
        .alerts
        .list_rules()
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        resources.push(resource(
            "AlertRule",
            GitOpsMetadata {
                id: Some(alert.id.clone()),
                name: alert.name.clone(),
                namespace: None,
                app: None,
            },
            serde_json::to_value(AlertRuleSummary {
                id: alert.id,
                name: alert.name,
                severity: alert.severity,
                condition: serde_json::from_str(&alert.condition_json)
                    .unwrap_or(serde_json::Value::Null),
                channels: serde_json::from_str(&alert.channels_json).unwrap_or_default(),
                enabled: alert.enabled,
                dedupe_seconds: u64::try_from(alert.dedupe_seconds).unwrap_or(0),
                silenced_until: alert.silenced_until,
                created_at: alert.created_at,
                updated_at: alert.updated_at,
            })
            .map_err(|error| ApiError::bad_request(error.to_string()))?,
        ));
    }

    resources.sort_by_key(resource_key);
    Ok(GitOpsManifest {
        api_version: "tikeo.io/v1alpha1".to_owned(),
        kind: "TikeoManifest".to_owned(),
        scope,
        resources,
    })
}

fn resource(kind: &str, metadata: GitOpsMetadata, spec: serde_json::Value) -> GitOpsResource {
    GitOpsResource {
        kind: kind.to_owned(),
        metadata,
        spec,
    }
}

fn diff_resources(
    current: &[GitOpsResource],
    desired: &[GitOpsResource],
) -> Result<Vec<GitOpsDiffChange>, ApiError> {
    let current_map = resource_map(current)?;
    let desired_map = resource_map(desired)?;
    let mut keys = BTreeSet::new();
    keys.extend(current_map.keys().cloned());
    keys.extend(desired_map.keys().cloned());
    let mut changes = Vec::new();
    for key in keys {
        match (current_map.get(&key), desired_map.get(&key)) {
            (None, Some(after)) => changes.push(change("create", &key, None, Some(after))?),
            (Some(before), None) => changes.push(change("delete", &key, Some(before), None)?),
            (Some(before), Some(after))
                if canonical_resource_json(before)? == canonical_resource_json(after)? =>
            {
                changes.push(change("unchanged", &key, Some(before), Some(after))?);
            }
            (Some(before), Some(after)) => {
                changes.push(change("update", &key, Some(before), Some(after))?);
            }
            (None, None) => {}
        }
    }
    Ok(changes)
}

fn resource_map(
    resources: &[GitOpsResource],
) -> Result<BTreeMap<String, GitOpsResource>, ApiError> {
    let mut map = BTreeMap::new();
    for resource in resources {
        let key = resource_key(resource);
        if map.insert(key.clone(), resource.clone()).is_some() {
            return Err(ApiError::bad_request(format!(
                "duplicate gitops resource key: {key}"
            )));
        }
    }
    Ok(map)
}

fn change(
    action: &str,
    key: &str,
    before: Option<&GitOpsResource>,
    after: Option<&GitOpsResource>,
) -> Result<GitOpsDiffChange, ApiError> {
    let before_json = before.map(canonical_resource_json).transpose()?;
    let after_json = after.map(canonical_resource_json).transpose()?;
    Ok(GitOpsDiffChange {
        action: action.to_owned(),
        key: key.to_owned(),
        kind: before
            .or(after)
            .map_or_else(String::new, |resource| resource.kind.clone()),
        name: before
            .or(after)
            .map_or_else(String::new, |resource| resource.metadata.name.clone()),
        before: before.cloned(),
        after: after.cloned(),
        diff: unified_line_diff(
            before_json.as_deref().unwrap_or(""),
            after_json.as_deref().unwrap_or(""),
        ),
    })
}

fn summarize_changes(changes: &[GitOpsDiffChange]) -> BTreeMap<String, u64> {
    let mut summary = BTreeMap::new();
    for change in changes {
        *summary.entry(change.action.clone()).or_insert(0) += 1;
    }
    summary
}

fn resource_key(resource: &GitOpsResource) -> String {
    format!(
        "{}/{}/{}/{}",
        resource.kind,
        resource.metadata.namespace.as_deref().unwrap_or("_"),
        resource.metadata.app.as_deref().unwrap_or("_"),
        resource.metadata.name,
    )
}

fn canonical_manifest_json(manifest: &GitOpsManifest) -> Result<String, ApiError> {
    serde_json::to_string_pretty(manifest).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn canonical_resource_json(resource: &GitOpsResource) -> Result<String, ApiError> {
    serde_json::to_string_pretty(resource).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn unified_line_diff(before: &str, after: &str) -> String {
    if before == after {
        return String::new();
    }
    let mut output = String::from("--- current\n+++ desired\n");
    for line in before.lines() {
        output.push('-');
        output.push_str(line);
        output.push('\n');
    }
    for line in after.lines() {
        output.push('+');
        output.push_str(line);
        output.push('\n');
    }
    output
}

fn json_to_yaml(value: &serde_json::Value) -> String {
    fn write(value: &serde_json::Value, indent: usize, out: &mut String) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    out.push_str(&" ".repeat(indent));
                    out.push_str(key);
                    match value {
                        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                            out.push_str(":\n");
                            write(value, indent + 2, out);
                        }
                        _ => {
                            out.push_str(": ");
                            write_scalar(value, out);
                            out.push('\n');
                        }
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    out.push_str(&" ".repeat(indent));
                    out.push_str("- ");
                    match item {
                        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                            out.push('\n');
                            write(item, indent + 2, out);
                        }
                        _ => {
                            write_scalar(item, out);
                            out.push('\n');
                        }
                    }
                }
            }
            _ => write_scalar(value, out),
        }
    }
    fn write_scalar(value: &serde_json::Value, out: &mut String) {
        match value {
            serde_json::Value::Null => out.push_str("null"),
            serde_json::Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            serde_json::Value::Number(value) => out.push_str(&value.to_string()),
            serde_json::Value::String(value) => {
                out.push_str(&serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned()));
            }
            _ => out.push_str(&value.to_string()),
        }
    }
    let mut out = String::new();
    write(value, 0, &mut out);
    out
}
