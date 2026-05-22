//! Rust Worker SDK for active outbound scheduler Worker Tunnel connections.

#![forbid(unsafe_code)]

use std::{collections::HashMap, time::Duration};

/// Generated Worker Tunnel protocol bindings bundled for standalone SDK publishing.
pub mod proto {
    /// Worker tunnel protocol bindings.
    pub mod worker {
        /// Version 1 worker tunnel protocol.
        pub mod v1 {
            #![allow(
                missing_docs,
                clippy::default_trait_access,
                clippy::derive_partial_eq_without_eq,
                clippy::doc_markdown,
                clippy::missing_const_for_fn,
                clippy::missing_errors_doc,
                clippy::too_many_lines
            )]
            tonic::include_proto!("scheduler.worker.v1");
        }
    }
}

use crate::proto::worker::v1::{
    DispatchTask, Heartbeat, Ping, RegisterWorker, ServerMessage, TaskLog, TaskResult,
    WorkerMessage, WorkerRegistered, server_message, task_processor_binding, worker_message,
    worker_tunnel_service_client::WorkerTunnelServiceClient,
};
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Status, Streaming};

/// Worker runtime configuration used during registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    /// Scheduler Worker Tunnel endpoint, for example `http://0.0.0.0:9998`.
    pub endpoint: String,
    /// Optional client-side stable instance hint for observability/reconnect correlation.
    ///
    /// The scheduler assigns the authoritative `worker_id` during registration.
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
    pub capabilities: Vec<String>,
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
            labels: HashMap::new(),
        }
    }

    fn register_message(&self) -> WorkerMessage {
        WorkerMessage {
            kind: Some(worker_message::Kind::Register(RegisterWorker {
                client_instance_id: self.client_instance_id.clone(),
                app: self.app.clone(),
                namespace: self.namespace.clone(),
                cluster: self.cluster.clone(),
                region: self.region.clone(),
                capabilities: self.capabilities.clone(),
                labels: self.labels.clone(),
            })),
        }
    }
}

/// Active Worker Tunnel session.
pub struct WorkerSession {
    worker_id: String,
    lease_seconds: u64,
    outbound: mpsc::Sender<WorkerMessage>,
    inbound: Streaming<ServerMessage>,
    heartbeat_sequence: u64,
}

impl WorkerSession {
    /// Registered worker id acknowledged by scheduler.
    #[must_use]
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Lease seconds returned by scheduler registration ack.
    #[must_use]
    pub const fn lease_seconds(&self) -> u64 {
        self.lease_seconds
    }

    /// Send one heartbeat and wait for the matching ping response.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed, the scheduler returns a gRPC error,
    /// or the response type is unexpected.
    pub async fn heartbeat(&mut self) -> Result<Ping, WorkerSdkError> {
        self.heartbeat_sequence = self.heartbeat_sequence.saturating_add(1);
        let sequence = self.heartbeat_sequence;
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::Heartbeat(Heartbeat {
                    worker_id: self.worker_id.clone(),
                    sequence,
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;

        loop {
            let message = self.next_server_message().await?;
            if let Some(server_message::Kind::Ping(ping)) = message.kind
                && ping.sequence == sequence
            {
                return Ok(ping);
            }
        }
    }

    /// Send heartbeats forever at the provided interval.
    ///
    /// # Errors
    ///
    /// Returns an error when a heartbeat fails.
    pub async fn heartbeat_loop(&mut self, interval: Duration) -> Result<(), WorkerSdkError> {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            self.heartbeat().await?;
        }
    }

    /// Emit one task log through the worker tunnel.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed.
    pub async fn emit_log(
        &self,
        instance_id: impl Into<String>,
        level: impl Into<String>,
        message: impl Into<String>,
        sequence: i64,
    ) -> Result<(), WorkerSdkError> {
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::TaskLog(TaskLog {
                    worker_id: self.worker_id.clone(),
                    instance_id: instance_id.into(),
                    level: level.into(),
                    message: message.into(),
                    sequence,
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)
    }

    /// Wait for one dispatched task, run it through the provided processor, and report the result.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel closes, the scheduler returns a gRPC error,
    /// or the result cannot be sent back.
    pub async fn process_next<P>(&mut self, processor: &P) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        loop {
            let message = self.next_server_message().await?;
            if let Some(server_message::Kind::DispatchTask(task)) = message.kind {
                return self.process_task(processor, task).await;
            }
        }
    }

    async fn process_task<P>(
        &self,
        processor: &P,
        task: DispatchTask,
    ) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        let context = task_context(&task);
        let outcome = if let Some(binding) = task.processor_binding.as_ref() {
            process_bound_task(binding, &task)
        } else {
            match processor.process(context.clone()).await {
                Ok(outcome) => outcome,
                Err(error) => TaskOutcome::Failed(error.to_string()),
            }
        };
        self.report_task_result(context.instance_id, &outcome)
            .await?;

        Ok(outcome)
    }

