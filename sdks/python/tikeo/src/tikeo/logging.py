"""SDK diagnostic logging for Python Worker clients.

The logging helpers in this module are deliberately separate from task-scoped instance logs.
Use :class:`tikeo.task.TaskContext` for user code output that must appear on a job instance.
Use this module for Worker Tunnel connectivity, registration, heartbeat, sandbox setup, and
management-client diagnostics.

Usage::

    from tikeo import configure_logging, LogConfig

    configure_logging(LogConfig.from_env())

Operational cautions:
    Keep the default INFO level in production. Enable DEBUG only for short troubleshooting
    windows because diagnostics may contain endpoints, worker ids, and processor names. Never log
    API keys, secrets, or raw task payloads through SDK diagnostics.
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass
from pathlib import Path

_LEVELS = {
    "debug": logging.DEBUG,
    "info": logging.INFO,
    "warn": logging.WARNING,
    "warning": logging.WARNING,
    "error": logging.ERROR,
}


@dataclass(slots=True)
class LogConfig:
    """Configuration for SDK diagnostic logs.

    Attributes:
        level: Minimum level name. Supported names are ``debug``, ``info``, ``warning``, and
            ``error``. Unknown values fall back to ``info``.
        log_dir: Optional directory that receives ``tikeo-sdk.log`` in addition to console output.
    """

    level: str = "info"
    log_dir: str | None = None

    @classmethod
    def from_env(cls) -> "LogConfig":
        """Build configuration from ``TIKEO_SDK_LOG_LEVEL`` and ``TIKEO_SDK_LOG_DIR``."""

        level = os.getenv("TIKEO_SDK_LOG_LEVEL", "info")
        log_dir = os.getenv("TIKEO_SDK_LOG_DIR") or None
        return cls(level=level, log_dir=log_dir)


def _level(value: str) -> int:
    return _LEVELS.get(value.strip().lower(), logging.INFO)


_logger = logging.getLogger("tikeo.sdk")
_configured = False


def configure_logging(config: LogConfig | None = None) -> logging.Logger:
    """Configure SDK diagnostics for console and optional file output.

    The returned logger is application-owned after configuration and can be further bridged into a
    larger logging stack. This function does not capture ``stdout``/``stderr`` and therefore cannot
    accidentally attach unrelated process logs to task instances.
    """

    global _configured
    config = config or LogConfig.from_env()
    _logger.handlers.clear()
    _logger.setLevel(_level(config.level))
    _logger.propagate = False

    formatter = logging.Formatter("[tikeo-sdk] %(levelname)s %(message)s")
    console = logging.StreamHandler()
    console.setFormatter(formatter)
    console.setLevel(_level(config.level))
    _logger.addHandler(console)

    if config.log_dir:
        Path(config.log_dir).mkdir(parents=True, exist_ok=True)
        file_handler = logging.FileHandler(Path(config.log_dir) / "tikeo-sdk.log", encoding="utf-8")
        file_handler.setFormatter(formatter)
        file_handler.setLevel(_level(config.level))
        _logger.addHandler(file_handler)

    _configured = True
    return _logger


def sdk_logger() -> logging.Logger:
    """Return the SDK diagnostic logger, configuring it from the environment on first use."""

    if not _configured:
        configure_logging(LogConfig.from_env())
    return _logger


def debug(message: str, *args: object) -> None:
    """Emit a DEBUG-level SDK diagnostic."""

    sdk_logger().debug(message, *args)


def info(message: str, *args: object) -> None:
    """Emit an INFO-level SDK diagnostic."""

    sdk_logger().info(message, *args)


def warning(message: str, *args: object) -> None:
    """Emit a WARNING-level SDK diagnostic."""

    sdk_logger().warning(message, *args)


def error(message: str, *args: object) -> None:
    """Emit an ERROR-level SDK diagnostic."""

    sdk_logger().error(message, *args)
