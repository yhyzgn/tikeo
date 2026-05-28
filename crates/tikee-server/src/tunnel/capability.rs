//! Structured Worker capability matching.

use tikee_proto::worker::v1::{PluginProcessorCapability, WorkerCapabilities};

/// Structured dispatch requirement for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerRequirement {
    /// Normal SDK processor selected by processor name.
    SdkProcessor {
        /// Processor name declared by the SDK worker.
        name: String,
    },
    /// Dynamic script runner selected by script language.
    ScriptRunner {
        /// Script language such as python, javascript, or wasm.
        language: String,
    },
    /// Plugin processor selected by plugin type and concrete processor name.
    PluginProcessor {
        /// Plugin processor type declared by the plugin registry.
        processor_type: String,
        /// Concrete processor name declared by the worker.
        processor_name: String,
    },
}

impl WorkerRequirement {
    /// Human-readable requirement label for logs and API compatibility surfaces.
    #[must_use]
    pub fn display_label(&self) -> String {
        match self {
            Self::SdkProcessor { name } => format!("SDK processor '{name}'"),
            Self::ScriptRunner { language } => format!("script runner '{language}'"),
            Self::PluginProcessor {
                processor_type,
                processor_name,
            } => {
                format!("plugin processor type '{processor_type}' name '{processor_name}'")
            }
        }
    }

    /// Best-effort parser for legacy capability strings used only for compatibility.
    #[must_use]
    pub fn from_legacy(required: &str) -> Option<Self> {
        let required = required.trim();
        if let Some(name) = required.strip_prefix("processor:") {
            let name = name.trim();
            return (!name.is_empty()).then(|| Self::SdkProcessor {
                name: name.to_owned(),
            });
        }
        if let Some(processor_type) = required.strip_prefix("plugin-processor:") {
            let processor_type = processor_type.trim();
            return (!processor_type.is_empty()).then(|| Self::PluginProcessor {
                processor_type: processor_type.to_owned(),
                processor_name: "*".to_owned(),
            });
        }
        if required == "script" {
            return Some(Self::ScriptRunner {
                language: "*".to_owned(),
            });
        }
        if let Some(language) = required.strip_prefix("script:") {
            let language = language.trim();
            return (!language.is_empty()).then(|| Self::ScriptRunner {
                language: language.to_owned(),
            });
        }
        None
    }

    pub(crate) fn matches_legacy_capability(&self, capability: &str) -> bool {
        let capability = capability.trim();
        if capability == "*" {
            return true;
        }
        match self {
            Self::SdkProcessor { name } => capability == format!("processor:{name}"),
            Self::ScriptRunner { language } => {
                capability == "script"
                    || capability == "script:*"
                    || language == "*" && capability.starts_with("script:")
                    || capability == format!("script:{language}")
            }
            Self::PluginProcessor {
                processor_type,
                processor_name: _,
            } => capability == format!("plugin-processor:{processor_type}"),
        }
    }
}

/// Return true when structured Worker capabilities satisfy a requirement.
#[must_use]
pub fn structured_capabilities_match(
    capabilities: &WorkerCapabilities,
    requirement: &WorkerRequirement,
) -> bool {
    match requirement {
        WorkerRequirement::SdkProcessor { name } => capabilities
            .sdk_processors
            .iter()
            .any(|processor| clean_eq(&processor.name, name)),
        WorkerRequirement::ScriptRunner { language } => {
            language == "*" && !capabilities.script_runners.is_empty()
                || capabilities
                    .script_runners
                    .iter()
                    .any(|runner| clean_eq(&runner.language, language))
        }
        WorkerRequirement::PluginProcessor {
            processor_type,
            processor_name,
        } => capabilities
            .plugin_processors
            .iter()
            .any(|plugin| plugin_processor_matches(plugin, processor_type, processor_name)),
    }
}

fn plugin_processor_matches(
    plugin: &PluginProcessorCapability,
    processor_type: &str,
    processor_name: &str,
) -> bool {
    clean_eq(&plugin.r#type, processor_type)
        && (processor_name == "*"
            || plugin
                .processor_names
                .iter()
                .any(|name| clean_eq(name, processor_name)))
}

fn clean_eq(left: &str, right: &str) -> bool {
    !left.trim().is_empty() && left.trim() == right.trim()
}
