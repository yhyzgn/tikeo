import { request } from './client';

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
