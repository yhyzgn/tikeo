export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

export interface Page<T> {
  items: T[];
  nextPageToken: string | null;
}


export interface PluginProcessorTypeSummary {
  type: string;
  label: string;
  capability: string;
  processorNames: string[];
  description: string | null;
  artifactRef?: string | null;
  containerImage?: string | null;
  entrypoint?: string[] | null;
  checksum?: string | null;
}

export interface PluginAlertChannelTypeSummary {
  type: string;
  label: string;
  targetKind: string;
  description: string | null;
  template: Record<string, unknown>;
}

export interface PluginSummary {
  id: string;
  name: string;
  kind: string;
  processorTypes: PluginProcessorTypeSummary[];
  alertChannelTypes: PluginAlertChannelTypeSummary[];
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreatePluginRequest {
  name: string;
  kind: string;
  processorTypes: PluginProcessorTypeSummary[];
  alertChannelTypes: PluginAlertChannelTypeSummary[];
  enabled: boolean;
}

export type UpdatePluginRequest = CreatePluginRequest;



export interface CalendarWindowSummary {
  start: string;
  end: string;
}

export interface CalendarSummary {
  id: string;
  namespace: string;
  app: string;
  name: string;
  timezone: string;
  excludedDates: string[];
  holidays: string[];
  maintenanceWindows: CalendarWindowSummary[];
  freezeWindows: CalendarWindowSummary[];
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface UpsertCalendarRequest {
  namespace: string;
  app: string;
  name: string;
  timezone?: string | null;
  excludedDates?: string[];
  holidays?: string[];
  maintenanceWindows?: CalendarWindowSummary[];
  freezeWindows?: CalendarWindowSummary[];
}

export interface GitOpsScope {
  namespace: string | null;
  app: string | null;
}

export interface GitOpsMetadata {
  id: string | null;
  name: string;
  namespace: string | null;
  app: string | null;
}

export interface GitOpsResource {
  kind: string;
  metadata: GitOpsMetadata;
  spec: Record<string, unknown>;
}

export interface GitOpsManifest {
  apiVersion: string;
  kind: string;
  scope: GitOpsScope;
  resources: GitOpsResource[];
}

export interface GitOpsManifestResponse {
  manifest: GitOpsManifest;
  format: string;
  manifestYaml: string | null;
  checksum: string;
}

export interface GitOpsDiffChange {
  action: string;
  key: string;
  kind: string;
  name: string;
  before: GitOpsResource | null;
  after: GitOpsResource | null;
  diff: string;
}

export interface GitOpsDiffResponse {
  currentChecksum: string;
  desiredChecksum: string;
  summary: Record<string, number>;
  changes: GitOpsDiffChange[];
}

export interface ClusterStatusResponse {
  mode: string;
  role: string;
  node_id: string;
  nodes: number;
  can_schedule: boolean;
  leader_fencing_token: string | null;
  detail: string;
}

export interface ClusterNodeDiagnostic {
  nodeId: string;
  endpoint: string;
  memberStatus: string;
  currentTerm: number | null;
  commitIndex: number | null;
  appliedIndex: number | null;
  leaderFencingToken: string | null;
  isRespondingNode: boolean;
  canSchedule: boolean;
}

export interface SmartGatewayDiagnostic {
  mode: 'diagnostic_safe_optimization' | string;
  status: 'ready' | 'degraded' | 'idle' | string;
  localGatewayNodeId: string;
  onlineWorkers: number;
  localGatewayWorkers: number;
  remoteGatewayWorkers: number;
  outboxTotal: number;
  queuedOrReroutePending: number;
  oldestQueuedAgeSeconds: number;
  safetyBoundary: string;
}

export interface ClusterDiagnosticsResponse {
  respondingNode: ClusterStatusResponse;
  status: ClusterStatusResponse;
  schedulingGated: boolean;
  metadata: Record<string, unknown> | null;
  nodes: ClusterNodeDiagnostic[];
  members: Array<{ nodeId: string; endpoint: string; status: string; updatedAt: string }>;
  transport: { appendEntriesPath: string; mutating: boolean; status: string };
  runtimeBoundary: string;
  smartGateway: SmartGatewayDiagnostic;
}

export interface JobRetryPolicy {
  enabled: boolean;
  maxAttempts: number;
  initialDelaySeconds: number;
  backoffMultiplier: number;
  maxDelaySeconds: number;
}

export interface JobCanaryPolicy {
  metricsGateEnabled: boolean;
  minimumSamples: number;
  evaluationWindow: number;
  maxFailureRate: number;
  autoRollback: boolean;
}

export interface CanaryMetricsGateSummary {
  status: string;
  inspectedSamples: number;
  failedSamples: number;
  failureRate: number;
  threshold: number;
  reason: string;
}

export interface JobSummary {
  id: string;
  namespace: string;
  app: string;
  name: string;
  scheduleType: string;
  scheduleExpr: string | null;
  misfirePolicy: string;
  scheduleStartAt: string | null;
  scheduleEndAt: string | null;
  scheduleCalendar: Record<string, unknown> | null;
  processorName: string | null;
  processorType: string | null;
  scriptId: string | null;
  enabled: boolean;
  canaryJobId: string | null;
  canaryPercent: number;
  canaryPolicy: JobCanaryPolicy;
  versionNumber: number;
  retryPolicy: JobRetryPolicy;
}


export interface JobVersionSummary {
  id: string;
  job_id: string;
  version_number: number;
  name: string;
  schedule_type: string;
  schedule_expr: string | null;
  misfire_policy: string;
  schedule_start_at: string | null;
  schedule_end_at: string | null;
  processor_name: string | null;
  script_id: string | null;
  enabled: boolean;
  created_by: string;
  change_reason: string;
  rolled_back_from_version: number | null;
  created_at: string;
}

export interface JobSchedulingHistorySummary {
  inspectedInstances: number;
  completedInstances: number;
  failedInstances: number;
  averageDurationSeconds: number;
  p50DurationSeconds: number;
  p95DurationSeconds: number;
  maxDurationSeconds: number;
}

export interface JobSchedulingWorkerCapacity {
  eligibleWorkerCount: number;
  advertisedCpuCores: number;
  advertisedMemoryMb: number;
}

export interface JobSchedulingPrediction {
  estimatedDurationSeconds: number;
  recommendedConcurrency: number;
  workerCapacity: JobSchedulingWorkerCapacity;
  reasons: string[];
}

export interface JobSchedulingAdvice {
  ready: boolean;
  severity: 'ok' | 'warning' | 'error' | string;
  reason: string;
  requiredCapability: string | null;
  eligibleWorkers: string[];
  recentInstances: number;
  recentFailures: number;
  history: JobSchedulingHistorySummary;
  prediction: JobSchedulingPrediction;
}

export interface JobTopologyResponse {
  nodes: JobTopologyNode[];
  edges: JobTopologyEdge[];
  unresolved: JobTopologyUnresolvedRef[];
}

export interface JobTopologyPosition {
  x: number;
  y: number;
}

export interface JobTopologyMetadata {
  layer?: number;
  position?: JobTopologyPosition;
  [key: string]: unknown;
}

export interface JobTopologyNode {
  id: string;
  type: 'job' | 'workflow' | 'workflow_node' | string;
  label: string;
  namespace: string | null;
  app: string | null;
  metadata: JobTopologyMetadata;
}

export interface JobTopologyEdge {
  id: string;
  from: string;
  to: string;
  type: 'workflow_job_ref' | 'workflow_job_dependency' | string;
  label: string | null;
  workflowId: string | null;
  workflowName: string | null;
  condition: string | null;
  metadata: Record<string, unknown>;
}

export interface JobTopologyUnresolvedRef {
  workflowId: string;
  workflowName: string;
  nodeKey: string;
  missingJobId: string;
  reason: string;
}

export interface JobImpactJobRef {
  id: string;
  name: string;
  namespace?: string;
  app?: string;
}

export interface JobImpactWorkflowRef {
  id: string;
  name: string;
  nodeKeys?: string[];
}

export interface JobImpactRiskSummary {
  workflowCount: number;
  upstreamCount: number;
  downstreamCount: number;
  unresolvedCount: number;
  riskLevel: string;
  reasons: string[];
}

export interface JobImpactResponse {
  targetJob: JobImpactJobRef;
  referencingWorkflows: JobImpactWorkflowRef[];
  upstreamJobs: JobImpactJobRef[];
  downstreamJobs: JobImpactJobRef[];
  riskSummary: JobImpactRiskSummary;
}

export interface CreateJobRequest {
  namespace?: string;
  app?: string;
  name: string;
  scheduleType?: string;
  scheduleExpr?: string | null;
  misfirePolicy?: string | null;
  scheduleStartAt?: string | null;
  scheduleEndAt?: string | null;
  scheduleCalendar?: Record<string, unknown> | null;
  processorName?: string | null;
  processorType?: string | null;
  scriptId?: string | null;
  enabled?: boolean;
  canaryJobId?: string | null;
  canaryPercent?: number;
  canaryPolicy?: JobCanaryPolicy;
  retryPolicy?: JobRetryPolicy;
}

export interface UpdateJobRequest {
  namespace?: string;
  app?: string;
  name?: string;
  scheduleType?: string;
  scheduleExpr?: string | null;
  misfirePolicy?: string | null;
  scheduleStartAt?: string | null;
  scheduleEndAt?: string | null;
  scheduleCalendar?: Record<string, unknown> | null;
  processorName?: string | null;
  processorType?: string | null;
  scriptId?: string | null;
  enabled?: boolean;
  canaryJobId?: string | null;
  canaryPercent?: number;
  canaryPolicy?: JobCanaryPolicy;
  retryPolicy?: JobRetryPolicy;
}

export interface InboundWebhookTriggerRequest {
  source?: string;
  eventType?: string;
  payload?: unknown;
}

export interface InboundWebhookTriggerResponse {
  accepted: boolean;
  instanceId: string;
  jobId: string;
  status: string;
  triggerType: string;
}

export interface BroadcastSelectorRequest {
  tags?: string[];
  region?: string;
  cluster?: string;
  labels?: Record<string, string>;
}

export interface TriggerJobRequest {
  triggerType?: string;
  executionMode?: 'single' | 'broadcast';
  broadcastSelector?: BroadcastSelectorRequest;
}

export interface CanaryRoutingSummary {
  enabled: boolean;
  routed: boolean;
  originalJobId: string;
  routedJobId: string;
  percent: number;
  rolledBack?: boolean;
  metricsGate?: CanaryMetricsGateSummary | null;
}

export interface JobInstanceResult {
  workerId: string;
  success: boolean;
  message: string;
  completedAt: string;
}

export interface JobInstanceSummary {
  id: string;
  jobId: string;
  status: string;
  triggerType: string;
  executionMode: string;
  createdAt: string;
  updatedAt: string;
  logCount: number;
  latestLog?: JobInstanceLogSummary | null;
  workerId?: string | null;
  result?: JobInstanceResult | null;
  canaryRouting?: CanaryRoutingSummary | null;
}

export interface JobInstanceAttemptSummary {
  id: string;
  instanceId: string;
  workerId: string;
  status: string;
  result?: JobInstanceResult | null;
  createdAt: string;
  updatedAt: string;
}

export interface JobInstanceLogSummary {
  id: string;
  instanceId: string;
  workerId: string;
  level: string;
  message: string;
  governanceEvent?: string | null;
  governanceFailureClass?: string | null;
  governanceMessage?: string | null;
  sequence: number;
  createdAt: string;
}


export interface NamespaceSummary {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface AppScopeSummary {
  id: string;
  namespace: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface WorkerPoolSummary {
  id: string;
  namespace: string;
  app: string;
  name: string;
  maxQueueDepth: number;
  maxConcurrency: number;
  createdAt: string;
  updatedAt: string;
}

export interface SecretSummary {
  id: string;
  namespace: string;
  app: string;
  name: string;
  valueRef: string;
  status: string;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateNamespaceRequest { name: string }
export interface CreateAppScopeRequest { namespace: string; name: string }
export interface CreateWorkerPoolRequest { namespace: string; app: string; name: string }
export type SecretReferenceRequest =
  | { kind: 'env'; name: string }
  | { kind: 'vault'; path: string; key: string }
  | { kind: 'secret'; provider: string; id: string; key?: string | null };
export interface CreateSecretRequest { namespace: string; app: string; name: string; reference: SecretReferenceRequest }
export interface UpdateWorkerPoolQuotaRequest { maxQueueDepth: number; maxConcurrency: number }

export interface UserSummary {
  id: string;
  username: string;
  email: string;
  role: string;
  bootstrapAdmin: boolean;
  createdAt: string;
}

export interface CreateUserRequest {
  username: string;
  email: string;
  password?: string;
  role: string;
}

export interface UpdateUserRequest {
  email?: string;
  password?: string;
  role?: string;
}

export interface BootstrapStatusResponse {
  initialized: boolean;
  registrationOpen: boolean;
  bootstrapAdminUsername: string | null;
}

export interface BootstrapRegisterRequest {
  username: string;
  email: string;
  password: string;
  confirmPassword: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface PermissionSummary {
  resource: string;
  action: string;
}


export interface RoleSummary {
  id: string;
  name: string;
  displayName: string;
  description: string;
  builtin: boolean;
  enabled: boolean;
  assignable: boolean;
  permissions: PermissionSummary[];
  menuKeys: string[];
  uiActionKeys: string[];
  createdAt: string;
  updatedAt: string;
}

export interface PermissionCatalogItem {
  id: string;
  resource: string;
  action: string;
  description: string;
}

export interface MenuPermissionCatalogItem {
  key: string;
  label: string;
  group: string;
  routePath: string;
  requiredPermission: PermissionSummary | null;
}

export interface UiActionPermissionCatalogItem {
  key: string;
  label: string;
  pageKey: string;
  operation: string;
  dangerous: boolean;
  requiredPermission: PermissionSummary | null;
}

export interface CreateRoleRequest {
  name: string;
  displayName: string;
  description?: string | null;
  enabled: boolean;
  permissionIds: string[];
  menuKeys: string[];
  uiActionKeys: string[];
}

export type UpdateRoleRequest = Omit<CreateRoleRequest, 'name'>;

export interface AuthSession {
  token: string;
  username: string;
  roles: string[];
  permissions: PermissionSummary[];
  bootstrap_admin: boolean;
  scope_limited: boolean;
  token_scopes: string[];
  scope_bindings: AccessScopeBinding[];
  menu_keys: string[];
  ui_action_keys: string[];
}

export interface AccessScopeBinding {
  namespace?: string | null;
  app?: string | null;
  worker_pool?: string | null;
}

export interface MeResponse {
  username: string;
  roles: string[];
  permissions: PermissionSummary[];
  bootstrap_admin: boolean;
  scope_limited: boolean;
  token_scopes: string[];
  scope_bindings: AccessScopeBinding[];
  menu_keys: string[];
  ui_action_keys: string[];
}


export interface SdkApiKeySummary {
  id: string;
  name: string;
  key_prefix: string;
  namespace: string;
  app: string;
  service_account_id: string;
  service_account_name: string;
  scopes: string[];
  status: string;
  expires_at: string | null;
  last_used_at: string | null;
  created_by: string;
  revoked_by: string | null;
  rotated_from: string | null;
  created_at: string;
  updated_at: string;
}

export interface ServiceAccountSummary {
  id: string;
  name: string;
  description: string | null;
  namespace: string;
  app: string;
  workerPool: string | null;
  status: string;
  createdBy: string;
  updatedBy: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateServiceAccountRequest {
  name: string;
  description?: string | null;
  namespace: string;
  app: string;
  workerPool?: string | null;
}

export interface UpdateServiceAccountRequest extends CreateServiceAccountRequest {
  status: string;
}

export interface CreateSdkApiKeyRequest {
  name: string;
  namespace: string;
  app: string;
  service_account_id: string;
  scopes: string[];
  expires_at?: string | null;
}

export interface CreatedSdkApiKey {
  key: SdkApiKeySummary;
  api_key: string;
}

export interface UpdateSdkApiKeyRequest {
  name: string;
  scopes: string[];
  expires_at?: string | null;
}

export interface OidcIdentitySummary {
  id: string;
  issuer: string;
  subject: string;
  username: string;
  namespace: string | null;
  app: string | null;
  worker_pool: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface UpsertOidcIdentityRequest {
  issuer: string;
  subject: string;
  username: string;
  namespace?: string | null;
  app?: string | null;
  worker_pool?: string | null;
}

export const API_BASE = import.meta.env.VITE_TIKEO_API_BASE ?? '';
const TOKEN_STORAGE_KEY = 'tikeo.auth.token';
let authToken: string | null = readStoredToken();

export class ApiClientError extends Error {
  readonly code: number;
  readonly status: number;