    async fn report_task_result(
        &self,
        instance_id: String,
        outcome: &TaskOutcome,
    ) -> Result<(), WorkerSdkError> {
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::TaskResult(TaskResult {
                    worker_id: self.worker_id.clone(),
                    instance_id,
                    success: matches!(outcome, TaskOutcome::Succeeded),
                    message: outcome.message().unwrap_or_default(),
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)
    }

    async fn next_server_message(&mut self) -> Result<ServerMessage, WorkerSdkError> {
        self.inbound
            .message()
            .await?
            .ok_or(WorkerSdkError::TunnelClosed)
    }
}

/// Worker tunnel client.
#[derive(Debug, Clone)]
pub struct WorkerClient {
    config: WorkerConfig,
}

impl WorkerClient {
    /// Create a client from worker configuration.
    #[must_use]
    pub const fn new(config: WorkerConfig) -> Self {
        Self { config }
    }

    /// Open the tunnel, register the worker, and return an active session.
    ///
    /// # Errors
    ///
    /// Returns an error when connecting, sending registration, or reading the ack fails.
    pub async fn connect(self) -> Result<WorkerSession, WorkerSdkError> {
        let mut client = WorkerTunnelServiceClient::connect(self.config.endpoint.clone()).await?;
        let (tx, rx) = mpsc::channel(16);
        tx.send(self.config.register_message())
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;

        let response = client.open_tunnel(ReceiverStream::new(rx)).await?;
        let mut inbound = response.into_inner();
        let registered = read_registration(&mut inbound).await?;

        Ok(WorkerSession {
            worker_id: registered.worker_id,
            lease_seconds: registered.lease_seconds,
            outbound: tx,
            inbound,
            heartbeat_sequence: 0,
        })
    }
}

async fn read_registration(
    inbound: &mut Streaming<ServerMessage>,
) -> Result<WorkerRegistered, WorkerSdkError> {
    let message = inbound
        .message()
        .await?
        .ok_or(WorkerSdkError::TunnelClosed)?;

    match message.kind {
        Some(server_message::Kind::Registered(registered)) => Ok(registered),
        Some(server_message::Kind::Ping(_) | server_message::Kind::DispatchTask(_)) | None => {
            Err(WorkerSdkError::UnexpectedMessage)
        }
    }
}

fn task_context(task: &DispatchTask) -> TaskContext {
    let processor_name = if task.processor_name.is_empty() {
        task.job_id.clone()
    } else {
        task.processor_name.clone()
    };
    TaskContext {
        job_id: task.job_id.clone(),
        processor_name,
        instance_id: task.instance_id.clone(),
        payload: task.payload.clone(),
    }
}

fn process_bound_task(
    binding: &crate::proto::worker::v1::TaskProcessorBinding,
    task: &DispatchTask,
) -> TaskOutcome {
    match binding.kind.as_ref() {
        Some(task_processor_binding::Kind::Wasm(wasm)) => process_wasm_binding(wasm, task),
        None => TaskOutcome::Failed("empty dynamic processor binding".to_owned()),
    }
}

#[cfg(feature = "wasm")]
fn process_wasm_binding(
    binding: &crate::proto::worker::v1::WasmProcessorBinding,
    _task: &DispatchTask,
) -> TaskOutcome {
    match wasm_runtime::execute(binding) {
        Ok(()) => TaskOutcome::Succeeded,
        Err(error) => TaskOutcome::Failed(error),
    }
}

#[cfg(not(feature = "wasm"))]
fn process_wasm_binding(
    binding: &crate::proto::worker::v1::WasmProcessorBinding,
    _task: &DispatchTask,
) -> TaskOutcome {
    TaskOutcome::Failed(format!(
        "wasm processor binding for script {} requires enabling scheduler-worker-sdk feature 'wasm'",
        binding.script_id
    ))
}

#[cfg(feature = "wasm")]
mod wasm_runtime {
    use std::{thread, time::Duration};

