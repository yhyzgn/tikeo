import { describe, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";

import {
  Client,
  DenoScriptRunner,
  LocalCommandScriptRunner,
  ManagementClient,
  SandboxToolResolver,
  ScriptRunnerRegistry,
  SrtScriptRunner,
  UnavailableScriptRunner,
  broadcastApiTrigger,
  defaultSandboxBackend,
  grpcTarget,
  localConfig,
  pluginApiJob,
  processDispatchTask,
  installConsoleTaskLogBridge,
  printTaskLogLocally,
  scriptApiJob,
  type ScriptRunnerTask,
  PluginTypes,
} from "../src/index";
import { ScriptTaskRuntimeDirs } from "../src/script/runtimeDirs";
import { writeSrtSettings } from "../src/script/index";

function sha256(content: Uint8Array): string { return createHash("sha256").update(content).digest("hex"); }
function task(language: string, content: string, overrides: Partial<ScriptRunnerTask> = {}): ScriptRunnerTask {
  const bytes = Buffer.from(content);
  return { scriptId: `script-${language}`, versionId: "sv-test", versionNumber: 1, language, content: bytes, contentSha256: sha256(bytes), timeoutMs: 1000, maxOutputBytes: 4096, ...overrides };
}
function executable(path: string, content: string): void { writeFileSync(path, content); chmodSync(path, 0o755); }
function report(path: string): Record<string, string> { return Object.fromEntries(readFileSync(path, "utf8").trim().split("\n").map((line) => line.split("=", 2) as [string, string])); }

describe("node sdk parity", () => {
  test("client registration and heartbeat dry run", () => {
    const config = localConfig("http://127.0.0.1:9998", "node-worker-1");
    config.namespace = "tenant-a";
    config.app = "billing";
    config.capabilities = ["legacy-tag", "legacy-tag", ""];
    config.addTag("nodejs");
    config.addNormalProcessor("demo.echo", "Node echo processor");
    config.addScriptRunner("python", "srt");
    config.addPluginProcessor(PluginTypes.SQL, "billing.sql-sync", "SQL sync processor");
    const client = new Client(config);
    const registration = client.registration();
    expect(registration.capabilities).toEqual(["legacy-tag"]);
    expect(registration.structured.normalProcessors).toEqual([{ name: "demo.echo", description: "Node echo processor" }]);
    expect(registration.structured.pluginProcessors[0].processors).toEqual([{ name: "billing.sql-sync", description: "SQL sync processor" }]);
    client.startDryRun(() => ({ success: true, message: "" }));
    const heartbeat = client.nextHeartbeat("worker-1", "fence-1", 3);
    expect(heartbeat.sequence).toBe(1);
    expect(heartbeat.generation).toBe(3);
  });

  test("config validation fails closed", () => {
    expect(() => new Client(localConfig("", ""))).toThrow("endpoint");
    const config = localConfig("http://127.0.0.1:9998", "node-worker-2");
    config.heartbeatEveryMs = 0;
    expect(() => new Client(config)).toThrow("heartbeat");
  });

  test("grpc target normalizes http urls", () => {
    expect(grpcTarget("127.0.0.1:9998")).toBe("127.0.0.1:9998");
    expect(grpcTarget(" http://127.0.0.1:9998 ")).toBe("127.0.0.1:9998");
    expect(grpcTarget("https://worker.example:443")).toBe("worker.example");
  });

  test("connect closes failed registration stream before retry", async () => {
    const config = localConfig("http://127.0.0.1:9998", "node-worker-retry-cleanup");
    const client = new Client(config) as any;
    let cancelled = false;
    let destroyed = false;
    let clientClosed = false;
    const listeners = new Map<string, Function>();
    const call = {
      write() {},
      once(event: string, fn: Function) { listeners.set(event, fn); return this; },
      off(event: string) { listeners.delete(event); return this; },
      cancel() { cancelled = true; },
      destroy() { destroyed = true; },
    };
    client.connectGrpc = () => ({ OpenTunnel: () => call, close: () => { clientClosed = true; } });
    const promise = client.connect();
    listeners.get("error")?.(new Error("FAILED_PRECONDITION follower"));

    await expect(promise).rejects.toThrow("FAILED_PRECONDITION");
    expect(cancelled).toBe(true);
    expect(destroyed).toBe(true);
    expect(clientClosed).toBe(true);
  });

  test("management client creates structured plugin and script jobs", async () => {
    const bodies: any[] = [];
    const paths: string[] = [];
    const server = Bun.serve({
      port: 0,
      async fetch(req) {
        expect(req.headers.get("x-tikeo-api-key")).toBe("key-1");
        const body = await req.json();
        bodies.push(body);
        paths.push(new URL(req.url).pathname);
        if (new URL(req.url).pathname.endsWith(":trigger")) {
          return Response.json({ code: 0, message: "ok", data: { id: "inst-1", jobId: "job-1", status: "pending", triggerType: body.triggerType, executionMode: "single", createdAt: "now", updatedAt: "now" } });
        }
        return Response.json({ code: 0, message: "ok", data: { id: "job-1", ...body } });
      },
    });
    try {
      const client = new ManagementClient(`http://127.0.0.1:${server.port}`, "key-1", "dev-alpha", "orders");
      await client.createJob(pluginApiJob("node-sql", "sql", "billing.sql-sync"));
      await client.createJob(scriptApiJob("node-script", "script_manual_shell_echo"));
      const instance = await client.triggerJob("job-1");
      expect(instance.triggerType).toBe("api");
      expect(instance.jobId).toBe("job-1");
    } finally { server.stop(true); }
    expect(bodies[0].processorType).toBe("sql");
    expect(bodies[0].retryPolicy.maxAttempts).toBe(3);
    expect(bodies[1].scriptId).toBe("script_manual_shell_echo");
    expect(paths[2]).toBe("/api/v1/jobs/job-1:trigger");
    expect(bodies[2].triggerType).toBe("api");
    expect(bodies[2].executionMode).toBe("single");
    expect(broadcastApiTrigger({ region: "us-east-1" })).toEqual({ triggerType: "api", executionMode: "broadcast", broadcastSelector: { region: "us-east-1" } });
  });

  test("console bridge mirrors processor logs only inside active task scope", async () => {
    const logs: [string, string][] = [];
    const bridge = installConsoleTaskLogBridge();
    try {
      console.info("nodejs outside task scope should stay console-only");
      const outcome = await processDispatchTask(() => {
        console.info("nodejs native logger info", { orderId: 42 });
        console.error("nodejs native logger error");
        return { success: true, message: "ok" };
      }, undefined, {
        instanceId: "inst-node-logger",
        jobId: "job-node-logger",
        processorName: "demo.logger",
        payload: new Uint8Array(),
        assignmentToken: "assign-node-logger",
      }, (level, message) => logs.push([level, message]));

      expect(outcome.success).toBe(true);
    } finally {
      bridge.restore();
    }

    expect(logs.some(([level, message]) => level === "info" && message.includes("nodejs native logger info") && message.includes("orderId"))).toBe(true);
    expect(logs.some(([level, message]) => level === "error" && message.includes("nodejs native logger error"))).toBe(true);
    expect(logs.some(([_level, message]) => message.includes("outside task scope"))).toBe(false);
  });

  test("local task log echo is not recaptured by console bridge", async () => {
    const logs: [string, string][] = [];
    const bridge = installConsoleTaskLogBridge();
    try {
      const outcome = await processDispatchTask(() => {
        throw new Error("nodejs bridge recursion probe");
      }, undefined, {
        instanceId: "inst-node-bridge-recursion",
        jobId: "job-node-bridge-recursion",
        processorName: "demo.bridge-recursion",
        payload: new Uint8Array(),
        assignmentToken: "assign-node-bridge-recursion",
      }, (level, message) => {
        logs.push([level, message]);
        if (logs.length > 1) throw new Error("task log bridge recaptured its own local echo");
        printTaskLogLocally(level, message);
      });

      expect(outcome.success).toBe(false);
      expect(outcome.message).toContain("nodejs bridge recursion probe");
    } finally {
      bridge.restore();
    }

    expect(logs).toHaveLength(1);
    expect(logs[0][0]).toBe("error");
    expect(logs[0][1]).toContain("nodejs bridge recursion probe");
  });

  test("processor exceptions are reported with stack trace task logs", async () => {
    const logs: [string, string][] = [];
    const task = {
      instanceId: "inst-node-exception",
      jobId: "job-node-exception",
      processorName: "demo.exception",
      payload: new Uint8Array(),
      assignmentToken: "assign-node-exception",
    };

    const outcome = await processDispatchTask(() => {
      throw new Error("nodejs runtime boom");
    }, undefined, task, (level, message) => logs.push([level, message]));

    expect(outcome.success).toBe(false);
    expect(outcome.message).toContain("nodejs runtime boom");
    expect(logs.some(([level, message]) => level === "error" && message.includes("nodejs runtime boom") && message.includes("at"))).toBe(true);
  });

  test("local shell runner executes immutable script snapshot", () => {
    const runner = new LocalCommandScriptRunner("shell", "custom");
    const outcome = runner.run(task("shell", "printf 'node-script-ok'\n"));
    expect(outcome.success).toBe(true);
    expect(outcome.message).toBe("node-script-ok");
  });

  test("local runner rejects unsafe policy", () => {
    const runner = new LocalCommandScriptRunner("shell", "custom");
    const outcome = runner.run(task("shell", "echo unsafe\n", { allowNetwork: true }));
    expect(outcome.success).toBe(false);
    expect(outcome.message).toContain("network");
  });

  test("unavailable runner is fail closed and not advertised", () => {
    const config = localConfig("http://127.0.0.1:9998", "node-worker-unavailable");
    const registry = new ScriptRunnerRegistry().register(new UnavailableScriptRunner("python", "srt", "srt is not installed"));
    registry.addCapabilities(config);
    expect(config.structured.scriptRunners).toEqual([]);
    const outcome = registry.get("python")!.run(task("python", "print(1)"));
    expect(outcome.success).toBe(false);
    expect(outcome.message).toContain("unavailable");
  });

  test("sandbox resolver does not advertise missing tools when auto install disabled", () => {
    const oldPath = process.env.PATH;
    const oldToolsDir = process.env.TIKEO_SANDBOX_TOOLS_DIR;
    try {
      process.env.PATH = "";
      process.env.TIKEO_SANDBOX_TOOLS_DIR = mkdtempSync(join(tmpdir(), "tikeo-node-host-tools-"));
      const resolver = new SandboxToolResolver(mkdtempSync(join(tmpdir(), "tikeo-node-tools-")), false);
      const [_path, ok] = resolver.resolveSrt();
      expect(ok).toBe(false);
    } finally {
      if (oldPath === undefined) delete process.env.PATH;
      else process.env.PATH = oldPath;
      if (oldToolsDir === undefined) delete process.env.TIKEO_SANDBOX_TOOLS_DIR;
      else process.env.TIKEO_SANDBOX_TOOLS_DIR = oldToolsDir;
    }
  });

  test("sandbox resolver auto install returns unavailable immediately", () => {
    const oldPath = process.env.PATH;
    const oldToolsDir = process.env.TIKEO_SANDBOX_TOOLS_DIR;
    try {
      process.env.PATH = "";
      process.env.TIKEO_SANDBOX_TOOLS_DIR = mkdtempSync(join(tmpdir(), "tikeo-node-host-tools-"));
      const resolver = new SandboxToolResolver(mkdtempSync(join(tmpdir(), "tikeo-node-tools-")), true, 1);
      const startedAt = Date.now();
      const [_path, ok] = resolver.resolveSrt();
      expect(ok).toBe(false);
      expect(Date.now() - startedAt).toBeLessThan(1_000);
    } finally {
      if (oldPath === undefined) delete process.env.PATH;
      else process.env.PATH = oldPath;
      if (oldToolsDir === undefined) delete process.env.TIKEO_SANDBOX_TOOLS_DIR;
      else process.env.TIKEO_SANDBOX_TOOLS_DIR = oldToolsDir;
    }
  });


  test("sandbox resolver strict sandbox isolation skips host path", () => {
    const oldPath = process.env.PATH;
    const oldToolsDir = process.env.TIKEO_SANDBOX_TOOLS_DIR;
    const root = mkdtempSync(join(tmpdir(), "tikeo-node-managed-only-"));
    try {
      const hostBin = join(root, "host-bin");
      mkdirSync(hostBin, { recursive: true });
      executable(join(hostBin, "srt"), "#!/bin/sh\necho host-srt\n");
      process.env.PATH = hostBin;
      process.env.TIKEO_SANDBOX_TOOLS_DIR = join(root, "managed-tools");
      const resolver = new SandboxToolResolver(root, false, 120_000, true);
      const [_path, ok] = resolver.resolveSrt();
      expect(ok).toBe(false);
      const [_interpreter, interpreterOk] = resolver.resolveInterpreter("sh");
      expect(interpreterOk).toBe(false);
    } finally {
      if (oldPath === undefined) delete process.env.PATH;
      else process.env.PATH = oldPath;
      if (oldToolsDir === undefined) delete process.env.TIKEO_SANDBOX_TOOLS_DIR;
      else process.env.TIKEO_SANDBOX_TOOLS_DIR = oldToolsDir;
      rmSync(root, { recursive: true, force: true });
    }
  });


  test("sandbox resolver uses host cache when worker state is empty", () => {
    const resolver = new SandboxToolResolver(mkdtempSync(join(tmpdir(), "tikeo-node-tools-")), false);
    const installDir = (resolver as unknown as { installDir(key: string): string }).installDir("srt");
    expect(installDir).toBe(join(homedir(), ".tikeo", "sandbox-tools", "srt"));
  });


  test("rhai resolver probes by running script file", () => {
    const root = mkdtempSync(join(tmpdir(), "tikeo-node-rhai-probe-"));
    const binary = join(root, "rhai-run");
    const reportFile = join(root, "report.txt");
    executable(binary, `#!/bin/sh\nprintf 'arg=%s\n' "$1" > '${reportFile}'\ntest -f "$1"\n`);
    const resolver = new SandboxToolResolver(root, false);
    const ok = (resolver as unknown as { toolWorks(binary: string, command: string): boolean }).toolWorks("rhai-run", binary);
    expect(ok).toBe(true);
    const values = report(reportFile);
    expect(values.arg).toEndWith(".rhai");
    expect(values.arg).not.toBe("--version");
    expect(values.arg).not.toBe("--help");
    rmSync(root, { recursive: true, force: true });
  });

  test("srt and deno runners advertise structured backends", () => {
    const registry = new ScriptRunnerRegistry().register(new SrtScriptRunner("python", "srt", "python3")).register(new DenoScriptRunner("javascript", "deno"));
    const config = localConfig("http://127.0.0.1:9998", "node-sandbox-test");
    registry.addCapabilities(config);
    const seen = Object.fromEntries(config.structured.scriptRunners.map((runner) => [runner.language, runner.sandboxBackend]));
    expect(seen).toEqual({ javascript: "deno", python: "srt" });
  });

  test("srt settings serialize empty policy lists as arrays", () => {
    const dirs = ScriptTaskRuntimeDirs.create("tikeo-node-srt-settings-test");
    const settings = writeSrtSettings(task("shell", "echo ok"), dirs);
    const parsed = JSON.parse(readFileSync(settings, "utf8"));
    expect(Array.isArray(parsed.network.allowedDomains)).toBe(true);
    expect(Array.isArray(parsed.filesystem.allowRead)).toBe(true);
    expect(parsed.filesystem.allowWrite).toContain(dirs.powerShellCache);
    rmSync(settings, { force: true });
    dirs.cleanup();
  });

  test("srt runner starts supported kinds inside sandbox home", () => {
    for (const [language, interpreter, content] of [["shell", "sh", "pwd\n"], ["python", "python3", "import os; print(os.getcwd())\n"], ["powershell", "pwsh", "Get-Location\n"], ["rhai", "rhai-run", "print(\"ok\");\n"]] as const) {
      const root = mkdtempSync(join(tmpdir(), "tikeo-node-srt-"));
      const reportFile = join(root, "report.txt");
      const runtime = join(root, "srt");
      executable(runtime, `#!/bin/sh\nprintf 'cwd=%s\\n' "$(pwd)" > '${reportFile}'\nprintf 'home=%s\\n' "$HOME" >> '${reportFile}'\nprintf 'tmp=%s\\n' "$TMPDIR" >> '${reportFile}'\nprintf 'claude_tmp=%s\\n' "$CLAUDE_CODE_TMPDIR" >> '${reportFile}'\nprintf 'args=%s\\n' "$*" >> '${reportFile}'\nexit 0\n`);
      const runner = new SrtScriptRunner(language, runtime, interpreter);
      const outcome = runner.run(task(language, content, { allowedEnvVars: ["HOME", "TMPDIR", "CLAUDE_CODE_TMPDIR"] }));
      expect(outcome.success).toBe(true);
      const values = report(reportFile);
      expect(values.cwd).toBe(values.home);
      expect(values.home).toContain(`tikeo-srt-${language}-runtime`);
      expect(values.claude_tmp).toBe(values.tmp);
      if (language === "rhai") expect(values.args).toContain("/home/script-");
    }
  });

  test("deno runner starts js and ts inside sandbox home", () => {
    for (const language of ["javascript", "typescript"]) {
      const root = mkdtempSync(join(tmpdir(), "tikeo-node-deno-"));
      const reportFile = join(root, "report.txt");
      const runtime = join(root, "deno");
      executable(runtime, `#!/bin/sh\ncat >/dev/null\nprintf 'cwd=%s\\n' "$(pwd)" > '${reportFile}'\nprintf 'home=%s\\n' "$HOME" >> '${reportFile}'\nprintf 'tmp=%s\\n' "$TMPDIR" >> '${reportFile}'\nprintf 'deno_dir=%s\\n' "$DENO_DIR" >> '${reportFile}'\nprintf 'args=%s\\n' "$*" >> '${reportFile}'\nexit 0\n`);
      const runner = new DenoScriptRunner(language, runtime);
      const outcome = runner.run(task(language, "console.log('ok')\n", { allowedEnvVars: ["HOME", "TMPDIR", "DENO_DIR"] }));
      expect(outcome.success).toBe(true);
      const values = report(reportFile);
      expect(values.cwd).toBe(values.home);
      expect(values.home).toContain(`tikeo-deno-${language}-runtime`);
      expect(values.deno_dir).toEndWith("/cache/deno");
      expect(values.args).toContain("run --no-prompt");
    }
  });

  test("auto sandbox defaults match Java lightweight defaults", () => {
    expect(defaultSandboxBackend("python")).toBe("srt");
    expect(defaultSandboxBackend("javascript")).toBe("deno");
    expect(defaultSandboxBackend("typescript")).toBe("deno");
  });
});
