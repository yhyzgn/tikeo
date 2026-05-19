export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

export interface Page<T> {
  items: T[];
  next_page_token: string | null;
}

export interface JobSummary {
  id: string;
  namespace: string;
  app: string;
  name: string;
  schedule_type: string;
  schedule_expr: string | null;
  enabled: boolean;
}

export interface CreateJobRequest {
  namespace?: string;
  app?: string;
  name: string;
  schedule_type?: string;
  schedule_expr?: string | null;
  enabled?: boolean;
}

export interface TriggerJobRequest {
  trigger_type?: string;
}

export interface JobInstanceSummary {
  id: string;
  job_id: string;
  status: string;
  trigger_type: string;
  created_at: string;
  updated_at: string;
}

const API_BASE = import.meta.env.VITE_SCHEDULER_API_BASE ?? '';

export class ApiClientError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = 'ApiClientError';
    this.code = code;
  }
}

export async function listJobs(): Promise<Page<JobSummary>> {
  return request<Page<JobSummary>>('/api/v1/jobs');
}

export async function createJob(payload: CreateJobRequest): Promise<JobSummary> {
  return request<JobSummary>('/api/v1/jobs', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function triggerJob(jobId: string, payload: TriggerJobRequest = {}): Promise<JobInstanceSummary> {
  return request<JobInstanceSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}:trigger`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function listJobInstances(jobId: string): Promise<Page<JobInstanceSummary>> {
  return request<Page<JobInstanceSummary>>(`/api/v1/jobs/${encodeURIComponent(jobId)}/instances`);
}

export async function getInstance(instanceId: string): Promise<JobInstanceSummary> {
  return request<JobInstanceSummary>(`/api/v1/instances/${encodeURIComponent(instanceId)}`);
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      'content-type': 'application/json',
      ...init.headers,
    },
  });
  const envelope = (await response.json()) as ApiResponse<T | null>;

  if (envelope.code !== 0) {
    throw new ApiClientError(envelope.code, envelope.message);
  }
  if (envelope.data === null) {
    throw new ApiClientError(-1, 'API returned null data for a non-empty operation');
  }
  return envelope.data;
}