  constructor(code: number, message: string, status = 0) {
    super(message);
    this.name = 'ApiClientError';
    this.code = code;
    this.status = status;
  }
}

export interface AuthErrorHandler {
  onUnauthorized?: () => void;
  onForbidden?: (message: string) => void;
}

let authErrorHandler: AuthErrorHandler | null = null;

export function setAuthErrorHandler(handler: AuthErrorHandler | null): void {
  authErrorHandler = handler;
}

export function getAuthToken(): string | null {
  return authToken;
}

function streamUrl(path: string): string {
  const token = getAuthToken();
  const separator = path.includes('?') ? '&' : '?';
  return token === null ? `${API_BASE}${path}` : `${API_BASE}${path}${separator}token=${encodeURIComponent(token)}`;
}

export function instanceLogStreamUrl(instanceId: string): string {
  return streamUrl(`/api/v1/instances/${encodeURIComponent(instanceId)}/logs/stream`);
}

export function instanceListStreamUrl(): string {
  return streamUrl('/api/v1/instances/stream');
}

export function workerStreamUrl(): string {
  return streamUrl('/api/v1/workers/stream');
}

export function dispatchQueueStreamUrl(): string {
  return streamUrl('/api/v1/dispatch-queue/stream');
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

export async function getBootstrapStatus(): Promise<BootstrapStatusResponse> {
  return request<BootstrapStatusResponse>('/api/v1/auth/bootstrap', { auth: false });
}

export async function registerBootstrapAdmin(payload: BootstrapRegisterRequest): Promise<AuthSession> {
  const session = await request<AuthSession>('/api/v1/auth/bootstrap/register', {
    method: 'POST',
    body: JSON.stringify(payload),
    auth: false,
  });
  setAuthToken(session.token);
  return session;
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

export async function listPlugins(): Promise<PluginSummary[]> {
  return request<PluginSummary[]>('/api/v1/plugins');
}

export async function createPlugin(payload: CreatePluginRequest): Promise<PluginSummary> {
  return request<PluginSummary>('/api/v1/plugins', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function updatePlugin(id: string, payload: UpdatePluginRequest): Promise<PluginSummary> {
  return request<PluginSummary>(`/api/v1/plugins/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function deletePlugin(id: string): Promise<void> {
  await request<void>(`/api/v1/plugins/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}




export async function listCalendars(params: { namespace?: string; app?: string } = {}): Promise<CalendarSummary[]> {
  const query = new URLSearchParams();
  if (params.namespace) query.set('namespace', params.namespace);
  if (params.app) query.set('app', params.app);
  const suffix = query.toString() ? `?${query}` : '';
  return request<CalendarSummary[]>(`/api/v1/calendars${suffix}`);
}

export async function createCalendar(payload: UpsertCalendarRequest): Promise<CalendarSummary> {
  return request<CalendarSummary>('/api/v1/calendars', { method: 'POST', body: JSON.stringify(payload) });
}

export async function deleteCalendar(id: string): Promise<void> {
  await request<void>(`/api/v1/calendars/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}

export async function exportGitOpsManifest(params: { namespace?: string; app?: string; format?: 'json' | 'yaml' } = {}): Promise<GitOpsManifestResponse> {
  const query = new URLSearchParams();
  if (params.namespace) query.set('namespace', params.namespace);
  if (params.app) query.set('app', params.app);
  if (params.format) query.set('format', params.format);
  const suffix = query.toString() ? `?${query}` : '';
  return request<GitOpsManifestResponse>(`/api/v1/gitops/manifest${suffix}`);
}

export async function diffGitOpsManifest(manifest: GitOpsManifest): Promise<GitOpsDiffResponse> {
  return request<GitOpsDiffResponse>('/api/v1/gitops/diff', {
    method: 'POST',
    body: JSON.stringify({ manifest }),
  });
}

export async function getClusterDiagnostics(): Promise<ClusterDiagnosticsResponse> {
  return request<ClusterDiagnosticsResponse>('/api/v1/cluster/diagnostics');
}


export async function listServiceAccounts(): Promise<ServiceAccountSummary[]> {
  return request<ServiceAccountSummary[]>('/api/v1/management/service-accounts');
}

export async function createServiceAccount(payload: CreateServiceAccountRequest): Promise<ServiceAccountSummary> {
  return request<ServiceAccountSummary>('/api/v1/management/service-accounts', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function updateServiceAccount(id: string, payload: UpdateServiceAccountRequest): Promise<ServiceAccountSummary> {
  return request<ServiceAccountSummary>(`/api/v1/management/service-accounts/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function disableServiceAccount(id: string): Promise<void> {
  await request<void>(`/api/v1/management/service-accounts/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    allowNullData: true,
  });
}

export async function listSdkApiKeys(): Promise<SdkApiKeySummary[]> {
  return request<SdkApiKeySummary[]>('/api/v1/management/api-keys');
}

export async function createSdkApiKey(payload: CreateSdkApiKeyRequest): Promise<CreatedSdkApiKey> {
  return request<CreatedSdkApiKey>('/api/v1/management/api-keys', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function updateSdkApiKey(id: string, payload: UpdateSdkApiKeyRequest): Promise<SdkApiKeySummary> {
  return request<SdkApiKeySummary>(`/api/v1/management/api-keys/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function deleteSdkApiKey(id: string): Promise<void> {
  await request<void>(`/api/v1/management/api-keys/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}

export async function listOidcIdentities(): Promise<OidcIdentitySummary[]> {
  return request<OidcIdentitySummary[]>('/api/v1/oidc-identities');
}

export async function upsertOidcIdentity(payload: UpsertOidcIdentityRequest): Promise<OidcIdentitySummary> {
  return request<OidcIdentitySummary>('/api/v1/oidc-identities', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function deleteOidcIdentity(id: string): Promise<void> {
  await request<void>(`/api/v1/oidc-identities/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}

export async function listJobs(): Promise<Page<JobSummary>> {
  return request<Page<JobSummary>>('/api/v1/jobs');
}

export async function getJobSchedulingAdvice(jobId: string): Promise<JobSchedulingAdvice> {
  return request<JobSchedulingAdvice>(`/api/v1/jobs/${encodeURIComponent(jobId)}/scheduling-advice`);
}

export async function getJobTopology(): Promise<JobTopologyResponse> {
  return request<JobTopologyResponse>('/api/v1/jobs/topology');
}

export async function getJobImpact(jobId: string): Promise<JobImpactResponse> {
  return request<JobImpactResponse>(`/api/v1/jobs/${encodeURIComponent(jobId)}/impact`);
}

export async function createJob(payload: CreateJobRequest): Promise<JobSummary> {
  return request<JobSummary>('/api/v1/jobs', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function updateJob(jobId: string, payload: UpdateJobRequest): Promise<JobSummary> {
  return request<JobSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function listJobVersions(jobId: string): Promise<Page<JobVersionSummary>> {
  return request<Page<JobVersionSummary>>(`/api/v1/jobs/${encodeURIComponent(jobId)}/versions`);
}

export async function rollbackJob(jobId: string, versionNumber: number): Promise<JobSummary> {
  return request<JobSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}/rollback`, {
    method: 'POST',
    body: JSON.stringify({ versionNumber }),
  });
}

export async function deleteJob(jobId: string): Promise<void> {
  await request<void>(`/api/v1/jobs/${encodeURIComponent(jobId)}`, { method: 'DELETE', allowNullData: true });
}

export async function triggerJob(jobId: string, payload: TriggerJobRequest = {}): Promise<JobInstanceSummary> {
  return request<JobInstanceSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}:trigger`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function triggerJobWebhookEvent(jobId: string, payload: InboundWebhookTriggerRequest): Promise<InboundWebhookTriggerResponse> {
  return request<InboundWebhookTriggerResponse>(`/api/v1/events/webhooks/${encodeURIComponent(jobId)}:trigger`, {
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

export async function cancelInstance(instanceId: string): Promise<JobInstanceSummary> {
  return request<JobInstanceSummary>(`/api/v1/instances/${encodeURIComponent(instanceId)}/cancel`, { method: 'POST', body: JSON.stringify({}) });
}

export async function listInstanceAttempts(instanceId: string): Promise<Page<JobInstanceAttemptSummary>> {
  return request<Page<JobInstanceAttemptSummary>>(`/api/v1/instances/${encodeURIComponent(instanceId)}/attempts`);
}

export async function listInstanceLogs(instanceId: string, options: { governanceOnly?: boolean } = {}): Promise<Page<JobInstanceLogSummary>> {
  const suffix = options.governanceOnly ? '?page_token=script_execution_governance' : '';
  return request<Page<JobInstanceLogSummary>>(`/api/v1/instances/${encodeURIComponent(instanceId)}/logs${suffix}`);
}


export async function listNamespaces(): Promise<NamespaceSummary[]> {
  return request<NamespaceSummary[]>('/api/v1/namespaces');
}

export async function createNamespace(payload: CreateNamespaceRequest): Promise<NamespaceSummary> {
  return request<NamespaceSummary>('/api/v1/namespaces', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function listAppScopes(params: { namespace?: string } = {}): Promise<AppScopeSummary[]> {
  const query = new URLSearchParams();
  if (params.namespace) query.set('namespace', params.namespace);
  const suffix = query.toString() ? `?${query}` : '';
  return request<AppScopeSummary[]>(`/api/v1/apps${suffix}`);
}

export async function createAppScope(payload: CreateAppScopeRequest): Promise<AppScopeSummary> {
  return request<AppScopeSummary>('/api/v1/apps', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function listWorkerPools(params: { namespace?: string; app?: string } = {}): Promise<WorkerPoolSummary[]> {
  const query = new URLSearchParams();
  if (params.namespace) query.set('namespace', params.namespace);
  if (params.app) query.set('app', params.app);
  const suffix = query.toString() ? `?${query}` : '';
  return request<WorkerPoolSummary[]>(`/api/v1/worker-pools${suffix}`);
}

export async function createWorkerPool(payload: CreateWorkerPoolRequest): Promise<WorkerPoolSummary> {
  return request<WorkerPoolSummary>('/api/v1/worker-pools', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function updateWorkerPoolQuota(id: string, payload: UpdateWorkerPoolQuotaRequest): Promise<WorkerPoolSummary> {
  return request<WorkerPoolSummary>(`/api/v1/worker-pools/${encodeURIComponent(id)}/quota`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function listSecrets(params: { namespace?: string; app?: string } = {}): Promise<SecretSummary[]> {
  const query = new URLSearchParams();
  if (params.namespace) query.set('namespace', params.namespace);
  if (params.app) query.set('app', params.app);
  const suffix = query.toString() ? `?${query}` : '';
  return request<SecretSummary[]>(`/api/v1/secrets${suffix}`);
}

export async function createSecret(payload: CreateSecretRequest): Promise<SecretSummary> {
  return request<SecretSummary>('/api/v1/secrets', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function deleteSecret(id: string): Promise<void> {
  await request<void>(`/api/v1/secrets/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}

export async function deleteNamespace(id: string): Promise<void> {
  await request<void>(`/api/v1/namespaces/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}

export async function deleteAppScope(id: string): Promise<void> {
  await request<void>(`/api/v1/apps/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
}

export async function deleteWorkerPool(id: string): Promise<void> {
  await request<void>(`/api/v1/worker-pools/${encodeURIComponent(id)}`, { method: 'DELETE', allowNullData: true });
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

export async function listRoles(): Promise<RoleSummary[]> {
  return request<RoleSummary[]>('/api/v1/roles');
}

export async function createRole(params: CreateRoleRequest): Promise<RoleSummary> {
  return request<RoleSummary>('/api/v1/roles', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

export async function updateRole(id: string, params: UpdateRoleRequest): Promise<RoleSummary> {
  return request<RoleSummary>(`/api/v1/roles/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(params),
  });
}

export async function deleteRole(id: string): Promise<unknown> {
  return request<unknown>(`/api/v1/roles/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

export async function listPermissionCatalog(): Promise<PermissionCatalogItem[]> {
  return request<PermissionCatalogItem[]>('/api/v1/permissions/catalog');
}

export async function listMenuPermissionCatalog(): Promise<MenuPermissionCatalogItem[]> {
  return request<MenuPermissionCatalogItem[]>('/api/v1/menu-permissions/catalog');
}

export async function listUiActionPermissionCatalog(): Promise<UiActionPermissionCatalogItem[]> {
  return request<UiActionPermissionCatalogItem[]>('/api/v1/ui-action-permissions/catalog');
}

export async function deleteUser(id: string): Promise<void> {
  await request<void>(`/api/v1/users/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    allowNullData: true,
  });
}

export interface ScriptExecutionPolicy {
  resources: { timeout_ms: number; max_memory_bytes: number; max_output_bytes: number };
  network: { enabled: boolean; allowed_hosts: string[] };
  filesystem: { read_only_paths: string[]; writable_paths: string[] };
  secrets: { refs: string[] };
  env_vars: string[];
  sandbox: { backend: string };
}

export interface ScriptReleaseSignatureSummary {
  approval_ticket: string;
  signature: string;
  verified_at: string;
  verified_by: string;
}

export interface ScriptReleaseGrantEvidenceSummary {
  url: string[];
  file_read: string[];
  file_write: string[];
  secret: string[];
  verified_at: string;
  verified_by: string;
}

export interface ScriptSummary {
  id: string;
  name: string;
  language: string;
  version: string;
  content: string;
  content_sha256: string;
  status: string;
  released_version_id: string | null;
  released_version_number: number | null;
  release_signature: ScriptReleaseSignatureSummary | null;
  release_grants: ScriptReleaseGrantEvidenceSummary | null;
  timeout_seconds: number | null;
  max_memory_bytes: number | null;
  allow_network: boolean;
  allowed_env_vars: string[] | null;
  policy: ScriptExecutionPolicy;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
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
  policy?: ScriptExecutionPolicy | null;
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
  policy?: ScriptExecutionPolicy | null;
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

export interface ScriptReleaseGrants {
  url: string[];
  file_read: string[];
  file_write: string[];
  secret: string[];
}

export interface ScriptReleaseRequest {
  version_number?: number | null;
  approval_ticket?: string | null;
  signature?: string | null;
  grants?: ScriptReleaseGrants | null;
}

export async function publishScript(
  id: string,
  versionNumber?: number,
  params: Omit<ScriptReleaseRequest, 'version_number'> = {},
): Promise<ScriptSummary> {
  return request<ScriptSummary>(`/api/v1/scripts/${encodeURIComponent(id)}/publish`, {
    method: 'POST',
    body: JSON.stringify({ version_number: versionNumber ?? null, ...params }),
  });
}

export async function rollbackScript(
  id: string,
  versionNumber: number,
  params: Omit<ScriptReleaseRequest, 'version_number'> = {},
): Promise<ScriptSummary> {
  return request<ScriptSummary>(`/api/v1/scripts/${encodeURIComponent(id)}/rollback`, {
    method: 'POST',
    body: JSON.stringify({ version_number: versionNumber, ...params }),
  });
}

export interface ScriptVersionSummary {
  id: string;
  script_id: string;
  version_number: number;
  content: string;
  content_sha256: string;
  language: string;
  status: string;
  timeout_seconds: number | null;
  max_memory_bytes: number | null;
  allow_network: boolean;
  allowed_env_vars: string[] | null;
  policy: ScriptExecutionPolicy;
  createdBy: string;
  createdAt: string;
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


export interface AlertDeliveryAttemptSummary {
  id: string;
  event_id: string;
  rule_id: string;
  provider: string;
  target: string;
  delivered: boolean;
  status_code: number | null;
  error: string | null;
  attempt: number;
  retry_state: string;
  next_retry_at: string | null;
  createdAt: string;
}

export interface AlertDeliveryQueueStatus {
  total_attempts: number;
  delivered: number;
  retry_pending: number;
  dead_letter: number;
  retry_consumed: number;
  failed: number;
  recent_dead_letters: AlertDeliveryAttemptSummary[];
}

export async function getAlertDeliveryQueueStatus(): Promise<AlertDeliveryQueueStatus> {
  return request<AlertDeliveryQueueStatus>('/api/v1/alert-delivery-attempts:queue-status');
}

export async function listAlertDeliveryAttempts(params: { retry_state?: string } = {}): Promise<AlertDeliveryAttemptSummary[]> {
  const query = new URLSearchParams();
  if (params.retry_state) query.set('retry_state', params.retry_state);
  const suffix = query.toString() ? `?${query}` : '';
  return request<AlertDeliveryAttemptSummary[]>(`/api/v1/alert-delivery-attempts${suffix}`);
}

export interface AuditLogSummary {
  id: string;
  actor: string;
  action: string;
  resource_type: string;
  resource_id: string;
  detail: string | null;
  before: string | null;
  after: string | null;
  trace_id: string | null;
  result: 'success' | 'failed' | string;
  failure_reason: string | null;
  ip_address: string | null;
  createdAt?: string;
  created_at?: string;
}

export interface AuditLogQuery {
  page_size?: number;
  page_token?: string;
  actor?: string;
  action?: string;
  resource_type?: string;
  resource_id?: string;
  failure_reason?: string;
  format?: string;
}

export interface AuditLogPage extends Page<AuditLogSummary> {
  total: number;
}

export interface AuditLogExport {
  format: string;
  items: AuditLogSummary[];
  exported: number;
  max_rows: number;
  redacted: boolean;
  governance: string;
}

function auditLogSearchParams(query: AuditLogQuery = {}): string {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (value !== undefined && value !== null && String(value).trim() !== '') {
      params.set(key, String(value));
    }
  });
  return params.toString();
}

export async function listAuditLogs(query: AuditLogQuery = {}): Promise<AuditLogPage> {
  const suffix = auditLogSearchParams(query);
  return request<AuditLogPage>(`/api/v1/audit-logs${suffix ? `?${suffix}` : ''}`);
}

export async function exportAuditLogs(query: AuditLogQuery = {}): Promise<AuditLogExport> {
  const suffix = auditLogSearchParams({ ...query, format: 'json' });
  return request<AuditLogExport>(`/api/v1/audit-logs:export${suffix ? `?${suffix}` : ''}`);
}

interface TikeoRequestInit extends RequestInit {
  auth?: boolean;
  allowNullData?: boolean;
}

export async function request<T>(path: string, init: TikeoRequestInit = {}): Promise<T> {
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
  const rawBody = await response.text();
  if (rawBody.trim() === '') {
    throw new ApiClientError(
      response.ok ? -1 : response.status,
      `API returned an empty response body for ${path}`,
      response.status,
    );
  }
  let envelope: ApiResponse<T | null>;
  try {
    envelope = JSON.parse(rawBody) as ApiResponse<T | null>;
  } catch {
    throw new ApiClientError(
      response.ok ? -1 : response.status,
      `API returned a non-JSON response for ${path}`,
      response.status,
    );
  }

  if (response.status === 401) {
    setAuthToken(null);
    authErrorHandler?.onUnauthorized?.();
  } else if (response.status === 403) {
    authErrorHandler?.onForbidden?.(envelope.message);
  }

  if (envelope.code !== 0) {
    throw new ApiClientError(envelope.code, envelope.message, response.status);
  }
  if (envelope.data === null) {
    if (allowNullData) {
      return null as T;
    }
    throw new ApiClientError(-1, 'API returned null data for a non-empty operation', response.status);
  }
  return envelope.data;
}

function readStoredToken(): string | null {
  if (typeof localStorage === 'undefined') {
    return null;
  }
  return localStorage.getItem(TOKEN_STORAGE_KEY);
}

export * from './workflow';
