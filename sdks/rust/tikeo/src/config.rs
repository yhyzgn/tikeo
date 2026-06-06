#![allow(clippy::redundant_pub_crate)]

use std::collections::HashMap;

use crate::proto::worker::v1::{
    PluginProcessorCapability, RegisterWorker, ScriptRunnerCapability, SdkProcessorCapability,
    WorkerCapabilities, WorkerClusterElection, WorkerMessage, worker_message,
};

/// Worker runtime configuration used during registration.
#[derive(Debug, Clone, PartialEq)]
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

    /// Advertise an SDK processor by structured name.
    pub fn add_sdk_processor(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !name.trim().is_empty()
            && !self
                .structured_capabilities
                .sdk_processors
                .iter()
                .any(|processor| processor.name == name)
        {
            self.structured_capabilities
                .sdk_processors
                .push(SdkProcessorCapability { name });
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
        processor_type: impl Into<String>,
        processor_name: impl Into<String>,
    ) {
        let processor_type = processor_type.into();
        let processor_name = processor_name.into();
        if processor_type.trim().is_empty() || processor_name.trim().is_empty() {
            return;
        }
        if let Some(plugin) = self
            .structured_capabilities
            .plugin_processors
            .iter_mut()
            .find(|plugin| plugin.r#type == processor_type)
        {
            push_unique(&mut plugin.processor_names, processor_name);
            return;
        }
        self.structured_capabilities
            .plugin_processors
            .push(PluginProcessorCapability {
                r#type: processor_type,
                processor_names: vec![processor_name],
            });
    }

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

fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !values.iter().any(|item| item == &value) {
        values.push(value);
    }
}
