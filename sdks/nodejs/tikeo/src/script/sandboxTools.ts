import { existsSync, mkdirSync, rmSync, chmodSync, symlinkSync, copyFileSync, writeFileSync, mkdtempSync, renameSync } from "node:fs";
import { homedir, platform, arch, tmpdir } from "node:os";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";

const backgroundInstalls = new Set<string>();

export class SandboxToolResolver {
  constructor(public stateDir = "", public autoInstall = true, public installTimeoutMs = 120_000, public requireManagedTools = false) {}

  resolveSrt(): [string, boolean] {
    return this.resolveTool("srt", "srt", (dir) => this.runInstaller(this.managedBin(dir), "npm", ["install", "-g", "--prefix", dir, process.env.TIKEO_SRT_NPM_PACKAGE || "@anthropic-ai/sandbox-runtime"]));
  }
  resolveRipgrep(): [string, boolean] { return this.resolveTool("rg", "rg", (dir) => this.runInstaller(this.managedBin(dir), "cargo", ["install", "--root", dir, "ripgrep"])); }
  resolveDeno(): [string, boolean] {
    return this.resolveTool("deno", "deno", (dir) => {
      if (process.platform === "win32") return this.runInstaller(this.managedBin(dir), "powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "irm https://deno.land/install.ps1 | iex"]);
      return this.runInstaller(this.managedBin(dir), "sh", ["-c", `curl -fsSL https://deno.land/install.sh | DENO_INSTALL='${dir}' sh`]);
    });
  }
  resolveRhai(): [string, boolean] { return this.resolveTool("rhai-run", "rhai-run", (dir) => this.runInstaller(this.managedBin(dir), "cargo", ["install", "--root", dir, "rhai", "--bins", "--features", "bin-features"])); }
  resolvePowerShell(): [string, boolean] { return this.resolveTool("pwsh", "pwsh", (dir) => this.installPowerShell(dir)); }
  resolveNode(): [string, boolean] { return this.resolveInterpreter("node"); }
  resolveNpm(): [string, boolean] { return this.resolveInterpreter("npm"); }

  resolveInterpreter(binary: string): [string, boolean] {
    if (this.requireManagedTools) {
      const path = join(this.managedBin(this.installDir(binary)), executableName(binary));
      return this.commandWorks(path, ["--version"]) || (binary === "sh" && this.commandWorks(path, ["-c", "exit 0"])) ? [path, true] : ["", false];
    }
    const path = findOnPath(binary);
    return path && this.commandWorks(path, ["--version"]) ? [path, true] : ["", false];
  }

  private resolveTool(key: string, binary: string, installer: (dir: string) => void | Promise<void>): [string, boolean] {
    if (!this.requireManagedTools) {
      const found = findOnPath(binary);
      if (found && this.toolWorks(binary, found)) return [found, true];
    }
    const legacyDir = this.legacyInstallDir(key);
    if (legacyDir) {
      const legacyLocal = join(this.managedBin(legacyDir), executableName(binary));
      if (this.toolWorks(binary, legacyLocal)) return [legacyLocal, true];
    }
    const dir = this.installDir(key);
    const local = join(this.managedBin(dir), executableName(binary));
    if (this.toolWorks(binary, local)) return [local, true];
    if (!this.autoInstall) return [local, false];
    this.scheduleBackgroundInstall(key, binary, dir, installer);
    return [local, false];
  }


  private scheduleBackgroundInstall(key: string, binary: string, dir: string, installer: (dir: string) => void | Promise<void>): void {
    const installKey = `${key}@${dir}`;
    if (backgroundInstalls.has(installKey)) return;
    backgroundInstalls.add(installKey);
    setImmediate(async () => {
      try {
        await installer(dir);
      } catch (error) {
        console.warn(`[tikeo.sandbox] background auto-install failed tool=${binary} error=${error instanceof Error ? error.message : String(error)}`);
        return;
      }
      const local = join(this.managedBin(dir), executableName(binary));
      if (!this.toolWorks(binary, local)) console.warn(`[tikeo.sandbox] background auto-install completed but tool is still unavailable tool=${binary} path=${local}`);
    });
  }

  private installDir(key: string): string { return join(hostSandboxToolsRoot(), key); }
  private legacyInstallDir(key: string): string { return this.stateDir.trim() ? join(this.stateDir.trim(), "sandbox-tools", key) : ""; }
  private managedBin(dir: string): string { return join(dir, "bin"); }

  private runInstaller(managedBin: string, command: string, args: string[]): Promise<void> {
    return this.runInstallerWithTimeout(this.installTimeoutMs, managedBin, command, args);
  }

  private runInstallerWithTimeout(timeoutMs: number, managedBin: string, command: string, args: string[]): Promise<void> {
    mkdirSync(managedBin, { recursive: true });
    const env = { ...process.env, PATH: dedupe([managedBin, ...(process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":")]).join(process.platform === "win32" ? ";" : ":") };
    return new Promise((resolve, reject) => {
      const child = spawn(command, args, { stdio: "inherit", env });
      const timer = setTimeout(() => {
        child.kill("SIGKILL");
        reject(new Error(`installer timed out: ${command}`));
      }, timeoutMs);
      child.once("error", (error) => {
        clearTimeout(timer);
        reject(error);
      });
      child.once("exit", (code) => {
        clearTimeout(timer);
        if (code === 0) resolve();
        else reject(new Error(`installer failed: ${command} exit=${code ?? "signal"}`));
      });
    });
  }

  private async installPowerShell(dir: string): Promise<void> {
    if (process.platform === "win32") { await this.runInstaller(this.managedBin(dir), "winget", ["install", "-e", "--id", "Microsoft.PowerShell"]); return; }
    const key = powerShellArchivePlatform();
    if (!key) throw new Error(`PowerShell auto-install does not support ${platform()}/${arch()}`);
    const version = process.env.TIKEO_POWERSHELL_VERSION || "7.5.4";
    const name = `powershell-${version}-${key}.tar.gz`;
    const url = process.env.TIKEO_POWERSHELL_DOWNLOAD_URL || `https://github.com/PowerShell/PowerShell/releases/download/v${version}/${name}`;
    const bin = this.managedBin(dir);
    mkdirSync(dir, { recursive: true });
    const releaseLock = acquireInstallLock(dir);
    try {
      const link = join(bin, "pwsh");
      if (this.toolWorks("pwsh", link)) return;
      const tmp = mkdtempSync(join(dir, ".pwsh-install-"));
      try {
        const archive = join(dir, name);
        const partialArchive = join(dir, `${name}.part`);
        const tmpArchive = join(tmp, name);
        const tmpExtract = join(tmp, "extract");
        const finalExtract = join(dir, `powershell-${version}`);
        mkdirSync(bin, { recursive: true }); mkdirSync(tmpExtract, { recursive: true });
        if (existsSync(archive)) {
          copyFileSync(archive, tmpArchive);
        } else {
          await this.runInstallerWithTimeout(powerShellInstallTimeout(this.installTimeoutMs), bin, "curl", ["-fL", "-C", "-", url, "-o", partialArchive]);
          copyFileSync(partialArchive, tmpArchive);
        }
        await this.runInstallerWithTimeout(powerShellInstallTimeout(this.installTimeoutMs), bin, "tar", ["-xzf", tmpArchive, "-C", tmpExtract]);
        const pwsh = join(tmpExtract, "pwsh");
        if (!existsSync(pwsh)) throw new Error("PowerShell archive did not contain pwsh");
        chmodSync(pwsh, 0o755);
        rmSync(finalExtract, { force: true, recursive: true });
        renameSync(tmpExtract, finalExtract);
        const installedPwsh = join(finalExtract, "pwsh");
        rmSync(link, { force: true });
        rmSync(partialArchive, { force: true });
        try { symlinkSync(installedPwsh, link); } catch { copyFileSync(installedPwsh, link); chmodSync(link, 0o755); }
      } finally {
        rmSync(tmp, { force: true, recursive: true });
      }
    } finally {
      releaseLock();
    }
  }

  private commandWorks(command: string, args: string[]): boolean {
    if ((command.includes("/") || command.includes("\\")) && !existsSync(command)) return false;
    const result = spawnSync(command, args, { stdio: "ignore", timeout: 2_000 });
    return result.status === 0;
  }

  private toolWorks(binary: string, command: string): boolean {
    if (binary === "srt") return this.commandWorks(command, ["--version"]) || this.commandWorks(command, ["--help"]);
    if (binary === "rhai-run") return this.rhaiWorks(command);
    return this.commandWorks(command, ["--version"]);
  }

  private rhaiWorks(command: string): boolean {
    if ((command.includes("/") || command.includes("\\")) && !existsSync(command)) return false;
    const dir = mkdtempSync(join(tmpdir(), "tikeo-rhai-probe-"));
    const script = join(dir, "probe.rhai");
    try {
      writeFileSync(script, 'print("tikeo-rhai-probe");\n');
      return this.commandWorks(command, [script]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }
}

function hostSandboxToolsRoot(): string {
  return (process.env.TIKEO_SANDBOX_TOOLS_DIR || "").trim() || join(homedir(), ".tikeo", "sandbox-tools");
}
function acquireInstallLock(dir: string): () => void {
  const lockDir = join(dir, ".install.lock");
  const deadline = Date.now() + 120_000;
  while (true) {
    try {
      mkdirSync(lockDir);
      return () => rmSync(lockDir, { force: true, recursive: true });
    } catch {
      if (Date.now() >= deadline) throw new Error(`timed out waiting for sandbox tool install lock: ${lockDir}`);
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 100);
    }
  }
}

function powerShellInstallTimeout(timeoutMs: number): number {
  const configured = (process.env.TIKEO_POWERSHELL_INSTALL_TIMEOUT_MILLIS || "").trim();
  if (configured) {
    const parsed = Number.parseInt(configured, 10);
    if (Number.isFinite(parsed) && parsed > 0) return Math.max(1_000, parsed);
  }
  return Math.max(timeoutMs, 1_800_000);
}

function executableName(binary: string): string { return process.platform === "win32" ? `${binary}.exe` : binary; }
function findOnPath(binary: string): string | undefined {
  const paths = (process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":");
  for (const p of paths) {
    const full = join(p, executableName(binary));
    if (existsSync(full)) return full;
  }
  return undefined;
}
function powerShellArchivePlatform(): string {
  const key = `${platform()}/${arch()}`;
  return ({ "linux/x64": "linux-x64", "linux/arm64": "linux-arm64", "darwin/x64": "osx-x64", "darwin/arm64": "osx-arm64" } as Record<string, string>)[key] ?? "";
}
function dedupe(values: string[]): string[] { return [...new Set(values.filter(Boolean))]; }
