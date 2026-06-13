import { describe, expect, test } from 'bun:test';

import type { NotificationChannelSummary } from '../../api/notifications';
import { buildChannelSubmitPayload, type ChannelFormValues } from './ChannelDrawer';
import { providerSchemaFor } from './providerSchema';

const editingChannel = {
  id: 'channel-1',
  scopeType: 'global',
  namespace: null,
  app: null,
  workerPool: null,
  name: 'Existing webhook',
  provider: 'webhook',
  enabled: true,
  configJson: '{"messageType":"json"}',
  targetRedacted: 'https://hooks.example.com/...',
  safetyPolicyJson: null,
  targetConfigured: true,
  secretConfigured: true,
  createdBy: null,
  updatedBy: null,
  createdAt: '2026-06-13T00:00:00Z',
  updatedAt: '2026-06-13T00:00:00Z',
} satisfies NotificationChannelSummary;

function baseValues(overrides: Partial<ChannelFormValues> = {}): ChannelFormValues {
  return {
    scopeType: 'global',
    name: 'Existing webhook',
    provider: 'webhook',
    enabled: true,
    messageType: 'json',
    config: {},
    secretRefs: {},
    template: { body: '{"text":"{{subject}}"}' },
    useInlineTemplate: false,
    advancedConfigJsonText: '{}',
    advancedSecretRefsJsonText: '',
    safetyPolicyJsonText: '',
    replaceConfig: false,
    replaceSecretRefs: false,
    ...overrides,
  };
}

describe('channel drawer submit payload builder', () => {
  test('preserves saved config and credentials for metadata-only edits', () => {
    const result = buildChannelSubmitPayload({
      editingChannel,
      schema: providerSchemaFor(null, 'webhook'),
      values: baseValues({ name: 'Renamed webhook', enabled: false }),
    });

    expect(result.mode).toBe('update');
    expect(result.payload).toMatchObject({ name: 'Renamed webhook', enabled: false, scopeType: 'global', safetyPolicy: null });
    expect(result.payload).not.toHaveProperty('config');
    expect(result.payload).not.toHaveProperty('secretRefs');
  });

  test('sends channel config only when replaceConfig is enabled and keeps form fields authoritative', () => {
    const result = buildChannelSubmitPayload({
      editingChannel,
      schema: providerSchemaFor(null, 'slack'),
      values: baseValues({
        provider: 'slack',
        replaceConfig: true,
        replaceSecretRefs: true,
        config: { threadTs: 'form-thread' },
        secretRefs: { url: 'env:SLACK_WEBHOOK_URL' },
        advancedConfigJsonText: '{"threadTs":"advanced-thread","custom":"kept"}',
      }),
    });

    expect(result.mode).toBe('update');
    expect(result.payload).toHaveProperty('config');
    expect(result.payload.config).toMatchObject({ messageType: 'json', threadTs: 'form-thread', custom: 'kept' });
    expect(result.payload).toHaveProperty('secretRefs');
  });

  test('sends credentials only when replaceSecretRefs is enabled and never uses redacted placeholders', () => {
    const schema = providerSchemaFor(null, 'webhook');
    const withSecrets = buildChannelSubmitPayload({
      editingChannel,
      schema,
      values: baseValues({ replaceSecretRefs: true, secretRefs: { url: 'env:WEBHOOK_URL' } }),
    });

    expect(withSecrets.payload).not.toHaveProperty('config');
    expect(withSecrets.payload).toHaveProperty('secretRefs');
    expect(withSecrets.payload.secretRefs).toMatchObject({ url: 'env:WEBHOOK_URL' });

    expect(() => buildChannelSubmitPayload({
      editingChannel,
      schema,
      values: baseValues({ replaceSecretRefs: true, secretRefs: { url: 'https://hooks.example.com/...' } }),
    })).toThrow('脱敏占位符');
  });
});
