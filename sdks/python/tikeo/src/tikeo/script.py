"""Script runner registry and sandbox implementations."""

from __future__ import annotations

import hashlib
import json
import os
import shlex
import subprocess
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from .config import WorkerConfig
from .runtime_dirs import ScriptTaskRuntimeDirs, append_allowed_unmanaged_env
from .sandbox_tools import SandboxToolResolver
from .task import TaskOutcome, failed


SUPPORTED_SCRIPT_LANGUAGES = ("shell", "python", "javascript", "typescript", "powershell", "php", "groovy", "rhai")


def normalize_script_language(language: str) -> str:
    match language.strip().lower():
        case "shell" | "sh" | "bash":
            return "shell"
        case "python" | "py":
            return "python"
        case "node" | "nodejs" | "javascript" | "js":
            return "javascript"
        case "typescript" | "ts":
            return "typescript"
        case "powershell" | "pwsh":
            return "powershell"
        case "php":
            return "php"
        case "groovy":
            return "groovy"
        case "rhai":
            return "rhai"
        case other:
            return other.strip().lower()


def default_sandbox_backend(language: str) -> str:
    return "deno" if normalize_script_language(language) in {"javascript", "typescript"} else "srt"


def normalize_script_sandbox_backend(backend: str, language: str) -> str:
    normalized = backend.strip().lower()
    if not normalized or normalized == "auto":
        return default_sandbox_backend(language)
    aliases = {
        "wasm_edge": "wasmedge", "wasm-edge": "wasmedge",
        "anthropic_srt": "srt", "anthropic-srt": "srt", "sandbox_runtime": "srt", "sandbox-runtime": "srt",
        "v8_isolate": "v8", "v8-isolate": "v8",
    }
    normalized = aliases.get(normalized, normalized)
    if normalized not in {"wasmtime", "wasmedge", "srt", "deno", "v8", "docker", "podman", "custom"}:
        raise ValueError(f"unsupported script sandbox backend: {backend}")
    return normalized


def default_script_command(language: str) -> tuple[str, list[str]]:
    match normalize_script_language(language):
        case "shell":
            return "sh", ["-s"]
        case "python":
            return "python3", ["-"]
        case "javascript" | "typescript":
            return "deno", ["run", "--no-prompt", "-"]
        case "powershell":
            return "pwsh", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "-"]
        case "php":
            return "php", []
        case "groovy":
            return "groovy", []
        case "rhai":
            return "rhai", []
        case _:
            return "sh", ["-s"]


@dataclass(slots=True)
class ScriptRunnerTask:
    script_id: str
    version_id: str
    version_number: int
    language: str
    content: bytes
    content_sha256: str
    timeout_ms: int = 30_000
    max_output_bytes: int = 1024 * 1024
    allow_network: bool = False
    allowed_env_vars: list[str] = field(default_factory=list)
    read_only_paths: list[str] = field(default_factory=list)
    writable_paths: list[str] = field(default_factory=list)
    secret_refs: list[str] = field(default_factory=list)
    allowed_network_hosts: list[str] = field(default_factory=list)
    sandbox_backend: str = ""
    instance_id: str = ""
    job_id: str = ""
    log: Callable[[str, str], None] | None = None


def validate_script_task(language: str, task: ScriptRunnerTask) -> None:
    if normalize_script_language(task.language) != language:
        raise ValueError(f"script runner language mismatch: task={task.language} runner={language}")
    if not task.script_id or task.version_number == 0 or not task.content:
        raise ValueError("script runner requires a released immutable script version snapshot")
    if not task.content_sha256:
        raise ValueError("script runner requires a content sha256 digest")
    digest = hashlib.sha256(task.content).hexdigest()
    if digest != task.content_sha256.lower():
        raise ValueError("script content digest mismatch")


def emit_script_command_output(log: Callable[[str, str], None] | None, level: str, output: bytes) -> None:
    if log is None or not output:
        return
    for line in output.decode(errors="replace").replace("\r\n", "\n").split("\n"):
        item = line.strip()
        if item:
            log(level, f"[script] {item}")


