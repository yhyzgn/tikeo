"""Worker configuration and structured capability models."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import timedelta


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
class ScriptRunnerCapability:
    """Structured script runner capability advertised by a Worker."""

    language: str
    sandbox_backend: str


@dataclass(slots=True)
class PluginProcessorCapability:
    """Structured plugin processor capability advertised by a Worker."""

    type: str
    processor_names: list[str] = field(default_factory=list)


@dataclass(slots=True)
class WorkerCapabilities:
    """Structured worker capabilities; routing must use these fields."""

    tags: list[str] = field(default_factory=list)
    sdk_processors: list[str] = field(default_factory=list)
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

    def add_sdk_processor(self, name: str) -> None:
        _append_unique(self.structured.sdk_processors, name)

    def add_script_runner(self, language: str, sandbox_backend: str) -> None:
        language = language.strip()
        if not language:
            return
        if any(runner.language == language for runner in self.structured.script_runners):
            return
        self.structured.script_runners.append(ScriptRunnerCapability(language, sandbox_backend.strip()))

    def add_plugin_processor(self, processor_type: str, processor_name: str) -> None:
        processor_type = processor_type.strip()
        processor_name = processor_name.strip()
        if not processor_type or not processor_name:
            return
        for plugin in self.structured.plugin_processors:
            if plugin.type == processor_type:
                _append_unique(plugin.processor_names, processor_name)
                return
        self.structured.plugin_processors.append(PluginProcessorCapability(processor_type, [processor_name]))

    def validate(self) -> None:
        for label, value in {
            "tikee worker endpoint": self.endpoint,
            "tikee client instance id": self.client_instance_id,
            "tikee worker namespace": self.namespace,
            "tikee worker app": self.app,
            "tikee worker name": self.name,
            "tikee worker cluster": self.cluster,
        }.items():
            if not value.strip():
                raise ValueError(f"{label} is required")
        if self.heartbeat_every.total_seconds() <= 0:
            raise ValueError("tikee heartbeat interval must be positive")

    def normalize(self) -> None:
        self.capabilities = _normalized(self.capabilities)
        self.structured.tags = _normalized(self.structured.tags)
        self.structured.sdk_processors = _normalized(self.structured.sdk_processors)
        for plugin in self.structured.plugin_processors:
            plugin.processor_names = _normalized(plugin.processor_names)


def local_config(endpoint: str, client_instance_id: str) -> WorkerConfig:
    """Return a development-friendly worker config."""

    return WorkerConfig(endpoint=endpoint, client_instance_id=client_instance_id)
