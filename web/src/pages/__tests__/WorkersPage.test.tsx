import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import type { WorkerSummary } from '../../api/client';
import { capabilityFilterValues, visibleCapabilityTags, visibleNormalProcessors } from '../workers/WorkerTable';
import { filterWorkers, groupWorkersByNamespaceApp } from '../workers/workerPageModel';

const pageSource = readFileSync(new URL('../WorkersPage.tsx', import.meta.url), 'utf8');
const tableSource = readFileSync(new URL('../workers/WorkerTable.tsx', import.meta.url), 'utf8');
const queuePageSource = readFileSync(new URL('../DispatchQueuePage.tsx', import.meta.url), 'utf8');
const queueSource = readFileSync(new URL('../workers/DispatchQueuePanel.tsx', import.meta.url), 'utf8');
const overviewSource = readFileSync(new URL('../workers/WorkerClusterOverview.tsx', import.meta.url), 'utf8');
const modelSource = readFileSync(new URL('../workers/workerPageModel.ts', import.meta.url), 'utf8');
const historySource = readFileSync(new URL('../workers/WorkerLifecycleHistory.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('Worker cluster page redesign', () => {
  test('splits the worker dashboard into focused operational components', () => {
    expect(pageSource).toContain('WorkerClusterOverview');
    expect(pageSource).toContain('WorkerTable');
    expect(pageSource).toContain('const workerData = await listWorkers();');
    expect(pageSource).toContain('setWorkers(workerData);');
    expect(pageSource).toContain('new EventSource(workerStreamUrl())');
    expect(pageSource).toContain("source.addEventListener('workers.snapshot'");
    expect(pageSource).not.toContain('DispatchQueuePanel');
    expect(pageSource).toContain('ROUTE_META.dispatchQueue.path');
    expect(pageSource).toContain('WorkerLifecycleHistory');
    expect(overviewSource).toContain('应用范围');
    expect(overviewSource).toContain('主节点');
  });

  test('adds worker filtering and queue status drill-down affordances', () => {
    expect(tableSource).toContain('搜索 Worker / 应用 / 区域 / 能力 / 处理器');
    expect(tableSource).toContain('命名空间');
    expect(tableSource).toContain('能力');
    expect(tableSource).toContain('worker-scope-collapse');
    expect(tableSource).toContain('主节点');
    expect(tableSource).toContain('从节点');
    expect(tableSource).toContain('worker.structuredCapabilities?.tags');
    expect(queuePageSource).toContain('调度队列');
    expect(queuePageSource).toContain('new EventSource(dispatchQueueStreamUrl())');
    expect(queuePageSource).toContain("source.addEventListener('dispatchQueue.snapshot'");
    expect(queueSource).toContain('Segmented');
    expect(queueSource).toContain('Pending');
    expect(modelSource).toContain('filterWorkers');
    expect(modelSource).toContain('filterQueueItems');
    expect(modelSource).toContain('groupWorkerSessionsByLayer');
    expect(historySource).toContain('异常/待确认');
    expect(historySource).toContain('worker-event-timeline');
  });

  test('includes responsive worker-specific layout styling', () => {
    expect(styles).toContain('.worker-cluster-hero__summary-grid');
    expect(styles).toContain('.worker-toolbar');
    expect(styles).toContain('.worker-scope-collapse');
    expect(styles).toContain('.worker-node__main');
    expect(styles).toContain('.dispatch-queue-item__meta');
    expect(styles).toContain('.worker-history-layer-switch');
    expect(styles).toContain('@media (max-width: 767px)');
  });
});

describe('Worker capability presentation model', () => {
  const worker: WorkerSummary = {
    workerId: 'worker-1',
    logicalInstanceId: 'demo-worker',
    clientInstanceId: 'spring-demo-worker',
    namespace: 'default',
    app: 'billing',
    cluster: 'standalone',
    region: 'local',
    capabilities: ['normal', 'legacy-script-shell', 'legacy-tag'],
    structuredCapabilities: {
      tags: ['normal', 'java'],
      normalProcessors: [{ name: 'demo.echo', description: 'Echo processor' }],
      scriptRunners: [{ language: 'shell', sandboxBackend: 'srt' }],
      pluginProcessors: [{ type: 'sql', processorNames: ['billing.sql-sync'] }],
    },
    master: {
      domain: 'billing-domain',
      isMaster: true,
      masterWorkerId: 'worker-1',
      term: 2,
      fencingToken: 'ft-2',
    },
    generation: 1,
    status: 'online',
    statusReason: null,
    replacedByWorkerId: null,
    lastSequence: 42,
  };

  test('keeps processor names out of the generic Capabilities column', () => {
    expect(visibleCapabilityTags(worker)).toEqual(['java', 'normal']);
    expect(visibleCapabilityTags(worker)).not.toContain('legacy-script-shell');
    expect(visibleNormalProcessors(worker)).toEqual(['demo.echo']);
  });

  test('still exposes structured processor choices through dedicated filters', () => {
    expect(capabilityFilterValues(worker)).toEqual([
      'java',
      'normal',
      'Normal:demo.echo',
      'Script:shell',
      'Plugin:sql:billing.sql-sync',
    ]);
    expect(filterWorkers([worker], { query: 'billing.sql-sync', namespace: '', capability: '' })).toHaveLength(1);
    expect(filterWorkers([worker], { query: '', namespace: '', capability: 'Normal:demo.echo' })).toHaveLength(1);
    expect(groupWorkersByNamespaceApp([worker])[0].scopeKey).toBe('default/billing');
    expect(groupWorkersByNamespaceApp([worker])[0].clusters[0].master?.workerId).toBe('worker-1');
  });
});
