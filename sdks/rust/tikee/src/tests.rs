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

use super::script::SrtScriptRunner;
use super::{
    ContainerScriptRunner, DenoScriptRunner, LocalSubprocessScriptRunner, ScriptRunner,
    ScriptRunnerKind, ScriptRunnerPolicy, ScriptRunnerRegistry, ScriptRunnerTask, TaskContext,
    TaskOutcome, TaskProcessor, UnsupportedScriptRunner, WorkerClient, WorkerConfig,
    WorkerSdkError,
};

#[test]
fn worker_config_registers_structured_capabilities_without_legacy_routing_strings() {
    let mut config = WorkerConfig::local("http://127.0.0.1:9998", "rust-demo");
    config.add_tag("rust");
    config.add_sdk_processor("demo.echo");
    config.add_script_runner("python", "container");
    config.add_plugin_processor("sql", "billing.sql-sync");

    let message = config.register_message();
    let register = match message.kind {
        Some(worker_message::Kind::Register(register)) => register,
        other => panic!("expected register message, got {other:?}"),
    };
    let Some(structured) = register.structured_capabilities else {
        panic!("structured capabilities should be present");
    };

    assert!(register.capabilities.is_empty());
    assert_eq!(structured.tags, vec!["rust"]);
    assert_eq!(structured.sdk_processors[0].name, "demo.echo");
    assert_eq!(structured.script_runners[0].language, "python");
    assert_eq!(structured.script_runners[0].sandbox_backend, "container");
    assert_eq!(structured.plugin_processors[0].r#type, "sql");
    assert_eq!(
        structured.plugin_processors[0].processor_names,
        vec!["billing.sql-sync"]
    );
}

#[test]
fn unsupported_script_runner_is_registered_but_not_advertised() {
    let mut registry = ScriptRunnerRegistry::new();
    registry.register(UnsupportedScriptRunner::new(
        ScriptRunnerKind::Python,
        "srt is not installed",
    ));

    assert!(registry.get(ScriptRunnerKind::Python).is_some());
    assert!(registry.structured_capabilities().is_empty());
}

#[test]
fn container_script_runner_runtime_path_advertises_canonical_backend() {
    let runner = ContainerScriptRunner::with_runtime(
        ScriptRunnerKind::Shell,
        "/usr/bin/podman",
        "alpine:3.20",
        std::iter::empty::<String>(),
    );
    let mut registry = ScriptRunnerRegistry::new();
    registry.register(runner);

    let capabilities = registry.structured_capabilities();
    assert_eq!(capabilities.len(), 1);
    assert_eq!(capabilities[0].sandbox_backend, "podman");
}

#[test]
fn task_outcome_success_can_carry_operator_message() {
    let outcome = TaskOutcome::Success("rust demo echo processed".to_owned());

    assert_eq!(
        outcome.message().as_deref(),
        Some("rust demo echo processed")
    );
    assert!(outcome.failure_class().is_none());
}

#[tokio::test]
async fn unsupported_script_runner_validates_default_deny_policy_before_execution() {
    assert_eq!(
        ScriptRunnerKind::from_language("python"),
        Some(ScriptRunnerKind::Python)
    );
    assert_eq!(ScriptRunnerKind::Js.as_str(), "javascript");
    assert_eq!(ScriptRunnerKind::Ts.as_str(), "typescript");

    let runner = UnsupportedScriptRunner::new(
        ScriptRunnerKind::Python,
        "SRT sandbox runtime is unavailable",
    );
    let task = ScriptRunnerTask {
        script_id: "script_py".to_owned(),
        version_id: "sv_1".to_owned(),
        version_number: 1,
        language: "python".to_owned(),
        content: "print(1)".to_owned(),
        content_sha256: format!("{:x}", sha2::Sha256::digest(b"print(1)")),
        policy: ScriptRunnerPolicy::default(),
        log: None,
    };
    let error = match runner.run(task).await {
        Ok(outcome) => panic!("runner should not execute yet: {outcome:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("backend is unavailable"));

    let dangerous = ScriptRunnerPolicy {
        allowed_network_hosts: vec!["api.example.com".to_owned()],
        ..ScriptRunnerPolicy::default()
    };
    let error = match dangerous.validate_default_deny() {
        Ok(()) => panic!("dangerous policy should be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("network access"));
}

#[test]
fn container_script_runner_builds_file_grant_docker_args() {
    let runner = ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20");
    let policy = ScriptRunnerPolicy {
        read_only_paths: vec!["/data/input".to_owned()],
        writable_paths: vec!["/data/output".to_owned()],
        ..ScriptRunnerPolicy::default()
    };
    let task = script_task("shell", "echo ok\n", policy);

    let args = runner
        .docker_args(&task)
        .unwrap_or_else(|error| panic!("file-grant container args should build: {error}"));

    assert!(args.windows(2).any(|pair| {
        pair[0] == "--mount" && pair[1] == "type=bind,src=/data/input,dst=/data/input,readonly"
    }));
    assert!(args.windows(2).any(|pair| {
        pair[0] == "--mount" && pair[1] == "type=bind,src=/data/output,dst=/data/output"
    }));
}

#[test]
fn container_script_runner_rejects_network_and_secret_grants_fail_closed() {
    let runner = ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20");
    let network_policy = ScriptRunnerPolicy {
        allow_network: true,
        allowed_network_hosts: vec!["api.example.com".to_owned()],
        ..ScriptRunnerPolicy::default()
    };
    let error = match runner.docker_args(&script_task("shell", "echo ok\n", network_policy)) {
        Ok(args) => panic!("docker args must fail closed for network grants: {args:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("network grants"));

    let secret_policy = ScriptRunnerPolicy {
        secret_refs: vec!["secret:db-readonly".to_owned()],
        ..ScriptRunnerPolicy::default()
    };
    let error = match runner.docker_args(&script_task("shell", "echo ok\n", secret_policy)) {
        Ok(args) => panic!("docker args must fail closed for secret grants: {args:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("secret refs"));
}

#[test]
fn container_script_runner_rejects_malformed_file_grants() {
    let runner = ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20");
    let policy = ScriptRunnerPolicy {
        read_only_paths: vec!["relative/path".to_owned()],
        ..ScriptRunnerPolicy::default()
    };
    let error = match runner.docker_args(&script_task("shell", "echo ok\n", policy)) {
        Ok(args) => panic!("relative file grant must be rejected: {args:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("clean and absolute"));
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
        allowed_network_hosts: vec!["api.example.com".to_owned()],
        ..ScriptRunnerPolicy::default()
    };
    let error = match runner.run(script_task("shell", "echo ok\n", policy)).await {
        Ok(outcome) => panic!("dangerous policy should fail before docker spawn: {outcome:?}"),
        Err(error) => error,
    };
    assert!(matches!(error, WorkerSdkError::UnsupportedScriptRunner(_)));
    assert!(error.to_string().contains("network grants"));
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
async fn srt_runner_preserves_node_path_for_env_launcher_after_env_clear() {
    let temp_root = std::env::temp_dir().join(format!(
        "tikee-rust-srt-node-path-test-{}",
        std::process::id()
    ));
    let runtime_dir = temp_root.join("srt-bin");
    let node_dir = temp_root.join("node-bin");
    std::fs::create_dir_all(&runtime_dir)
        .unwrap_or_else(|error| panic!("create fake runtime dir: {error}"));
    std::fs::create_dir_all(&node_dir)
        .unwrap_or_else(|error| panic!("create fake node dir: {error}"));

    let runtime = runtime_dir.join("srt");
    let node = node_dir.join("node");
    write_executable(&runtime, "#!/bin/sh\nenv node \"$@\"\n");
    write_executable(&node, "#!/bin/sh\nprintf 'fake-node-srt-ok\\n'\n");

    let runner = SrtScriptRunner::new(ScriptRunnerKind::Shell, runtime, "sh", [node_dir]);
    let outcome = runner
        .run(script_task(
            "shell",
            "echo should-not-reach-host\n",
            ScriptRunnerPolicy::default(),
        ))
        .await
        .unwrap_or_else(|error| {
            panic!("fake SRT launcher should find node on sanitized PATH: {error}")
        });

    assert_eq!(outcome, TaskOutcome::Succeeded);
    let _ = std::fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn srt_runner_starts_supported_kinds_inside_task_sandbox_home() {
    for (kind, language, interpreter) in [
        (ScriptRunnerKind::Shell, "shell", "sh"),
        (ScriptRunnerKind::Python, "python", "python3"),
        (ScriptRunnerKind::PowerShell, "powershell", "pwsh"),
        (ScriptRunnerKind::Rhai, "rhai", "rhai-run"),
        (ScriptRunnerKind::Php, "php", "php"),
        (ScriptRunnerKind::Groovy, "groovy", "groovy"),
    ] {
        let temp_root = std::env::temp_dir().join(format!(
            "tikee-rust-srt-home-test-{}-{}",
            language,
            std::process::id()
        ));
        let runtime_dir = temp_root.join("runtime-bin");
        std::fs::create_dir_all(&runtime_dir)
            .unwrap_or_else(|error| panic!("create fake runtime dir: {error}"));
        let report = temp_root.join("report.txt");
        let runtime = runtime_dir.join("srt");
        write_executable(
            &runtime,
            &format!(
                r#"#!/bin/sh
printf 'cwd=%s\n' "$(pwd)" > {}
printf 'home=%s\n' "$HOME" >> {}
printf 'tmp=%s\n' "$TMPDIR" >> {}
printf 'claude_tmp=%s\n' "$CLAUDE_CODE_TMPDIR" >> {}
printf 'args=%s\n' "$*" >> {}
exit 0
"#,
                report.display(),
                report.display(),
                report.display(),
                report.display(),
                report.display()
            ),
        );

        let runner = SrtScriptRunner::new(kind, runtime, interpreter, std::iter::empty());
        let outcome = runner
            .run(script_task(
                language,
                runner_home_probe_content(kind),
                ScriptRunnerPolicy::default(),
            ))
            .await
            .unwrap_or_else(|error| panic!("fake SRT should run for {language}: {error}"));
        assert_eq!(outcome, TaskOutcome::Succeeded, "kind={language}");

        let report_content = std::fs::read_to_string(&report)
            .unwrap_or_else(|error| panic!("read fake SRT report for {language}: {error}"));
        let cwd = report_value(&report_content, "cwd");
        let home = report_value(&report_content, "home");
        let tmp = report_value(&report_content, "tmp");
        let claude_tmp = report_value(&report_content, "claude_tmp");
        let args = report_value(&report_content, "args");

        assert_eq!(cwd, home, "{language} should start SRT in sandbox HOME");
        assert!(
            home.contains(&format!("tikee-srt-{}-runtime", kind.as_str())),
            "{language} HOME should be a task runtime dir: {home}"
        );
        let runtime_root = std::path::Path::new(home)
            .parent()
            .unwrap_or_else(|| panic!("{language} HOME should have a runtime root: {home}"));
        assert_eq!(std::path::Path::new(tmp), runtime_root.join("tmp"));
        assert_eq!(claude_tmp, tmp);
        assert!(
            !home.starts_with("/tmp/tikee-rhai-script"),
            "{language} must not use legacy temp-file directories as HOME"
        );
        if kind == ScriptRunnerKind::Rhai {
            assert!(
                args.contains("/home/script-"),
                "rhai script file should live under sandbox HOME: {args}"
            );
        }
        let _ = std::fs::remove_dir_all(temp_root);
    }
}

#[tokio::test]
async fn deno_runner_starts_js_and_ts_inside_task_sandbox_home() {
    for (kind, language) in [
        (ScriptRunnerKind::Js, "javascript"),
        (ScriptRunnerKind::Ts, "typescript"),
    ] {
        let temp_root = std::env::temp_dir().join(format!(
            "tikee-rust-deno-home-test-{}-{}",
            language,
            std::process::id()
        ));
        let runtime_dir = temp_root.join("runtime-bin");
        std::fs::create_dir_all(&runtime_dir)
            .unwrap_or_else(|error| panic!("create fake deno runtime dir: {error}"));
        let report = temp_root.join("report.txt");
        let runtime = runtime_dir.join("deno");
        write_executable(
            &runtime,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
printf 'cwd=%s\n' "$(pwd)" > {}
printf 'home=%s\n' "$HOME" >> {}
printf 'tmp=%s\n' "$TMPDIR" >> {}
printf 'deno_dir=%s\n' "$DENO_DIR" >> {}
printf 'args=%s\n' "$*" >> {}
exit 0
"#,
                report.display(),
                report.display(),
                report.display(),
                report.display(),
                report.display()
            ),
        );

        let runner = DenoScriptRunner::new(kind, runtime);
        let outcome = runner
            .run(script_task(
                language,
                "console.log(JSON.stringify({ ok: true }))\n",
                ScriptRunnerPolicy::default(),
            ))
            .await
            .unwrap_or_else(|error| panic!("fake Deno should run for {language}: {error}"));
        assert_eq!(outcome, TaskOutcome::Succeeded, "kind={language}");

        let report_content = std::fs::read_to_string(&report)
            .unwrap_or_else(|error| panic!("read fake Deno report for {language}: {error}"));
        let cwd = report_value(&report_content, "cwd");
        let home = report_value(&report_content, "home");
        let tmp = report_value(&report_content, "tmp");
        let deno_dir = report_value(&report_content, "deno_dir");
        let args = report_value(&report_content, "args");

        assert_eq!(cwd, home, "{language} should start Deno in sandbox HOME");
        assert!(
            home.contains(&format!("tikee-deno-{}-runtime", kind.as_str())),
            "{language} HOME should be a task runtime dir: {home}"
        );
        let runtime_root = std::path::Path::new(home)
            .parent()
            .unwrap_or_else(|| panic!("{language} HOME should have a runtime root: {home}"));
        assert_eq!(std::path::Path::new(tmp), runtime_root.join("tmp"));
        assert_eq!(
            std::path::Path::new(deno_dir),
            runtime_root.join("cache/deno")
        );
        assert!(
            args.contains("run --no-prompt"),
            "unexpected Deno args: {args}"
        );
        let _ = std::fs::remove_dir_all(temp_root);
    }
}

#[tokio::test]
async fn worker_session_records_only_task_logger_lines_for_processors() {
    let (addr, server, mut events) = start_mock_tunnel_server(Some(DispatchTask {
        instance_id: "instance-rust-console".to_owned(),
        job_id: "job-rust-console".to_owned(),
        payload: b"hello".to_vec(),
        processor_name: "demo.console".to_owned(),
        processor_binding: None,
        assignment_token: "assign-token-console".to_owned(),
    }))
    .await;
    let config = WorkerConfig::local(format!("http://{addr}"), "worker-rust-console");
    let mut session = WorkerClient::new(config)
        .connect()
        .await
        .unwrap_or_else(|error| panic!("worker should register: {error}"));

    let outcome = session
        .process_next(&ConsoleProcessor)
        .await
        .unwrap_or_else(|error| panic!("task should process: {error}"));
    assert_eq!(outcome, TaskOutcome::Succeeded);

    let logs = collect_until_result(&mut events).await;
    assert!(
        logs.iter().any(|log| log.level == "info"
            && log.message == "rust processor task logger info"
            && log.assignment_token == "assign-token-console"),
        "missing captured stdout log: {logs:?}"
    );
    assert!(
        logs.iter().any(|log| log.level == "error"
            && log.message == "rust processor task logger error"
            && log.assignment_token == "assign-token-console"),
        "missing captured stderr log: {logs:?}"
    );
    server.abort();
}

#[tokio::test]
async fn worker_session_records_script_runner_pipe_output_as_task_logs() {
    let dispatch = script_dispatch_task(
        "instance-rust-script-console",
        "printf 'rust script stdout\\n'; printf 'rust script stderr\\n' >&2\n",
    );
    let (addr, server, mut events) = start_mock_tunnel_server(Some(dispatch)).await;
    let config = WorkerConfig::local(format!("http://{addr}"), "worker-rust-script-console");
    let mut session = WorkerClient::new(config)
        .connect()
        .await
        .unwrap_or_else(|error| panic!("worker should register: {error}"));
    let mut registry = ScriptRunnerRegistry::new();
    registry.register(LocalSubprocessScriptRunner::new(ScriptRunnerKind::Shell));

    let outcome = session
        .process_next_with_script_runners(&EchoProcessor, &registry)
        .await
        .unwrap_or_else(|error| panic!("script should process: {error}"));
    assert_eq!(outcome, TaskOutcome::Succeeded);

    let logs = collect_until_result(&mut events).await;
    assert!(
        logs.iter()
            .any(|log| log.level == "info" && log.message == "[script] rust script stdout"),
        "missing script stdout log: {logs:?}"
    );
    assert!(
        logs.iter()
            .any(|log| log.level == "error" && log.message == "[script] rust script stderr"),
        "missing script stderr log: {logs:?}"
    );
    server.abort();
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
    assert_eq!(result.assignment_token, "assign-token-1");
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
async fn worker_session_close_sends_graceful_unregister() {
    let (addr, server, mut events) = start_mock_tunnel_server(None).await;
    let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-stop");
    let session = WorkerClient::new(config)
        .connect()
        .await
        .unwrap_or_else(|error| panic!("worker should register: {error}"));

    session
        .close()
        .await
        .unwrap_or_else(|error| panic!("graceful close should send unregister: {error}"));

    while let Some(message) = events.recv().await {
        if let Some(worker_message::Kind::Unregister(unregister)) = message.kind {
            assert_eq!(unregister.worker_id, "mock-worker-sdk-stop");
            assert_eq!(unregister.generation, 1);
            assert_eq!(unregister.fencing_token, "mock-fencing-token");
            server.abort();
            return;
        }
    }
    panic!("unregister should arrive before the tunnel closes");
}

#[tokio::test]
async fn worker_session_processes_dispatched_task_and_reports_result() {
    let (addr, server, mut events) = start_mock_tunnel_server(Some(DispatchTask {
        instance_id: "instance-1".to_owned(),
        job_id: "job-1".to_owned(),
        payload: b"hello".to_vec(),
        processor_name: "demo.echo".to_owned(),
        processor_binding: None,
        assignment_token: "assign-token-1".to_owned(),
    }))
    .await;

    let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-2");
    let mut session = WorkerClient::new(config)
        .connect()
        .await
        .unwrap_or_else(|error| panic!("worker should register: {error}"));
    session
        .emit_task_log("instance-1", "assign-token-1", "info", "starting")
        .await
        .unwrap_or_else(|error| panic!("log should emit: {error}"));

    let outcome = session
        .process_next(&EchoProcessor)
        .await
        .unwrap_or_else(|error| panic!("task should process: {error}"));
    assert_eq!(outcome, TaskOutcome::Succeeded);

    let mut saw_manual_log = false;
    let mut saw_received_log = false;
    let mut saw_completed_log = false;
    let mut saw_result = false;
    while let Some(message) = events.recv().await {
        match message.kind {
            Some(worker_message::Kind::TaskLog(log)) => {
                if log.instance_id == "instance-1"
                    && log.message == "starting"
                    && log.assignment_token == "assign-token-1"
                {
                    saw_manual_log = true;
                }
                if log.instance_id == "instance-1"
                    && log.message.contains("received task instance-1")
                    && log.assignment_token == "assign-token-1"
                {
                    saw_received_log = true;
                }
                if log.instance_id == "instance-1"
                    && log
                        .message
                        .contains("completed task instance-1 success=true")
                    && log.assignment_token == "assign-token-1"
                {
                    saw_completed_log = true;
                }
            }
            Some(worker_message::Kind::TaskResult(result)) => {
                saw_result = result.instance_id == "instance-1" && result.success;
                break;
            }
            _ => {}
        }
    }
    assert!(
        saw_manual_log,
        "mock tunnel should receive emitted task log"
    );
    assert!(
        saw_received_log,
        "mock tunnel should receive automatic task-start log"
    );
    assert!(
        saw_completed_log,
        "mock tunnel should receive automatic task-complete log"
    );
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

async fn collect_until_result(
    events: &mut mpsc::Receiver<WorkerMessage>,
) -> Vec<crate::proto::worker::v1::TaskLog> {
    let mut logs = Vec::new();
    while let Some(message) = events.recv().await {
        match message.kind {
            Some(worker_message::Kind::TaskLog(log)) => logs.push(log),
            Some(worker_message::Kind::TaskResult(_)) => return logs,
            _ => {}
        }
    }
    panic!("task result should arrive");
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
                                    generation: 1,
                                    fencing_token: "mock-fencing-token".to_owned(),
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
                        worker_message::Kind::TaskResult(_)
                        | worker_message::Kind::TaskLog(_)
                        | worker_message::Kind::TaskCheckpoint(_)
                        | worker_message::Kind::Unregister(_),
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

struct ConsoleProcessor;

#[async_trait::async_trait]
impl TaskProcessor for ConsoleProcessor {
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        assert_eq!(task.payload, b"hello");
        println!("rust processor task logger info should stay console-only");
        eprintln!("rust processor task logger error should stay console-only");
        task.log_info("rust processor task logger info");
        task.log_error("rust processor task logger error");
        Ok(TaskOutcome::Succeeded)
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
        log: None,
    }
}

fn runner_home_probe_content(kind: ScriptRunnerKind) -> &'static str {
    match kind {
        ScriptRunnerKind::Shell => "pwd\n",
        ScriptRunnerKind::Python => "import os; print(os.getcwd())\n",
        ScriptRunnerKind::PowerShell => "Get-Location\n",
        ScriptRunnerKind::Rhai => "print(\"ok\");\n",
        ScriptRunnerKind::Js
        | ScriptRunnerKind::Ts
        | ScriptRunnerKind::Php
        | ScriptRunnerKind::Groovy => "",
    }
}

fn report_value<'a>(report: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}=");
    report
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("missing {key} in fake SRT report:\n{report}"))
}

fn write_executable(path: &std::path::Path, content: &str) {
    std::fs::write(path, content).unwrap_or_else(|error| {
        panic!("write executable {}: {error}", path.display());
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .unwrap_or_else(|error| panic!("read executable metadata {}: {error}", path.display()))
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap_or_else(|error| {
            panic!("set executable permissions {}: {error}", path.display());
        });
    }
}

fn script_dispatch_task(instance_id: &str, content: &str) -> DispatchTask {
    DispatchTask {
        instance_id: instance_id.to_owned(),
        job_id: "job-script".to_owned(),
        payload: Vec::new(),
        processor_name: "script_shell".to_owned(),
        assignment_token: "assign-token-1".to_owned(),
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
                    allowed_network_hosts: Vec::new(),
                    read_only_paths: Vec::new(),
                    writable_paths: Vec::new(),
                    secret_refs: Vec::new(),
                    sandbox_backend: "auto".to_owned(),
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
        processor_name: "script_wasm".to_owned(),
        assignment_token: String::new(),
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
