"""Single source of truth for task-scoped script runtime directories."""

from __future__ import annotations

import os
import shutil
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

_SEQUENCE = 0


@dataclass(slots=True)
class ScriptTaskRuntimeDirs:
    root: Path
    home: Path
    config: Path
    cache: Path
    data: Path
    modules: Path
    dotnet_home: Path
    powershell_cache: Path
    tmp: Path
    deno_dir: Path

    @classmethod
    def create(cls, prefix: str) -> "ScriptTaskRuntimeDirs":
        root = Path(tempfile.mkdtemp(prefix=f"{prefix.strip()}-"))
        data = root / "data"
        cache = root / "cache"
        dirs = cls(
            root=root,
            home=root / "home",
            config=root / "config",
            cache=cache,
            data=data,
            modules=data / "powershell" / "Modules",
            dotnet_home=root / "dotnet",
            powershell_cache=cache / "powershell",
            tmp=root / "tmp",
            deno_dir=cache / "deno",
        )
        for directory in dirs.required_directories():
            directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        return dirs

    def required_directories(self) -> list[Path]:
        return [self.root, self.home, self.config, self.cache, self.data, self.modules, self.dotnet_home, self.powershell_cache, self.tmp, self.deno_dir]

    def writable_paths(self) -> list[str]:
        return [str(path) for path in self.required_directories()]

    def working_dir(self) -> Path:
        return self.home

    def script_file(self, extension: str) -> Path:
        global _SEQUENCE
        _SEQUENCE += 1
        return self.home / f"script-{int(time.time() * 1000)}-{_SEQUENCE}.{extension}"

    def base_environment(self, extra_path: list[str] | None = None) -> dict[str, str]:
        path_parts = [part for part in (extra_path or []) if part]
        if os.environ.get("PATH"):
            path_parts.extend(os.environ["PATH"].split(os.pathsep))
        env = {
            "HOME": str(self.home),
            "XDG_CONFIG_HOME": str(self.config),
            "XDG_CACHE_HOME": str(self.cache),
            "XDG_DATA_HOME": str(self.data),
            "TMPDIR": str(self.tmp),
            "TERM": "dumb",
            "NO_COLOR": "1",
        }
        if path_parts:
            env["PATH"] = os.pathsep.join(_dedupe(path_parts))
        return env

    def srt_environment(self, extra_path: list[str] | None = None) -> dict[str, str]:
        env = self.base_environment(extra_path)
        env["CLAUDE_CODE_TMPDIR"] = str(self.tmp)
        env["CLAUDE_TMPDIR"] = str(self.tmp)
        return env

    def powershell_environment(self, env: dict[str, str]) -> dict[str, str]:
        env.update({
            "PSModulePath": str(self.modules),
            "DOTNET_CLI_HOME": str(self.dotnet_home),
            "POWERSHELL_TELEMETRY_OPTOUT": "1",
            "POWERSHELL_UPDATECHECK": "Off",
        })
        return env

    def deno_environment(self) -> dict[str, str]:
        env = self.base_environment()
        env["DENO_DIR"] = str(self.deno_dir)
        return env

    def cleanup(self) -> None:
        shutil.rmtree(self.root, ignore_errors=True)


def append_allowed_unmanaged_env(env: dict[str, str], allowed: list[str]) -> dict[str, str]:
    managed = {
        "HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME", "TMPDIR", "TERM", "NO_COLOR",
        "CLAUDE_CODE_TMPDIR", "CLAUDE_TMPDIR", "PSModulePath", "DOTNET_CLI_HOME",
        "POWERSHELL_TELEMETRY_OPTOUT", "POWERSHELL_UPDATECHECK", "DENO_DIR",
    }
    for name in allowed:
        key = name.strip()
        if key and key not in managed and key in os.environ:
            env[key] = os.environ[key]
    return env


def _dedupe(values: list[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for value in values:
        if value and value not in seen:
            seen.add(value)
            out.append(value)
    return out
