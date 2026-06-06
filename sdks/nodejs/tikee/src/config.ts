export interface ScriptRunnerCapability {
  language: string;
  sandboxBackend: string;
}

export interface PluginProcessorCapability {
  type: string;
  processorNames: string[];
}

export interface WorkerCapabilities {
  tags: string[];
  sdkProcessors: string[];
  scriptRunners: ScriptRunnerCapability[];
  pluginProcessors: PluginProcessorCapability[];
}

export interface WorkerConfigInput {
  endpoint: string;
  clientInstanceId: string;
  namespace?: string;
  app?: string;
  name?: string;
  region?: string;
  version?: string;
  cluster?: string;
  capabilities?: string[];
  labels?: Record<string, string>;
  structured?: Partial<WorkerCapabilities>;
  heartbeatEveryMs?: number;
}

export class WorkerConfig {
  endpoint: string;
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
  heartbeatEveryMs: number;

  constructor(input: WorkerConfigInput) {
    this.endpoint = input.endpoint;
    this.clientInstanceId = input.clientInstanceId;
    this.namespace = input.namespace ?? "default";
    this.app = input.app ?? "default";
    this.name = input.name || input.clientInstanceId;
    this.region = input.region ?? "local";
    this.version = input.version ?? "dev";
    this.cluster = input.cluster ?? "local";
    this.capabilities = [...(input.capabilities ?? [])];
    this.labels = { ...(input.labels ?? {}) };
    this.structured = {
      tags: [...(input.structured?.tags ?? [])],
      sdkProcessors: [...(input.structured?.sdkProcessors ?? [])],
      scriptRunners: [...(input.structured?.scriptRunners ?? [])],
      pluginProcessors: [...(input.structured?.pluginProcessors ?? [])],
    };
    this.heartbeatEveryMs = input.heartbeatEveryMs ?? 10_000;
  }

  addTag(tag: string): void { appendUnique(this.structured.tags, tag); }
  addSDKProcessor(name: string): void { appendUnique(this.structured.sdkProcessors, name); }

  addScriptRunner(language: string, sandboxBackend: string): void {
    const item = language.trim();
    if (!item || this.structured.scriptRunners.some((runner) => runner.language === item)) return;
    this.structured.scriptRunners.push({ language: item, sandboxBackend: sandboxBackend.trim() });
  }

  addPluginProcessor(type: string, processorName: string): void {
    const processorType = type.trim();
    const name = processorName.trim();
    if (!processorType || !name) return;
    const existing = this.structured.pluginProcessors.find((plugin) => plugin.type === processorType);
    if (existing) appendUnique(existing.processorNames, name);
    else this.structured.pluginProcessors.push({ type: processorType, processorNames: [name] });
  }

  validate(): void {
    const required: Record<string, string> = {
      "tikee worker endpoint": this.endpoint,
      "tikee client instance id": this.clientInstanceId,
      "tikee worker namespace": this.namespace,
      "tikee worker app": this.app,
      "tikee worker name": this.name,
      "tikee worker cluster": this.cluster,
    };
    for (const [label, value] of Object.entries(required)) {
      if (!value.trim()) throw new Error(`${label} is required`);
    }
    if (this.heartbeatEveryMs <= 0) throw new Error("tikee heartbeat interval must be positive");
  }

  normalize(): void {
    this.capabilities = normalized(this.capabilities);
    this.structured.tags = normalized(this.structured.tags);
    this.structured.sdkProcessors = normalized(this.structured.sdkProcessors);
    for (const plugin of this.structured.pluginProcessors) plugin.processorNames = normalized(plugin.processorNames);
  }
}

export function localConfig(endpoint: string, clientInstanceId: string): WorkerConfig {
  return new WorkerConfig({ endpoint, clientInstanceId });
}

function appendUnique(values: string[], value: string): void {
  const item = value.trim();
  if (item && !values.includes(item)) values.push(item);
}

function normalized(values: string[]): string[] {
  const out: string[] = [];
  for (const value of values) appendUnique(out, value);
  return out;
}
