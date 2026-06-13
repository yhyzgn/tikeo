import { afterEach, describe, expect, mock, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { createJobNotificationBinding, createNotificationChannel, deleteJobNotificationBinding, getNotificationMessageTrace, previewJobNotificationBinding, renderNotificationTemplate, testNotificationChannel, updateJobNotificationBinding, updateNotificationChannel, validateJobNotificationBinding } from './notifications';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe('notification api client', () => {
  test('renders templates through slash render route so dotted template keys are path-safe', async () => {
    let calledUrl = '';
    globalThis.fetch = mock(async (url: string | URL | Request) => {
      calledUrl = String(url);
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { provider: 'slack', messageType: 'text', rendered: { text: 'ok' } } }));
    }) as unknown as typeof fetch;

    await expect(renderNotificationTemplate('ops.slack.failure', { sample: { subject: 'ok' } })).resolves.toMatchObject({ rendered: { text: 'ok' } });

    expect(calledUrl).toBe('/api/v1/notification-templates/ops.slack.failure/render');
    expect(calledUrl).not.toContain(':render');
  });

  test('metadata-only channel updates omit config and secretRefs so stored values are preserved server-side', async () => {
    let payload: Record<string, unknown> | null = null;
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      payload = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { id: 'channel-1', scopeType: 'global', namespace: null, app: null, workerPool: null, name: 'renamed', provider: 'webhook', enabled: true, configJson: '{}', targetRedacted: 'webhook:secret-ref', safetyPolicyJson: null, targetConfigured: true, secretConfigured: true, createdBy: null, updatedBy: null, createdAt: 'now', updatedAt: 'now' } }));
    }) as unknown as typeof fetch;

    await updateNotificationChannel('channel-1', { name: 'renamed', enabled: true });

    expect(payload as unknown).toEqual({ name: 'renamed', enabled: true });
  });

  test('channel create can send config and secretRefs separately without exposing a secretRefsJson field', async () => {
    let payload: Record<string, unknown> | null = null;
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      payload = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { id: 'channel-1', scopeType: 'global', namespace: null, app: null, workerPool: null, name: 'webhook', provider: 'webhook', enabled: true, configJson: '{}', targetRedacted: 'webhook:secret-ref', safetyPolicyJson: null, targetConfigured: true, secretConfigured: true, createdBy: null, updatedBy: null, createdAt: 'now', updatedAt: 'now' } }));
    }) as unknown as typeof fetch;

    await createNotificationChannel({
      scopeType: 'global',
      name: 'webhook',
      provider: 'webhook',
      config: { messageType: 'json' },
      secretRefs: { url: 'env:TIKEO_NOTIFICATION_CHANNEL_WEBHOOK_JSON_WEBHOOK_URL' },
    });

    expect(payload as unknown).toMatchObject({ config: { messageType: 'json' }, secretRefs: { url: 'env:TIKEO_NOTIFICATION_CHANNEL_WEBHOOK_JSON_WEBHOOK_URL' } });
    expect(payload as unknown).not.toHaveProperty('secretRefsJson');
  });

  test('channel test send posts a sample notification to the path-safe test-send endpoint', async () => {
    let calledUrl = '';
    let payload: Record<string, unknown> | null = null;
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calledUrl = String(url);
      payload = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          channelId: 'channel.1',
          messageId: 'notification-message-1',
          attemptId: 'notification-delivery-1',
          provider: 'webhook',
          targetRedacted: 'http://127.0.0.1:1/...',
          delivered: true,
          statusCode: 202,
          retryState: 'delivered',
          error: null,
          renderedPayload: { text: 'ok' },
          createdAt: 'now',
        },
      }));
    }) as unknown as typeof fetch;

    await expect(testNotificationChannel('channel.1', {
      subject: 'Smoke test',
      body: 'Verify notification channel delivery',
      severity: 'info',
    })).resolves.toMatchObject({ delivered: true, statusCode: 202, provider: 'webhook' });

    expect(calledUrl).toBe('/api/v1/notification-channels/channel.1/test-send');
    expect(calledUrl).not.toContain(':test');
    expect(payload as unknown).toMatchObject({
      subject: 'Smoke test',
      body: 'Verify notification channel delivery',
      severity: 'info',
    });
  });

  test('job notification binding client uses job-scoped path-safe endpoints', async () => {
    const calls: Array<{ url: string; method: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({
        url: String(url),
        method: init?.method ?? 'GET',
        body: init?.body ? JSON.parse(String(init.body)) : undefined,
      });
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { id: 'binding.1', jobId: 'job/1' } }));
    }) as unknown as typeof fetch;

    await createJobNotificationBinding('job/1', { name: 'failures', trigger: 'failure', channelIds: ['channel.1'] });
    await updateJobNotificationBinding('job/1', 'binding.1', { enabled: false });
    await deleteJobNotificationBinding('job/1', 'binding.1');

    expect(calls[0]).toMatchObject({ url: '/api/v1/jobs/job%2F1/notification-bindings', method: 'POST', body: { name: 'failures', trigger: 'failure', channelIds: ['channel.1'] } });
    expect(calls[1]).toMatchObject({ url: '/api/v1/jobs/job%2F1/notification-bindings/binding.1', method: 'PATCH', body: { enabled: false } });
    expect(calls[2]).toMatchObject({ url: '/api/v1/jobs/job%2F1/notification-bindings/binding.1', method: 'DELETE' });
  });

  test('job notification binding validation and preview use action suffix endpoints', async () => {
    const calls: Array<{ url: string; method: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), method: init?.method ?? 'GET', body: init?.body ? JSON.parse(String(init.body)) : undefined });
      const data = String(url).endsWith(':preview')
        ? { jobId: 'job.1', trigger: 'failure', eventTypes: ['job_instance.failed'], sampleContext: {}, renderedTemplate: null, validation: { valid: true, eventTypes: [], channelCount: 1, missingChannelIds: [], disabledChannelIds: [], issues: [] } }
        : { valid: true, eventTypes: ['job_instance.failed'], channelCount: 1, missingChannelIds: [], disabledChannelIds: [], issues: [] };
      return new Response(JSON.stringify({ code: 0, message: 'success', data }));
    }) as unknown as typeof fetch;

    await validateJobNotificationBinding('job.1', { name: 'failures', trigger: 'failure', channelIds: ['channel.1'] });
    await previewJobNotificationBinding('job.1', { name: 'failures', trigger: 'failure', channelIds: ['channel.1'] });

    expect(calls[0]).toMatchObject({ url: '/api/v1/jobs/job.1/notification-bindings:validate', method: 'POST' });
    expect(calls[1]).toMatchObject({ url: '/api/v1/jobs/job.1/notification-bindings:preview', method: 'POST' });
    expect(calls[0].body).toMatchObject({ trigger: 'failure', channelIds: ['channel.1'] });
  });

  test('message trace client calls the path-safe trace endpoint', async () => {
    let calledUrl = '';
    globalThis.fetch = mock(async (url: string | URL | Request) => {
      calledUrl = String(url);
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { message: {}, policy: null, attempts: [], job: null, instance: null, logs: { url: null, excerpt: [], truncated: false } } }));
    }) as unknown as typeof fetch;

    await getNotificationMessageTrace('notification.message/1');

    expect(calledUrl).toBe('/api/v1/notification-messages/notification.message%2F1/trace');
    expect(calledUrl).not.toContain(':trace');
  });

});


test('public job instance trace requests do not attach auth', () => {
  const source = readFileSync(new URL('./notifications.ts', import.meta.url), 'utf8');
  expect(source).toContain('getPublicJobInstanceTrace');
  expect(source).toContain('/api/v1/public/job-instances/');
  expect(source).toContain('{ auth: false }');
});
