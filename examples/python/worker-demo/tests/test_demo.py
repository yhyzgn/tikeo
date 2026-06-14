from __future__ import annotations

import logging
from types import SimpleNamespace

import tikeo
from tikeo.client import process_dispatch_task
from tikeo_python_worker_demo.__main__ import script_sandbox_backend, process_task


def test_demo_does_not_advertise_local_scripts_by_default(monkeypatch):
    monkeypatch.delenv("TIKEO_ENABLE_LOCAL_SCRIPT_SHELL", raising=False)
    assert not bool(__import__("os").environ.get("TIKEO_ENABLE_LOCAL_SCRIPT_SHELL"))


def test_auto_sandbox_backend_matches_java_lightweight_defaults(monkeypatch):
    monkeypatch.delenv("TIKEO_WORKER_SCRIPT_SANDBOX", raising=False)
    assert script_sandbox_backend("python") == "srt"
    assert script_sandbox_backend("javascript") == "deno"
    assert script_sandbox_backend("typescript") == "deno"


def _task(instance_id: str, processor_name: str, payload: bytes):
    return SimpleNamespace(
        instance_id=instance_id,
        job_id="job-1",
        processor_name=processor_name,
        payload=payload,
        processor_binding=None,
    )


def test_demo_processors_emit_standard_logging_through_task_bridge():
    logs: list[tuple[str, str]] = []
    root = logging.getLogger()
    previous_level = root.level
    root.setLevel(logging.INFO)
    handler = tikeo.install_task_log_handler(root)
    try:
        outcome = process_dispatch_task(
            process_task,
            None,
            _task("inst-1", "demo.echo", b"hello"),
            lambda level, message: logs.append((level, message)),
        )
    finally:
        root.removeHandler(handler)
        root.setLevel(previous_level)
        root.setLevel(previous_level)

    assert outcome.success
    assert outcome.message == "python demo echo processed"
    assert any(level == "info" and "[demo.echo]" in message and "hello" in message for level, message in logs)


def test_demo_fail_and_exception_logs_are_bridged_from_standard_logging():
    root = logging.getLogger()
    previous_level = root.level
    root.setLevel(logging.INFO)
    handler = tikeo.install_task_log_handler(root)
    try:
        fail_logs: list[tuple[str, str]] = []
        failure = process_dispatch_task(
            process_task,
            None,
            _task("inst-fail", "demo.fail", b"bad-input"),
            lambda level, message: fail_logs.append((level, message)),
        )
        assert not failure.success
        assert failure.message == "python demo intentional failure"
        assert any(level == "error" and "[demo.fail]" in message and "bad-input" in message for level, message in fail_logs)

        exception_logs: list[tuple[str, str]] = []
        exception = process_dispatch_task(
            process_task,
            None,
            _task("inst-exception", "demo.exception", b"bad-input"),
            lambda level, message: exception_logs.append((level, message)),
        )
        assert not exception.success
        assert "python demo runtime exception" in exception.message
        assert any(level == "error" and "[demo.exception]" in message and "bad-input" in message for level, message in exception_logs)
        assert any(level == "error" and "Traceback" in message and "python demo runtime exception" in message for level, message in exception_logs)
    finally:
        root.removeHandler(handler)
