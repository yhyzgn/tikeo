import { request } from './client';

export interface NotificationChannelTypeSummary {
  type: string;
  label: string;
  category: string;
  targetKind: string;
  description: string;
  requiredConfigKeys: string[];
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
