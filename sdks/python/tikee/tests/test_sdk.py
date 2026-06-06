from __future__ import annotations

import hashlib
import json
import os
import stat
from pathlib import Path

import pytest

import tikee
from tikee.runtime_dirs import ScriptTaskRuntimeDirs
from tikee.script import _write_srt_settings


def sha256_hex(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def script_task(language: str, content: bytes, **kwargs):
    return tikee.ScriptRunnerTask(
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
    config = tikee.local_config("http://127.0.0.1:9998", "python-worker-1")
    config.namespace = "tenant-a"
    config.app = "billing"
    config.capabilities = ["legacy-tag", "legacy-tag", ""]
    config.add_tag("python")
    config.add_sdk_processor("demo.echo")
    config.add_script_runner("python", "srt")
    config.add_plugin_processor("sql", "billing.sql-sync")
    client = tikee.Client(config)

    registration = client.registration()
    assert registration.client_instance_id == "python-worker-1"
    assert registration.namespace == "tenant-a"
    assert registration.app == "billing"
    assert registration.capabilities == ["legacy-tag"]
    assert registration.structured.sdk_processors == ["demo.echo"]
    assert registration.structured.script_runners[0].language == "python"
    assert registration.structured.plugin_processors[0].processor_names == ["billing.sql-sync"]

    client.start_dry_run(lambda task: tikee.succeeded())
    heartbeat = client.next_heartbeat("worker-1", "fence-1", 3)
    assert heartbeat.sequence == 1
    assert heartbeat.generation == 3
    assert heartbeat.fencing_token == "fence-1"


def test_config_validation_fails_closed():
    with pytest.raises(ValueError, match="endpoint"):
        tikee.Client(tikee.WorkerConfig(endpoint="", client_instance_id=""))
    config = tikee.local_config("http://127.0.0.1:9998", "python-worker-2")
    config.heartbeat_every = config.heartbeat_every * 0
    with pytest.raises(ValueError, match="heartbeat"):
        tikee.Client(config)


def test_grpc_target_normalizes_http_urls():
    assert tikee.grpc_target("127.0.0.1:9998") == "127.0.0.1:9998"
    assert tikee.grpc_target(" http://127.0.0.1:9998 ") == "127.0.0.1:9998"
    assert tikee.grpc_target("https://worker.example:443") == "worker.example:443"


def test_management_client_creates_structured_plugin_and_script_jobs():
    from http.server import BaseHTTPRequestHandler, HTTPServer
    import threading

    bodies = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            assert self.headers.get("x-tikee-api-key") == "key-1"
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            bodies.append(body)
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
        client = tikee.ManagementClient(f"http://127.0.0.1:{server.server_port}", "key-1", "dev-alpha", "orders")
        client.create_job(tikee.plugin_api_job("python-sql", "sql", "billing.sql-sync"))
        client.create_job(tikee.script_api_job("python-script", "script_manual_shell_echo"))
    finally:
        server.shutdown()
    assert bodies[0]["processorType"] == "sql"
    assert bodies[0]["retryPolicy"]["maxAttempts"] == 3
    assert bodies[1]["scriptId"] == "script_manual_shell_echo"


def test_local_command_script_runner_executes_released_shell_snapshot():
    runner = tikee.LocalCommandScriptRunner("shell", "custom")
    outcome = runner.run(script_task("shell", b"printf 'python-script-ok'\n"))
    assert outcome.success
    assert outcome.message == "python-script-ok"


def test_local_command_script_runner_rejects_unsafe_policy():
    runner = tikee.LocalCommandScriptRunner("shell", "custom")
    outcome = runner.run(script_task("shell", b"echo unsafe\n", allow_network=True))
    assert not outcome.success
    assert "network" in outcome.message


def test_unavailable_script_runner_is_fail_closed_but_not_advertised():
    config = tikee.local_config("http://127.0.0.1:9998", "python-worker-unavailable")
    registry = tikee.ScriptRunnerRegistry().register(tikee.UnavailableScriptRunner("python", "srt", "srt is not installed"))
    registry.add_capabilities(config)
    assert config.structured.script_runners == []
    outcome = registry.get("python").run(script_task("python", b"print(1)"))
    assert not outcome.success
    assert "unavailable" in outcome.message


def test_sandbox_tool_resolver_does_not_advertise_missing_tools_when_auto_install_disabled(tmp_path):
    resolver = tikee.SandboxToolResolver(state_dir=str(tmp_path), auto_install=False)
    _path, ok = resolver.resolve_srt()
    assert not ok


def test_script_registry_adds_structured_capabilities():
    registry = tikee.ScriptRunnerRegistry().register(tikee.SrtScriptRunner("python", "srt", "python3")).register(tikee.DenoScriptRunner("javascript", "deno"))
    config = tikee.local_config("http://127.0.0.1:9998", "python-sandbox-test")
    registry.add_capabilities(config)
    seen = {runner.language: runner.sandbox_backend for runner in config.structured.script_runners}
    assert seen == {"javascript": "deno", "python": "srt"}


def test_srt_settings_serialize_empty_policy_lists_as_arrays():
    dirs = ScriptTaskRuntimeDirs.create("tikee-python-srt-settings-test")
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
    runner = tikee.SrtScriptRunner(language, str(runtime), interpreter)
    outcome = runner.run(script_task(language, content, allowed_env_vars=["HOME", "TMPDIR", "CLAUDE_CODE_TMPDIR"]))
    assert outcome.success
    values = read_report(report)
    assert values["cwd"] == values["home"]
    assert f"tikee-srt-{tikee.normalize_script_language(language)}-runtime" in values["home"]
    assert values["claude_tmp"] == values["tmp"]
    if language == "rhai":
        assert "/home/script-" in values["args"]


def test_deno_runner_starts_js_and_ts_inside_task_sandbox_home(tmp_path):
    for language in ["javascript", "typescript"]:
        report = tmp_path / f"{language}.txt"
        runtime = tmp_path / f"deno-{language}"
        write_executable(runtime, f"#!/bin/sh\ncat >/dev/null\nprintf 'cwd=%s\\n' \"$(pwd)\" > {report!s}\nprintf 'home=%s\\n' \"$HOME\" >> {report!s}\nprintf 'tmp=%s\\n' \"$TMPDIR\" >> {report!s}\nprintf 'deno_dir=%s\\n' \"$DENO_DIR\" >> {report!s}\nprintf 'args=%s\\n' \"$*\" >> {report!s}\nexit 0\n")
        runner = tikee.DenoScriptRunner(language, str(runtime))
        outcome = runner.run(script_task(language, b"console.log('ok')\n", allowed_env_vars=["HOME", "TMPDIR", "DENO_DIR"]))
        assert outcome.success
        values = read_report(report)
        assert values["cwd"] == values["home"]
        assert f"tikee-deno-{language}-runtime" in values["home"]
        assert values["deno_dir"].endswith("/cache/deno")
        assert "run --no-prompt" in values["args"]
