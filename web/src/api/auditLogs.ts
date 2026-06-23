import { request, type Page } from './client';

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
