import { mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

let sequence = 0;

export class ScriptTaskRuntimeDirs {
  root: string;
  home: string;
  config: string;
  cache: string;
  data: string;
  modules: string;
  dotnetHome: string;
  powerShellCache: string;
  tmp: string;
  denoDir: string;

  private constructor(root: string) {
    this.root = root;
    this.home = join(root, "home");
    this.config = join(root, "config");
    this.cache = join(root, "cache");
    this.data = join(root, "data");
    this.modules = join(this.data, "powershell", "Modules");
    this.dotnetHome = join(root, "dotnet");
    this.powerShellCache = join(this.cache, "powershell");
    this.tmp = join(root, "tmp");
    this.denoDir = join(this.cache, "deno");
  }

  static create(prefix: string): ScriptTaskRuntimeDirs {
    const root = mkdtempSync(join(tmpdir(), `${prefix.trim()}-`));
    const dirs = new ScriptTaskRuntimeDirs(root);
    for (const dir of dirs.requiredDirectories()) mkdirSync(dir, { recursive: true, mode: 0o700 });
    return dirs;
  }

  requiredDirectories(): string[] { return [this.root, this.home, this.config, this.cache, this.data, this.modules, this.dotnetHome, this.powerShellCache, this.tmp, this.denoDir]; }
  writablePaths(): string[] { return this.requiredDirectories(); }
  workingDir(): string { return this.home; }
  scriptFile(extension: string): string { return join(this.home, `script-${Date.now()}-${sequence++}.${extension}`); }

  baseEnvironment(extraPath: string[] = []): NodeJS.ProcessEnv {
    const pathParts = dedupe([...extraPath.filter(Boolean), ...(process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":")]);
    return {
      HOME: this.home,
      XDG_CONFIG_HOME: this.config,
      XDG_CACHE_HOME: this.cache,
      XDG_DATA_HOME: this.data,
      TMPDIR: this.tmp,
      TERM: "dumb",
      NO_COLOR: "1",
      PATH: pathParts.join(process.platform === "win32" ? ";" : ":"),
    };
  }

  srtEnvironment(extraPath: string[] = []): NodeJS.ProcessEnv {
    return { ...this.baseEnvironment(extraPath), CLAUDE_CODE_TMPDIR: this.tmp, CLAUDE_TMPDIR: this.tmp };
  }

  powerShellEnvironment(env: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
    return { ...env, PSModulePath: this.modules, DOTNET_CLI_HOME: this.dotnetHome, POWERSHELL_TELEMETRY_OPTOUT: "1", POWERSHELL_UPDATECHECK: "Off" };
  }

  denoEnvironment(): NodeJS.ProcessEnv {
    return { ...this.baseEnvironment(), DENO_DIR: this.denoDir };
  }

  cleanup(): void { rmSync(this.root, { recursive: true, force: true }); }
}

export function appendAllowedUnmanagedEnv(env: NodeJS.ProcessEnv, allowed: string[]): NodeJS.ProcessEnv {
  const managed = new Set(["HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME", "TMPDIR", "TERM", "NO_COLOR", "CLAUDE_CODE_TMPDIR", "CLAUDE_TMPDIR", "PSModulePath", "DOTNET_CLI_HOME", "POWERSHELL_TELEMETRY_OPTOUT", "POWERSHELL_UPDATECHECK", "DENO_DIR"]);
  for (const name of allowed) {
    const key = name.trim();
    if (key && !managed.has(key) && process.env[key] !== undefined) env[key] = process.env[key];
  }
  return env;
}

function dedupe(values: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const value of values) if (value && !seen.has(value)) { seen.add(value); out.push(value); }
  return out;
}
