import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";

import type { WorkerCapabilities } from "./config.js";
import { WorkerConfig } from "./config.js";
import { sdkLog } from "./logging.js";
import type { ScriptRunnerRegistry, ScriptRunnerTask } from "./script/index.js";
import { failed, TaskContext, type TaskOutcome, type TaskProcessor } from "./task.js";
import { runWithoutTaskLogBridgeCapture, runWithTaskLogScope } from "./taskLogging.js";

export interface Registration {
  clientInstanceId: string;
  namespace: string;
  app: string;
  name: string;
  region: string;
  version: string;
  cluster: string;
  capabilities: string[];
  labels: Record<string, string>;
  structured: WorkerCapabilities;
}

export interface Heartbeat {
  workerId: string;
  sequence: number;
  generation: number;
  fencingToken: string;
  sentAt: Date;
}

export class Client {
  private seq = 0;
  private open = false;
  constructor(private config: WorkerConfig) {
    config.validate();
    config.normalize();
  }

  registration(): Registration {
    return {
      clientInstanceId: this.config.clientInstanceId,
      namespace: this.config.namespace,
      app: this.config.app,
      name: this.config.name,
      region: this.config.region,
      version: this.config.version,
      cluster: this.config.cluster,
      capabilities: [...this.config.capabilities],
      labels: { ...this.config.labels },
      structured: JSON.parse(JSON.stringify(this.config.structured)),
    };
  }

  startDryRun(processor: TaskProcessor): void {
    if (!processor) throw new Error("tikeo task processor is required");
    this.open = true;
  }

  nextHeartbeat(workerId: string, fencingToken: string, generation: number): Heartbeat {
    if (!this.open) throw new Error("tikeo worker client is not started");
    if (!workerId) throw new Error("tikeo worker id is required");
    this.seq += 1;
    return { workerId, sequence: this.seq, generation, fencingToken, sentAt: new Date() };
  }

  close(): void { this.open = false; }

  connectGrpc(): any {
    sdkLog("info", `connecting worker tunnel endpoint=${this.config.endpoint} client_instance_id=${this.config.clientInstanceId}`);
    const proto = loadProto();
    return new proto.tikeo.worker.v1.WorkerTunnelService(grpcTarget(this.config.endpoint), grpc.credentials.createInsecure());
  }

  async connect(): Promise<Session> {
    const client = this.connectGrpc();
    const call = client.OpenTunnel();
    try {
      call.write({ register: this.registerMessage() });
      const ack = await nextStreamMessage(call);
      const registered = ack?.registered;
      if (!registered?.workerId) throw new Error("tikeo worker expected registration ack");
      sdkLog("info", `registered worker_id=${registered.workerId} lease_seconds=${registered.leaseSeconds ?? 0} generation=${registered.generation ?? 0}`);
      return new Session(client, call, registered.workerId, Number(registered.leaseSeconds ?? 0), Number(registered.generation ?? 0), registered.fencingToken ?? "", this.config.heartbeatEveryMs);
    } catch (error) {
      closeGrpcCall(call);
      closeGrpcClient(client);
      throw error;
    }
  }

  private registerMessage(): any {
    return {
      clientInstanceId: this.config.clientInstanceId,
      app: this.config.app,
      namespace: this.config.namespace,
      cluster: this.config.cluster,
      region: this.config.region,
      capabilities: [...this.config.capabilities],
      labels: { ...this.config.labels },
      structuredCapabilities: {
        tags: [...this.config.structured.tags],
        sdkProcessors: this.config.structured.sdkProcessors.map((name) => ({ name })),
        scriptRunners: this.config.structured.scriptRunners.map((runner) => ({ language: runner.language, sandboxBackend: runner.sandboxBackend })),
        pluginProcessors: this.config.structured.pluginProcessors.map((plugin) => ({ type: plugin.type, processorNames: [...plugin.processorNames] })),
      },
      election: { enabled: true, priority: 100 },
    };
  }
}

export class Session {
  private sequence = 0;
  private logSequence = 0;
  constructor(private client: any, private call: any, public workerId: string, public leaseSeconds: number, public generation: number, private fencingToken: string, private heartbeatEveryMs: number) {}

  sendHeartbeat(): number {
    this.sequence += 1;
    sdkLog("debug", `sending heartbeat worker_id=${this.workerId} sequence=${this.sequence}`);
    this.call.write({ heartbeat: { workerId: this.workerId, sequence: this.sequence, generation: this.generation, fencingToken: this.fencingToken } });
    return this.sequence;
  }

  startHeartbeat(): () => void {
    this.sendHeartbeat();
    const id = setInterval(() => this.sendHeartbeat(), this.heartbeatEveryMs);
    return () => clearInterval(id);
  }

  emitTaskLog(instanceId: string, assignmentToken: string, level: string, message: string): number {
    this.logSequence += 1;
    this.call.write({ taskLog: { workerId: this.workerId, instanceId, level: level || "info", message, sequence: this.logSequence, assignmentToken } });
    return this.logSequence;
  }

