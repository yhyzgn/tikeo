use std::{net::SocketAddr, pin::Pin};

use sha2::{Digest, Sha256};
use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};
use tokio_stream::{Stream, StreamExt, wrappers::TcpListenerStream};
use tonic::{Request, Response, Status, transport::Server};

use crate::proto::worker::v1::{
    DispatchTask, Ping, ScriptProcessorBinding, ServerMessage, TaskProcessorBinding,
    WasmProcessorBinding, WorkerMessage, WorkerRegistered, server_message, task_processor_binding,
    worker_message, worker_tunnel_service_server,
    worker_tunnel_service_server::WorkerTunnelServiceServer,
};

use super::{
    ContainerScriptRunner, LocalSubprocessScriptRunner, ScriptRunner, ScriptRunnerKind,
    ScriptRunnerPolicy, ScriptRunnerRegistry, ScriptRunnerTask, TaskContext, TaskOutcome,
    TaskProcessor, UnsupportedScriptRunner, WorkerClient, WorkerConfig, WorkerSdkError,
};

#[tokio::test]
async fn unsupported_script_runner_validates_default_deny_policy_before_execution() {
    assert_eq!(
        ScriptRunnerKind::from_language("python"),
        Some(ScriptRunnerKind::Python)
    );
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

#[test]
fn container_script_runner_builds_default_deny_docker_args() {
    let runner = ContainerScriptRunner::with_runtime(
        ScriptRunnerKind::Shell,
        "docker",
        "alpine:3.20",
        ["--pull=never".to_owned()],
    );
    let task = script_task("shell", "echo ok\n", ScriptRunnerPolicy::default());

    let args = runner
        .docker_args(&task)
        .unwrap_or_else(|error| panic!("container args should build: {error}"));

    assert!(args.iter().any(|arg| arg == "--network=none"));
    assert!(args.iter().any(|arg| arg == "--read-only"));
    assert!(args.iter().any(|arg| arg == "--pull=never"));
    assert!(args.iter().any(|arg| arg == "--memory=67108864"));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--tmpfs" && pair[1] == "/tmp:rw,noexec,nosuid,size=16m")
    );
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--env" && pair[1] == "TIKEE_SCRIPT_ID=script_shell")
    );
    assert_eq!(
        args.iter()
            .rev()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>(),
        vec!["alpine:3.20".to_owned(), "sh".to_owned(), "-s".to_owned()]
    );
}

#[tokio::test]
async fn container_script_runner_rejects_dangerous_policy_before_runtime() {
    let runner = ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20");
    let policy = ScriptRunnerPolicy {
        allow_network: true,
        ..ScriptRunnerPolicy::default()
    };
    let error = match runner.run(script_task("shell", "echo ok\n", policy)).await {
        Ok(outcome) => panic!("dangerous policy should fail before docker spawn: {outcome:?}"),
        Err(error) => error,
    };
    assert!(matches!(error, WorkerSdkError::UnsupportedScriptRunner(_)));
    assert!(error.to_string().contains("network access"));
}

#[tokio::test]
async fn local_subprocess_shell_runner_executes_released_snapshot() {
    let content = "exit 0\n";
    let runner = LocalSubprocessScriptRunner::new(ScriptRunnerKind::Shell);
    let outcome = runner
        .run(script_task("shell", content, ScriptRunnerPolicy::default()))
        .await
        .unwrap_or_else(|error| panic!("shell runner should execute: {error}"));
    assert_eq!(outcome, TaskOutcome::Succeeded);
}

