import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../WorkflowsPage.tsx', import.meta.url), 'utf8');

describe('workflow notification node editor contract', () => {
  test('uses Notification Center refs instead of legacy raw target fields', () => {
    expect(source).toContain('listNotificationChannels');
    expect(source).toContain('listNotificationTemplates');
    expect(source).toContain('channelRefs: []');
    expect(source).toContain('templateRef');
    expect(source).toContain('usePolicies');
    expect(source).toContain('Notification Center');
    expect(source).not.toContain("notification: { channel: 'webhook', target: '', template: '' }");
    expect(source).not.toContain('目标地址 / 群 / 收件人');
  });
});
