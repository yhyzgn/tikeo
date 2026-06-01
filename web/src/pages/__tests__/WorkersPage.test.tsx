import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import type { WorkerSummary } from '../../api/client';
import { capabilityFilterValues, visibleCapabilityTags, visibleSdkProcessors } from '../workers/WorkerTable';
import { filterWorkers } from '../workers/workerPageModel';

const pageSource = readFileSync(new URL('../WorkersPage.tsx', import.meta.url), 'utf8');
const tableSource = readFileSync(new URL('../workers/WorkerTable.tsx', import.meta.url), 'utf8');
const queueSource = readFileSync(new URL('../workers/DispatchQueuePanel.tsx', import.meta.url), 'utf8');
const overviewSource = readFileSync(new URL('../workers/WorkerClusterOverview.tsx', import.meta.url), 'utf8');
const modelSource = readFileSync(new URL('../workers/workerPageModel.ts', import.meta.url), 'utf8');
const historySource = readFileSync(new URL('../workers/WorkerLifecycleHistory.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('Worker cluster page redesign', () => {
  test('splits the worker dashboard into focused operational components', () => {
    expect(pageSource).toContain('WorkerClusterOverview');
    expect(pageSource).toContain('WorkerQueueStats');
    expect(pageSource).toContain('WorkerTable');
    expect(pageSource).toContain('DispatchQueuePanel');
    expect(pageSource).toContain('WorkerLifecycleHistory');
    expect(overviewSource).toContain('Queue Pressure');
  });

  test('adds worker filtering and queue status drill-down affordances', () => {
    expect(tableSource).toContain('搜索 worker / app / region / capability');
    expect(tableSource).toContain('Namespace');
    expect(tableSource).toContain('Capability');
    expect(tableSource).toContain("title: 'Capabilities'");
    expect(tableSource).toContain('worker.structuredCapabilities?.tags');
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
    capabilities: ['sdk', 'processor:demo.echo', 'script:shell', 'legacy-tag'],
    structuredCapabilities: {
      tags: ['sdk', 'java'],
      sdkProcessors: ['demo.echo'],
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
    expect(visibleCapabilityTags(worker)).toEqual(['java', 'legacy-tag', 'sdk']);
    expect(visibleCapabilityTags(worker)).not.toContain('processor:demo.echo');
    expect(visibleCapabilityTags(worker)).not.toContain('script:shell');
    expect(visibleSdkProcessors(worker)).toEqual(['demo.echo']);
  });

  test('still exposes structured processor choices through dedicated filters', () => {
    expect(capabilityFilterValues(worker)).toEqual([
      'java',
      'legacy-tag',
      'sdk',
      'SDK:demo.echo',
      'Script:shell',
      'Plugin:sql:billing.sql-sync',
    ]);
    expect(filterWorkers([worker], { query: 'billing.sql-sync', namespace: '', capability: '' })).toHaveLength(1);
    expect(filterWorkers([worker], { query: '', namespace: '', capability: 'SDK:demo.echo' })).toHaveLength(1);
  });
});