    use sha2::{Digest, Sha256};
    use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimitsBuilder};

    use crate::proto::worker::v1::WasmProcessorBinding;

    pub fn execute(binding: &WasmProcessorBinding) -> Result<(), String> {
        validate(binding)?;
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|error| format!("wasm engine error: {error}"))?;
        let module = Module::from_binary(&engine, &binding.module)
            .map_err(|error| format!("wasm module error: {error}"))?;
        let memory_size = usize::try_from(binding.max_memory_bytes).unwrap_or(usize::MAX);
        let limits = StoreLimitsBuilder::new().memory_size(memory_size).build();
        let mut store = Store::new(&engine, limits);
        store
            .set_fuel(binding.fuel)
            .map_err(|error| format!("wasm fuel error: {error}"))?;
        store.limiter(|limits| limits);
        let timeout = Duration::from_millis(binding.timeout_ms);
        let deadline_engine = engine.clone();
        let _interrupter = thread::spawn(move || {
            thread::sleep(timeout);
            deadline_engine.increment_epoch();
        });
        store.set_epoch_deadline(1);
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|error| format!("wasm instantiate error: {error}"))?;
        let entrypoint = instance
            .get_typed_func::<(), ()>(&mut store, &binding.entrypoint)
            .map_err(|error| format!("wasm entrypoint error: {error}"))?;
        entrypoint
            .call(&mut store, ())
            .map_err(|error| format!("wasm trap: {error}"))
    }

    fn validate(binding: &WasmProcessorBinding) -> Result<(), String> {
        if !binding.module_sha256.trim().is_empty() {
            let actual = format!("{:x}", Sha256::digest(&binding.module));
            if !actual.eq_ignore_ascii_case(binding.module_sha256.trim()) {
                return Err("wasm module sha256 digest mismatch".to_owned());
            }
        }
        if binding.runtime != "wasmtime" {
            return Err(format!("unsupported wasm runtime: {}", binding.runtime));
        }
        if binding.entrypoint.trim().is_empty() {
            return Err("wasm entrypoint must not be empty".to_owned());
        }
        if binding.timeout_ms == 0 {
            return Err("wasm timeout must be greater than zero".to_owned());
        }
        if binding.max_memory_bytes == 0 {
            return Err("wasm memory limit must be greater than zero".to_owned());
        }
        if binding.fuel == 0 {
            return Err("wasm fuel budget must be greater than zero".to_owned());
        }
        if binding.allow_network {
            return Err(
                "wasm network capability is not supported by the Rust SDK adapter yet".to_owned(),
            );
        }
        Ok(())
    }
}


/// Supported non-WASM dynamic script runner kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptRunnerKind {
    /// POSIX shell runner.
    Shell,
    /// Python runner.
    Python,
    /// Node.js runner.
    Node,
    /// PowerShell runner.
    PowerShell,
    /// Rhai expression/script runner.
    Rhai,
}

impl ScriptRunnerKind {
    /// Parse a wire language value into a runner kind.
    #[must_use]
    pub fn from_language(language: &str) -> Option<Self> {
        match language.trim().to_ascii_lowercase().as_str() {
            "shell" | "sh" | "bash" => Some(Self::Shell),
            "python" | "py" => Some(Self::Python),
            "node" | "nodejs" | "javascript" | "js" | "typescript" | "ts" => Some(Self::Node),
            "powershell" | "pwsh" => Some(Self::PowerShell),
            "rhai" => Some(Self::Rhai),
            _ => None,
        }
    }

    /// Stable runner name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Python => "python",
            Self::Node => "node",
            Self::PowerShell => "powershell",
            Self::Rhai => "rhai",
        }
    }
}

