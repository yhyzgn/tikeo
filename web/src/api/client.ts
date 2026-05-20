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

export interface PermissionSummary {
  resource: string;
  action: string;
}

export interface AuthSession {
  token: string;
  username: string;
  roles: string[];
  permissions: PermissionSummary[];
}

export interface MeResponse {
  username: string;
  roles: string[];
  permissions: PermissionSummary[];
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

export interface ScriptSummary {
  id: string;
  name: string;
  language: string;
  version: string;
  content: string;
  status: string;
  timeout_seconds: number | null;
  max_memory_bytes: number | null;
  allow_network: boolean;
  allowed_env_vars: string[] | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface CreateScriptRequest {
  name: string;
  language: string;
  version: string;
  content: string;
  timeout_seconds?: number | null;
  max_memory_bytes?: number | null;
  allow_network?: boolean;
  allowed_env_vars?: string[] | null;
}

export interface UpdateScriptRequest {
  name?: string;
  language?: string;
  version?: string;
  content?: string;
  status?: string;
  timeout_seconds?: number | null;
  max_memory_bytes?: number | null;
  allow_network?: boolean;
  allowed_env_vars?: string[] | null;
}

export async function listScripts(): Promise<Page<ScriptSummary>> {
  return request<Page<ScriptSummary>>('/api/v1/scripts');
}

export async function createScript(params: CreateScriptRequest): Promise<ScriptSummary> {
  return request<ScriptSummary>('/api/v1/scripts', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

export async function getScript(id: string): Promise<ScriptSummary> {
  return request<ScriptSummary>(`/api/v1/scripts/${encodeURIComponent(id)}`);
}

export async function updateScript(id: string, params: UpdateScriptRequest): Promise<ScriptSummary> {
  return request<ScriptSummary>(`/api/v1/scripts/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(params),
  });
}

export interface ScriptVersionSummary {
  id: string;
  script_id: string;
  version_number: number;
  content: string;
  language: string;
  status: string;
  timeout_seconds: number | null;
  max_memory_bytes: number | null;
  allow_network: boolean;
  allowed_env_vars: string[] | null;
  created_by: string;
  created_at: string;
}

export interface FieldChange {
  field: string;
  before: string;
  after: string;
}

export interface ScriptDiffResult {
  content_diff: string;
  policy_diff: FieldChange[];
}

export async function listScriptVersions(id: string): Promise<ScriptVersionSummary[]> {
  return request<ScriptVersionSummary[]>(`/api/v1/scripts/${encodeURIComponent(id)}/versions`);
}

export async function diffScriptVersions(id: string, v1: number, v2: number): Promise<ScriptDiffResult> {
  return request<ScriptDiffResult>(`/api/v1/scripts/${encodeURIComponent(id)}/diff?v1=${v1}&v2=${v2}`);
}

export async function deleteScript(id: string): Promise<void> {
  await request<void>(`/api/v1/scripts/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    allowNullData: true,
  });
}

export interface AuditLogSummary {
  id: string;
  actor: string;
  action: string;
  resource_type: string;
  resource_id: string;
  detail: string | null;
  ip_address: string | null;
  created_at: string;
}

export async function listAuditLogs(): Promise<Page<AuditLogSummary>> {
  return request<Page<AuditLogSummary>>('/api/v1/audit-logs');
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

export interface WorkflowNodeSpec {
  key: string;
  name?: string | null;
  kind?: 'job' | 'map' | 'map_reduce' | 'sub_workflow' | string | null;
  job_id?: string | null;
  child_workflow_id?: string | null;
  map_items?: unknown[] | null;
  config?: unknown;
}

export interface WorkflowEdgeSpec {
  from: string;
  to: string;
  condition?: 'always' | 'on_success' | 'on_failure' | null;
}

export interface WorkflowDefinition {
  nodes: WorkflowNodeSpec[];
  edges: WorkflowEdgeSpec[];
}

export interface WorkflowSummary {
  id: string;
  name: string;
  definition: WorkflowDefinition;
  status: string;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface WorkflowValidationResult {
  valid: boolean;
  errors: string[];
}

export interface WorkflowDryRunResponse {
  validation: WorkflowValidationResult;
  start_nodes: string[];
  node_count: number;
  edge_count: number;
}

export interface WorkflowNodeInstanceSummary {
  id: string;
  workflow_instance_id: string;
  node_key: string;
  status: string;
  job_instance_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface WorkflowInstanceSummary {
  id: string;
  workflow_id: string;
  status: string;
  trigger_type: string;
  nodes: WorkflowNodeInstanceSummary[];
  created_at: string;
  updated_at: string;
}

export interface WorkflowAdvanceResult {
  instance: WorkflowInstanceSummary;
  queued_nodes: string[];
  completed: boolean;
}

export interface InstanceEventSummary {
  id: string;
  instance_id: string;
  instance_type: string;
  event_type: string;
  message: string;
  payload: string | null;
  created_at: string;
}

export async function listWorkflows(): Promise<WorkflowSummary[]> {
  return request<WorkflowSummary[]>('/api/v1/workflows');
}

export async function createWorkflow(payload: { name: string; definition: WorkflowDefinition }): Promise<WorkflowSummary> {
  return request<WorkflowSummary>('/api/v1/workflows', { method: 'POST', body: JSON.stringify(payload) });
}

export async function validateWorkflow(id: string): Promise<WorkflowValidationResult> {
  return request<WorkflowValidationResult>(`/api/v1/workflows/${encodeURIComponent(id)}/validate`, { method: 'POST', body: JSON.stringify({}) });
}

export async function dryRunWorkflow(definition: WorkflowDefinition): Promise<WorkflowDryRunResponse> {
  return request<WorkflowDryRunResponse>('/api/v1/workflows/dry-run', { method: 'POST', body: JSON.stringify(definition) });
}

export async function runWorkflow(id: string): Promise<WorkflowInstanceSummary> {
  return request<WorkflowInstanceSummary>(`/api/v1/workflows/${encodeURIComponent(id)}/run`, { method: 'POST', body: JSON.stringify({ trigger_type: 'api' }) });
}

export async function advanceWorkflowInstance(instanceId: string, payload: { node_key: string; status: string; message?: string }): Promise<WorkflowAdvanceResult> {
  return request<WorkflowAdvanceResult>(`/api/v1/workflow-instances/${encodeURIComponent(instanceId)}/advance`, { method: 'POST', body: JSON.stringify(payload) });
}

export function workflowEventStreamUrl(instanceId: string): string {
  return `${API_BASE}/api/v1/events/instances/${encodeURIComponent(instanceId)}/stream`;
}
