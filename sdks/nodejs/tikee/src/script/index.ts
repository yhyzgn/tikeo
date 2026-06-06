import { createHash } from "node:crypto";
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { tmpdir, homedir } from "node:os";
import { isAbsolute, join } from "node:path";
import { spawnSync } from "node:child_process";

import type { WorkerConfig } from "../config.js";
import type { TaskLogSink, TaskOutcome } from "../task.js";
import { failed } from "../task.js";
import { ScriptTaskRuntimeDirs, appendAllowedUnmanagedEnv } from "./runtimeDirs.js";
import { SandboxToolResolver } from "./sandboxTools.js";

export { SandboxToolResolver } from "./sandboxTools.js";
export { ScriptTaskRuntimeDirs } from "./runtimeDirs.js";

export interface ScriptRunnerTask {
  scriptId: string;
  versionId: string;
  versionNumber: number;
  language: string;
  content: Uint8Array;
  contentSha256: string;
  timeoutMs?: number;
  maxOutputBytes?: number;
  allowNetwork?: boolean;
  allowedEnvVars?: string[];
  readOnlyPaths?: string[];
  writablePaths?: string[];
  secretRefs?: string[];
  allowedNetworkHosts?: string[];
  sandboxBackend?: string;
  instanceId?: string;
  jobId?: string;
  log?: TaskLogSink;
}

export interface ScriptRunner {
  language: string;
  sandboxBackend: string;
  run(task: ScriptRunnerTask): Promise<TaskOutcome> | TaskOutcome;
  advertiseCapability?(): boolean;
}

export function normalizeScriptLanguage(language: string): string {
  switch (language.trim().toLowerCase()) {
    case "shell": case "sh": case "bash": return "shell";
    case "python": case "py": return "python";
    case "node": case "nodejs": case "javascript": case "js": return "javascript";
    case "typescript": case "ts": return "typescript";
    case "powershell": case "pwsh": return "powershell";
    case "php": return "php";
    case "groovy": return "groovy";
    case "rhai": return "rhai";
    default: return language.trim().toLowerCase();
  }
}

export function defaultSandboxBackend(language: string): string { return ["javascript", "typescript"].includes(normalizeScriptLanguage(language)) ? "deno" : "srt"; }

export function normalizeScriptSandboxBackend(backend: string, language: string): string {
  let value = backend.trim().toLowerCase();
  if (!value || value === "auto") return defaultSandboxBackend(language);
  const aliases: Record<string, string> = { "wasm_edge": "wasmedge", "wasm-edge": "wasmedge", "anthropic_srt": "srt", "anthropic-srt": "srt", "sandbox_runtime": "srt", "sandbox-runtime": "srt", "v8_isolate": "v8", "v8-isolate": "v8" };
  value = aliases[value] ?? value;
  if (!["wasmtime", "wasmedge", "srt", "deno", "v8", "docker", "podman", "custom"].includes(value)) throw new Error(`unsupported script sandbox backend: ${backend}`);
  return value;
}

export function defaultScriptCommand(language: string): [string, string[]] {
  switch (normalizeScriptLanguage(language)) {
    case "shell": return ["sh", ["-s"]];
    case "python": return ["python3", ["-"]];
    case "javascript": case "typescript": return ["deno", ["run", "--no-prompt", "-"]];
    case "powershell": return ["pwsh", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "-"]];
    case "php": return ["php", []];
    case "groovy": return ["groovy", []];
    case "rhai": return ["rhai", []];
    default: return ["sh", ["-s"]];
  }
}

export class ScriptRunnerRegistry {
  private runners = new Map<string, ScriptRunner>();
  register(runner: ScriptRunner): ScriptRunnerRegistry {
    this.runners.set(normalizeScriptLanguage(runner.language), runner);
    return this;
  }
  get(language: string): ScriptRunner | undefined { return this.runners.get(normalizeScriptLanguage(language)); }
  addCapabilities(config: WorkerConfig): void {
    for (const language of [...this.runners.keys()].sort()) {
      const runner = this.runners.get(language)!;
      if (runner.advertiseCapability?.() === false) continue;
      config.addScriptRunner(runner.language, runner.sandboxBackend);
    }
  }
}

