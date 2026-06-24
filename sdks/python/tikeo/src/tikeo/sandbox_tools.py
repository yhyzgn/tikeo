"""Sandbox tool resolver and installer for Python SDK script runners."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
import threading
import tempfile
import time
from dataclasses import dataclass
from contextlib import contextmanager
from pathlib import Path

_BACKGROUND_INSTALLS: set[str] = set()
_BACKGROUND_INSTALLS_LOCK = threading.Lock()


def _env_or(key: str, fallback: str) -> str:
    return os.environ.get(key, fallback) or fallback


@dataclass(slots=True)
class SandboxToolResolver:
    """Resolve and optionally install lightweight sandbox tools."""

    state_dir: str = ""
    auto_install: bool = True
    install_timeout: float = 120.0
    require_managed_tools: bool = False

    def resolve_srt(self) -> tuple[str, bool]:
        return self._resolve_tool("srt", "srt", lambda d: self._run_installer(self._managed_bin(d), "npm", "install", "-g", "--prefix", str(d), _env_or("TIKEO_SRT_NPM_PACKAGE", "@anthropic-ai/sandbox-runtime")))

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
        if self.require_managed_tools:
            path = self._managed_bin(self._install_dir(binary)) / self._executable_name(binary)
            if self._command_works(str(path), "--version") or (binary == "sh" and self._command_works(str(path), "-c", "exit 0")):
                return str(path), True
            return "", False
        path = shutil.which(binary)
        if path and self._command_works(path, "--version"):
            return path, True
        return "", False

    def _resolve_tool(self, key: str, binary: str, installer) -> tuple[str, bool]:
        if not self.require_managed_tools:
            path = shutil.which(binary)
            if path and self._tool_works(binary, path):
                return path, True
        legacy_directory = self._legacy_install_dir(key)
        if legacy_directory is not None:
            legacy_local = self._managed_bin(legacy_directory) / self._executable_name(binary)
            if self._tool_works(binary, str(legacy_local)):
                return str(legacy_local), True
        directory = self._install_dir(key)
        local = self._managed_bin(directory) / self._executable_name(binary)
        if self._tool_works(binary, str(local)):
            return str(local), True
        if not self.auto_install:
            return str(local), False
        self._schedule_background_install(key, binary, directory, installer)
        return str(local), False


    def _schedule_background_install(self, key: str, binary: str, directory: Path, installer) -> None:
        install_key = f"{key}@{directory}"
        with _BACKGROUND_INSTALLS_LOCK:
            if install_key in _BACKGROUND_INSTALLS:
                return
            _BACKGROUND_INSTALLS.add(install_key)

        def run() -> None:
            try:
                installer(directory)
            except Exception as error:
                print(f"[tikeo.sandbox] background auto-install failed tool={binary} error={error}", file=sys.stderr)
                return
            local = self._managed_bin(directory) / self._executable_name(binary)
            if not self._tool_works(binary, str(local)):
                print(
                    f"[tikeo.sandbox] background auto-install completed but tool is still unavailable tool={binary} path={local}",
                    file=sys.stderr,
                )

        threading.Thread(target=run, name=f"tikeo-sandbox-install-{binary}", daemon=True).start()

    def _install_dir(self, key: str) -> Path:
        return _host_sandbox_tools_root() / key

    def _legacy_install_dir(self, key: str) -> Path | None:
        state = self.state_dir.strip()
        return Path(state) / "sandbox-tools" / key if state else None

    @staticmethod
    def _managed_bin(directory: Path) -> Path:
        return directory / "bin"

    @staticmethod
    def _executable_name(binary: str) -> str:
        return f"{binary}.exe" if os.name == "nt" else binary

    def _run_installer(self, managed_bin: Path, *command: str) -> None:
        self._run_installer_with_timeout(self.install_timeout, managed_bin, *command)

    def _run_installer_with_timeout(self, timeout: float, managed_bin: Path, *command: str) -> None:
        managed_bin.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env["PATH"] = os.pathsep.join(_dedupe([str(managed_bin), *env.get("PATH", "").split(os.pathsep)]))
        subprocess.run(command, check=True, timeout=timeout, env=env)

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
        version = _env_or("TIKEO_POWERSHELL_VERSION", "7.5.4")
        archive_name = f"powershell-{version}-{platform_key}.tar.gz"
        url = _env_or("TIKEO_POWERSHELL_DOWNLOAD_URL", f"https://github.com/PowerShell/PowerShell/releases/download/v{version}/{archive_name}")
        bin_dir = self._managed_bin(directory)
        directory.mkdir(parents=True, exist_ok=True)
        with _install_lock(directory):
            link = bin_dir / "pwsh"
            if self._tool_works("pwsh", str(link)):
                return
            tmp_root = Path(tempfile.mkdtemp(prefix=".pwsh-install-", dir=directory))
            try:
                archive = directory / archive_name
                tmp_archive = tmp_root / archive_name
                partial_archive = directory / f"{archive_name}.part"
                tmp_extract_dir = tmp_root / "extract"
                final_extract_dir = directory / f"powershell-{version}"
                bin_dir.mkdir(parents=True, exist_ok=True)
                tmp_extract_dir.mkdir(parents=True, exist_ok=True)
                if archive.is_file():
                    shutil.copy2(archive, tmp_archive)
                else:
                    self._run_installer_with_timeout(
                        _powershell_install_timeout(self.install_timeout),
                        bin_dir,
                        "curl",
                        "-fL",
                        "-C",
                        "-",
                        url,
                        "-o",
                        str(partial_archive),
                    )
                    shutil.copy2(partial_archive, tmp_archive)
                self._run_installer_with_timeout(
                    _powershell_install_timeout(self.install_timeout),
                    bin_dir,
                    "tar",
                    "-xzf",
                    str(tmp_archive),
                    "-C",
                    str(tmp_extract_dir),
                )
                pwsh = tmp_extract_dir / "pwsh"
                if not pwsh.is_file():
                    raise RuntimeError("PowerShell archive did not contain pwsh")
                pwsh.chmod(0o755)
                if final_extract_dir.exists():
                    shutil.rmtree(final_extract_dir)
                tmp_extract_dir.rename(final_extract_dir)
                installed_pwsh = final_extract_dir / "pwsh"
                if link.exists() or link.is_symlink():
                    link.unlink()
                partial_archive.unlink(missing_ok=True)
                try:
                    link.symlink_to(installed_pwsh)
                except OSError:
                    shutil.copy2(installed_pwsh, link)
                    link.chmod(0o755)
            finally:
                shutil.rmtree(tmp_root, ignore_errors=True)

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
            return self._rhai_works(command)
        return self._command_works(command, "--version")

    def _rhai_works(self, command: str) -> bool:
        if os.sep in command and not Path(command).exists():
            return False
        script: Path | None = None
        try:
            with tempfile.NamedTemporaryFile("w", suffix=".rhai", delete=False) as handle:
                handle.write('print("tikeo-rhai-probe");\n')
                script = Path(handle.name)
            subprocess.run([command, str(script)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=2.0, check=True)
            return True
        except Exception:
            return False
        finally:
            if script is not None:
                script.unlink(missing_ok=True)


def _dedupe(values: list[str]) -> list[str]:
    out: list[str] = []
    for value in values:
        if value and value not in out:
            out.append(value)
    return out


@contextmanager
def _install_lock(directory: Path):
    lock_dir = directory / ".install.lock"
    deadline = time.monotonic() + 120.0
    while True:
        try:
            lock_dir.mkdir()
            break
        except FileExistsError:
            if lock_dir.is_file():
                lock_dir.unlink(missing_ok=True)
                continue
            if time.monotonic() >= deadline:
                raise TimeoutError(f"timed out waiting for sandbox tool install lock: {lock_dir}")
            time.sleep(0.1)
    try:
        yield
    finally:
        shutil.rmtree(lock_dir, ignore_errors=True)


def _powershell_install_timeout(timeout: float) -> float:
    configured = os.environ.get("TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS", "").strip()
    if configured:
        try:
            return max(1.0, int(configured) / 1000.0)
        except ValueError:
            pass
    return max(timeout, 1800.0)


def _host_sandbox_tools_root() -> Path:
    configured = os.environ.get("TIKEO_SANDBOX_TOOLS_DIR", "").strip()
    return Path(configured) if configured else Path.home() / ".tikeo" / "sandbox-tools"