def limit_output(message: str, max_bytes: int) -> str:
    if max_bytes <= 0 or len(message.encode()) <= max_bytes:
        return message
    return message.encode()[:max_bytes].decode(errors="ignore")


class ScriptRunner:
    language: str
    sandbox_backend: str

    def run(self, task: ScriptRunnerTask) -> TaskOutcome:  # pragma: no cover - protocol boundary
        raise NotImplementedError

    def advertise_capability(self) -> bool:
        return True


class ScriptRunnerRegistry:
    def __init__(self) -> None:
        self._runners: dict[str, ScriptRunner] = {}

    def register(self, runner: ScriptRunner) -> "ScriptRunnerRegistry":
        if runner is not None:
            language = normalize_script_language(runner.language)
            if language:
                self._runners[language] = runner
        return self

    def get(self, language: str) -> ScriptRunner | None:
        return self._runners.get(normalize_script_language(language))

    def add_capabilities(self, config: WorkerConfig) -> None:
        for language in sorted(self._runners):
            runner = self._runners[language]
            if runner.advertise_capability():
                config.add_script_runner(runner.language, runner.sandbox_backend)


class UnavailableScriptRunner(ScriptRunner):
    def __init__(self, language: str, sandbox_backend: str, reason: str) -> None:
        self.language = normalize_script_language(language)
        try:
            self.sandbox_backend = normalize_script_sandbox_backend(sandbox_backend, self.language)
        except ValueError as exc:
            self.sandbox_backend = default_sandbox_backend(self.language)
            reason = f"{reason}; {exc}".strip("; ")
        self.reason = reason

    def advertise_capability(self) -> bool:
        return False

    def run(self, task: ScriptRunnerTask) -> TaskOutcome:
        try:
            validate_script_task(self.language, task)
        except ValueError as exc:
            return failed(str(exc))
        return failed(f"{self.language} script runner backend is unavailable: {self.reason}")


class LocalCommandScriptRunner(ScriptRunner):
    def __init__(self, language: str, sandbox_backend: str = "custom") -> None:
        self.language = normalize_script_language(language)
        self.sandbox_backend = normalize_script_sandbox_backend(sandbox_backend, self.language)
        if self.sandbox_backend != "custom":
            raise ValueError(f"local command script runner must use custom sandbox backend, got {self.sandbox_backend}")
        self.command, self.args = default_script_command(self.language)

    def run(self, task: ScriptRunnerTask) -> TaskOutcome:
        try:
            validate_script_task(self.language, task)
            if task.allow_network or task.allowed_network_hosts:
                raise ValueError("local script runner rejects network access")
            if task.secret_refs:
                raise ValueError("local script runner rejects secret refs")
            if task.read_only_paths or task.writable_paths:
                raise ValueError("local script runner rejects filesystem grants")
        except ValueError as exc:
            return failed(str(exc))
        return _run_command([self.command, *self.args], task, input_bytes=task.content)


class ContainerScriptRunner(ScriptRunner):
    def __init__(self, language: str, runtime_command: str, image: str, runtime_args: list[str] | None = None) -> None:
        self.language = normalize_script_language(language)
        self.sandbox_backend = normalize_script_sandbox_backend(runtime_command, self.language)
        if self.sandbox_backend not in {"docker", "podman"}:
            raise ValueError(f"container script runner requires docker or podman backend, got {self.sandbox_backend}")
        if not image.strip():
            raise ValueError(f"container script runner requires an image for {self.language}")
        self.runtime_command = self.sandbox_backend
        self.image = image.strip()
        self.runtime_args = list(runtime_args or [])

    def run(self, task: ScriptRunnerTask) -> TaskOutcome:
        try:
            validate_script_task(self.language, task)
            if task.allow_network or task.allowed_network_hosts:
                raise ValueError("container script runner rejects network grants without host-level filtering")
            if task.secret_refs:
                raise ValueError("container script runner rejects secret refs without a worker-local secret provider")
            args = self._container_args(task)
        except ValueError as exc:
            return failed(str(exc))
        return _run_command([self.runtime_command, *args], task, input_bytes=task.content)

    def _container_args(self, task: ScriptRunnerTask) -> list[str]:
        args = ["run", "--rm", "-i", "--network=none", "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=16m", "--memory", "67108864", *self.runtime_args]
        for path in task.read_only_paths:
            args.extend(["--mount", _container_mount(path, True)])
        for path in task.writable_paths:
            args.extend(["--mount", _container_mount(path, False)])
        args.extend(["--env", f"TIKEO_SCRIPT_ID={task.script_id}", "--env", f"TIKEO_SCRIPT_VERSION_ID={task.version_id}", "--env", f"TIKEO_SCRIPT_VERSION_NUMBER={task.version_number}", self.image])
        command, command_args = default_script_command(self.language)
        return [*args, command, *command_args]


