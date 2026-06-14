import {
  Client,
  ContainerScriptRunner,
  DenoScriptRunner,
  LocalCommandScriptRunner,
  ManagementClient,
  SandboxToolResolver,
  ScriptRunnerRegistry,
  SrtScriptRunner,
  TaskContext,
  installConsoleTaskLogBridge,
  TaskOutcome,
  apiJob,
  failed,
  localConfig,
  normalizeScriptLanguage,
  pluginApiJob,
  type WorkerConfig,
} from "@yhyzgn/tikeo";

export async function main(): Promise<void> {
  installConsoleTaskLogBridge();
  const config = localConfig(envOr("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"), envOr("TIKEO_WORKER_CLIENT_INSTANCE_ID", "nodejs-worker-demo-local"));
  config.namespace = envOr("TIKEO_WORKER_NAMESPACE", "dev-alpha");
  config.app = envOr("TIKEO_WORKER_APP", "orders");
  config.cluster = envOr("TIKEO_WORKER_CLUSTER", "local");
  config.region = envOr("TIKEO_WORKER_REGION", "local");
  config.addTag("nodejs");
  config.addTag("manual-demo");
  for (const processor of csvOr("TIKEO_WORKER_SDK_PROCESSORS", "demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception")) config.addSDKProcessor(processor);
  config.labels.worker_pool = envOr("TIKEO_WORKER_POOL", "nodejs-blue");
  if (enabledByDefault("TIKEO_ENABLE_PLUGIN_SQL")) {
    config.addPluginProcessor(envOr("TIKEO_PLUGIN_SQL_TYPE", "sql"), envOr("TIKEO_PLUGIN_SQL_PROCESSOR", "billing.sql-sync"));
    config.labels.plugin_sql = "enabled";
  }
  const scripts = configureScripts(config);
  const client = new Client(config);
  console.log("nodejs worker demo configured: " + JSON.stringify(client.registration(), null, 2));

  if (enabled("TIKEO_MANAGEMENT_CREATE_EXAMPLES")) {
    const mgmt = new ManagementClient(envOr("TIKEO_HTTP_URL", "http://127.0.0.1:8080"), process.env.TIKEO_API_KEY ?? "", config.namespace, config.app);
    for (const job of [apiJob("nodejs-echo-api", "demo.echo"), pluginApiJob("nodejs-sql-sync-api", "sql", "billing.sql-sync")]) {
      try {
        const created = await mgmt.createJob(job);
        const instance = await mgmt.triggerJob(created.id);
        console.log(`created and triggered job ${created.namespace}/${created.app} ${created.name} instance=${instance.id} trigger_type=${instance.triggerType}`);
      }
      catch (error) { console.warn(`create job ${job.name} failed: ${(error as Error).message}`); }
    }
  }

  if (dryRunEnabled()) {
    client.startDryRun(processTask);
    const heartbeat = client.nextHeartbeat("dry-run-worker", "dry-run-fence", 1);
    console.log(`dry_run_heartbeat_sequence=${heartbeat.sequence}`);
    return;
  }

  // Live tunnel mode is available through the SDK client. Keep demo loop conservative and reconnecting.
  const oneshot = enabled("TIKEO_WORKER_ONESHOT");
  while (true) {
    try {
      const session = await client.connect();
      const stop = session.startHeartbeat();
      console.log(`nodejs worker connected: worker_id=${session.workerId} generation=${session.generation} lease_seconds=${session.leaseSeconds}`);
      try {
        while (true) {
          const outcome = await session.processNext(processTask, scripts);
          console.log(`processed task success=${outcome.success} message=${outcome.message}`);
          if (oneshot) return;
          await new Promise((resolve) => setTimeout(resolve, 50));
        }
      } finally {
        stop();
        session.close();
      }
    } catch (error) {
      console.warn(`worker tunnel ended, reconnecting: ${(error as Error).message}`);
      await new Promise((resolve) => setTimeout(resolve, 2_000));
    }
  }
}

export function configureScripts(config: WorkerConfig): ScriptRunnerRegistry {
  const scripts = new ScriptRunnerRegistry();
  const resolver = new SandboxToolResolver(envOr("TIKEO_WORKER_STATE_DIR", ""), !disabled("TIKEO_SANDBOX_AUTO_INSTALL"));
  for (const language of csvOr("TIKEO_WORKER_SCRIPT_LANGUAGES", "shell,python,javascript,typescript,powershell,php,groovy,rhai")) {
    if (disabled("TIKEO_ENABLE_SCRIPT_" + language.toUpperCase())) continue;
    const backend = scriptSandboxBackend(language);
    try {
      if (backend === "srt") {
        const [srt, srtOk] = resolver.resolveSrt();
        const [rg, rgOk] = resolver.resolveRipgrep();
        const [interpreter, interpreterOk] = resolveSrtInterpreter(language, resolver);
        if (srtOk && rgOk && interpreterOk) { scripts.register(new SrtScriptRunner(language, srt, interpreter, sandboxToolPathEntries(srt, rg, interpreter, resolver))); console.log(`script runner ${language} registered backend=srt interpreter=${interpreter}`); continue; }
        console.warn(`script runner ${language} skipped: srtOk=${srtOk} rgOk=${rgOk} interpreterOk=${interpreterOk} interpreter=${interpreter}`);
      } else if (backend === "deno" || backend === "v8") {
        const [deno, ok] = resolver.resolveDeno();
        if (ok) { scripts.register(new DenoScriptRunner(language, deno)); console.log(`script runner ${language} registered backend=deno runtime=${deno}`); continue; }
        console.warn(`script runner ${language} skipped: deno unavailable runtime=${deno}`);
      } else if (backend === "docker" || backend === "podman") {
        scripts.register(new ContainerScriptRunner(language, backend, scriptImage(language))); console.log(`script runner ${language} registered backend=${backend}`); continue;
      } else if (enabled("TIKEO_ENABLE_LOCAL_SCRIPT_" + language.toUpperCase())) {
        scripts.register(new LocalCommandScriptRunner(language, "custom")); console.log(`script runner ${language} registered backend=custom`); continue;
      }
    } catch (error) { console.warn(`script runner ${language} skipped: ${(error as Error).message}`); }
  }
  scripts.addCapabilities(config);
  return scripts;
}

