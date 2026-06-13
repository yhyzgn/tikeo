import { request } from './client';

export interface NotificationChannelTypeSummary {
  type: string;
  label: string;
  category: string;
  targetKind: string;
  description: string;
  requiredConfigKeys: string[];
  requiredTargetKeys: string[];
  secretConfigKeys: string[];
  supportsTestSend: boolean;
  pluginProvided: boolean;
  template: Record<string, unknown>;
}

export interface NotificationChannelSummary {
  id: string;
  scopeType: string;
  namespace: string | null;
  app: string | null;
  workerPool: string | null;
  name: string;
  provider: string;
  enabled: boolean;
  configJson: string;
  targetRedacted: string;
  safetyPolicyJson: string | null;
  targetConfigured: boolean;
  secretConfigured: boolean;
  createdBy: string | null;
  updatedBy: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateNotificationChannelRequest {
  scopeType: string;
  namespace?: string | null;
  app?: string | null;
  workerPool?: string | null;
  name: string;
  provider: string;
  enabled?: boolean;
  config?: Record<string, unknown>;
  secretRefs?: Record<string, unknown>;
  safetyPolicy?: Record<string, unknown> | null;
}

export interface UpdateNotificationChannelRequest {
  scopeType?: string;
  namespace?: string | null;
  app?: string | null;
  workerPool?: string | null;
  name?: string;
  provider?: string;
  enabled?: boolean;
  config?: Record<string, unknown>;
  secretRefs?: Record<string, unknown>;
  safetyPolicy?: Record<string, unknown> | null;
}

export interface NotificationPolicySummary {
  id: string;
  name: string;
  enabled: boolean;
  ownerType: string;
  ownerId: string | null;
  eventFamily: string;
  eventFilterJson: string;
  channelRefsJson: string;
  templateRef: string | null;
  severity: string;
  dedupeSeconds: number;
  throttleJson: string | null;
  quietHoursJson: string | null;
  escalationJson: string | null;
  createdBy: string | null;
  updatedBy: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateNotificationPolicyRequest {
  ownerType: string;
  ownerId?: string | null;
  name: string;
  eventFamily: string;
  eventFilter?: Record<string, unknown>;
  channelRefs: Array<Record<string, unknown>>;
  templateRef?: string | null;
  severity: string;
  enabled?: boolean;
  dedupeSeconds?: number;
}

export interface UpdateNotificationPolicyRequest {
  ownerType?: string;
  ownerId?: string | null;
  name?: string;
  eventFamily?: string;
  eventFilter?: Record<string, unknown>;
  channelRefs?: Array<Record<string, unknown>>;
  templateRef?: string | null;
  severity?: string;
  enabled?: boolean;
  dedupeSeconds?: number;
  throttle?: Record<string, unknown> | null;
  quietHours?: Record<string, unknown> | null;
  escalation?: Record<string, unknown> | null;
}

export interface NotificationPolicyValidationSummary {
  policyId: string;
  valid: boolean;
  channelCount: number;
  missingChannelIds: string[];
  disabledChannelIds: string[];
  issues: string[];
}


export interface NotificationTemplateSummary {
  id: string;
  templateKey: string;
  name: string;
  description: string | null;
  provider: string;
  messageType: string;
  enabled: boolean;
  bodyJson: string;
  variablesJson: string;
  createdBy: string | null;
  updatedBy: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateNotificationTemplateRequest {
  templateKey: string;
  name: string;
  description?: string | null;
  provider: string;
  messageType: string;
  enabled?: boolean;
  body?: Record<string, unknown>;
  variables?: unknown;
}

export interface UpdateNotificationTemplateRequest {
  templateKey?: string;
  name?: string;
  description?: string | null;
  provider?: string;
  messageType?: string;
  enabled?: boolean;
  body?: Record<string, unknown>;
  variables?: unknown;
}

export interface RenderNotificationTemplateResult {
  provider: string;
  messageType: string;
  rendered: Record<string, unknown> | unknown[] | string | number | boolean | null;
}

export interface NotificationMessageSummary {
  id: string;
  sourceType: string;
  sourceId: string;
  policyId: string;
  eventType: string;
  resourceType: string;
  resourceId: string;
  severity: string;
  subject: string;
  body: string;
  payloadJson: string;
  dedupeKey: string;
  traceId: string | null;
  status: string;
  createdAt: string;
  updatedAt: string;
}

export interface NotificationDeliveryAttemptSummary {
  id: string;
  messageId: string;
  policyId: string;
  channelId: string;
  provider: string;
  targetRedacted: string;
  attempt: number;
  delivered: boolean;
  statusCode: number | null;
  error: string | null;
  retryState: string;
  nextRetryAt: string | null;
  createdAt: string;
}

export interface TestNotificationChannelRequest {
  subject?: string;
  body?: string;
  eventType?: string;
  resourceType?: string;
  resourceId?: string;
  severity?: string;
  payload?: Record<string, unknown>;
}

export interface TestNotificationChannelResult {
  channelId: string;
  messageId: string;
  attemptId: string;
  provider: string;
  targetRedacted: string;
  delivered: boolean;
  statusCode: number | null;
  retryState: string;
  error: string | null;
  renderedPayload: Record<string, unknown> | unknown[] | string | number | boolean | null;
  createdAt: string;
}

export interface NotificationDeliveryQueueStatus {
  totalAttempts: number;
  delivered: number;
  retryPending: number;
  deadLetter: number;
  retryConsumed: number;
  failed: number;
  recentDeadLetters: NotificationDeliveryAttemptSummary[];
}

export interface NotificationDeliveryRetryResult {
  scanned: number;
  delivered: number;
  retried: number;
  deadLettered: number;
  skipped: number;
}

export interface JobNotificationBindingSummary {
  id: string;
  jobId: string;
  name: string;
  trigger: string;
  eventTypes: string[];
  channelIds: string[];
  templateRef: string | null;
  enabled: boolean;
  severity: string;
  dedupeSeconds: number;
  includeLogLink: boolean;
  includeLogExcerpt: boolean;
  logExcerptLines: number;
  policy: NotificationPolicySummary;
}

export interface SaveJobNotificationBindingRequest {
  name: string;
  trigger: string;
  eventTypes?: string[];
  channelIds: string[];
  templateRef?: string | null;
  enabled?: boolean;
  severity?: string;
  dedupeSeconds?: number;
  includeLogLink?: boolean;
  includeLogExcerpt?: boolean;
  logExcerptLines?: number;
}

export interface JobNotificationBindingValidationSummary {
  valid: boolean;
  eventTypes: string[];
  channelCount: number;
  missingChannelIds: string[];
  disabledChannelIds: string[];
  issues: string[];
}

export interface JobNotificationBindingPreview {
  jobId: string;
  trigger: string;
  eventTypes: string[];
  sampleContext: Record<string, unknown>;
  renderedTemplate: Record<string, unknown> | unknown[] | string | number | boolean | null;
  validation: JobNotificationBindingValidationSummary;
}

export interface NotificationTraceLogLine {
  level: string;
  workerId: string;
  sequence: number;
  message: string;
  createdAt: string;
}

export interface NotificationMessageTrace {
  message: NotificationMessageSummary;
  policy: NotificationPolicySummary | null;
  attempts: NotificationDeliveryAttemptSummary[];
  job: { id: string; namespace: string; app: string; name: string } | null;
  instance: { id: string; jobId: string; status: string; triggerType: string; executionMode: string; createdAt: string; updatedAt: string; workerId: string | null } | null;
  logs: { url: string | null; excerpt: NotificationTraceLogLine[]; truncated: boolean };
}

function queryString(params: Record<string, string | boolean | undefined> = {}): string {
  const query = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && String(value).trim() !== '') query.set(key, String(value));
  });
  const rendered = query.toString();
  return rendered ? `?${rendered}` : '';
}

