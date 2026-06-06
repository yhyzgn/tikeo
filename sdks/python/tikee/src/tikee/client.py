"""Worker Tunnel client and session implementation."""

from __future__ import annotations

import queue
import sys
import threading
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any
from urllib.parse import urlparse

import grpc

from .config import WorkerCapabilities, WorkerConfig
from .proto_loader import worker_modules
from .script import ScriptRunnerRegistry, ScriptRunnerTask
from .task import TaskContext, TaskOutcome, TaskProcessor, failed


@dataclass(slots=True)
class Registration:
    client_instance_id: str
    namespace: str
    app: str
    name: str
    region: str
    version: str
    cluster: str
    capabilities: list[str]
    labels: dict[str, str]
    structured: WorkerCapabilities


@dataclass(slots=True)
class Heartbeat:
    worker_id: str
    sequence: int
    generation: int
    fencing_token: str
    sent_at: datetime


class Client:
    """Python Worker client matching the Rust/Go SDK boundary."""

    def __init__(self, config: WorkerConfig) -> None:
        config.validate()
        config.normalize()
        self._config = config
        self._seq = 0
        self._open = False

    def registration(self) -> Registration:
        return Registration(
            client_instance_id=self._config.client_instance_id,
            namespace=self._config.namespace,
            app=self._config.app,
            name=self._config.name,
            region=self._config.region,
            version=self._config.version,
            cluster=self._config.cluster,
            capabilities=list(self._config.capabilities),
            labels=dict(self._config.labels),
            structured=self._config.structured,
        )

    def start_dry_run(self, processor: TaskProcessor) -> None:
        if processor is None:
            raise ValueError("tikee task processor is required")
        self._open = True

    def next_heartbeat(self, worker_id: str, fencing_token: str, generation: int) -> Heartbeat:
        if not self._open:
            raise RuntimeError("tikee worker client is not started")
        if not worker_id:
            raise ValueError("tikee worker id is required")
        self._seq += 1
        return Heartbeat(worker_id, self._seq, generation, fencing_token, datetime.now(timezone.utc))

    def close(self) -> None:
        self._open = False

    def connect_grpc(self) -> grpc.Channel:
        return grpc.insecure_channel(grpc_target(self._config.endpoint))

    def connect(self) -> "Session":
        pb2, pb2_grpc = worker_modules()
        channel = self.connect_grpc()
        stub = pb2_grpc.WorkerTunnelServiceStub(channel)
        outbound: queue.Queue[Any] = queue.Queue()

        def messages():
            while True:
                item = outbound.get()
                if item is None:
                    return
                yield item

        stream = stub.OpenTunnel(messages())
        outbound.put(self._register_message(pb2))
        ack = next(stream)
        registered = getattr(ack, "registered", None)
        if not registered or not registered.worker_id:
            channel.close()
            raise RuntimeError("tikee worker expected registration ack")
        return Session(pb2, channel, stream, outbound, registered.worker_id, registered.lease_seconds, registered.generation, registered.fencing_token, self._config.heartbeat_every.total_seconds())

    def _register_message(self, pb2: Any) -> Any:
        register = pb2.RegisterWorker(
            client_instance_id=self._config.client_instance_id,
            app=self._config.app,
            namespace=self._config.namespace,
            cluster=self._config.cluster,
            region=self._config.region,
            capabilities=list(self._config.capabilities),
            labels=dict(self._config.labels),
            structured_capabilities=_to_proto_capabilities(pb2, self._config.structured),
            election=pb2.WorkerClusterElection(enabled=True, priority=100),
        )
        return pb2.WorkerMessage(register=register)


