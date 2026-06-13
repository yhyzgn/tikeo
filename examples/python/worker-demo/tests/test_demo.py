import os

import tikeo
from tikeo_python_worker_demo.__main__ import script_sandbox_backend, process_task


def test_demo_does_not_advertise_local_scripts_by_default():
    assert os.environ.get("TIKEO_ENABLE_LOCAL_SCRIPT_SHELL", "").lower() not in {"1", "true", "yes", "on"}


def test_auto_sandbox_backend_matches_java_lightweight_defaults(monkeypatch):
    monkeypatch.delenv("TIKEO_WORKER_SCRIPT_SANDBOX", raising=False)
    assert script_sandbox_backend("python") == "srt"
    assert script_sandbox_backend("javascript") == "deno"
    assert script_sandbox_backend("typescript") == "deno"


def test_demo_processors_emit_task_logs():
    logs = []
    outcome = process_task(tikeo.TaskContext("inst-1", "job-1", "demo.echo", b"hello", lambda level, message: logs.append((level, message))))
    assert outcome.success
    assert outcome.message == "python demo echo processed"
    assert any("[demo.echo]" in message for _level, message in logs)


def test_demo_fail_returns_business_failure_and_exception_raises_runtime_error():
    fail_logs = []
    failure = process_task(tikeo.TaskContext("inst-fail", "job-1", "demo.fail", b"bad-input", lambda level, message: fail_logs.append((level, message))))
    assert not failure.success
    assert failure.message == "python demo intentional failure"
    assert any(level == "error" and "[demo.fail]" in message and "bad-input" in message for level, message in fail_logs)

    exception_logs = []
    try:
        process_task(tikeo.TaskContext("inst-exception", "job-1", "demo.exception", b"bad-input", lambda level, message: exception_logs.append((level, message))))
    except RuntimeError as exc:
        assert "python demo runtime exception" in str(exc)
    else:
        raise AssertionError("demo.exception should raise RuntimeError")
    assert any(level == "error" and "[demo.exception]" in message and "bad-input" in message for level, message in exception_logs)
