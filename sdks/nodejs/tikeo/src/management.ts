export const API_KEY_HEADER = "x-tikeo-api-key";

export interface JobRetryPolicy {
  enabled: boolean;
  maxAttempts: number;
  initialDelaySeconds: number;
  backoffMultiplier: number;
  maxDelaySeconds: number;
}

export function defaultJobRetryPolicy(): JobRetryPolicy {
  return { enabled: true, maxAttempts: 3, initialDelaySeconds: 5, backoffMultiplier: 2, maxDelaySeconds: 60 };
}

export interface JobDefinition {
  id: string;
  namespace: string;
  app: string;
  name: string;
  scheduleType: string;
  scheduleExpr?: string | null;
  processorName?: string | null;
  processorType?: string | null;
  scriptId?: string | null;
  enabled: boolean;
  retryPolicy?: JobRetryPolicy | null;
}

export interface JobInstance {
  id: string;
  jobId: string;
  status: string;
  triggerType: string;
  executionMode: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateJobRequest {
  name: string;
  scheduleType: string;
  scheduleExpr?: string | null;
  processorName?: string | null;
  processorType?: string | null;
  scriptId?: string | null;
  enabled?: boolean;
  retryPolicy?: JobRetryPolicy | null;
}

export interface BroadcastSelectorRequest {
  tags?: string[];
  region?: string;
  cluster?: string;
  labels?: Record<string, string>;
}

export interface TriggerJobRequest {
  triggerType?: string;
  executionMode?: "single" | "broadcast";
  broadcastSelector?: BroadcastSelectorRequest;
}

export function apiTrigger(): TriggerJobRequest {
  return { triggerType: "api", executionMode: "single" };
}

export function broadcastApiTrigger(selector?: BroadcastSelectorRequest): TriggerJobRequest {
  return { triggerType: "api", executionMode: "broadcast", broadcastSelector: selector };
}

export function apiJob(name: string, processorName: string): CreateJobRequest {
  return { name, scheduleType: "api", processorName, enabled: true, retryPolicy: defaultJobRetryPolicy() };
}

export function pluginApiJob(name: string, processorType: string, processorName: string): CreateJobRequest {
  return { name, scheduleType: "api", processorType, processorName, enabled: true, retryPolicy: defaultJobRetryPolicy() };
}

export function scriptApiJob(name: string, scriptId: string): CreateJobRequest {
  return { name, scheduleType: "api", scriptId, enabled: true, retryPolicy: defaultJobRetryPolicy() };
}

export class ManagementClient {
  private endpoint: string;
  constructor(endpoint: string, private apiKey: string, private namespace = "default", private app = "default") {
    this.endpoint = endpoint.trim().replace(/\/$/, "");
    this.namespace = namespace.trim() || "default";
    this.app = app.trim() || "default";
  }

  async listJobs(): Promise<JobDefinition[]> {
    const data = await this.send("GET", "/jobs");
    const items = Array.isArray(data?.items) ? data.items : [];
    return items.filter((job: JobDefinition) => job.namespace === this.namespace && job.app === this.app);
  }

  async createJob(request: CreateJobRequest): Promise<JobDefinition> {
    const payload: Record<string, unknown> = {
      namespace: this.namespace,
      app: this.app,
      name: request.name,
      scheduleType: request.scheduleType,
      scheduleExpr: request.scheduleExpr,
      processorName: request.processorName,
      processorType: request.processorType,
      scriptId: request.scriptId,
      enabled: request.enabled,
      retryPolicy: request.retryPolicy,
    };
    for (const key of Object.keys(payload)) if (payload[key] === undefined || payload[key] === null) delete payload[key];
    return this.send("POST", "/jobs", payload) as Promise<JobDefinition>;
  }

  async triggerJob(jobId: string, request: TriggerJobRequest = apiTrigger()): Promise<JobInstance> {
    return this.send("POST", `/jobs/${encodeURIComponent(jobId)}:trigger`, request) as Promise<JobInstance>;
  }

  private async send(method: string, path: string, body?: unknown): Promise<any> {
    const response = await fetch(`${this.endpoint}/api/v1${path}`, {
      method,
      headers: { accept: "application/json", [API_KEY_HEADER]: this.apiKey, ...(body ? { "content-type": "application/json" } : {}) },
      body: body ? JSON.stringify(body) : undefined,
    });
    const envelope = await response.json();
    if (!response.ok || envelope.code !== 0) throw new Error(`tikeo management request failed: status=${response.status} message=${envelope.message ?? ""}`);
    if (envelope.data === undefined || envelope.data === null) throw new Error("tikeo management response data was null");
    return envelope.data;
  }
}