class SrtScriptRunner(ScriptRunner):
    def __init__(self, language: str, runtime_command: str, interpreter: str, extra_path: list[str] | None = None) -> None:
        self.language = normalize_script_language(language)
        if not runtime_command.strip() or not interpreter.strip():
            raise ValueError("SRT runner requires runtime and interpreter commands")
        self.runtime_command = runtime_command
        self.interpreter = interpreter
        self.extra_path = list(extra_path or [])
        self.sandbox_backend = "srt"

    def run(self, task: ScriptRunnerTask) -> TaskOutcome:
        try:
            validate_script_task(self.language, task)
            if task.secret_refs:
                raise ValueError("SRT script runner rejects secret refs without a worker-local secret provider")
        except ValueError as exc:
            return failed(str(exc))
        runtime_dirs = ScriptTaskRuntimeDirs.create(f"tikeo-srt-{self.language}-runtime")
        settings_file = None
        try:
            script_file = None
            if self.language == "rhai":
                script_file = runtime_dirs.script_file("rhai")
                script_file.write_bytes(task.content)
            settings_file = _write_srt_settings(task, runtime_dirs, script_file)
            command = [self.runtime_command, "--settings", str(settings_file), "-c", self._shell_command(task.content.decode(errors="replace"), script_file)]
            env = runtime_dirs.srt_environment(self.extra_path)
            if self.language == "powershell":
                env = runtime_dirs.powershell_environment(env)
            _add_script_env(env, task)
            append_allowed_unmanaged_env(env, task.allowed_env_vars)
            outcome = _run_command(command, task, cwd=runtime_dirs.working_dir(), env=env)
            if self.language == "rhai" and outcome.success:
                # rhai-run writes diagnostics to stdout in some versions while exiting 0.
                return outcome
            return outcome
        finally:
            if settings_file:
                Path(settings_file).unlink(missing_ok=True)
            runtime_dirs.cleanup()

    def _shell_command(self, source: str, script_file: Path | None) -> str:
        match self.language:
            case "shell":
                return source
            case "python":
                return _heredoc(f"{self.interpreter} -", "PY", source)
            case "powershell":
                return _heredoc(f"{self.interpreter} -NoLogo -NoProfile -NonInteractive -InputFormat Text -OutputFormat Text -Command -", "PWSH", source)
            case "php" | "groovy":
                return _heredoc(self.interpreter, self.language.upper(), source)
            case "rhai":
                return f"{self.interpreter} {shlex.quote(str(script_file))}" if script_file else _heredoc(self.interpreter, "RHAI", source)
            case _:
                return _heredoc(self.interpreter, "SCRIPT", source)


