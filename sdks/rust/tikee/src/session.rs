use std::time::Duration;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Streaming;

use crate::{
    config::WorkerConfig,
    error::WorkerSdkError,
    proto::worker::v1::{
        DispatchTask, Heartbeat, Ping, ScriptProcessorBinding, ServerMessage, TaskLog, TaskResult,
        UnregisterWorker, WorkerMessage, WorkerRegistered, server_message, task_processor_binding,
        worker_message,
        worker_tunnel_service_client::WorkerTunnelServiceClient,
    },
    script::{ScriptRunnerKind, ScriptRunnerPolicy, ScriptRunnerRegistry, ScriptRunnerTask},
    task::{TaskOutcome, TaskProcessor, task_context},
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
}

impl WorkerSession {
    /// Registered worker id acknowledged by tikee.
    #[must_use]
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Lease seconds returned by tikee registration ack.
    #[must_use]
    pub const fn lease_seconds(&self) -> u64 {
        self.lease_seconds
    }

    /// Worker session generation acknowledged by tikee.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Send one heartbeat and wait for the matching ping response.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed, the tikee returns a gRPC error,
    /// or the response type is unexpected.
    pub async fn heartbeat(&mut self) -> Result<Ping, WorkerSdkError> {
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
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)
    }

    /// Wait for one dispatched task, run it through the provided processor, and report the result.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel closes, the tikee returns a gRPC error,
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
    /// Returns an error when the tunnel closes, the tikee returns a gRPC error,
    /// or the result cannot be sent back.
    pub async fn process_next_with_script_runners<P>(
        &mut self,
        processor: &P,
        script_runners: &ScriptRunnerRegistry,
    ) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        loop {
            let message = self.next_server_message().await?;
            if let Some(server_message::Kind::DispatchTask(task)) = message.kind {
                return self.process_task(processor, script_runners, task).await;
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
        let context = task_context(&task);
        let outcome = if let Some(binding) = task.processor_binding.as_ref() {
            process_bound_task(binding, &task, script_runners).await
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
                    message: task_result_message(outcome),
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
            generation: registered.generation,
            fencing_token: registered.fencing_token,
            outbound: tx,
            inbound,
            heartbeat_sequence: 0,
        })
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
) -> TaskOutcome {
    match binding.kind.as_ref() {
        Some(task_processor_binding::Kind::Wasm(wasm)) => process_wasm_binding(wasm, task),
        Some(task_processor_binding::Kind::Script(script)) => {
            process_script_binding(script, script_runners).await
        }
        None => TaskOutcome::Failed("empty dynamic processor binding".to_owned()),
    }
}

async fn process_script_binding(
    binding: &ScriptProcessorBinding,
    script_runners: &ScriptRunnerRegistry,
) -> TaskOutcome {
    let Some(kind) = ScriptRunnerKind::from_language(&binding.language) else {
        return TaskOutcome::Failed(format!("unsupported script language: {}", binding.language));
    };
    let Some(runner) = script_runners.get(kind) else {
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
            env_vars: binding.allowed_env_vars.clone(),
            read_only_paths: binding.read_only_paths.clone(),
            writable_paths: binding.writable_paths.clone(),
            secret_refs: binding.secret_refs.clone(),
        },
    };
    match runner.run(task).await {
        Ok(outcome) => outcome,
        Err(error) => TaskOutcome::Failed(error.to_string()),
    }
}