export function listNotificationChannelTypes(): Promise<NotificationChannelTypeSummary[]> {
  return request<NotificationChannelTypeSummary[]>('/api/v1/notification-channel-types');
}

export function listNotificationChannels(params: { provider?: string; enabled?: boolean } = {}): Promise<NotificationChannelSummary[]> {
  return request<NotificationChannelSummary[]>(`/api/v1/notification-channels${queryString(params)}`);
}

export function createNotificationChannel(payload: CreateNotificationChannelRequest): Promise<NotificationChannelSummary> {
  return request<NotificationChannelSummary>('/api/v1/notification-channels', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateNotificationChannel(id: string, payload: UpdateNotificationChannelRequest): Promise<NotificationChannelSummary> {
  return request<NotificationChannelSummary>(`/api/v1/notification-channels/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function testNotificationChannel(id: string, payload: TestNotificationChannelRequest = {}): Promise<TestNotificationChannelResult> {
  return request<TestNotificationChannelResult>(`/api/v1/notification-channels/${encodeURIComponent(id)}/test-send`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export async function deleteNotificationChannel(id: string): Promise<void> {
  await request<Record<string, never>>(`/api/v1/notification-channels/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

export function listNotificationPolicies(params: { event_family?: string; enabled?: boolean } = {}): Promise<NotificationPolicySummary[]> {
  return request<NotificationPolicySummary[]>(`/api/v1/notification-policies${queryString(params)}`);
}

export function createNotificationPolicy(payload: CreateNotificationPolicyRequest): Promise<NotificationPolicySummary> {
  return request<NotificationPolicySummary>('/api/v1/notification-policies', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateNotificationPolicy(id: string, payload: UpdateNotificationPolicyRequest): Promise<NotificationPolicySummary> {
  return request<NotificationPolicySummary>(`/api/v1/notification-policies/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function deleteNotificationPolicy(id: string): Promise<void> {
  await request<Record<string, never>>(`/api/v1/notification-policies/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

export function validateNotificationPolicy(id: string): Promise<NotificationPolicyValidationSummary> {
  return request<NotificationPolicyValidationSummary>(`/api/v1/notification-policies/${encodeURIComponent(id)}:validate`, {
    method: 'POST',
  });
}


export function listNotificationTemplates(params: { provider?: string; message_type?: string; enabled?: boolean } = {}): Promise<NotificationTemplateSummary[]> {
  return request<NotificationTemplateSummary[]>(`/api/v1/notification-templates${queryString(params)}`);
}

export function createNotificationTemplate(payload: CreateNotificationTemplateRequest): Promise<NotificationTemplateSummary> {
  return request<NotificationTemplateSummary>('/api/v1/notification-templates', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateNotificationTemplate(id: string, payload: UpdateNotificationTemplateRequest): Promise<NotificationTemplateSummary> {
  return request<NotificationTemplateSummary>(`/api/v1/notification-templates/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function deleteNotificationTemplate(id: string): Promise<void> {
  await request<Record<string, never>>(`/api/v1/notification-templates/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

export function renderNotificationTemplate(id: string, payload: { provider?: string; messageType?: string; template?: Record<string, unknown>; sample?: Record<string, unknown> } = {}): Promise<RenderNotificationTemplateResult> {
  return request<RenderNotificationTemplateResult>(`/api/v1/notification-templates/${encodeURIComponent(id)}/render`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function listNotificationMessages(params: { status?: string; event_type?: string } = {}): Promise<NotificationMessageSummary[]> {
  return request<NotificationMessageSummary[]>(`/api/v1/notification-messages${queryString(params)}`);
}

export function listNotificationDeliveryAttempts(params: { retry_state?: string } = {}): Promise<NotificationDeliveryAttemptSummary[]> {
  return request<NotificationDeliveryAttemptSummary[]>(`/api/v1/notification-delivery-attempts${queryString(params)}`);
}

export function getNotificationDeliveryQueueStatus(): Promise<NotificationDeliveryQueueStatus> {
  return request<NotificationDeliveryQueueStatus>('/api/v1/notification-delivery-attempts:queue-status');
}

export function retryDueNotificationDeliveryAttempts(params: { limit?: number; maxAttempts?: number; backoffSeconds?: number } = {}): Promise<NotificationDeliveryRetryResult> {
  return request<NotificationDeliveryRetryResult>('/api/v1/notification-delivery-attempts:retry-due', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

export function listJobNotificationBindings(jobId: string): Promise<JobNotificationBindingSummary[]> {
  return request<JobNotificationBindingSummary[]>(`/api/v1/jobs/${encodeURIComponent(jobId)}/notification-bindings`);
}

export function createJobNotificationBinding(jobId: string, payload: SaveJobNotificationBindingRequest): Promise<JobNotificationBindingSummary> {
  return request<JobNotificationBindingSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}/notification-bindings`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateJobNotificationBinding(jobId: string, bindingId: string, payload: Partial<SaveJobNotificationBindingRequest>): Promise<JobNotificationBindingSummary> {
  return request<JobNotificationBindingSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}/notification-bindings/${encodeURIComponent(bindingId)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export async function deleteJobNotificationBinding(jobId: string, bindingId: string): Promise<void> {
  await request<Record<string, never>>(`/api/v1/jobs/${encodeURIComponent(jobId)}/notification-bindings/${encodeURIComponent(bindingId)}`, { method: 'DELETE' });
}

export function validateJobNotificationBinding(jobId: string, payload: SaveJobNotificationBindingRequest): Promise<JobNotificationBindingValidationSummary> {
  return request<JobNotificationBindingValidationSummary>(`/api/v1/jobs/${encodeURIComponent(jobId)}/notification-bindings:validate`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function previewJobNotificationBinding(jobId: string, payload: SaveJobNotificationBindingRequest): Promise<JobNotificationBindingPreview> {
  return request<JobNotificationBindingPreview>(`/api/v1/jobs/${encodeURIComponent(jobId)}/notification-bindings:preview`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function getNotificationMessageTrace(messageId: string): Promise<NotificationMessageTrace> {
  return request<NotificationMessageTrace>(`/api/v1/notification-messages/${encodeURIComponent(messageId)}/trace`);
}

export function getPublicJobInstanceTrace(instanceId: string): Promise<NotificationMessageTrace> {
  return request<NotificationMessageTrace>(`/api/v1/public/job-instances/${encodeURIComponent(instanceId)}/trace`, { auth: false });
}