export class UnavailableScriptRunner implements ScriptRunner {
  language: string;
  sandboxBackend: string;
  constructor(language: string, sandboxBackend: string, private reason: string) {
    this.language = normalizeScriptLanguage(language);
    try { this.sandboxBackend = normalizeScriptSandboxBackend(sandboxBackend, this.language); }
    catch (error) { this.sandboxBackend = defaultSandboxBackend(this.language); this.reason = `${reason}; ${(error as Error).message}`; }
  }
  advertiseCapability(): boolean { return false; }
  run(task: ScriptRunnerTask): TaskOutcome {
    try { validateScriptTask(this.language, task); } catch (error) { return failed((error as Error).message); }
    return failed(`${this.language} script runner backend is unavailable: ${this.reason}`);
  }
}

export class LocalCommandScriptRunner implements ScriptRunner {
  language: string;
  sandboxBackend: string;
  private command: string;
  private args: string[];
  constructor(language: string, sandboxBackend = "custom") {
    this.language = normalizeScriptLanguage(language);
    this.sandboxBackend = normalizeScriptSandboxBackend(sandboxBackend, this.language);
    if (this.sandboxBackend !== "custom") throw new Error(`local command script runner must use custom sandbox backend, got ${this.sandboxBackend}`);
    [this.command, this.args] = defaultScriptCommand(this.language);
  }
  run(task: ScriptRunnerTask): TaskOutcome {
    try {
      validateScriptTask(this.language, task);
      if (task.allowNetwork || task.allowedNetworkHosts?.length) throw new Error("local script runner rejects network access");
      if (task.secretRefs?.length) throw new Error("local script runner rejects secret refs");
      if (task.readOnlyPaths?.length || task.writablePaths?.length) throw new Error("local script runner rejects filesystem grants");
    } catch (error) { return failed((error as Error).message); }
    return runCommand([this.command, ...this.args], task, Buffer.from(task.content));
  }
}

export class ContainerScriptRunner implements ScriptRunner {
  language: string;
  sandboxBackend: string;
  constructor(language: string, runtimeCommand: string, private image: string, private runtimeArgs: string[] = []) {
    this.language = normalizeScriptLanguage(language);
    this.sandboxBackend = normalizeScriptSandboxBackend(runtimeCommand, this.language);
    if (!["docker", "podman"].includes(this.sandboxBackend)) throw new Error(`container script runner requires docker or podman backend, got ${this.sandboxBackend}`);
    if (!image.trim()) throw new Error(`container script runner requires an image for ${this.language}`);
  }
  run(task: ScriptRunnerTask): TaskOutcome {
    try {
      validateScriptTask(this.language, task);
      if (task.allowNetwork || task.allowedNetworkHosts?.length) throw new Error("container script runner rejects network grants without host-level filtering");
      if (task.secretRefs?.length) throw new Error("container script runner rejects secret refs without a worker-local secret provider");
      return runCommand([this.sandboxBackend, ...this.containerArgs(task)], task, Buffer.from(task.content));
    } catch (error) { return failed((error as Error).message); }
  }
  private containerArgs(task: ScriptRunnerTask): string[] {
    const args = ["run", "--rm", "-i", "--network=none", "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=16m", "--memory", "67108864", ...this.runtimeArgs];
    for (const path of task.readOnlyPaths ?? []) args.push("--mount", containerMount(path, true));
    for (const path of task.writablePaths ?? []) args.push("--mount", containerMount(path, false));
    args.push("--env", `TIKEE_SCRIPT_ID=${task.scriptId}`, "--env", `TIKEE_SCRIPT_VERSION_ID=${task.versionId}`, "--env", `TIKEE_SCRIPT_VERSION_NUMBER=${task.versionNumber}`, this.image);
    const [cmd, cmdArgs] = defaultScriptCommand(this.language);
    return [...args, cmd, ...cmdArgs];
  }
}

