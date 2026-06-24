//! Structured Worker capability matching.

use tikeo_proto::worker::v1::{PluginProcessorCapability, WorkerCapabilities};

/// Structured dispatch requirement for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerRequirement {
    /// Normal application processor selected by processor name.
    NormalProcessor {
        /// Processor name declared by the Worker.
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
            Self::NormalProcessor { name } => format!("normal processor '{name}'"),
            Self::ScriptRunner { language } => format!("script runner '{language}'"),
            Self::PluginProcessor {
                processor_type,
                processor_name,
            } => {
                format!("plugin processor type '{processor_type}' name '{processor_name}'")
            }
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
        WorkerRequirement::NormalProcessor { name } => capabilities
            .normal_processors
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
                .any(|name| clean_eq(name, processor_name))
            || plugin
                .processors
                .iter()
                .any(|processor| clean_eq(&processor.name, processor_name)))
}

fn clean_eq(left: &str, right: &str) -> bool {
    !left.trim().is_empty() && left.trim() == right.trim()
}
