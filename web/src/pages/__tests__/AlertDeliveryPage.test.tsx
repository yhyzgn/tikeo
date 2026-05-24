import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const clientSource = readFileSync(new URL('../../api/client.ts', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../AlertDeliveryPage.tsx', import.meta.url), 'utf8');

describe('alert delivery operations page', () => {
  test('wires retry and DLQ queue status into a governance menu page', () => {
    expect(routesSource).toContain('alerts:');
    expect(routesSource).toContain('/alerts');
    expect(routesSource).toContain('告警投递');
    expect(appSource).toContain('AlertDeliveryPage');
    expect(appSource).toContain('ROUTE_META.alerts.path');
    expect(clientSource).toContain('/api/v1/alert-delivery-attempts:queue-status');
  });

  test('renders operator-facing retry and DLQ status with redacted targets', () => {
    expect(pageSource).toContain('最近 DLQ');
    expect(pageSource).toContain('retry_pending');
    expect(pageSource).toContain('dead_letter');
    expect(pageSource).toContain('Provider target 已脱敏');
    expect(pageSource).toContain('recent_dead_letters');
  });
});