/// Default-deny policy snapshot for non-WASM dynamic script runners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRunnerPolicy {
    /// Maximum wall-clock runtime in milliseconds.
    pub timeout_ms: u64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum captured output bytes.
    pub max_output_bytes: u64,
    /// Whether network egress is allowed. Current SDK abstraction rejects it.
    pub allow_network: bool,
    /// Allowed environment variable names.
    pub env_vars: Vec<String>,
    /// Read-only filesystem paths granted to the runner. Current SDK abstraction rejects them.
    pub read_only_paths: Vec<String>,
    /// Writable filesystem paths granted to the runner. Current SDK abstraction rejects them.
    pub writable_paths: Vec<String>,
    /// Secret references granted to the runner. Current SDK abstraction rejects them.
    pub secret_refs: Vec<String>,
}

impl Default for ScriptRunnerPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_memory_bytes: 64 * 1024 * 1024,
            max_output_bytes: 1024 * 1024,
            allow_network: false,
            env_vars: Vec::new(),
            read_only_paths: Vec::new(),
            writable_paths: Vec::new(),
            secret_refs: Vec::new(),
        }
    }
}

impl ScriptRunnerPolicy {
    /// Validate the SDK-side policy boundary before any future local runner executes code.
    ///
    /// # Errors
    ///
    /// Returns an error for zero limits or dangerous capabilities that require future
    /// platform policy grants.
    pub fn validate_default_deny(&self) -> Result<(), WorkerSdkError> {
        if self.timeout_ms == 0 {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script timeout must be greater than zero".to_owned(),
            ));
        }
        if self.max_memory_bytes == 0 {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script memory limit must be greater than zero".to_owned(),
            ));
        }
        if self.max_output_bytes == 0 {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script output limit must be greater than zero".to_owned(),
            ));
        }
        if self.allow_network {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script network access requires a future URL policy grant".to_owned(),
            ));
        }
        if !self.read_only_paths.is_empty() || !self.writable_paths.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script filesystem access requires a future filesystem policy grant".to_owned(),
            ));
        }
        if !self.secret_refs.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script secret access requires a future secret policy grant".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Future non-WASM dynamic script runner contract.
#[async_trait]
pub trait ScriptRunner: Send + Sync + 'static {
    /// Runner language/kind.
    fn kind(&self) -> ScriptRunnerKind;

    /// Execute a released immutable script snapshot.
    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError>;
}

/// Immutable script snapshot passed to a future non-WASM runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRunnerTask {
    /// Script id.
    pub script_id: String,
    /// Immutable script version id.
    pub version_id: String,
    /// Immutable script version number.
    pub version_number: u64,
    /// Script language.
    pub language: String,
    /// Script source content from the released version snapshot.
    pub content: String,
    /// Content SHA-256 digest.
    pub content_sha256: String,
    /// Default-deny execution policy snapshot.
    pub policy: ScriptRunnerPolicy,
}

/// Placeholder runner used until language-specific sandbox implementations are enabled.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedScriptRunner;

#[async_trait]
impl ScriptRunner for UnsupportedScriptRunner {
    fn kind(&self) -> ScriptRunnerKind {
        ScriptRunnerKind::Shell
    }

    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError> {
        task.policy.validate_default_deny()?;
        Err(WorkerSdkError::UnsupportedScriptRunner(format!(
            "{} script runner is not enabled; use a dedicated sandbox runner before executing dynamic scripts",
            task.language
        )))
    }
}

/// User-provided async processor interface for future task dispatch support.
#[async_trait]
pub trait TaskProcessor: Send + Sync + 'static {
    /// Execute one task payload.
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError>;
}

/// Minimal task context placeholder reserved for Worker dispatch protocol evolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContext {
    /// Job identifier.
    pub job_id: String,
    /// Explicit processor key/name for SDK routing.
    pub processor_name: String,
    /// Instance identifier.
    pub instance_id: String,
    /// Raw task payload.
    pub payload: Vec<u8>,
}

/// Minimal processor outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    /// Task completed successfully.
    Succeeded,
    /// Task failed with a message safe to send back to scheduler.
    Failed(String),
}

impl TaskOutcome {
    fn message(&self) -> Option<String> {
        match self {
            Self::Succeeded => None,
            Self::Failed(message) => Some(message.clone()),
        }
    }
}

