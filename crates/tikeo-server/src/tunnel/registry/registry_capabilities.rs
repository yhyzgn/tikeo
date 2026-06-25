use std::collections::HashMap;

use tikeo_proto::worker::v1::{
    PluginProcessorCapability, ProcessorCapability, ScriptRunnerCapability, WorkerCapabilities,
};

/// Worker capabilities json.
pub(super) fn worker_capabilities_json(capabilities: Option<&WorkerCapabilities>) -> String {
    let Some(capabilities) = capabilities else {
        return "{}".to_owned();
    };
    serde_json::to_string(&serde_json::json!({
        "tags": capabilities.tags,
        "normalProcessors": capabilities.normal_processors.iter().map(|processor| serde_json::json!({
            "name": processor.name,
            "description": processor.description,
        })).collect::<Vec<_>>(),
        "scriptRunners": capabilities.script_runners.iter().map(|runner| serde_json::json!({
            "language": runner.language,
            "sandboxBackend": runner.sandbox_backend,
        })).collect::<Vec<_>>(),
        "pluginProcessors": capabilities.plugin_processors.iter().map(|processor| serde_json::json!({
            "type": processor.r#type,
            "processorNames": plugin_processor_names(processor),
            "processors": plugin_processors(processor),
        })).collect::<Vec<_>>(),
    }))
    .unwrap_or_else(|_| "{}".to_owned())
}

/// Parse persisted capabilities.
pub(super) fn parse_persisted_capabilities(value: &str) -> WorkerCapabilities {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(value) else {
        return WorkerCapabilities::default();
    };
    WorkerCapabilities {
        tags: value
            .get("tags")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect(),
        normal_processors: parse_normal_processors(&value),
        script_runners: parse_script_runners(&value),
        plugin_processors: parse_plugin_processors(&value),
    }
}

/// Parse persisted labels.
pub(super) fn parse_persisted_labels(value: &str) -> HashMap<String, String> {
    serde_json::from_str::<HashMap<String, String>>(value).unwrap_or_default()
}

fn parse_normal_processors(value: &serde_json::Value) -> Vec<ProcessorCapability> {
    let mut processors = Vec::new();
    if let Some(values) = value
        .get("normalProcessors")
        .and_then(serde_json::Value::as_array)
    {
        for processor in values {
            if let Some(name) = processor.get("name").and_then(serde_json::Value::as_str) {
                if name.trim().is_empty() {
                    continue;
                }
                let description = processor
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                processors.push(ProcessorCapability {
                    name: name.to_owned(),
                    description,
                });
            } else if let Some(name) = processor.as_str()
                && !name.trim().is_empty()
            {
                processors.push(ProcessorCapability {
                    name: name.to_owned(),
                    description: String::new(),
                });
            }
        }
    }
    processors
}

fn parse_script_runners(value: &serde_json::Value) -> Vec<ScriptRunnerCapability> {
    value
        .get("scriptRunners")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|runner| {
            Some(ScriptRunnerCapability {
                language: runner.get("language")?.as_str()?.to_owned(),
                sandbox_backend: runner
                    .get("sandboxBackend")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        })
        .collect()
}

fn parse_plugin_processors(value: &serde_json::Value) -> Vec<PluginProcessorCapability> {
    value
        .get("pluginProcessors")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|processor| {
            Some(PluginProcessorCapability {
                r#type: processor.get("type")?.as_str()?.to_owned(),
                processor_names: parse_processor_names(processor),
                processors: parse_processor_capabilities(processor.get("processors")),
            })
        })
        .collect()
}

fn parse_processor_capabilities(value: Option<&serde_json::Value>) -> Vec<ProcessorCapability> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|processor| {
            let name = processor.get("name")?.as_str()?.trim();
            (!name.is_empty()).then(|| ProcessorCapability {
                name: name.to_owned(),
                description: processor
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        })
        .collect()
}

fn parse_processor_names(processor: &serde_json::Value) -> Vec<String> {
    let mut names = processor
        .get("processorNames")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for processor in parse_processor_capabilities(processor.get("processors")) {
        if !names.iter().any(|name| name == &processor.name) {
            names.push(processor.name);
        }
    }
    names
}

fn plugin_processor_names(processor: &PluginProcessorCapability) -> Vec<&str> {
    let mut names = Vec::new();
    for name in &processor.processor_names {
        if !name.trim().is_empty() && !names.iter().any(|existing| existing == &name.as_str()) {
            names.push(name.as_str());
        }
    }
    for processor in &processor.processors {
        if !processor.name.trim().is_empty()
            && !names
                .iter()
                .any(|existing| existing == &processor.name.as_str())
        {
            names.push(processor.name.as_str());
        }
    }
    names
}

fn plugin_processors(processor: &PluginProcessorCapability) -> Vec<serde_json::Value> {
    let mut processors = processor
        .processors
        .iter()
        .filter(|processor| !processor.name.trim().is_empty())
        .map(|processor| {
            serde_json::json!({
                "name": processor.name,
                "description": processor.description,
            })
        })
        .collect::<Vec<_>>();
    for name in &processor.processor_names {
        if name.trim().is_empty()
            || processors.iter().any(|processor| {
                processor.get("name").and_then(serde_json::Value::as_str) == Some(name.as_str())
            })
        {
            continue;
        }
        processors.push(serde_json::json!({ "name": name, "description": "" }));
    }
    processors
}