class DenoScriptRunner(ScriptRunner):
    def __init__(self, language: str, command: str) -> None:
        self.language = normalize_script_language(language)
        if self.language not in {"javascript", "typescript"}:
            raise ValueError("Deno runner supports JavaScript and TypeScript only")
        if not command.strip():
            raise ValueError("Deno runner requires a command")
        self.command = command
        self.sandbox_backend = "deno"

    def run(self, task: ScriptRunnerTask) -> TaskOutcome:
        try:
            validate_script_task(self.language, task)
            if task.secret_refs:
                raise ValueError("Deno script runner rejects secret refs without a worker-local secret provider")
        except ValueError as exc:
            return failed(str(exc))
        runtime_dirs = ScriptTaskRuntimeDirs.create(f"tikeo-deno-{self.language}-runtime")
        try:
            args = [self.command, "run", "--no-prompt"]
            if task.allow_network:
                args.append("--allow-net")
            elif task.allowed_network_hosts:
                args.append("--allow-net=" + ",".join(task.allowed_network_hosts))
            if task.allowed_env_vars:
                args.append("--allow-env=" + ",".join(task.allowed_env_vars))
            if task.read_only_paths:
                args.append("--allow-read=" + ",".join(task.read_only_paths))
            writable = [*task.writable_paths, *runtime_dirs.writable_paths()]
            if writable:
                args.append("--allow-write=" + ",".join(writable))
            args.append("-")
            env = runtime_dirs.deno_environment()
            _add_script_env(env, task)
            append_allowed_unmanaged_env(env, task.allowed_env_vars)
            return _run_command(args, task, input_bytes=task.content, cwd=runtime_dirs.working_dir(), env=env)
        finally:
            runtime_dirs.cleanup()


def _run_command(command: list[str], task: ScriptRunnerTask, input_bytes: bytes | None = None, cwd: Path | None = None, env: dict[str, str] | None = None) -> TaskOutcome:
    try:
        completed = subprocess.run(command, input=input_bytes, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=max(task.timeout_ms, 1) / 1000, check=False)
    except subprocess.TimeoutExpired:
        return failed("script runner timed out")
    except FileNotFoundError as exc:
        return failed(str(exc))
    emit_script_command_output(task.log, "info", completed.stdout)
    emit_script_command_output(task.log, "error", completed.stderr)
    stdout = completed.stdout.decode(errors="replace").strip()
    stderr = completed.stderr.decode(errors="replace").strip()
    message = stdout or stderr
    if completed.returncode != 0:
        return failed(limit_output(message or f"script runner exited with status {completed.returncode}", task.max_output_bytes))
    return TaskOutcome(True, limit_output(message, task.max_output_bytes))


def _write_srt_settings(task: ScriptRunnerTask, runtime_dirs: ScriptTaskRuntimeDirs, script_file: Path | None) -> Path:
    allow_read = list(task.read_only_paths)
    if script_file:
        allow_read.append(str(script_file))
    settings = {
        "network": {"allowUnixSocket": False, "allowedDomains": list(task.allowed_network_hosts), "deniedDomains": []},
        "filesystem": {"allowRead": allow_read, "allowWrite": [*task.writable_paths, *runtime_dirs.writable_paths()], "denyRead": _sensitive_read_denies(), "denyWrite": []},
    }
    fd, name = tempfile.mkstemp(prefix="tikeo-srt-settings-", suffix=".json")
    with os.fdopen(fd, "w", encoding="utf-8") as handle:
        json.dump(settings, handle)
    return Path(name)


def _sensitive_read_denies() -> list[str]:
    home = Path.home()
    return [str(home / path) for path in [".ssh", ".gnupg", ".aws", ".kube", ".docker", os.path.join(".config", "tikeo")]]


def _heredoc(command: str, marker: str, content: str) -> str:
    delimiter = marker
    while delimiter in content:
        delimiter += "_TIKEO"
    return f"{command} <<'{delimiter}'\n{content}\n{delimiter}"


def _container_mount(path: str, read_only: bool) -> str:
    trimmed = path.strip()
    if not trimmed or trimmed != path or not Path(trimmed).is_absolute() or ".." in Path(trimmed).parts:
        raise ValueError(f"script file grant path must be clean and absolute: {path}")
    return f"type=bind,src={trimmed},dst={trimmed}{',readonly' if read_only else ''}"


def _add_script_env(env: dict[str, str], task: ScriptRunnerTask) -> None:
    env["TIKEO_SCRIPT_ID"] = task.script_id
    env["TIKEO_SCRIPT_VERSION_ID"] = task.version_id
    env["TIKEO_SCRIPT_VERSION_NUMBER"] = str(task.version_number)
