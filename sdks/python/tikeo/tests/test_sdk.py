from __future__ import annotations

import hashlib
import logging
import json
from types import SimpleNamespace
import stat
import time
from pathlib import Path

import pytest

import tikeo
from tikeo.runtime_dirs import ScriptTaskRuntimeDirs
from tikeo.script import _write_srt_settings


def sha256_hex(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def script_task(language: str, content: bytes, **kwargs):
    return tikeo.ScriptRunnerTask(
        script_id=f"script-{language}",
        version_id="sv-test",
        version_number=1,
        language=language,
        content=content,
        content_sha256=sha256_hex(content),
        timeout_ms=1000,
        max_output_bytes=4096,
        **kwargs,
    )


def test_client_registration_and_heartbeat_dry_run():
    config = tikeo.local_config("http://127.0.0.1:9998", "python-worker-1")
    config.namespace = "tenant-a"
    config.app = "billing"
    config.capabilities = ["legacy-tag", "legacy-tag", ""]
    config.add_tag("python")
    config.add_normal_processor("demo.echo", "Python echo processor")
    config.add_script_runner("python", "srt")
    config.add_plugin_processor(tikeo.PluginType.SQL, "billing.sql-sync", "SQL sync processor")
    client = tikeo.Client(config)

    registration = client.registration()
    assert registration.client_instance_id == "python-worker-1"
    assert registration.namespace == "tenant-a"
    assert registration.app == "billing"
    assert registration.capabilities == ["legacy-tag"]
    assert registration.structured.normal_processors[0].name == "demo.echo"
    assert registration.structured.normal_processors[0].description == "Python echo processor"
    assert registration.structured.script_runners[0].language == "python"
    assert registration.structured.plugin_processors[0].processor_names == ["billing.sql-sync"]
    assert registration.structured.plugin_processors[0].processors[0].description == "SQL sync processor"

    client.start_dry_run(lambda task: tikeo.succeeded())
    heartbeat = client.next_heartbeat("worker-1", "fence-1", 3)
    assert heartbeat.sequence == 1
    assert heartbeat.generation == 3
    assert heartbeat.fencing_token == "fence-1"


def test_config_validation_fails_closed():
    with pytest.raises(ValueError, match="endpoint"):
        tikeo.Client(tikeo.WorkerConfig(endpoint="", client_instance_id=""))
    config = tikeo.local_config("http://127.0.0.1:9998", "python-worker-2")
    config.heartbeat_every = config.heartbeat_every * 0
    with pytest.raises(ValueError, match="heartbeat"):
        tikeo.Client(config)


def test_grpc_target_normalizes_http_urls():
    assert tikeo.grpc_target("127.0.0.1:9998") == "127.0.0.1:9998"
    assert tikeo.grpc_target(" http://127.0.0.1:9998 ") == "127.0.0.1:9998"
    assert tikeo.grpc_target("https://worker.example:443") == "worker.example:443"


def test_management_client_creates_structured_plugin_and_script_jobs():
    from http.server import BaseHTTPRequestHandler, HTTPServer
    import threading

    bodies = []
    paths = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            assert self.headers.get("x-tikeo-api-key") == "key-1"
            paths.append(self.path)
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            bodies.append(body)
            if self.path.endswith(":trigger"):
                payload = {
                    "code": 0,
                    "message": "ok",
                    "data": {
                        "id": "inst-1",
                        "jobId": "job-1",
                        "status": "pending",
                        "triggerType": body["triggerType"],
                        "executionMode": "single",
                        "createdAt": "now",
                        "updatedAt": "now",
                    },
                }
                data = json.dumps(payload).encode()
                self.send_response(200)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
                return
            payload = {"code": 0, "message": "ok", "data": {"id": "job-1", **body}}
            data = json.dumps(payload).encode()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def log_message(self, *_args):
            return

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        client = tikeo.ManagementClient(f"http://127.0.0.1:{server.server_port}", "key-1", "dev-alpha", "orders")
        client.create_job(tikeo.plugin_api_job("python-sql", "sql", "billing.sql-sync"))
        client.create_job(tikeo.script_api_job("python-script", "script_manual_shell_echo"))
        instance = client.trigger_job("job-1")
    finally:
        server.shutdown()
    assert bodies[0]["processorType"] == "sql"
    assert bodies[0]["retryPolicy"]["maxAttempts"] == 3
    assert bodies[1]["scriptId"] == "script_manual_shell_echo"
    assert paths[2] == "/api/v1/jobs/job-1:trigger"
    assert bodies[2]["triggerType"] == "api"
    assert bodies[2]["executionMode"] == "single"
    assert instance.job_id == "job-1"
    assert instance.trigger_type == "api"
    broadcast = tikeo.broadcast_api_trigger(tikeo.BroadcastSelectorRequest(region="us-east-1"))
    assert broadcast.to_json() == {
        "triggerType": "api",
        "executionMode": "broadcast",
        "broadcastSelector": {"region": "us-east-1"},
    }



def test_standard_logging_bridge_mirrors_processor_logs_only_inside_active_task_scope():
    from tikeo.client import process_dispatch_task

    logs = []
    logger = logging.getLogger("tikeo.tests.task_bridge")
    logger.handlers.clear()
    logger.setLevel(logging.INFO)
    logger.propagate = False
    handler = tikeo.install_task_log_handler(logger)

    task = SimpleNamespace(
        instance_id="inst-python-logger",
        job_id="job-python-logger",
        processor_name="demo.logger",
        payload=b"",
        processor_binding=None,
    )

    logger.info("python outside task scope should stay console-only")

    def processor(_task):
        logger.info("python native logger info order_id=%s", 42)
        logger.error("python native logger error")
        return tikeo.succeeded()

    outcome = process_dispatch_task(processor, None, task, lambda level, message: logs.append((level, message)))

    logger.removeHandler(handler)
    assert outcome.success
    assert any(level == "info" and "python native logger info" in message and "42" in message for level, message in logs)
    assert any(level == "error" and "python native logger error" in message for level, message in logs)
    assert not any("outside task scope" in message for _level, message in logs)


def test_processor_exceptions_are_reported_with_traceback_task_logs():
    from tikeo.client import process_dispatch_task

    logs = []
    task = SimpleNamespace(
        instance_id="inst-python-exception",
        job_id="job-python-exception",
        processor_name="demo.exception",
        payload=b"",
        processor_binding=None,
    )

    def processor(_task):
        raise RuntimeError("python runtime boom")

    outcome = process_dispatch_task(processor, None, task, lambda level, message: logs.append((level, message)))

    assert not outcome.success
    assert "python runtime boom" in outcome.message
    assert any(level == "error" and "Traceback" in message and "python runtime boom" in message for level, message in logs)

def test_local_command_script_runner_executes_released_shell_snapshot():
    runner = tikeo.LocalCommandScriptRunner("shell", "custom")
    outcome = runner.run(script_task("shell", b"printf 'python-script-ok'\n"))
    assert outcome.success
    assert outcome.message == "python-script-ok"


def test_local_command_script_runner_rejects_unsafe_policy():
    runner = tikeo.LocalCommandScriptRunner("shell", "custom")
    outcome = runner.run(script_task("shell", b"echo unsafe\n", allow_network=True))
    assert not outcome.success
    assert "network" in outcome.message


def test_unavailable_script_runner_is_fail_closed_but_not_advertised():
    config = tikeo.local_config("http://127.0.0.1:9998", "python-worker-unavailable")
    registry = tikeo.ScriptRunnerRegistry().register(tikeo.UnavailableScriptRunner("python", "srt", "srt is not installed"))
    registry.add_capabilities(config)
    assert config.structured.script_runners == []
    outcome = registry.get("python").run(script_task("python", b"print(1)"))
    assert not outcome.success
    assert "unavailable" in outcome.message


def test_sandbox_tool_resolver_does_not_advertise_missing_tools_when_auto_install_disabled(tmp_path, monkeypatch):
    monkeypatch.setenv("PATH", "")
    monkeypatch.setenv("TIKEO_SANDBOX_TOOLS_DIR", str(tmp_path / "host-tools"))
    resolver = tikeo.SandboxToolResolver(state_dir=str(tmp_path), auto_install=False)
    _path, ok = resolver.resolve_srt()
    assert not ok




def test_sandbox_tool_resolver_auto_install_returns_unavailable_immediately(tmp_path, monkeypatch):
    monkeypatch.setenv("PATH", "")
    monkeypatch.setenv("TIKEO_SANDBOX_TOOLS_DIR", str(tmp_path / "host-tools"))
    resolver = tikeo.SandboxToolResolver(state_dir=str(tmp_path), auto_install=True, install_timeout=0.001)
    started_at = time.monotonic()
    _path, ok = resolver.resolve_srt()
    assert not ok
    assert time.monotonic() - started_at < 1.0



def test_sandbox_tool_resolver_strict_sandbox_isolation_skips_host_path(tmp_path, monkeypatch):
    host_bin = tmp_path / "host-bin"
    host_bin.mkdir()
    write_executable(host_bin / "srt", "#!/bin/sh\necho host-srt\n")
    monkeypatch.setenv("PATH", str(host_bin))
    monkeypatch.setenv("TIKEO_SANDBOX_TOOLS_DIR", str(tmp_path / "managed-tools"))
    resolver = tikeo.SandboxToolResolver(state_dir=str(tmp_path), auto_install=False, strict_sandbox_isolation=True)
    _path, ok = resolver.resolve_srt()
    assert not ok
    _interpreter, interpreter_ok = resolver.resolve_interpreter("sh")
    assert not interpreter_ok

def test_sandbox_tool_resolver_uses_host_cache_when_worker_state_is_empty(tmp_path):
    resolver = tikeo.SandboxToolResolver(state_dir=str(tmp_path), auto_install=False)
    assert resolver._install_dir("srt") == Path.home() / ".tikeo" / "sandbox-tools" / "srt"


def test_script_registry_adds_structured_capabilities():
    registry = tikeo.ScriptRunnerRegistry().register(tikeo.SrtScriptRunner("python", "srt", "python3")).register(tikeo.DenoScriptRunner("javascript", "deno"))
    config = tikeo.local_config("http://127.0.0.1:9998", "python-sandbox-test")
    registry.add_capabilities(config)
    seen = {runner.language: runner.sandbox_backend for runner in config.structured.script_runners}
    assert seen == {"javascript": "deno", "python": "srt"}




def test_rhai_resolver_probes_by_running_script_file(tmp_path):
    binary = tmp_path / "rhai-run"
    report = tmp_path / "report.txt"
    write_executable(binary, f"#!/bin/sh\nprintf 'arg=%s\n' \"$1\" > {report!s}\ntest -f \"$1\"\n")
    resolver = tikeo.SandboxToolResolver(state_dir=str(tmp_path), auto_install=False)
    assert resolver._tool_works("rhai-run", str(binary))
    values = read_report(report)
    assert values["arg"].endswith(".rhai")
    assert values["arg"] != "--version"
    assert values["arg"] != "--help"


def test_srt_settings_serialize_empty_policy_lists_as_arrays():
    dirs = ScriptTaskRuntimeDirs.create("tikeo-python-srt-settings-test")
    try:
        settings = _write_srt_settings(script_task("shell", b"echo ok"), dirs, None)
        parsed = json.loads(settings.read_text())
        assert isinstance(parsed["network"]["allowedDomains"], list)
        assert isinstance(parsed["filesystem"]["allowRead"], list)
        assert str(dirs.powershell_cache) in parsed["filesystem"]["allowWrite"]
    finally:
        settings.unlink(missing_ok=True)
        dirs.cleanup()


def write_executable(path: Path, content: str) -> None:
    path.write_text(content)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def read_report(path: Path) -> dict[str, str]:
    return dict(line.split("=", 1) for line in path.read_text().strip().splitlines() if "=" in line)


@pytest.mark.parametrize("language,interpreter,content", [
    ("shell", "sh", b"pwd\n"),
    ("python", "python3", b"import os; print(os.getcwd())\n"),
    ("powershell", "pwsh", b"Get-Location\n"),
    ("rhai", "rhai-run", b"print(\"ok\");\n"),
])
def test_srt_runner_starts_supported_kinds_inside_task_sandbox_home(tmp_path, language, interpreter, content):
    report = tmp_path / "report.txt"
    runtime = tmp_path / "srt"
    write_executable(runtime, f"#!/bin/sh\nprintf 'cwd=%s\\n' \"$(pwd)\" > {report!s}\nprintf 'home=%s\\n' \"$HOME\" >> {report!s}\nprintf 'tmp=%s\\n' \"$TMPDIR\" >> {report!s}\nprintf 'claude_tmp=%s\\n' \"$CLAUDE_CODE_TMPDIR\" >> {report!s}\nprintf 'args=%s\\n' \"$*\" >> {report!s}\nexit 0\n")
    runner = tikeo.SrtScriptRunner(language, str(runtime), interpreter)
    outcome = runner.run(script_task(language, content, allowed_env_vars=["HOME", "TMPDIR", "CLAUDE_CODE_TMPDIR"]))
    assert outcome.success
    values = read_report(report)
    assert values["cwd"] == values["home"]
    assert f"tikeo-srt-{tikeo.normalize_script_language(language)}-runtime" in values["home"]
    assert values["claude_tmp"] == values["tmp"]
    if language == "rhai":
        assert "/home/script-" in values["args"]


def test_deno_runner_starts_js_and_ts_inside_task_sandbox_home(tmp_path):
    for language in ["javascript", "typescript"]:
        report = tmp_path / f"{language}.txt"
        runtime = tmp_path / f"deno-{language}"
        write_executable(runtime, f"#!/bin/sh\ncat >/dev/null\nprintf 'cwd=%s\\n' \"$(pwd)\" > {report!s}\nprintf 'home=%s\\n' \"$HOME\" >> {report!s}\nprintf 'tmp=%s\\n' \"$TMPDIR\" >> {report!s}\nprintf 'deno_dir=%s\\n' \"$DENO_DIR\" >> {report!s}\nprintf 'args=%s\\n' \"$*\" >> {report!s}\nexit 0\n")
        runner = tikeo.DenoScriptRunner(language, str(runtime))
        outcome = runner.run(script_task(language, b"console.log('ok')\n", allowed_env_vars=["HOME", "TMPDIR", "DENO_DIR"]))
        assert outcome.success
        values = read_report(report)
        assert values["cwd"] == values["home"]
        assert f"tikeo-deno-{language}-runtime" in values["home"]
        assert values["deno_dir"].endswith("/cache/deno")
        assert "run --no-prompt" in values["args"]