#[tokio::test]
async fn local_subprocess_runner_enforces_digest_and_release_snapshot() {
    let runner = LocalSubprocessScriptRunner::new(ScriptRunnerKind::Shell);
    let mut task = script_task("shell", "exit 0\n", ScriptRunnerPolicy::default());
    task.content_sha256 = "deadbeef".to_owned();
    let error = match runner.run(task).await {
        Ok(outcome) => panic!("digest mismatch should fail before execution: {outcome:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("digest mismatch"));

    let mut task = script_task("shell", "exit 0\n", ScriptRunnerPolicy::default());
    task.version_id.clear();
    let error = match runner.run(task).await {
        Ok(outcome) => panic!("missing release snapshot should fail before execution: {outcome:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("released immutable"));
}

#[tokio::test]
async fn local_subprocess_runner_enforces_timeout_and_output_limit() {
    let timeout_runner =
        LocalSubprocessScriptRunner::with_command(ScriptRunnerKind::Shell, "sh", ["-s".to_owned()]);
    let timeout_policy = ScriptRunnerPolicy {
        timeout_ms: 10,
        ..ScriptRunnerPolicy::default()
    };
    let error = match timeout_runner
        .run(script_task("shell", "sleep 1\n", timeout_policy))
        .await
    {
        Ok(outcome) => panic!("sleeping script should time out: {outcome:?}"),
        Err(error) => error,
    };
    assert!(matches!(error, WorkerSdkError::ScriptTimeout { .. }));

    let output_policy = ScriptRunnerPolicy {
        max_output_bytes: 4,
        ..ScriptRunnerPolicy::default()
    };
    let error = match timeout_runner
        .run(script_task("shell", "printf 12345\n", output_policy))
        .await
    {
        Ok(outcome) => panic!("large output should be rejected: {outcome:?}"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        WorkerSdkError::ScriptOutputLimitExceeded { .. }
    ));
}

#[tokio::test]
async fn local_subprocess_runner_reports_unavailable_runtime() {
    let runner = LocalSubprocessScriptRunner::with_command(
        ScriptRunnerKind::Shell,
        "definitely-missing-tikee-shell-runtime",
        ["-s".to_owned()],
    );
    let error = match runner
        .run(script_task(
            "shell",
            "exit 0\n",
            ScriptRunnerPolicy::default(),
        ))
        .await
    {
        Ok(outcome) => panic!("missing executable should fail: {outcome:?}"),
        Err(error) => error,
    };
    assert!(matches!(error, WorkerSdkError::ScriptRuntimeUnavailable(_)));
}

#[tokio::test]
async fn worker_session_executes_script_binding_with_registered_runner() {
    let dispatch = script_dispatch_task("instance-script-ok", "exit 0\n");
    let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
    let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-script-ok");
    let mut session = WorkerClient::new(config)
        .connect()
        .await
        .unwrap_or_else(|error| panic!("worker should register: {error}"));
    let mut registry = ScriptRunnerRegistry::new();
    registry.register(LocalSubprocessScriptRunner::new(ScriptRunnerKind::Shell));

    let outcome = session
        .process_next_with_script_runners(&EchoProcessor, &registry)
        .await
        .unwrap_or_else(|error| panic!("script result should report: {error}"));

    assert_eq!(outcome, TaskOutcome::Succeeded);
    let result = next_task_result(&mut events).await;
    assert!(result.success);
    server.abort();
}

#[tokio::test]
async fn worker_session_rejects_script_binding_without_registered_runner() {
    let dispatch = script_dispatch_task("instance-script-missing-runner", "exit 0\n");
    let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
    let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-script-missing");
    let mut session = WorkerClient::new(config)
        .connect()
        .await
        .unwrap_or_else(|error| panic!("worker should register: {error}"));

    let outcome = session
        .process_next(&EchoProcessor)
        .await
        .unwrap_or_else(|error| panic!("script rejection should report: {error}"));

    assert!(matches!(outcome, TaskOutcome::Failed(message) if message.contains("not registered")));
    let result = next_task_result(&mut events).await;
    assert!(!result.success);
    assert!(result.message.contains("not registered"));
    assert!(result.message.contains("script_missing_worker_runner"));
    server.abort();
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

    assert!(matches!(outcome, TaskOutcome::Failed(message) if message.contains("feature 'wasm'")));
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

    assert!(matches!(outcome, TaskOutcome::Failed(message) if message.contains("digest mismatch")));
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
                                kind: Some(server_message::Kind::Registered(WorkerRegistered {
                                    worker_id: format!("mock-{}", register.client_instance_id),
                                    lease_seconds: 30,
                                })),
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

fn script_task(language: &str, content: &str, policy: ScriptRunnerPolicy) -> ScriptRunnerTask {
    ScriptRunnerTask {
        script_id: format!("script_{language}"),
        version_id: "sv_1".to_owned(),
        version_number: 1,
        language: language.to_owned(),
        content: content.to_owned(),
        content_sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
        policy,
    }
}

fn script_dispatch_task(instance_id: &str, content: &str) -> DispatchTask {
    DispatchTask {
        instance_id: instance_id.to_owned(),
        job_id: "job-script".to_owned(),
        payload: Vec::new(),
        processor_name: "script:script_shell".to_owned(),
        processor_binding: Some(Box::new(TaskProcessorBinding {
            kind: Some(task_processor_binding::Kind::Script(
                ScriptProcessorBinding {
                    script_id: "script_shell".to_owned(),
                    version: "1.0.0".to_owned(),
                    language: "shell".to_owned(),
                    content: content.as_bytes().to_vec(),
                    version_id: "sv_shell_1".to_owned(),
                    version_number: 1,
                    content_sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
                    timeout_ms: 1_000,
                    max_memory_bytes: 64 * 1024 * 1024,
                    max_output_bytes: 1024 * 1024,
                    allow_network: false,
                    allowed_env_vars: Vec::new(),
                    read_only_paths: Vec::new(),
                    writable_paths: Vec::new(),
                    secret_refs: Vec::new(),
                },
            )),
        })),
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