export class SrtScriptRunner implements ScriptRunner {
  sandboxBackend = "srt";
  language: string;
  constructor(language: string, private runtimeCommand: string, private interpreter: string, private extraPath: string[] = []) {
    this.language = normalizeScriptLanguage(language);
    if (!runtimeCommand.trim() || !interpreter.trim()) throw new Error("SRT runner requires runtime and interpreter commands");
  }
  run(task: ScriptRunnerTask): TaskOutcome {
    try {
      validateScriptTask(this.language, task);
      if (task.secretRefs?.length) throw new Error("SRT script runner rejects secret refs without a worker-local secret provider");
    } catch (error) { return failed((error as Error).message); }
    const dirs = ScriptTaskRuntimeDirs.create(`tikee-srt-${this.language}-runtime`);
    let settings = "";
    try {
      let scriptFile = "";
      if (this.language === "rhai") { scriptFile = dirs.scriptFile("rhai"); writeFileSync(scriptFile, task.content); }
      settings = writeSrtSettings(task, dirs, scriptFile);
      let env = dirs.srtEnvironment(this.extraPath);
      if (this.language === "powershell") env = dirs.powerShellEnvironment(env);
      addScriptEnv(env, task);
      appendAllowedUnmanagedEnv(env, task.allowedEnvVars ?? []);
      return runCommand([this.runtimeCommand, "--settings", settings, "-c", this.shellCommand(Buffer.from(task.content).toString(), scriptFile)], task, undefined, dirs.workingDir(), env);
    } finally { if (settings) rmSync(settings, { force: true }); dirs.cleanup(); }
  }
  private shellCommand(source: string, scriptFile: string): string {
    switch (this.language) {
      case "shell": return source;
      case "python": return heredoc(`${this.interpreter} -`, "PY", source);
      case "powershell": return heredoc(`${this.interpreter} -NoLogo -NoProfile -NonInteractive -InputFormat Text -OutputFormat Text -Command -`, "PWSH", source);
      case "php": case "groovy": return heredoc(this.interpreter, this.language.toUpperCase(), source);
      case "rhai": return scriptFile ? `${this.interpreter} '${scriptFile.replace(/'/g, `'\\''`)}'` : heredoc(this.interpreter, "RHAI", source);
      default: return heredoc(this.interpreter, "SCRIPT", source);
    }
  }
}

export class DenoScriptRunner implements ScriptRunner {
  sandboxBackend = "deno";
  language: string;
  constructor(language: string, private command: string) {
    this.language = normalizeScriptLanguage(language);
    if (!["javascript", "typescript"].includes(this.language)) throw new Error("Deno runner supports JavaScript and TypeScript only");
    if (!command.trim()) throw new Error("Deno runner requires a command");
  }
  run(task: ScriptRunnerTask): TaskOutcome {
    try { validateScriptTask(this.language, task); if (task.secretRefs?.length) throw new Error("Deno script runner rejects secret refs without a worker-local secret provider"); }
    catch (error) { return failed((error as Error).message); }
    const dirs = ScriptTaskRuntimeDirs.create(`tikee-deno-${this.language}-runtime`);
    try {
      const args = [this.command, "run", "--no-prompt"];
      if (task.allowNetwork) args.push("--allow-net"); else if (task.allowedNetworkHosts?.length) args.push(`--allow-net=${task.allowedNetworkHosts.join(",")}`);
      if (task.allowedEnvVars?.length) args.push(`--allow-env=${task.allowedEnvVars.join(",")}`);
      if (task.readOnlyPaths?.length) args.push(`--allow-read=${task.readOnlyPaths.join(",")}`);
      const writable = [...(task.writablePaths ?? []), ...dirs.writablePaths()];
      if (writable.length) args.push(`--allow-write=${writable.join(",")}`);
      args.push("-");
      const env = dirs.denoEnvironment(); addScriptEnv(env, task); appendAllowedUnmanagedEnv(env, task.allowedEnvVars ?? []);
      return runCommand(args, task, Buffer.from(task.content), dirs.workingDir(), env);
    } finally { dirs.cleanup(); }
  }
}

export function validateScriptTask(language: string, task: ScriptRunnerTask): void {
  if (normalizeScriptLanguage(task.language) !== language) throw new Error(`script runner language mismatch: task=${task.language} runner=${language}`);
  if (!task.scriptId || !task.versionNumber || !task.content?.length) throw new Error("script runner requires a released immutable script version snapshot");
  if (!task.contentSha256) throw new Error("script runner requires a content sha256 digest");
  const digest = createHash("sha256").update(task.content).digest("hex");
  if (digest !== task.contentSha256.toLowerCase()) throw new Error("script content digest mismatch");
}

export function emitScriptCommandOutput(log: TaskLogSink | undefined, level: string, output: Buffer): void {
  if (!log || output.length === 0) return;
  for (const line of output.toString().replace(/\r\n/g, "\n").split("\n")) {
    const item = line.trim();
    if (item) log(level, `[script] ${item}`);
  }
}

function runCommand(command: string[], task: ScriptRunnerTask, input?: Buffer, cwd?: string, env?: NodeJS.ProcessEnv): TaskOutcome {
  const result = spawnSync(command[0], command.slice(1), { input, cwd, env, encoding: "buffer", timeout: task.timeoutMs ?? 30_000 });
  if (result.error) return failed(result.error.message.includes("ETIMEDOUT") ? "script runner timed out" : result.error.message);
  const stdout = result.stdout ? Buffer.from(result.stdout) : Buffer.alloc(0);
  const stderr = result.stderr ? Buffer.from(result.stderr) : Buffer.alloc(0);
  emitScriptCommandOutput(task.log, "info", stdout);
  emitScriptCommandOutput(task.log, "error", stderr);
  const message = stdout.toString().trim() || stderr.toString().trim();
  if (result.status !== 0) return failed(limitOutput(message || `script runner exited with status ${result.status}`, task.maxOutputBytes ?? 1024 * 1024));
  return { success: true, message: limitOutput(message, task.maxOutputBytes ?? 1024 * 1024) };
}

export function writeSrtSettings(task: ScriptRunnerTask, dirs: ScriptTaskRuntimeDirs, scriptFile = ""): string {
  const allowRead = [...(task.readOnlyPaths ?? []), ...(scriptFile ? [scriptFile] : [])];
  const settings = {
    network: { allowUnixSocket: false, allowedDomains: task.allowedNetworkHosts ?? [], deniedDomains: [] as string[] },
    filesystem: { allowRead, allowWrite: [...(task.writablePaths ?? []), ...dirs.writablePaths()], denyRead: sensitiveReadDenies(), denyWrite: [] as string[] },
  };
  const file = join(mkdtempSync(join(tmpdir(), "tikee-srt-settings-dir-")), "settings.json");
  writeFileSync(file, JSON.stringify(settings));
  return file;
}

function sensitiveReadDenies(): string[] { return [".ssh", ".gnupg", ".aws", ".kube", ".docker", join(".config", "tikee")].map((path) => join(homedir(), path)); }
function heredoc(command: string, marker: string, content: string): string { let delimiter = marker; while (content.includes(delimiter)) delimiter += "_TIKEE"; return `${command} <<'${delimiter}'\n${content}\n${delimiter}`; }
function containerMount(path: string, readOnly: boolean): string { const trimmed = path.trim(); if (!trimmed || trimmed !== path || !isAbsolute(trimmed) || trimmed.split(/[\\/]/).includes("..")) throw new Error(`script file grant path must be clean and absolute: ${path}`); return `type=bind,src=${trimmed},dst=${trimmed}${readOnly ? ",readonly" : ""}`; }
function addScriptEnv(env: NodeJS.ProcessEnv, task: ScriptRunnerTask): void { env.TIKEE_SCRIPT_ID = task.scriptId; env.TIKEE_SCRIPT_VERSION_ID = task.versionId; env.TIKEE_SCRIPT_VERSION_NUMBER = String(task.versionNumber); }
function limitOutput(message: string, max: number): string { return Buffer.byteLength(message) <= max ? message : Buffer.from(message).subarray(0, max).toString(); }
