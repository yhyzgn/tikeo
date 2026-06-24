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
    ...(structured?.tags ?? []),
    ...(structured?.normalProcessors?.map((processor) => `${processor.name} ${processor.description}`) ?? []),
    ...(structured?.scriptRunners.map((runner) => `${runner.language} ${runner.sandboxBackend}`) ?? []),
    ...(structured?.pluginProcessors.flatMap((plugin) => [plugin.type, ...plugin.processorNames, ...(plugin.processors?.flatMap((processor) => [processor.name, processor.description]) ?? [])]) ?? []),
  ].join(' ').toLowerCase();
}

export function filterWorkers(workers: WorkerSummary[], filters: WorkerFilters): WorkerSummary[] {
  const query = filters.query.trim().toLowerCase();
  return workers.filter((worker) => {
    const matchesQuery = !query || workerSearchText(worker).includes(query);
    const matchesNamespace = !filters.namespace || worker.namespace === filters.namespace;
    const workerFilterValues = [
      ...(worker.structuredCapabilities?.tags ?? []),
      ...(worker.structuredCapabilities?.normalProcessors?.map((processor) => `Normal:${processor.name}`) ?? []),
      ...(worker.structuredCapabilities?.scriptRunners.map((runner) => `Script:${runner.language}`) ?? []),
      ...(worker.structuredCapabilities?.pluginProcessors.flatMap((plugin) =>
        (plugin.processors?.map((processor) => `Plugin:${plugin.type}:${processor.name}`) ?? plugin.processorNames.map((name) => `Plugin:${plugin.type}:${name}`))
      ) ?? []),
    ];
    const matchesCapability = !filters.capability || workerFilterValues.includes(filters.capability);
    return matchesQuery && matchesNamespace && matchesCapability;
  });
}


export interface WorkerClusterGroup {
  cluster: string;
  region: string;
  master: WorkerSummary | null;
  followers: WorkerSummary[];
  workers: WorkerSummary[];
}

export interface WorkerScopeGroup {
  scopeKey: string;
  namespace: string;
  app: string;
  clusters: WorkerClusterGroup[];
  workers: WorkerSummary[];
}

export function groupWorkersByNamespaceApp(workers: WorkerSummary[]): WorkerScopeGroup[] {
  const scopeMap = new Map<string, WorkerSummary[]>();
  for (const worker of workers) {
    const key = `${worker.namespace}/${worker.app}`;
    scopeMap.set(key, [...(scopeMap.get(key) ?? []), worker]);
  }

  return [...scopeMap.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([scopeKey, scopeWorkers]) => {
      const [namespace, app] = scopeKey.split('/');
      const clusterMap = new Map<string, WorkerSummary[]>();
      for (const worker of scopeWorkers) {
        const clusterKey = `${worker.cluster}@@${worker.region}`;
        clusterMap.set(clusterKey, [...(clusterMap.get(clusterKey) ?? []), worker]);
      }
      const clusters = [...clusterMap.entries()]
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([clusterKey, clusterWorkers]) => {
          const [cluster, region] = clusterKey.split('@@');
          const ordered = [...clusterWorkers].sort((left, right) => {
            if (left.master?.isMaster && !right.master?.isMaster) return -1;
            if (!left.master?.isMaster && right.master?.isMaster) return 1;
            return left.workerId.localeCompare(right.workerId);
          });
          return {
            cluster,
            region,
            master: ordered.find((worker) => worker.master?.isMaster) ?? null,
            followers: ordered.filter((worker) => !worker.master?.isMaster),
            workers: ordered,
          };
        });
      return { scopeKey, namespace, app, clusters, workers: scopeWorkers };
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
