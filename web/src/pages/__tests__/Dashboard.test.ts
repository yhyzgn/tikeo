import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../Dashboard.tsx', import.meta.url), 'utf8');

describe('dashboard realtime overview', () => {
  test('subscribes to instance and worker SSE streams with a 3s fallback refresh', () => {
    expect(source).toContain('instanceListStreamUrl');
    expect(source).toContain('workerStreamUrl');
    expect(source).toContain('listWorkers');
    expect(source).toContain('new EventSource(instanceListStreamUrl())');
    expect(source).toContain('new EventSource(workerStreamUrl())');
    expect(source).toContain("instanceSource.addEventListener('instances.snapshot'");
    expect(source).toContain("workerSource.addEventListener('workers.snapshot'");
    expect(source).toContain('setOnlineWorkers(snapshot.workers.online);');
    expect(source).toContain('window.setInterval(() => { void load({ silent: true }); }, 3000)');
    expect(source).toContain('window.clearInterval(fallbackTimer);');
  });
});
