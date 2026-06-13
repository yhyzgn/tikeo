use std::{
    backtrace::Backtrace,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Streaming;

use crate::{
    config::WorkerConfig,
    error::WorkerSdkError,
    logging::{SdkLogLevel, sdk_log},
    proto::worker::v1::{
        DispatchTask, Heartbeat, Ping, ScriptProcessorBinding, ServerMessage, TaskLog, TaskResult,
        UnregisterWorker, WorkerMessage, WorkerRegistered, server_message, task_processor_binding,
        worker_message, worker_tunnel_service_client::WorkerTunnelServiceClient,
    },
    script::{ScriptRunnerKind, ScriptRunnerPolicy, ScriptRunnerRegistry, ScriptRunnerTask},
    task::{TaskLogger, TaskOutcome, TaskProcessor, task_context},
    wasm::process_wasm_binding,
};

/// Active Worker Tunnel session.
pub struct WorkerSession {
    worker_id: String,
    lease_seconds: u64,
    generation: u64,
    fencing_token: String,
    outbound: mpsc::Sender<WorkerMessage>,
    inbound: Streaming<ServerMessage>,
    heartbeat_sequence: u64,
    log_sequence: Arc<AtomicI64>,
}

impl WorkerSession {
    /// Registered worker id acknowledged by tikeo.
    #[must_use]
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Lease seconds returned by tikeo registration ack.
    #[must_use]
    pub const fn lease_seconds(&self) -> u64 {
        self.lease_seconds
    }

    /// Worker session generation acknowledged by tikeo.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Send one heartbeat and wait for the matching ping response.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed, the tikeo returns a gRPC error,
    /// or the response type is unexpected.
    pub async fn heartbeat(&mut self) -> Result<Ping, WorkerSdkError> {
        let sequence = self.send_heartbeat_message().await?;
        sdk_log(
            SdkLogLevel::Debug,
            format!(
                "sent heartbeat worker_id={} sequence={sequence}",
                self.worker_id
            ),
        );

        loop {
            let message = self.next_server_message().await?;
            if let Some(server_message::Kind::Ping(ping)) = message.kind
                && ping.sequence == sequence
            {
                return Ok(ping);
            }
        }
    }

    async fn send_heartbeat_message(&mut self) -> Result<u64, WorkerSdkError> {
        self.heartbeat_sequence = self.heartbeat_sequence.saturating_add(1);
        let sequence = self.heartbeat_sequence;
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::Heartbeat(Heartbeat {
                    worker_id: self.worker_id.clone(),
                    sequence,
                    generation: self.generation,
                    fencing_token: self.fencing_token.clone(),
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;
        Ok(sequence)
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

    /// Gracefully unregister this worker session before closing the tunnel.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is already closed before the unregister message is queued.
    pub async fn close(self) -> Result<(), WorkerSdkError> {
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::Unregister(UnregisterWorker {
                    worker_id: self.worker_id,
                    generation: self.generation,
                    fencing_token: self.fencing_token,
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)
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
                    assignment_token: String::new(),
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)
    }

    /// Emit one task log with the assignment token required by the server.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed.
    pub async fn emit_task_log(
        &self,
        instance_id: impl Into<String>,
        assignment_token: impl Into<String>,
        level: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<i64, WorkerSdkError> {
        let level = level.into();
        let message = message.into();
        let sequence = self.log_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        print_task_log_locally(&level, &message);
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::TaskLog(TaskLog {
                    worker_id: self.worker_id.clone(),
                    instance_id: instance_id.into(),
                    level,
                    message,
                    sequence,
                    assignment_token: assignment_token.into(),
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;
        Ok(sequence)
    }

    /// Wait for one dispatched task, run it through the provided processor, and report the result.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel closes, the tikeo returns a gRPC error,
    /// or the result cannot be sent back.
    pub async fn process_next<P>(&mut self, processor: &P) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        self.process_next_with_script_runners(processor, &ScriptRunnerRegistry::default())
            .await
    }

    /// Wait for one dispatched task, run dynamic script bindings through explicitly registered
    /// script runners, and report the result.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel closes, the tikeo returns a gRPC error,
    /// or the result cannot be sent back.
    pub async fn process_next_with_script_runners<P>(
        &mut self,
        processor: &P,
        script_runners: &ScriptRunnerRegistry,
    ) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        let heartbeat_interval = heartbeat_interval(self.lease_seconds);
        let mut ticker = tokio::time::interval(heartbeat_interval);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.send_heartbeat_message().await?;
                }
                message = self.next_server_message() => {
                    if let Some(server_message::Kind::DispatchTask(task)) = message?.kind {
                        self.send_heartbeat_message().await?;
                        sdk_log(SdkLogLevel::Info, "dispatch received from worker tunnel");
                        return self.process_task(processor, script_runners, task).await;
                    }
                }
            }
        }
    }

    async fn process_task<P>(
        &self,
        processor: &P,
        script_runners: &ScriptRunnerRegistry,
        task: DispatchTask,
    ) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        let instance_id = task.instance_id.clone();
        let assignment_token = task.assignment_token.clone();
        let processor_name = task.processor_name.clone();
        self.emit_task_log_safely(
            &instance_id,
            &assignment_token,
            "info",
            format!("received task {instance_id} processor={processor_name}"),
        )
        .await;
        let task_logs = TaskLogBuffer::default();
        let emit_log = task_logs.logger();
        let context = task_context(&task, Arc::clone(&emit_log));
        let result_instance_id = context.instance_id.clone();
        let outcome = if let Some(binding) = task.processor_binding.as_ref() {
            process_bound_task(binding, &task, script_runners, emit_log).await
        } else {
            match processor.process(context).await {
                Ok(outcome) => outcome,
                Err(error) => {
                    (emit_log)(
                        "error",
                        format!("processor failed: {error}\n{}", Backtrace::force_capture()),
                    );
                    TaskOutcome::Failed(error.to_string())
                }
            }
        };
        for entry in task_logs.drain() {
            self.emit_task_log_safely(&instance_id, &assignment_token, &entry.level, entry.message)
                .await;
        }
        let level = if outcome.is_success() {
            "info"
        } else {
            "error"
        };
        let result_message = task_result_message(&outcome);
        self.emit_task_log_safely(
            &instance_id,
            &assignment_token,
            level,
            format!(
                "completed task {instance_id} success={} message={result_message}",
                outcome.is_success()
            ),
        )
        .await;
        self.report_task_result(result_instance_id, task.assignment_token, &outcome)
            .await?;

        Ok(outcome)
    }

    async fn emit_task_log_safely(
        &self,
        instance_id: &str,
        assignment_token: &str,
        level: &str,
        message: String,
    ) {
        if let Err(error) = self
            .emit_task_log(instance_id, assignment_token, level, message)
            .await
        {
            sdk_log(
                SdkLogLevel::Warning,
                format!("failed to emit task log instance_id={instance_id}: {error}"),
            );
        }
    }

    async fn report_task_result(
        &self,
        instance_id: String,
        assignment_token: String,
        outcome: &TaskOutcome,
    ) -> Result<(), WorkerSdkError> {
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::TaskResult(TaskResult {
                    worker_id: self.worker_id.clone(),
                    instance_id,
                    success: outcome.is_success(),
                    message: task_result_message(outcome),
                    assignment_token,
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

#[derive(Debug, Clone)]
struct BufferedTaskLog {
    level: String,
    message: String,
}

#[derive(Default, Clone)]
struct TaskLogBuffer {
    entries: Arc<Mutex<Vec<BufferedTaskLog>>>,
}

impl TaskLogBuffer {
    fn logger(&self) -> TaskLogger {
        let entries = Arc::clone(&self.entries);
        Arc::new(move |level: &str, message: String| {
            if let Ok(mut guard) = entries.lock() {
                guard.push(BufferedTaskLog {
                    level: level.to_owned(),
                    message,
                });
            }
        })
    }

    fn drain(&self) -> Vec<BufferedTaskLog> {
        self.entries
            .lock()
            .map(|mut guard| guard.drain(..).collect())
            .unwrap_or_default()
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
        sdk_log(
            SdkLogLevel::Info,
            format!(
                "connecting worker tunnel endpoint={} client_instance_id={}",
                self.config.endpoint, self.config.client_instance_id
            ),
        );
        let mut client = WorkerTunnelServiceClient::connect(self.config.endpoint.clone()).await?;
        let (tx, rx) = mpsc::channel(16);
        tx.send(self.config.register_message())
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;

        let response = client.open_tunnel(ReceiverStream::new(rx)).await?;
        let mut inbound = response.into_inner();
        let registered = read_registration(&mut inbound).await?;
        sdk_log(
            SdkLogLevel::Info,
            format!(
                "registered worker_id={} lease_seconds={} generation={}",
                registered.worker_id, registered.lease_seconds, registered.generation
            ),
        );

        Ok(WorkerSession {
            worker_id: registered.worker_id,
            lease_seconds: registered.lease_seconds,
            generation: registered.generation,
            fencing_token: registered.fencing_token,
            outbound: tx,
            inbound,
            heartbeat_sequence: 0,
            log_sequence: Arc::new(AtomicI64::new(0)),
        })
    }
}

fn heartbeat_interval(lease_seconds: u64) -> Duration {
    Duration::from_secs((lease_seconds / 3).clamp(1, 10))
}

fn print_task_log_locally(level: &str, message: &str) {
    let line = format!("[tikeo-worker] {message}");
    if level.eq_ignore_ascii_case("error") {
        eprintln!("{line}");
    } else {
        println!("{line}");
    }
}

fn task_result_message(outcome: &TaskOutcome) -> String {
    match (outcome.failure_class(), outcome.message()) {
        (Some(class), Some(message)) => serde_json::json!({
            "failure_class": class,
            "message": message,
        })
        .to_string(),
        (_, Some(message)) => message,
        _ => String::new(),
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

async fn process_bound_task(
    binding: &crate::proto::worker::v1::TaskProcessorBinding,
    task: &DispatchTask,
    script_runners: &ScriptRunnerRegistry,
    log: TaskLogger,
) -> TaskOutcome {
    match binding.kind.as_ref() {
        Some(task_processor_binding::Kind::Wasm(wasm)) => process_wasm_binding(wasm, task),
        Some(task_processor_binding::Kind::Script(script)) => {
            process_script_binding(script, script_runners, log).await
        }
        None => TaskOutcome::Failed("empty dynamic processor binding".to_owned()),
    }
}

async fn process_script_binding(
    binding: &ScriptProcessorBinding,
    script_runners: &ScriptRunnerRegistry,
    log: TaskLogger,
) -> TaskOutcome {
    let Some(kind) = ScriptRunnerKind::from_language(&binding.language) else {
        return TaskOutcome::Failed(format!("unsupported script language: {}", binding.language));
    };
    let Some(runner) = script_runners.get(kind) else {
        sdk_log(
            SdkLogLevel::Warning,
            format!("missing script runner language={}", binding.language),
        );
        return TaskOutcome::Failed(format!(
            "{} script runner is not registered on this worker",
            kind.as_str()
        ));
    };
    let task = ScriptRunnerTask {
        script_id: binding.script_id.clone(),
        version_id: binding.version_id.clone(),
        version_number: binding.version_number,
        language: binding.language.clone(),
        content: String::from_utf8_lossy(&binding.content).into_owned(),
        content_sha256: binding.content_sha256.clone(),
        policy: ScriptRunnerPolicy {
            timeout_ms: binding.timeout_ms,
            max_memory_bytes: binding.max_memory_bytes,
            max_output_bytes: binding.max_output_bytes,
            allow_network: binding.allow_network,
            allowed_network_hosts: binding.allowed_network_hosts.clone(),
            env_vars: binding.allowed_env_vars.clone(),
            read_only_paths: binding.read_only_paths.clone(),
            writable_paths: binding.writable_paths.clone(),
            secret_refs: binding.secret_refs.clone(),
        },
        log: None,
    };
    match runner.run(task.with_log_sink(log)).await {
        Ok(outcome) => outcome,
        Err(error) => TaskOutcome::Failed(error.to_string()),
    }
}
