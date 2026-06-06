"""Sandbox tool resolver and installer for Python SDK script runners."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


def _env_or(key: str, fallback: str) -> str:
    return os.environ.get(key, fallback) or fallback


@dataclass(slots=True)
class SandboxToolResolver:
    """Resolve and optionally install lightweight sandbox tools."""

    state_dir: str = ""
    auto_install: bool = True
    install_timeout: float = 120.0

    def resolve_srt(self) -> tuple[str, bool]:
        return self._resolve_tool("srt", "srt", lambda d: self._run_installer(self._managed_bin(d), "npm", "install", "-g", "--prefix", str(d), _env_or("TIKEE_SRT_NPM_PACKAGE", "@anthropic-ai/sandbox-runtime")))

    def resolve_ripgrep(self) -> tuple[str, bool]:
        return self._resolve_tool("rg", "rg", lambda d: self._run_installer(self._managed_bin(d), "cargo", "install", "--root", str(d), "ripgrep"))

    def resolve_deno(self) -> tuple[str, bool]:
        def install(directory: Path) -> None:
            if os.name == "nt":
                self._run_installer(self._managed_bin(directory), "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "irm https://deno.land/install.ps1 | iex")
            else:
                self._run_installer(self._managed_bin(directory), "sh", "-c", f"curl -fsSL https://deno.land/install.sh | DENO_INSTALL='{directory}' sh")
        return self._resolve_tool("deno", "deno", install)

    def resolve_rhai(self) -> tuple[str, bool]:
        return self._resolve_tool("rhai-run", "rhai-run", lambda d: self._run_installer(self._managed_bin(d), "cargo", "install", "--root", str(d), "rhai", "--bins", "--features", "bin-features"))

    def resolve_powershell(self) -> tuple[str, bool]:
        return self._resolve_tool("pwsh", "pwsh", self._install_powershell)

    def resolve_node(self) -> tuple[str, bool]:
        return self.resolve_interpreter("node")

    def resolve_npm(self) -> tuple[str, bool]:
        return self.resolve_interpreter("npm")

    def resolve_interpreter(self, binary: str) -> tuple[str, bool]:
        path = shutil.which(binary)
        if path and self._command_works(path, "--version"):
            return path, True
        return "", False

    def _resolve_tool(self, key: str, binary: str, installer) -> tuple[str, bool]:
        path = shutil.which(binary)
        if path and self._tool_works(binary, path):
            return path, True
        directory = self._install_dir(key)
        local = self._managed_bin(directory) / self._executable_name(binary)
        if self._tool_works(binary, str(local)):
            return str(local), True
        if not self.auto_install:
            return str(local), False
        try:
            installer(directory)
        except Exception:
            return str(local), False
        return str(local), self._tool_works(binary, str(local))

    def _install_dir(self, key: str) -> Path:
        base = Path(self.state_dir.strip()) if self.state_dir.strip() else Path.home() / ".tikee"
        return base / "sandbox-tools" / key

    @staticmethod
    def _managed_bin(directory: Path) -> Path:
        return directory / "bin"

    @staticmethod
    def _executable_name(binary: str) -> str:
        return f"{binary}.exe" if os.name == "nt" else binary

    def _run_installer(self, managed_bin: Path, *command: str) -> None:
        managed_bin.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env["PATH"] = os.pathsep.join(_dedupe([str(managed_bin), *env.get("PATH", "").split(os.pathsep)]))
        subprocess.run(command, check=True, timeout=self.install_timeout, env=env)

    def _install_powershell(self, directory: Path) -> None:
        if os.name == "nt":
            self._run_installer(self._managed_bin(directory), "winget", "install", "-e", "--id", "Microsoft.PowerShell")
            return
        machine = platform.machine().lower()
        system = platform.system().lower()
        platform_key = {
            ("linux", "x86_64"): "linux-x64",
            ("linux", "amd64"): "linux-x64",
            ("linux", "aarch64"): "linux-arm64",
            ("darwin", "x86_64"): "osx-x64",
            ("darwin", "arm64"): "osx-arm64",
        }.get((system, machine))
        if not platform_key:
            raise RuntimeError(f"PowerShell auto-install does not support {system}/{machine}")
        version = _env_or("TIKEE_POWERSHELL_VERSION", "7.5.4")
        archive_name = f"powershell-{version}-{platform_key}.tar.gz"
        url = _env_or("TIKEE_POWERSHELL_DOWNLOAD_URL", f"https://github.com/PowerShell/PowerShell/releases/download/v{version}/{archive_name}")
        bin_dir = self._managed_bin(directory)
        extract_dir = directory / f"powershell-{version}"
        bin_dir.mkdir(parents=True, exist_ok=True)
        extract_dir.mkdir(parents=True, exist_ok=True)
        archive = directory / archive_name
        self._run_installer(bin_dir, "curl", "-fsSL", url, "-o", str(archive))
        self._run_installer(bin_dir, "tar", "-xzf", str(archive), "-C", str(extract_dir))
        pwsh = extract_dir / "pwsh"
        pwsh.chmod(0o755)
        link = bin_dir / "pwsh"
        if link.exists() or link.is_symlink():
            link.unlink()
        try:
            link.symlink_to(pwsh)
        except OSError:
            shutil.copy2(pwsh, link)
            link.chmod(0o755)

    def _command_works(self, command: str, *args: str) -> bool:
        if os.sep in command and not Path(command).exists():
            return False
        try:
            subprocess.run([command, *args], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=2.0, check=True)
            return True
        except Exception:
            return False

    def _tool_works(self, binary: str, command: str) -> bool:
        if binary == "srt":
            return self._command_works(command, "--version") or self._command_works(command, "--help")
        if binary == "rhai-run":
            return self._command_works(command, "--help") or self._command_works(command, "--version")
        return self._command_works(command, "--version")


def _dedupe(values: list[str]) -> list[str]:
    out: list[str] = []
    for value in values:
        if value and value not in out:
            out.append(value)
    return out
