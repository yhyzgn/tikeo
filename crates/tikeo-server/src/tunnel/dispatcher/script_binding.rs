use tikeo_core::{ScriptExecutionPolicy, ScriptLanguage, WasmProcessorSpec};
use tikeo_proto::worker::v1::{
    ScriptProcessorBinding, TaskProcessorBinding, WasmProcessorBinding, task_processor_binding,
};
use tikeo_storage::{ScriptSummary, ScriptVersionSummary};

pub(super) fn script_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let policy = script_policy(version.policy.clone());
    let release_grants = script.release_grants.as_ref();
    let language = parse_script_language(&version.language).map_or_else(
        || version.language.clone(),
        |language| language.as_str().to_owned(),
    );
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Script(
            ScriptProcessorBinding {
                script_id: script.id.clone(),
                version: script.version.clone(),
                language,
                content: version.content.as_bytes().to_vec(),
                version_id: version.id.clone(),
                version_number: u64::try_from(version.version_number).unwrap_or_default(),
                content_sha256: version.content_sha256.clone(),
                timeout_ms: policy.resources.timeout_ms,
                max_memory_bytes: policy.resources.max_memory_bytes,
                max_output_bytes: policy.resources.max_output_bytes,
                allow_network: policy.network.enabled
                    || release_grants.is_some_and(|grants| !grants.url.is_empty()),
                allowed_env_vars: policy.env_vars,
                read_only_paths: release_grants
                    .map(|grants| grants.file_read.clone())
                    .unwrap_or(policy.filesystem.read_only_paths),
                writable_paths: release_grants
                    .map(|grants| grants.file_write.clone())
                    .unwrap_or(policy.filesystem.writable_paths),
                secret_refs: release_grants
                    .map(|grants| grants.secret.clone())
                    .unwrap_or(policy.secrets.refs),
                allowed_network_hosts: release_grants
                    .map(|grants| grants.url.clone())
                    .unwrap_or(policy.network.allowed_hosts),
                sandbox_backend: policy.sandbox.backend.as_str().to_owned(),
            },
        )),
    }
}

pub(super) fn script_policy(value: serde_json::Value) -> ScriptExecutionPolicy {
    serde_json::from_value(value).unwrap_or_default()
}

pub(super) fn wasm_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let spec = script_version_to_wasm_spec(version);
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Wasm(WasmProcessorBinding {
            script_id: script.id.clone(),
            version: script.version.clone(),
            module: version.content.as_bytes().to_vec(),
            runtime: spec.runtime.as_str().to_owned(),
            entrypoint: spec.entrypoint,
            timeout_ms: spec.resources.timeout_ms,
            max_memory_bytes: spec.resources.max_memory_bytes,
            fuel: spec.resources.fuel,
            allow_network: spec.capabilities.network,
            allowed_env_vars: spec.capabilities.env_vars,
            version_id: version.id.clone(),
            version_number: u64::try_from(version.version_number).unwrap_or_default(),
            module_sha256: version.content_sha256.clone(),
            module_signature: String::new(),
        })),
    }
}

pub(super) fn script_version_to_wasm_spec(version: &ScriptVersionSummary) -> WasmProcessorSpec {
    let mut spec = WasmProcessorSpec::default();
    spec.resources.timeout_ms = version
        .timeout_seconds
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .map_or(spec.resources.timeout_ms, |seconds| {
            seconds.saturating_mul(1000)
        });
    spec.resources.max_memory_bytes = version
        .max_memory_bytes
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(spec.resources.max_memory_bytes);
    spec.capabilities.network = version.allow_network;
    spec.capabilities.env_vars = version.allowed_env_vars.clone().unwrap_or_default();
    spec
}

pub(super) fn parse_script_language(language: &str) -> Option<ScriptLanguage> {
    language.parse::<ScriptLanguage>().ok()
}
