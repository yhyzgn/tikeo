"""Task processing models."""

from __future__ import annotations

from collections.abc import Callable
from typing import Protocol
from dataclasses import dataclass, field


@dataclass(slots=True)
class TaskContext:
    """Ergonomic task shape passed to Python processors."""

    instance_id: str
    job_id: str
    processor_name: str
    payload: bytes = b""
    log: Callable[[str, str], None] | None = None

    def log_info(self, message: str) -> None:
        self._log("info", message)

    def log_error(self, message: str) -> None:
        self._log("error", message)

    def _log(self, level: str, message: str) -> None:
        if self.log is not None:
            self.log(level, message)


@dataclass(slots=True)
class TaskOutcome:
    """Worker result reported to tikeo."""

    success: bool
    message: str = ""


class TaskProcessor(Protocol):
    """Callable task processor protocol."""

    def __call__(self, task: TaskContext) -> TaskOutcome: ...


def succeeded(message: str = "") -> TaskOutcome:
    return TaskOutcome(True, message)


def failed(message: str) -> TaskOutcome:
    return TaskOutcome(False, message)


@dataclass(slots=True)
class CapturedTaskLog:
    level: str
    message: str


@dataclass(slots=True)
class CapturedTaskLogs:
    entries: list[CapturedTaskLog] = field(default_factory=list)

    def add(self, level: str, message: str) -> None:
        self.entries.append(CapturedTaskLog(level, message))