  async processNext(processor: TaskProcessor, scripts?: ScriptRunnerRegistry): Promise<TaskOutcome> {
    while (true) {
      const message = await nextStreamMessage(this.call);
      const task = message?.dispatchTask ?? message?.dispatch_task;
      if (!task?.instanceId && !task?.instance_id) continue;
      const normalized = normalizeDispatchTask(task);
      this.emitTaskLogSafely(normalized, "info", `received task ${normalized.instanceId} processor=${normalized.processorName}`);
      const outcome = await processDispatchTask(processor, scripts, normalized, (level, msg) => this.emitTaskLogSafely(normalized, level, msg));
      const level = outcome.success ? "info" : "error";
      this.emitTaskLogSafely(normalized, level, `completed task ${normalized.instanceId} success=${outcome.success} message=${outcome.message}`);
      this.call.write({ taskResult: { workerId: this.workerId, instanceId: normalized.instanceId, success: outcome.success, message: outcome.message, assignmentToken: normalized.assignmentToken } });
      return outcome;
    }
  }

  close(): void {
    try {
      this.call.write({ unregister: { workerId: this.workerId, generation: this.generation, fencingToken: this.fencingToken } });
      this.call.end();
    } finally {
      closeGrpcCall(this.call);
      closeGrpcClient(this.client);
    }
  }

  private emitTaskLogSafely(task: any, level: string, message: string): void {
    printTaskLogLocally(level, message);
    try {
      this.emitTaskLog(task.instanceId, task.assignmentToken, level, message);
    } catch (error) {
      sdkLog("warning", `failed to emit task log instance_id=${task.instanceId} error=${(error as Error).message}`);
    }
  }
}

export async function processDispatchTask(processor: TaskProcessor, scripts: ScriptRunnerRegistry | undefined, task: any, log: (level: string, message: string) => void): Promise<TaskOutcome> {
  try {
    const script = task.processorBinding?.script;
    if (script) {
      const runner = scripts?.get(script.language);
      if (!runner) {
        sdkLog("warning", `missing script runner language=${script.language}`);
        return failed(`script runner is not registered for language: ${script.language}`);
      }
      return await runner.run({
        scriptId: script.scriptId,
        versionId: script.versionId,
        versionNumber: Number(script.versionNumber),
        language: script.language,
        content: script.content,
        contentSha256: script.contentSha256,
        timeoutMs: Number(script.timeoutMs || 30_000),
        maxOutputBytes: Number(script.maxOutputBytes || 1024 * 1024),
        allowNetwork: Boolean(script.allowNetwork),
        allowedEnvVars: script.allowedEnvVars ?? [],
        readOnlyPaths: script.readOnlyPaths ?? [],
        writablePaths: script.writablePaths ?? [],
        secretRefs: script.secretRefs ?? [],
        allowedNetworkHosts: script.allowedNetworkHosts ?? [],
        sandboxBackend: script.sandboxBackend ?? "",
        instanceId: task.instanceId,
        jobId: task.jobId,
        log,
      });
    }
    const context = new TaskContext(task.instanceId, task.jobId, task.processorName || task.jobId, task.payload ?? new Uint8Array(), log);
    return await runWithTaskLogScope({ instanceId: context.instanceId, jobId: context.jobId, processorName: context.processorName, log }, () => processor(context));
  } catch (error) {
    const message = processorErrorMessage(error);
    const stack = processorErrorStack(error);
    sdkLog("error", `processor failed instance_id=${task.instanceId ?? ""} error=${message}`);
    log("error", stack);
    return failed(message);
  }
}

function processorErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) return error.message;
  return String(error);
}

function processorErrorStack(error: unknown): string {
  if (error instanceof Error && error.stack) return error.stack;
  return `Error: ${processorErrorMessage(error)}`;
}

export function printTaskLogLocally(level: string, message: string): void {
  runWithoutTaskLogBridgeCapture(() => {
    const line = `[tikeo-worker] ${message}`;
    if (level.toLowerCase() === "error") console.error(line);
    else console.log(line);
  });
}

export function grpcTarget(endpoint: string): string {
  const value = endpoint.trim();
  if (!value) throw new Error("tikeo worker endpoint is required");
  try {
    const url = new URL(value);
    if (url.protocol === "http:" || url.protocol === "https:") return url.host;
  } catch {
    return value;
  }
  return value;
}

function loadProto(): any {
  const here = dirname(fileURLToPath(import.meta.url));
  const protoPath = join(here, "proto", "worker.proto");
  const definition = protoLoader.loadSync(protoPath, { keepCase: false, longs: Number, enums: String, defaults: true, oneofs: true });
  return grpc.loadPackageDefinition(definition) as any;
}

function closeGrpcCall(call: any): void {
  call.cancel?.();
  call.destroy?.();
}

function closeGrpcClient(client: any): void {
  client.close?.();
}

function nextStreamMessage(call: any): Promise<any> {
  return new Promise((resolve, reject) => {
    const cleanup = (fn: (value: any) => void, value: any) => {
      call.off?.("data", onData);
      call.off?.("error", onError);
      call.off?.("end", onEnd);
      fn(value);
    };
    const onData = (message: any) => cleanup(resolve, message);
    const onError = (error: Error) => cleanup(reject, error);
    const onEnd = () => cleanup(reject, new Error("worker tunnel closed"));
    call.once("data", onData);
    call.once("error", onError);
    call.once("end", onEnd);
  });
}

function normalizeDispatchTask(task: any): any {
  return {
    instanceId: task.instanceId ?? task.instance_id,
    jobId: task.jobId ?? task.job_id,
    payload: task.payload ?? new Uint8Array(),
    processorName: task.processorName ?? task.processor_name ?? task.jobId ?? task.job_id,
    processorBinding: task.processorBinding ?? task.processor_binding,
    assignmentToken: task.assignmentToken ?? task.assignment_token ?? "",
  };
}
