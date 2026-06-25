from __future__ import annotations

import logging
from types import SimpleNamespace

import tikeo
import tikeo_python_worker_demo.__main__ as demo
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



def test_main_handles_ctrl_c_shutdown_without_traceback(monkeypatch, caplog):
    class StopEvent:
        def __init__(self):
            self.was_set = False

        def set(self):
            self.was_set = True

    class FakeSession:
        def __init__(self):
            self.worker_id = "worker-python-test"
            self.generation = 1
            self.lease_seconds = 30
            self.stop = StopEvent()
            self.closed = False

        def start_heartbeat(self):
            return self.stop

        def process_next(self, _processor, _scripts):
            raise KeyboardInterrupt

        def close(self):
            self.closed = True

    class FakeClient:
        last_session = None

        def __init__(self, _config):
            pass

        def registration(self):
            return demo.tikeo.Registration(
                client_instance_id="python-worker-demo-local",
                namespace="dev-alpha",
                app="orders",
                name="python-worker",
                region="local",
                version="dev",
                cluster="local",
                capabilities=[],
                labels={},
                structured=demo.tikeo.WorkerCapabilities(),
            )

        def connect(self):
            self.last_session = FakeSession()
            FakeClient.last_session = self.last_session
            return self.last_session

    monkeypatch.setattr(demo, "configure_scripts", lambda _config: demo.tikeo.ScriptRunnerRegistry())
    monkeypatch.setattr(demo.tikeo, "Client", FakeClient)
    monkeypatch.delenv("TIKEO_WORKER_DRY_RUN", raising=False)
    monkeypatch.delenv("TIKEO_WORKER_CONNECT", raising=False)
    caplog.set_level(logging.INFO)

    demo.main()

    assert FakeClient.last_session is not None
    assert FakeClient.last_session.stop.was_set
    assert FakeClient.last_session.closed
    assert "python worker interrupted, shutting down" in caplog.text


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
