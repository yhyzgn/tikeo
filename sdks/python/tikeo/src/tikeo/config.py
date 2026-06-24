"""Worker configuration and structured capability models."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import timedelta
from enum import StrEnum


def _append_unique(values: list[str], value: str) -> None:
    item = value.strip()
    if item and item not in values:
        values.append(item)


def _normalized(values: list[str]) -> list[str]:
    out: list[str] = []
    for value in values:
        _append_unique(out, value)
    return out


@dataclass(slots=True)
class ProcessorCapability:
    """Structured processor capability with optional display metadata."""

    name: str
    description: str = ""


class PluginType(StrEnum):
    """Constrained plugin processor type values."""

    SQL = "sql"
    HTTP = "http"
    NOTIFICATION = "notification"
    CUSTOM = "custom"


@dataclass(slots=True)
class ScriptRunnerCapability:
    """Structured script runner capability advertised by a Worker."""

    language: str
    sandbox_backend: str


@dataclass(slots=True)
class PluginProcessorCapability:
    """Structured plugin processor capability advertised by a Worker."""

    type: PluginType
    processors: list[ProcessorCapability] = field(default_factory=list)
    processor_names: list[str] = field(default_factory=list)


@dataclass(slots=True)
class WorkerCapabilities:
    """Structured worker capabilities; routing must use these fields."""

    tags: list[str] = field(default_factory=list)
    normal_processors: list[ProcessorCapability] = field(default_factory=list)
    script_runners: list[ScriptRunnerCapability] = field(default_factory=list)
    plugin_processors: list[PluginProcessorCapability] = field(default_factory=list)


@dataclass(slots=True)
class WorkerConfig:
    """One outbound Worker Tunnel client instance configuration."""

    endpoint: str
    client_instance_id: str
    namespace: str = "default"
    app: str = "default"
    name: str = ""
    region: str = "local"
    version: str = "dev"
    cluster: str = "local"
    capabilities: list[str] = field(default_factory=list)
    labels: dict[str, str] = field(default_factory=dict)
    structured: WorkerCapabilities = field(default_factory=WorkerCapabilities)
    heartbeat_every: timedelta = timedelta(seconds=10)

    def __post_init__(self) -> None:
        if not self.name:
            self.name = self.client_instance_id

    def add_tag(self, tag: str) -> None:
        _append_unique(self.structured.tags, tag)

    def add_normal_processor(self, name: str, description: str = "") -> None:
        _append_unique_processor(self.structured.normal_processors, ProcessorCapability(name, description))

    def add_script_runner(self, language: str, sandbox_backend: str) -> None:
        language = language.strip()
        if not language:
            return
        if any(runner.language == language for runner in self.structured.script_runners):
            return
        self.structured.script_runners.append(ScriptRunnerCapability(language, sandbox_backend.strip()))

    def add_plugin_processor(self, processor_type: PluginType, processor_name: str, description: str = "") -> None:
        if not isinstance(processor_type, PluginType):
            raise ValueError(f"unsupported tikeo plugin processor type: {processor_type}")
        processor = _clean_processor(ProcessorCapability(processor_name, description))
        if processor is None:
            return
        for plugin in self.structured.plugin_processors:
            if str(plugin.type) == processor_type:
                _append_unique_processor(plugin.processors, processor)
                _append_unique(plugin.processor_names, processor.name)
                return
        self.structured.plugin_processors.append(PluginProcessorCapability(processor_type, [processor], [processor.name]))

    def validate(self) -> None:
        for label, value in {
            "tikeo worker endpoint": self.endpoint,
            "tikeo client instance id": self.client_instance_id,
            "tikeo worker namespace": self.namespace,
            "tikeo worker app": self.app,
            "tikeo worker name": self.name,
            "tikeo worker cluster": self.cluster,
        }.items():
            if not value.strip():
                raise ValueError(f"{label} is required")
        if self.heartbeat_every.total_seconds() <= 0:
            raise ValueError("tikeo heartbeat interval must be positive")

    def normalize(self) -> None:
        self.capabilities = _normalized(self.capabilities)
        self.structured.tags = _normalized(self.structured.tags)
        self.structured.normal_processors = _normalized_processors(self.structured.normal_processors)
        normalized_plugins: list[PluginProcessorCapability] = []
        for plugin in self.structured.plugin_processors:
            if not isinstance(plugin.type, PluginType):
                continue
            plugin.processor_names = _normalized(plugin.processor_names)
            plugin.processors = _normalized_processors(plugin.processors)
            for name in plugin.processor_names:
                _append_unique_processor(plugin.processors, ProcessorCapability(name))
            plugin.processor_names = [processor.name for processor in plugin.processors]
            normalized_plugins.append(plugin)
        self.structured.plugin_processors = normalized_plugins


def _clean_processor(value: ProcessorCapability) -> ProcessorCapability | None:
    name = value.name.strip()
    if not name:
        return None
    return ProcessorCapability(name, value.description.strip())


def _append_unique_processor(values: list[ProcessorCapability], value: ProcessorCapability) -> None:
    item = _clean_processor(value)
    if item is None:
        return
    for existing in values:
        if existing.name == item.name:
            if not existing.description and item.description:
                existing.description = item.description
            return
    values.append(item)


def _normalized_processors(values: list[ProcessorCapability]) -> list[ProcessorCapability]:
    out: list[ProcessorCapability] = []
    for value in values:
        _append_unique_processor(out, value)
    return out


def local_config(endpoint: str, client_instance_id: str) -> WorkerConfig:
    """Return a development-friendly worker config."""

    return WorkerConfig(endpoint=endpoint, client_instance_id=client_instance_id)
