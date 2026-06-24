export interface ProcessorCapability {
  name: string;
  description?: string;
}

export type PluginType = "sql" | "http" | "notification" | "custom";

export const PluginTypes = {
  SQL: "sql" as PluginType,
  HTTP: "http" as PluginType,
  NOTIFICATION: "notification" as PluginType,
  CUSTOM: "custom" as PluginType,
} as const;

export interface ScriptRunnerCapability {
  language: string;
  sandboxBackend: string;
}

export interface PluginProcessorCapability {
  type: PluginType;
  processors: ProcessorCapability[];
  processorNames: string[];
}

export interface WorkerCapabilities {
  tags: string[];
  normalProcessors: ProcessorCapability[];
  /** @deprecated Prefer normalProcessors. */
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
      normalProcessors: [...(input.structured?.normalProcessors ?? [])],
      scriptRunners: [...(input.structured?.scriptRunners ?? [])],
      pluginProcessors: (input.structured?.pluginProcessors ?? []).map((plugin) => ({
        type: plugin.type,
        processors: [...(plugin.processors ?? [])],
        processorNames: [...(plugin.processorNames ?? [])],
      })),
    };
    this.heartbeatEveryMs = input.heartbeatEveryMs ?? 10_000;
  }

  addTag(tag: string): void { appendUnique(this.structured.tags, tag); }
  addNormalProcessor(name: string, description = ""): void { appendUniqueProcessor(this.structured.normalProcessors, { name, description }); }
  addScriptRunner(language: string, sandboxBackend: string): void {
    const item = language.trim();
    if (!item || this.structured.scriptRunners.some((runner) => runner.language === item)) return;
    this.structured.scriptRunners.push({ language: item, sandboxBackend: sandboxBackend.trim() });
  }

  addPluginProcessor(type: PluginType, processorName: string, description = ""): void {
    const processorType = String(type).trim() as PluginType;
    if (!isPluginType(processorType)) throw new Error(`unsupported tikeo plugin processor type: ${type}`);
    const processor = cleanProcessor({ name: processorName, description });
    if (!processor) return;
    const existing = this.structured.pluginProcessors.find((plugin) => plugin.type === processorType);
    if (existing) {
      appendUniqueProcessor(existing.processors, processor);
      appendUnique(existing.processorNames, processor.name);
    } else {
      this.structured.pluginProcessors.push({ type: processorType, processors: [processor], processorNames: [processor.name] });
    }
  }

  validate(): void {
    const required: Record<string, string> = {
      "tikeo worker endpoint": this.endpoint,
      "tikeo client instance id": this.clientInstanceId,
      "tikeo worker namespace": this.namespace,
      "tikeo worker app": this.app,
      "tikeo worker name": this.name,
      "tikeo worker cluster": this.cluster,
    };
    for (const [label, value] of Object.entries(required)) {
      if (!value.trim()) throw new Error(`${label} is required`);
    }
    if (this.heartbeatEveryMs <= 0) throw new Error("tikeo heartbeat interval must be positive");
  }

  normalize(): void {
    this.capabilities = normalized(this.capabilities);
    this.structured.tags = normalized(this.structured.tags);
    this.structured.normalProcessors = normalizedProcessors(this.structured.normalProcessors);
    for (const plugin of this.structured.pluginProcessors) {
      plugin.processorNames = normalized(plugin.processorNames);
      plugin.processors = normalizedProcessors(plugin.processors);
      for (const name of plugin.processorNames) appendUniqueProcessor(plugin.processors, { name });
      plugin.processorNames = plugin.processors.map((processor) => processor.name);
    }
    this.structured.pluginProcessors = this.structured.pluginProcessors.filter((plugin) => isPluginType(plugin.type));
  }
}

export function localConfig(endpoint: string, clientInstanceId: string): WorkerConfig {
  return new WorkerConfig({ endpoint, clientInstanceId });
}

function isPluginType(value: string): value is PluginType {
  return Object.values(PluginTypes).includes(value as PluginType);
}

function cleanProcessor(value: ProcessorCapability): ProcessorCapability | undefined {
  const name = value.name.trim();
  if (!name) return undefined;
  return { name, description: value.description?.trim() ?? "" };
}

function appendUniqueProcessor(values: ProcessorCapability[], value: ProcessorCapability): void {
  const item = cleanProcessor(value);
  if (!item) return;
  const existing = values.find((processor) => processor.name === item.name);
  if (existing) {
    if (!existing.description && item.description) existing.description = item.description;
  } else {
    values.push(item);
  }
}

function normalizedProcessors(values: ProcessorCapability[]): ProcessorCapability[] {
  const out: ProcessorCapability[] = [];
  for (const value of values) appendUniqueProcessor(out, value);
  return out;
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