/// Worker SDK errors.
#[derive(Debug, Error)]
pub enum WorkerSdkError {
    /// gRPC transport error.
    #[error("worker tunnel transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
    /// gRPC status error.
    #[error("worker tunnel status error: {0}")]
    Status(#[from] Status),
    /// The tunnel closed before the expected response arrived.
    #[error("worker tunnel closed")]
    TunnelClosed,
    /// Scheduler returned an unexpected server message.
    #[error("unexpected worker tunnel server message")]
    UnexpectedMessage,
    /// A dynamic script runner was requested before a safe sandbox implementation exists.
    #[error("unsupported script runner: {0}")]
    UnsupportedScriptRunner(String),
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, pin::Pin};

    use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};
    use tokio_stream::{Stream, StreamExt, wrappers::TcpListenerStream};
    use tonic::{Request, Response, Status, transport::Server};

    use sha2::{Digest, Sha256};

    use crate::proto::worker::v1::{
        DispatchTask, Ping, ServerMessage, TaskProcessorBinding, WasmProcessorBinding,
        WorkerMessage, WorkerRegistered, server_message, task_processor_binding, worker_message,
        worker_tunnel_service_server, worker_tunnel_service_server::WorkerTunnelServiceServer,
    };

    use super::{
        ScriptRunner, ScriptRunnerKind, ScriptRunnerPolicy, ScriptRunnerTask, TaskContext,
        TaskOutcome, TaskProcessor, UnsupportedScriptRunner, WorkerClient, WorkerConfig,
        WorkerSdkError,
    };


    #[tokio::test]
    async fn unsupported_script_runner_validates_default_deny_policy_before_execution() {
        assert_eq!(ScriptRunnerKind::from_language("python"), Some(ScriptRunnerKind::Python));
        assert_eq!(ScriptRunnerKind::Node.as_str(), "node");

        let runner = UnsupportedScriptRunner;
        let task = ScriptRunnerTask {
            script_id: "script_py".to_owned(),
            version_id: "sv_1".to_owned(),
            version_number: 1,
            language: "python".to_owned(),
            content: "print(1)".to_owned(),
            content_sha256: "digest".to_owned(),
            policy: ScriptRunnerPolicy::default(),
        };
        let error = match runner.run(task).await {
            Ok(outcome) => panic!("runner should not execute yet: {outcome:?}"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("not enabled"));

        let dangerous = ScriptRunnerPolicy {
            allow_network: true,
            ..ScriptRunnerPolicy::default()
        };
        let error = match dangerous.validate_default_deny() {
            Ok(()) => panic!("dangerous policy should be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("network access"));
    }

    #[tokio::test]
    async fn worker_client_registers_and_sends_heartbeat() {
        let (addr, server, _events) = start_mock_tunnel_server(None).await;
        let mut config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-1");
        config.app = "billing".to_owned();
        config.namespace = "default".to_owned();

        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));
        let ping = session
            .heartbeat()
            .await
            .unwrap_or_else(|error| panic!("heartbeat should ping: {error}"));

        assert_eq!(session.worker_id(), "mock-worker-sdk-1");
        assert_eq!(session.lease_seconds(), 30);
        assert_eq!(ping.sequence, 1);

        server.abort();
    }

    #[tokio::test]
    async fn worker_session_processes_dispatched_task_and_reports_result() {
        let (addr, server, mut events) = start_mock_tunnel_server(Some(DispatchTask {
            instance_id: "instance-1".to_owned(),
            job_id: "job-1".to_owned(),
            payload: b"hello".to_vec(),
            processor_name: "demo.echo".to_owned(),
            processor_binding: None,
        }))
        .await;

        let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-2");
        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));
        session
            .emit_log("instance-1", "info", "starting", 1)
            .await
            .unwrap_or_else(|error| panic!("log should emit: {error}"));

        let outcome = session
            .process_next(&EchoProcessor)
            .await
            .unwrap_or_else(|error| panic!("task should process: {error}"));
        assert_eq!(outcome, TaskOutcome::Succeeded);

        let mut saw_log = false;
        let mut saw_result = false;
        while let Some(message) = events.recv().await {
            match message.kind {
                Some(worker_message::Kind::TaskLog(log)) => {
                    saw_log = log.instance_id == "instance-1" && log.message == "starting";
                }
                Some(worker_message::Kind::TaskResult(result)) => {
                    saw_result = result.instance_id == "instance-1" && result.success;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_log, "mock tunnel should receive emitted task log");
        assert!(saw_result, "mock tunnel should receive task result");

        server.abort();
    }

    async fn start_mock_tunnel_server(
        dispatch: Option<DispatchTask>,
    ) -> (SocketAddr, JoinHandle<()>, mpsc::Receiver<WorkerMessage>) {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap_or_else(|error| panic!("listener should bind: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener should expose addr: {error}"));
        let incoming = TcpListenerStream::new(listener);
        let (events_tx, events_rx) = mpsc::channel(16);
        let service = WorkerTunnelServiceServer::new(MockTunnel {
            dispatch,
            events: events_tx,
        });
        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(service)
                .serve_with_incoming(incoming)
                .await
                .unwrap_or_else(|error| panic!("test server should run: {error}"));
        });
        (addr, server, events_rx)
    }

    #[cfg(not(feature = "wasm"))]
    #[tokio::test]
    async fn worker_session_reports_wasm_binding_requires_feature_when_disabled() {
        let dispatch = wasm_dispatch_task(
            "instance-wasm-disabled",
            wat_bytes(r#"(module (func (export "_start")))"#),
            false,
        );
        let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
        let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-wasm-disabled");
        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));

        let outcome = session
            .process_next(&EchoProcessor)
            .await
            .unwrap_or_else(|error| panic!("wasm disabled result should report: {error}"));

        assert!(
            matches!(outcome, TaskOutcome::Failed(message) if message.contains("feature 'wasm'"))
        );
        let result = next_task_result(&mut events).await;
        assert!(!result.success);
        assert!(result.message.contains("feature 'wasm'"));
        server.abort();
    }

    #[cfg(feature = "wasm")]
    #[tokio::test]
    async fn worker_session_executes_wasm_binding_when_feature_enabled() {
        let dispatch = wasm_dispatch_task(
            "instance-wasm-enabled",
            wat_bytes(r#"(module (func (export "_start")))"#),
            false,
        );
        let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
        let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-wasm-enabled");
        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));

        let outcome = session
            .process_next(&EchoProcessor)
            .await
            .unwrap_or_else(|error| panic!("wasm result should report: {error}"));

        assert_eq!(outcome, TaskOutcome::Succeeded);
        let result = next_task_result(&mut events).await;
        assert!(result.success);
        server.abort();
    }

    #[cfg(feature = "wasm")]
    #[tokio::test]
    async fn worker_session_rejects_wasm_digest_mismatch() {
        let mut dispatch = wasm_dispatch_task(
            "instance-wasm-digest",
            wat_bytes(r#"(module (func (export "_start")))"#),
            false,
        );
        if let Some(binding) = dispatch.processor_binding.as_mut()
            && let Some(task_processor_binding::Kind::Wasm(wasm)) = binding.kind.as_mut()
        {
            wasm.module_sha256 = "deadbeef".to_owned();
        }
        let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
        let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-wasm-digest");
        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));

        let outcome = session
            .process_next(&EchoProcessor)
            .await
            .unwrap_or_else(|error| panic!("wasm rejection should report: {error}"));

        assert!(
            matches!(outcome, TaskOutcome::Failed(message) if message.contains("digest mismatch"))
        );
        let result = next_task_result(&mut events).await;
        assert!(!result.success);
        assert!(result.message.contains("digest mismatch"));
        server.abort();
    }

    #[cfg(feature = "wasm")]
    #[tokio::test]
    async fn worker_session_rejects_wasm_network_capability() {
        let dispatch = wasm_dispatch_task(
            "instance-wasm-network",
            wat_bytes(r#"(module (func (export "_start")))"#),
            true,
        );
        let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
        let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-wasm-network");
        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));

        let outcome = session
            .process_next(&EchoProcessor)
            .await
            .unwrap_or_else(|error| panic!("wasm rejection should report: {error}"));

        assert!(
            matches!(outcome, TaskOutcome::Failed(message) if message.contains("network capability"))
        );
        let result = next_task_result(&mut events).await;
        assert!(!result.success);
        assert!(result.message.contains("network capability"));
        server.abort();
    }

    async fn next_task_result(
        events: &mut mpsc::Receiver<WorkerMessage>,
    ) -> crate::proto::worker::v1::TaskResult {
        while let Some(message) = events.recv().await {
            if let Some(worker_message::Kind::TaskResult(result)) = message.kind {
                return result;
            }
        }
        panic!("task result should arrive");
    }

    struct MockTunnel {
        dispatch: Option<DispatchTask>,
        events: mpsc::Sender<WorkerMessage>,
    }

    type ResponseStream = Pin<Box<dyn Stream<Item = Result<ServerMessage, Status>> + Send>>;

    #[tonic::async_trait]
    impl worker_tunnel_service_server::WorkerTunnelService for MockTunnel {
        type OpenTunnelStream = ResponseStream;
        type SubscribeTaskLogsStream =
            Pin<Box<dyn Stream<Item = Result<crate::proto::worker::v1::TaskLog, Status>> + Send>>;

        async fn open_tunnel(
            &self,
            request: Request<tonic::Streaming<WorkerMessage>>,
        ) -> Result<Response<Self::OpenTunnelStream>, Status> {
            let mut inbound = request.into_inner();
            let (outbound_tx, outbound_rx) = mpsc::channel(16);
            let events = self.events.clone();
            let dispatch = self.dispatch.clone();
            tokio::spawn(async move {
                while let Some(message) = inbound.next().await {
                    let Ok(message) = message else { break };
                    let _ = events.send(message.clone()).await;
                    match message.kind {
                        Some(worker_message::Kind::Register(register)) => {
                            let _ = outbound_tx
                                .send(Ok(ServerMessage {
                                    kind: Some(server_message::Kind::Registered(
                                        WorkerRegistered {
                                            worker_id: format!(
                                                "mock-{}",
                                                register.client_instance_id
                                            ),
                                            lease_seconds: 30,
                                        },
                                    )),
                                }))
                                .await;
                            if let Some(task) = dispatch.clone() {
                                let _ = outbound_tx
                                    .send(Ok(ServerMessage {
                                        kind: Some(server_message::Kind::DispatchTask(task)),
                                    }))
                                    .await;
                            }
                        }
                        Some(worker_message::Kind::Heartbeat(heartbeat)) => {
                            let _ = outbound_tx
                                .send(Ok(ServerMessage {
                                    kind: Some(server_message::Kind::Ping(Ping {
                                        sequence: heartbeat.sequence,
                                    })),
                                }))
                                .await;
                        }
                        Some(
                            worker_message::Kind::TaskResult(_) | worker_message::Kind::TaskLog(_),
                        )
                        | None => {}
                    }
                }
            });

            Ok(Response::new(Box::pin(
                tokio_stream::wrappers::ReceiverStream::new(outbound_rx),
            )))
        }

        async fn subscribe_task_logs(
            &self,
            _request: Request<crate::proto::worker::v1::SubscribeTaskLogsRequest>,
        ) -> Result<Response<Self::SubscribeTaskLogsStream>, Status> {
            Ok(Response::new(Box::pin(tokio_stream::empty())))
        }
    }

    struct EchoProcessor;

    #[async_trait::async_trait]
    impl TaskProcessor for EchoProcessor {
        async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
            assert_eq!(task.payload, b"hello");
            Ok(TaskOutcome::Succeeded)
        }
    }

    fn wasm_dispatch_task(instance_id: &str, module: Vec<u8>, allow_network: bool) -> DispatchTask {
        let module_sha256 = format!("{:x}", Sha256::digest(&module));
        DispatchTask {
            instance_id: instance_id.to_owned(),
            job_id: "job-wasm".to_owned(),
            payload: Vec::new(),
            processor_name: "script:script_wasm".to_owned(),
            processor_binding: Some(Box::new(TaskProcessorBinding {
                kind: Some(task_processor_binding::Kind::Wasm(WasmProcessorBinding {
                    script_id: "script_wasm".to_owned(),
                    version: "1.0.0".to_owned(),
                    module,
                    runtime: "wasmtime".to_owned(),
                    entrypoint: "_start".to_owned(),
                    timeout_ms: 1_000,
                    max_memory_bytes: 1024 * 1024,
                    fuel: 1_000_000,
                    allow_network,
                    allowed_env_vars: Vec::new(),
                    version_id: "sv_1".to_owned(),
                    version_number: 1,
                    module_sha256,
                    module_signature: String::new(),
                })),
            })),
        }
    }

    fn wat_bytes(source: &str) -> Vec<u8> {
        wat::parse_str(source).unwrap_or_else(|error| panic!("wat fixture should compile: {error}"))
    }
}
