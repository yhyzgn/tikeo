import type { DispatchQueueSummary, QueueOverview, WorkerSummary } from '../../api/client';

export type QueueStatusFilter = 'all' | 'pending' | 'running' | 'done' | 'failed';

export interface WorkerFilters {
  query: string;
  namespace: string;
  capability: string;
}

export function workerSearchText(worker: WorkerSummary): string {
  return [
    worker.worker_id,
    worker.namespace,
    worker.app,
    worker.cluster,
    worker.region,
    ...worker.capabilities,
  ].join(' ').toLowerCase();
}

export function filterWorkers(workers: WorkerSummary[], filters: WorkerFilters): WorkerSummary[] {
  const query = filters.query.trim().toLowerCase();
  return workers.filter((worker) => {
    const matchesQuery = !query || workerSearchText(worker).includes(query);
    const matchesNamespace = !filters.namespace || worker.namespace === filters.namespace;
    const matchesCapability = !filters.capability || worker.capabilities.includes(filters.capability);
    return matchesQuery && matchesNamespace && matchesCapability;
  });
}

export function uniqueSorted(values: string[]): string[] {
  return [...new Set(values.filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

export function filterQueueItems(items: DispatchQueueSummary[], status: QueueStatusFilter): DispatchQueueSummary[] {
  if (status === 'all') {
    return items;
  }
  return items.filter((item) => item.status === status);
}

export function queueHealth(queue: QueueOverview): { label: string; tone: 'healthy' | 'busy' | 'blocked' } {
  if (queue.failed > 0) {
    return { label: '需要处理失败项', tone: 'blocked' };
  }
  if (queue.pending > 0 || queue.running > 0) {
    return { label: '队列处理中', tone: 'busy' };
  }
  return { label: '队列空闲', tone: 'healthy' };
}

export function queueStatusColor(status: string): string {
  if (status === 'pending') return 'gold';
  if (status === 'running') return 'processing';
  if (status === 'done') return 'green';
  if (status === 'failed') return 'red';
  return 'default';
}