class Session:
    def __init__(self, pb2: Any, channel: grpc.Channel, stream: Any, outbound: queue.Queue[Any], worker_id: str, lease_seconds: int, generation: int, fencing_token: str, heartbeat_every: float) -> None:
        self._pb2 = pb2
        self._channel = channel
        self._stream = stream
        self._outbound = outbound
        self._worker_id = worker_id
        self._lease_seconds = lease_seconds
        self._generation = generation
        self._fencing_token = fencing_token
        self._heartbeat_every = heartbeat_every
        self._sequence = 0
        self._log_sequence = 0

    @property
    def worker_id(self) -> str:
        return self._worker_id

    @property
    def lease_seconds(self) -> int:
        return self._lease_seconds

    @property
    def generation(self) -> int:
        return self._generation

    def send_heartbeat(self) -> int:
        self._sequence += 1
        self._outbound.put(self._pb2.WorkerMessage(heartbeat=self._pb2.Heartbeat(worker_id=self._worker_id, sequence=self._sequence, generation=self._generation, fencing_token=self._fencing_token)))
        return self._sequence

    def start_heartbeat(self) -> threading.Event:
        stop = threading.Event()

        def loop() -> None:
            self.send_heartbeat()
            while not stop.wait(self._heartbeat_every):
                self.send_heartbeat()

        threading.Thread(target=loop, daemon=True).start()
        return stop

    def emit_task_log(self, instance_id: str, assignment_token: str, level: str, message: str) -> int:
        self._log_sequence += 1
        self._outbound.put(self._pb2.WorkerMessage(task_log=self._pb2.TaskLog(worker_id=self._worker_id, instance_id=instance_id, level=level or "info", message=message, sequence=self._log_sequence, assignment_token=assignment_token)))
        return self._log_sequence

    def process_next(self, processor: TaskProcessor, scripts: ScriptRunnerRegistry | None = None) -> TaskOutcome:
        for message in self._stream:
            task = getattr(message, "dispatch_task", None)
            if not task or not task.instance_id:
                continue
            self._emit_task_log_safely(task, "info", f"received task {task.instance_id} processor={task.processor_name}")
            outcome = process_dispatch_task(processor, scripts, task, lambda level, msg: self._emit_task_log_safely(task, level, msg))
            level = "info" if outcome.success else "error"
            self._emit_task_log_safely(task, level, f"completed task {task.instance_id} success={str(outcome.success).lower()} message={outcome.message}")
            self._outbound.put(self._pb2.WorkerMessage(task_result=self._pb2.TaskResult(worker_id=self._worker_id, instance_id=task.instance_id, success=outcome.success, message=outcome.message, assignment_token=task.assignment_token)))
            return outcome
        raise RuntimeError("worker tunnel closed")

    def close(self) -> None:
        self._outbound.put(self._pb2.WorkerMessage(unregister=self._pb2.UnregisterWorker(worker_id=self._worker_id, generation=self._generation, fencing_token=self._fencing_token)))
        self._outbound.put(None)
        self._channel.close()

    def _emit_task_log_safely(self, task: Any, level: str, message: str) -> None:
        print_task_log_locally(level, message)
        self.emit_task_log(task.instance_id, task.assignment_token, level, message)


def process_dispatch_task(processor: TaskProcessor, scripts: ScriptRunnerRegistry | None, task: Any, log: callable) -> TaskOutcome:
    try:
        binding = getattr(task, "processor_binding", None)
        if binding and binding.HasField("script"):
            script = binding.script
            runner = scripts.get(script.language) if scripts else None
            if runner is None:
                return failed(f"script runner is not registered for language: {script.language}")
            return runner.run(ScriptRunnerTask(
                script_id=script.script_id,
                version_id=script.version_id,
                version_number=script.version_number,
                language=script.language,
                content=bytes(script.content),
                content_sha256=script.content_sha256,
                timeout_ms=script.timeout_ms or 30_000,
                max_output_bytes=script.max_output_bytes or 1024 * 1024,
                allow_network=script.allow_network,
                allowed_env_vars=list(script.allowed_env_vars),
                read_only_paths=list(script.read_only_paths),
                writable_paths=list(script.writable_paths),
                secret_refs=list(script.secret_refs),
                allowed_network_hosts=list(script.allowed_network_hosts),
                sandbox_backend=getattr(script, "sandbox_backend", ""),
                instance_id=task.instance_id,
                job_id=task.job_id,
                log=log,
            ))
        return processor(TaskContext(instance_id=task.instance_id, job_id=task.job_id, processor_name=task.processor_name or task.job_id, payload=bytes(task.payload), log=log))
    except Exception as exc:
        return failed(str(exc))


def print_task_log_locally(level: str, message: str) -> None:
    line = f"[tikee-worker] {message}"
    stream = sys.stderr if level.lower() == "error" else sys.stdout
    print(line, file=stream)


def grpc_target(endpoint: str) -> str:
    value = endpoint.strip()
    if not value:
        raise ValueError("tikee worker endpoint is required")
    parsed = urlparse(value)
    if parsed.scheme in {"http", "https"}:
        if not parsed.netloc:
            raise ValueError("tikee worker endpoint host is required")
        return parsed.netloc
    return value


def _to_proto_capabilities(pb2: Any, capabilities: WorkerCapabilities) -> Any:
    out = pb2.WorkerCapabilities(tags=list(capabilities.tags))
    out.sdk_processors.extend(pb2.SdkProcessorCapability(name=name) for name in capabilities.sdk_processors)
    out.script_runners.extend(pb2.ScriptRunnerCapability(language=r.language, sandbox_backend=r.sandbox_backend) for r in capabilities.script_runners)
    out.plugin_processors.extend(pb2.PluginProcessorCapability(type=p.type, processor_names=list(p.processor_names)) for p in capabilities.plugin_processors)
    return out
