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
  execution_mode?: 'single' | 'broadcast';
}

export interface JobInstanceSummary {
  id: string;
  job_id: string;
  status: string;
  trigger_type: string;
  execution_mode: string;
  created_at: string;
  updated_at: string;
}

export interface JobInstanceAttemptSummary {
  id: string;
  instance_id: string;
  worker_id: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface JobInstanceLogSummary {
  id: string;
  instance_id: string;
  worker_id: string;
  level: string;
  message: string;
  sequence: number;
  created_at: string;
}

export interface UserSummary {
  id: string;
  username: string;
  role: string;
  created_at: string;
}

export interface CreateUserRequest {
  username: string;
  password?: string;
  role: string;
}

export interface UpdateUserRequest {
  password?: string;
  role?: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface AuthSession {
  token: string;
  username: string;
  roles: string[];
}

export interface MeResponse {
  username: string;
  roles: string[];
}

const API_BASE = import.meta.env.VITE_SCHEDULER_API_BASE ?? '';
const TOKEN_STORAGE_KEY = 'scheduler.auth.token';
let authToken: string | null = readStoredToken();

export class ApiClientError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = 'ApiClientError';
    this.code = code;
  }
}

export function getAuthToken(): string | null {
  return authToken;
}

export function setAuthToken(token: string | null): void {
  authToken = token;
  if (typeof localStorage === 'undefined') {
    return;
  }
  if (token === null) {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
  } else {
    localStorage.setItem(TOKEN_STORAGE_KEY, token);
  }
}

export async function login(payload: LoginRequest): Promise<AuthSession> {
  const session = await request<AuthSession>('/api/v1/auth/login', {
    method: 'POST',
    body: JSON.stringify(payload),
    auth: false,
  });
  setAuthToken(session.token);
  return session;
}

export async function me(): Promise<MeResponse> {
  return request<MeResponse>('/api/v1/auth/me');
}

export async function logout(): Promise<void> {
  await request<null>('/api/v1/auth/logout', { method: 'POST', allowNullData: true });
  setAuthToken(null);
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

export async function listInstanceAttempts(instanceId: string): Promise<Page<JobInstanceAttemptSummary>> {
  return request<Page<JobInstanceAttemptSummary>>(`/api/v1/instances/${encodeURIComponent(instanceId)}/attempts`);
}

export async function listInstanceLogs(instanceId: string): Promise<Page<JobInstanceLogSummary>> {
  return request<Page<JobInstanceLogSummary>>(`/api/v1/instances/${encodeURIComponent(instanceId)}/logs`);
}

export async function listUsers(): Promise<UserSummary[]> {
  return request<UserSummary[]>('/api/v1/users');
}

export async function createUser(params: CreateUserRequest): Promise<UserSummary> {
  return request<UserSummary>('/api/v1/users', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

export async function updateUser(id: string, params: UpdateUserRequest): Promise<UserSummary> {
  return request<UserSummary>(`/api/v1/users/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(params),
  });
}

export async function deleteUser(id: string): Promise<void> {
  await request<void>(`/api/v1/users/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    allowNullData: true,
  });
}

interface SchedulerRequestInit extends RequestInit {
  auth?: boolean;
  allowNullData?: boolean;
}

async function request<T>(path: string, init: SchedulerRequestInit = {}): Promise<T> {
  const { auth = true, allowNullData = false, headers, ...fetchInit } = init;
  const mergedHeaders = new Headers(headers);
  if (!mergedHeaders.has('content-type')) {
    mergedHeaders.set('content-type', 'application/json');
  }
  if (auth && authToken !== null && !mergedHeaders.has('authorization')) {
    mergedHeaders.set('authorization', `Bearer ${authToken}`);
  }

  const response = await fetch(`${API_BASE}${path}`, {
    ...fetchInit,
    headers: mergedHeaders,
  });
  const envelope = (await response.json()) as ApiResponse<T | null>;

  if (envelope.code !== 0) {
    throw new ApiClientError(envelope.code, envelope.message);
  }
  if (envelope.data === null) {
    if (allowNullData) {
      return null as T;
    }
    throw new ApiClientError(-1, 'API returned null data for a non-empty operation');
  }
  return envelope.data;
}

function readStoredToken(): string | null {
  if (typeof localStorage === 'undefined') {
    return null;
  }
  return localStorage.getItem(TOKEN_STORAGE_KEY);
}
