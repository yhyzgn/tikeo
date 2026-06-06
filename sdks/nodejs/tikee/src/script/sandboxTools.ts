import { existsSync, mkdirSync, rmSync, chmodSync, symlinkSync, copyFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { homedir, platform, arch, tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

export class SandboxToolResolver {
  constructor(public stateDir = "", public autoInstall = true, public installTimeoutMs = 120_000) {}

  resolveSrt(): [string, boolean] {
    return this.resolveTool("srt", "srt", (dir) => this.runInstaller(this.managedBin(dir), "npm", ["install", "-g", "--prefix", dir, process.env.TIKEE_SRT_NPM_PACKAGE || "@anthropic-ai/sandbox-runtime"]));
  }
  resolveRipgrep(): [string, boolean] { return this.resolveTool("rg", "rg", (dir) => this.runInstaller(this.managedBin(dir), "cargo", ["install", "--root", dir, "ripgrep"])); }
  resolveDeno(): [string, boolean] {
    return this.resolveTool("deno", "deno", (dir) => {
      if (process.platform === "win32") this.runInstaller(this.managedBin(dir), "powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "irm https://deno.land/install.ps1 | iex"]);
      else this.runInstaller(this.managedBin(dir), "sh", ["-c", `curl -fsSL https://deno.land/install.sh | DENO_INSTALL='${dir}' sh`]);
    });
  }
  resolveRhai(): [string, boolean] { return this.resolveTool("rhai-run", "rhai-run", (dir) => this.runInstaller(this.managedBin(dir), "cargo", ["install", "--root", dir, "rhai", "--bins", "--features", "bin-features"])); }
  resolvePowerShell(): [string, boolean] { return this.resolveTool("pwsh", "pwsh", (dir) => this.installPowerShell(dir)); }
  resolveNode(): [string, boolean] { return this.resolveInterpreter("node"); }
  resolveNpm(): [string, boolean] { return this.resolveInterpreter("npm"); }

  resolveInterpreter(binary: string): [string, boolean] {
    const path = findOnPath(binary);
    return path && this.commandWorks(path, ["--version"]) ? [path, true] : ["", false];
  }

  private resolveTool(key: string, binary: string, installer: (dir: string) => void): [string, boolean] {
    const found = findOnPath(binary);
    if (found && this.toolWorks(binary, found)) return [found, true];
    const dir = this.installDir(key);
    const local = join(this.managedBin(dir), executableName(binary));
    if (this.toolWorks(binary, local)) return [local, true];
    if (!this.autoInstall) return [local, false];
    try { installer(dir); } catch { return [local, false]; }
    return [local, this.toolWorks(binary, local)];
  }

  private installDir(key: string): string { return join(this.stateDir.trim() || join(homedir(), ".tikee"), "sandbox-tools", key); }
  private managedBin(dir: string): string { return join(dir, "bin"); }

  private runInstaller(managedBin: string, command: string, args: string[]): void {
    mkdirSync(managedBin, { recursive: true });
    const env = { ...process.env, PATH: dedupe([managedBin, ...(process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":")]).join(process.platform === "win32" ? ";" : ":") };
    const result = spawnSync(command, args, { stdio: "inherit", env, timeout: this.installTimeoutMs });
    if (result.status !== 0) throw new Error(`installer failed: ${command}`);
  }

  private installPowerShell(dir: string): void {
    if (process.platform === "win32") { this.runInstaller(this.managedBin(dir), "winget", ["install", "-e", "--id", "Microsoft.PowerShell"]); return; }
    const key = powerShellArchivePlatform();
    if (!key) throw new Error(`PowerShell auto-install does not support ${platform()}/${arch()}`);
    const version = process.env.TIKEE_POWERSHELL_VERSION || "7.5.4";
    const name = `powershell-${version}-${key}.tar.gz`;
    const url = process.env.TIKEE_POWERSHELL_DOWNLOAD_URL || `https://github.com/PowerShell/PowerShell/releases/download/v${version}/${name}`;
    const bin = this.managedBin(dir);
    const archivePath = join(dir, name);
    const extract = join(dir, `powershell-${version}`);
    mkdirSync(bin, { recursive: true }); mkdirSync(extract, { recursive: true });
    this.runInstaller(bin, "curl", ["-fsSL", url, "-o", archivePath]);
    this.runInstaller(bin, "tar", ["-xzf", archivePath, "-C", extract]);
    rmSync(archivePath, { force: true });
    const pwsh = join(extract, "pwsh"); chmodSync(pwsh, 0o755);
    const link = join(bin, "pwsh"); rmSync(link, { force: true });
    try { symlinkSync(pwsh, link); } catch { copyFileSync(pwsh, link); chmodSync(link, 0o755); }
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
    const dir = mkdtempSync(join(tmpdir(), "tikee-rhai-probe-"));
    const script = join(dir, "probe.rhai");
    try {
      writeFileSync(script, 'print("tikee-rhai-probe");\n');
      return this.commandWorks(command, [script]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }
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
