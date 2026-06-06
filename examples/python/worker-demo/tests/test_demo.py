import os

import tikee
from tikee_python_worker_demo.__main__ import script_sandbox_backend, process_task


def test_demo_does_not_advertise_local_scripts_by_default():
    assert os.environ.get("TIKEE_ENABLE_LOCAL_SCRIPT_SHELL", "").lower() not in {"1", "true", "yes", "on"}


def test_auto_sandbox_backend_matches_java_lightweight_defaults(monkeypatch):
    monkeypatch.delenv("TIKEE_WORKER_SCRIPT_SANDBOX", raising=False)
    assert script_sandbox_backend("python") == "srt"
    assert script_sandbox_backend("javascript") == "deno"
    assert script_sandbox_backend("typescript") == "deno"


def test_demo_processors_emit_task_logs():
    logs = []
    outcome = process_task(tikee.TaskContext("inst-1", "job-1", "demo.echo", b"hello", lambda level, message: logs.append((level, message))))
    assert outcome.success
    assert outcome.message == "python demo echo processed"
    assert any("[demo.echo]" in message for _level, message in logs)
