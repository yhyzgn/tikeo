import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { normalizeKeepAlivePath } from './KeepAliveOutlet';

const appSource = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
const keepAliveSource = readFileSync(new URL('./KeepAliveOutlet.tsx', import.meta.url), 'utf8');
const jobsSource = readFileSync(new URL('../pages/JobsPage.tsx', import.meta.url), 'utf8');
const workersSource = readFileSync(new URL('../pages/WorkersPage.tsx', import.meta.url), 'utf8');
const workflowsSource = readFileSync(new URL('../pages/WorkflowsPage.tsx', import.meta.url), 'utf8');

describe('keep-alive route shell', () => {
  test('normalizes exact top-level paths for cached list pages', () => {
    expect(normalizeKeepAlivePath('/jobs/')).toBe('/jobs');
    expect(normalizeKeepAlivePath('/')).toBe('/');
  });

  test('caches selected list pages and refreshes them on route activation', () => {
    expect(appSource).toContain('KeepAliveOutlet');
    expect(appSource).toContain('KEEP_ALIVE_ROUTES');
    expect(keepAliveSource).toContain('visitedPaths');
    expect(keepAliveSource).toContain('hidden={!active}');
    expect(jobsSource).toContain("useRouteActive(ROUTE_META.jobs.path)");
    expect(workflowsSource).toContain("useRouteActive(ROUTE_META.workflows.path)");
    expect(workersSource).toContain("useRouteActive(ROUTE_META.workers.path)");
    expect(workersSource).toContain('if (!active) return undefined');
  });
});
