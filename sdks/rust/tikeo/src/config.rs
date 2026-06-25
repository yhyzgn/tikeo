use std::collections::HashMap;

use crate::proto::worker::v1::{
    PluginProcessorCapability, ProcessorCapability, RegisterWorker, ScriptRunnerCapability,
    WorkerCapabilities, WorkerClusterElection, WorkerMessage, worker_message,
};

/// Worker runtime configuration used during registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    /// Tikeo Worker Tunnel endpoint, for example `http://0.0.0.0:9998`.
    pub endpoint: String,
    /// Optional client-side stable instance hint for observability/reconnect correlation.
    ///
    /// The tikeo assigns the authoritative `worker_id` during registration.
    pub client_instance_id: String,
    /// Application name.
    pub app: String,
    /// Namespace name.
    pub namespace: String,
    /// Cluster name reported by this worker.
    pub cluster: String,
    /// Region reported by this worker.
    pub region: String,
    /// Runtime capabilities.
    ///
    /// Legacy free-form capabilities are preserved only as operator metadata.
    /// Dispatch routing uses `structured_capabilities`.
    pub capabilities: Vec<String>,
    /// Structured Worker capabilities used for dispatch routing.
    pub structured_capabilities: WorkerCapabilities,
    /// Worker labels.
    pub labels: HashMap<String, String>,
}

impl WorkerConfig {
    /// Build a minimal local-development worker configuration.
    #[must_use]
    /// Local.
    pub fn local(endpoint: impl Into<String>, client_instance_id: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client_instance_id: client_instance_id.into(),
            app: "default".to_owned(),
            namespace: "default".to_owned(),
            cluster: "local".to_owned(),
            region: "local".to_owned(),
            capabilities: Vec::new(),
            structured_capabilities: WorkerCapabilities::default(),
            labels: HashMap::new(),
        }
    }

    /// Add an operator-facing structured tag.
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        push_unique(&mut self.structured_capabilities.tags, tag.into());
    }

    /// Advertise a normal application processor by structured name.
    pub fn add_normal_processor(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) {
        let processor = ProcessorCapability {
            name: name.into().trim().to_owned(),
            description: description.into().trim().to_owned(),
        };
        if !processor.name.is_empty()
            && !self
                .structured_capabilities
                .normal_processors
                .iter()
                .any(|existing| existing.name == processor.name)
        {
            self.structured_capabilities
                .normal_processors
                .push(processor);
        }
    }

    /// Advertise a dynamic script runner by language and sandbox backend.
    pub fn add_script_runner(
        &mut self,
        language: impl Into<String>,
        sandbox_backend: impl Into<String>,
    ) {
        let language = language.into();
        let sandbox_backend = sandbox_backend.into();
        if !language.trim().is_empty()
            && !self
                .structured_capabilities
                .script_runners
                .iter()
                .any(|runner| runner.language == language)
        {
            self.structured_capabilities
                .script_runners
                .push(ScriptRunnerCapability {
                    language,
                    sandbox_backend,
                });
        }
    }

    /// Advertise a plugin processor type and concrete processor name.
    pub fn add_plugin_processor(
        &mut self,
        processor_type: PluginType,
        processor_name: impl Into<String>,
        description: impl Into<String>,
    ) {
        let processor_type = processor_type.as_str().to_owned();
        let processor = ProcessorCapability {
            name: processor_name.into().trim().to_owned(),
            description: description.into().trim().to_owned(),
        };
        if processor_type.trim().is_empty() || processor.name.is_empty() {
            return;
        }
        if let Some(plugin) = self
            .structured_capabilities
            .plugin_processors
            .iter_mut()
            .find(|plugin| plugin.r#type == processor_type)
        {
            push_unique(&mut plugin.processor_names, processor.name.clone());
            push_unique_processor(&mut plugin.processors, processor);
            return;
        }
        self.structured_capabilities
            .plugin_processors
            .push(PluginProcessorCapability {
                r#type: processor_type,
                processor_names: vec![processor.name.clone()],
                processors: vec![processor],
            });
    }

    /// Register message.
    pub(crate) fn register_message(&self) -> WorkerMessage {
        WorkerMessage {
            kind: Some(worker_message::Kind::Register(RegisterWorker {
                client_instance_id: self.client_instance_id.clone(),
                app: self.app.clone(),
                namespace: self.namespace.clone(),
                cluster: self.cluster.clone(),
                region: self.region.clone(),
                capabilities: self.capabilities.clone(),
                labels: self.labels.clone(),
                structured_capabilities: Some(self.structured_capabilities.clone()),
                election: Some(WorkerClusterElection {
                    enabled: true,
                    domain: String::new(),
                    priority: 100,
                }),
            })),
        }
    }
}

/// Constrained plugin processor type values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// SQL-oriented plugin processor.
    Sql,
    /// HTTP/API plugin processor.
    Http,
    /// Notification plugin processor.
    Notification,
    /// Explicit extension point for project-specific plugin types.
    Custom,
}

impl PluginType {
    /// Stable lowercase value sent to the tikeo server.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sql => "sql",
            Self::Http => "http",
            Self::Notification => "notification",
            Self::Custom => "custom",
        }
    }
}

fn push_unique_processor(values: &mut Vec<ProcessorCapability>, value: ProcessorCapability) {
    if value.name.trim().is_empty() {
        return;
    }
    if let Some(existing) = values
        .iter_mut()
        .find(|existing| existing.name == value.name)
    {
        if existing.description.is_empty() && !value.description.is_empty() {
            existing.description = value.description;
        }
        return;
    }
    values.push(value);
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !values.iter().any(|item| item == &value) {
        values.push(value);
    }
}