export function processTask(task: TaskContext): TaskOutcome {
  console.info(`[nodejs-worker] processor=${task.processorName} instance=${task.instanceId} payload_bytes=${task.payload.length}`);
  const payload = new TextDecoder().decode(task.payload);
  switch (task.processorName || "demo.echo") {
    case "":
    case "demo.echo": console.info(`[demo.echo] payload='${payload}'`); return { success: true, message: "nodejs demo echo processed" };
    case "demo.context": console.info(`[demo.context] jobId=${task.jobId} instanceId=${task.instanceId}`); return { success: true, message: `nodejs demo context processed instance=${task.instanceId}` };
    case "demo.bytes": console.info(`[demo.bytes] payload='${payload}' length=${task.payload.length}`); return { success: true, message: `nodejs demo bytes processed payload_bytes=${task.payload.length}` };
    case "demo.heartbeat": console.info(`[demo.heartbeat] tick jobId=${task.jobId} instanceId=${task.instanceId}`); return { success: true, message: "nodejs demo heartbeat processed" };
    case "billing.sql-sync": console.info(`[billing.sql-sync] plugin SQL processor received payload='${payload}'`); return { success: true, message: "nodejs demo sql plugin processed" };
    case "demo.fail": console.error(`[demo.fail] intentional failure payload='${payload}'`); return failed("nodejs demo intentional failure");
    case "demo.exception": console.error(`[demo.exception] throwing runtime exception payload='${payload}'`); throw new Error("nodejs demo runtime exception");
    default: console.error(`[nodejs-worker] unsupported processor=${task.processorName}`); return failed(`unsupported nodejs demo processor: ${task.processorName}`);
  }
}

export function scriptSandboxBackend(language: string): string {
  const value = (process.env.TIKEO_WORKER_SCRIPT_SANDBOX ?? "").trim();
  if (value && value.toLowerCase() !== "auto") return value.toLowerCase();
  return ["javascript", "js", "typescript", "ts"].includes(language.trim().toLowerCase()) ? "deno" : "srt";
}

function resolveSrtInterpreter(language: string, resolver: SandboxToolResolver): [string, boolean] {
  switch (language.trim().toLowerCase()) {
    case "shell": case "sh": case "bash": return resolver.resolveInterpreter("sh");
    case "python": case "py": return resolver.resolveInterpreter("python3");
    case "powershell": case "pwsh": return resolver.resolvePowerShell();
    case "php": return resolver.resolveInterpreter("php");
    case "groovy": return resolver.resolveInterpreter("groovy");
    case "rhai": return resolver.resolveRhai();
    default: return resolver.resolveInterpreter("sh");
  }
}

function sandboxToolPathEntries(srt: string, rg: string, interpreter: string, resolver: SandboxToolResolver): string[] {
  const entries = [toolParent(srt), toolParent(rg), toolParent(interpreter)].filter(Boolean);
  for (const [value, ok] of [resolver.resolveNode(), resolver.resolveNpm()]) if (ok && toolParent(value)) entries.push(toolParent(value));
  return [...new Set(entries)];
}
function toolParent(command: string): string { const index = Math.max(command.lastIndexOf("/"), command.lastIndexOf("\\")); return index > 0 ? command.slice(0, index) : ""; }
function scriptImage(language: string): string {
  const normalized = normalizeScriptLanguage(language);
  return envOr(`TIKEO_${normalized.toUpperCase()}_IMAGE`, ({ shell: "alpine:latest", python: "python:alpine", javascript: "denoland/deno:alpine", typescript: "denoland/deno:alpine", powershell: "mcr.microsoft.com/powershell:latest", php: "php:cli-alpine", groovy: "groovy:latest", rhai: "rhaiscript/rhai:latest" } as Record<string, string>)[normalized] ?? "");
}
export function envOr(key: string, fallback: string): string { return process.env[key]?.trim() || fallback; }
export function csvOr(key: string, fallback: string): string[] { return envOr(key, fallback).split(",").map((x) => x.trim()).filter(Boolean); }
export function enabled(key: string): boolean { return ["1", "true", "yes", "on"].includes((process.env[key] ?? "").trim().toLowerCase()); }
export function disabled(key: string): boolean { return ["0", "false", "no", "off"].includes((process.env[key] ?? "").trim().toLowerCase()); }
export function enabledByDefault(key: string): boolean { return !disabled(key); }
export function dryRunEnabled(): boolean { return enabled("TIKEO_WORKER_DRY_RUN") || disabled("TIKEO_WORKER_CONNECT"); }

if (import.meta.main) await main();
