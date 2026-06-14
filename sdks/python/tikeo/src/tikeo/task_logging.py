"""Task-scoped logging bridge for Python processors.

This module bridges Python's standard :mod:`logging` framework into the currently executing Tikeo
job instance. It uses ``contextvars`` so unrelated process logs are not attached to a task. Install
``TikeoTaskLogHandler`` on the logger(s) that should be mirrored into instance logs, then keep using
normal ``logging.getLogger(__name__)`` calls in business code.
"""

from __future__ import annotations

import contextvars
import logging
from contextlib import contextmanager
from dataclasses import dataclass
from typing import Callable, Iterator

TaskLogSink = Callable[[str, str], None]


@dataclass(slots=True)
class TaskLogScope:
    instance_id: str
    job_id: str
    processor_name: str
    log: TaskLogSink


_current_scope: contextvars.ContextVar[TaskLogScope | None] = contextvars.ContextVar("tikeo_task_log_scope", default=None)


def current_task_log_scope() -> TaskLogScope | None:
    return _current_scope.get()


@contextmanager
def task_log_scope(scope: TaskLogScope) -> Iterator[None]:
    token = _current_scope.set(scope)
    try:
        yield
    finally:
        _current_scope.reset(token)


def emit_current_task_log(level: str, message: str) -> bool:
    scope = current_task_log_scope()
    if scope is None:
        return False
    scope.log(level or "info", message)
    return True


class TikeoTaskLogHandler(logging.Handler):
    """Logging handler that mirrors records into the active Tikeo task scope."""

    def __init__(self, level: int = logging.NOTSET) -> None:
        super().__init__(level=level)
        self.setFormatter(logging.Formatter("%(message)s"))

    def emit(self, record: logging.LogRecord) -> None:
        scope = current_task_log_scope()
        if scope is None:
            return
        try:
            message = self.format(record)
        except Exception:  # pragma: no cover - logging defensive path
            message = record.getMessage()
        scope.log(_level_name(record.levelno), message)


def _level_name(levelno: int) -> str:
    if levelno >= logging.ERROR:
        return "error"
    if levelno >= logging.WARNING:
        return "warning"
    if levelno <= logging.DEBUG:
        return "debug"
    return "info"


def install_task_log_handler(logger: logging.Logger | None = None, level: int = logging.INFO) -> TikeoTaskLogHandler:
    """Install and return a task-log handler on ``logger`` (root logger by default)."""

    target = logger if logger is not None else logging.getLogger()
    for handler in target.handlers:
        if isinstance(handler, TikeoTaskLogHandler):
            return handler
    handler = TikeoTaskLogHandler(level=level)
    target.addHandler(handler)
    return handler
