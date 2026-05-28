import type { DispatchQueueSummary, QueueOverview, WorkerSessionHistorySummary, WorkerSummary } from '../../api/client';

export type QueueStatusFilter = 'all' | 'pending' | 'running' | 'done' | 'failed';

export interface WorkerFilters {
  query: string;
  namespace: string;
  capability: string;
}

export function workerSearchText(worker: WorkerSummary): string {
  const structured = worker.structuredCapabilities;
  return [
    worker.workerId,
    worker.namespace,
    worker.app,
    worker.cluster,
    worker.region,
    worker.logicalInstanceId,
    worker.clientInstanceId ?? '',
    worker.status,
    worker.statusReason ?? '',
    ...worker.capabilities,
    ...(structured?.tags ?? []),
    ...(structured?.sdkProcessors ?? []),
    ...(structured?.scriptRunners.map((runner) => `${runner.language} ${runner.sandboxBackend}`) ?? []),
    ...(structured?.pluginProcessors.flatMap((plugin) => [plugin.type, ...plugin.processorNames]) ?? []),
  ].join(' ').toLowerCase();
}

export function filterWorkers(workers: WorkerSummary[], filters: WorkerFilters): WorkerSummary[] {
  const query = filters.query.trim().toLowerCase();
  return workers.filter((worker) => {
    const matchesQuery = !query || workerSearchText(worker).includes(query);
    const matchesNamespace = !filters.namespace || worker.namespace === filters.namespace;
    const workerFilterValues = [
      ...(worker.structuredCapabilities?.tags ?? worker.capabilities),
      ...(worker.structuredCapabilities?.sdkProcessors.map((name) => `SDK:${name}`) ?? []),
      ...(worker.structuredCapabilities?.scriptRunners.map((runner) => `Script:${runner.language}`) ?? []),
      ...(worker.structuredCapabilities?.pluginProcessors.flatMap((plugin) =>
        plugin.processorNames.map((name) => `Plugin:${plugin.type}:${name}`)
      ) ?? []),
    ];
    const matchesCapability = !filters.capability || workerFilterValues.includes(filters.capability);
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


export function sessionStatusColor(status: string): string {
  if (status === 'online' || status === 'active' || status === 'session_registered') return 'green';
  if (status === 'replaced' || status === 'stopped' || status === 'graceful_shutdown') return 'blue';
  if (status === 'offline' || status === 'lease_expired_unknown' || status === 'heartbeat_timeout') return 'red';
  if (status === 'degraded' || status === 'stale_worker_message') return 'orange';
  return 'default';
}

export function groupWorkerSessionsByLayer(sessions: WorkerSessionHistorySummary[]): {
  active: WorkerSessionHistorySummary[];
  degraded: WorkerSessionHistorySummary[];
  history: WorkerSessionHistorySummary[];
} {
  return {
    active: sessions.filter((session) => session.status === 'online'),
    degraded: sessions.filter((session) => session.status === 'offline' || session.status === 'degraded'),
    history: sessions.filter((session) => !['online', 'offline', 'degraded'].includes(session.status)),
  };
}
