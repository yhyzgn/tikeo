import { afterEach, describe, expect, mock, test } from 'bun:test';

import { ApiClientError, createJob, listInstanceLogs, listJobs, login, setAuthToken, triggerJob } from './client';

const originalFetch = globalThis.fetch;

function resetTokenStorage() {
  setAuthToken(null);
}

afterEach(() => {
  globalThis.fetch = originalFetch;
  resetTokenStorage();
});

describe('api client envelope handling', () => {
  test('returns data when code is zero', async () => {
    const body = {
      code: 0,
      message: 'success',
      data: { items: [], next_page_token: null },
    };
    globalThis.fetch = mock(async () => new Response(JSON.stringify(body))) as unknown as typeof fetch;

    await expect(listJobs()).resolves.toEqual({ items: [], next_page_token: null });
  });

  test('throws when business code is non-zero', async () => {
    const body = {
      code: 40001,
      message: 'bad request',
      data: null,
    };
    globalThis.fetch = mock(async () => new Response(JSON.stringify(body), { status: 400 })) as unknown as typeof fetch;

    await expect(listJobs()).rejects.toBeInstanceOf(ApiClientError);
  });

  test('loads instance logs through the envelope', async () => {
    const body = {
      code: 0,
      message: 'success',
      data: {
        items: [{ id: 'log_1', instance_id: 'inst_1', worker_id: 'worker_1', level: 'info', message: 'hello', sequence: 1, created_at: '2026-05-19T00:00:00Z' }],
        next_page_token: null,
      },
    };
    globalThis.fetch = mock(async () => new Response(JSON.stringify(body))) as unknown as typeof fetch;

    await expect(listInstanceLogs('inst_1')).resolves.toEqual(body.data);
  });

  test('stores login token and sends authorization for protected mutations', async () => {
    const calls: RequestInit[] = [];
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { token: 'atk_test_token', username: 'scheduler_init', roles: ['admin'], permissions: [{ resource: 'users', action: 'manage' }] },
      }));
    }) as unknown as typeof fetch;

    await login({ username: 'scheduler_init', password: 'Scheduler@2026!' });

    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'job_1', namespace: 'default', app: 'default', name: 'demo', schedule_type: 'api', schedule_expr: null, enabled: true },
      }));
    }) as unknown as typeof fetch;

    await createJob({ name: 'demo' });
    const headers = calls.at(-1)?.headers;
    expect(headers).toBeInstanceOf(Headers);
    expect((headers as Headers).get('authorization')).toBe('Bearer atk_test_token');
  });

  test('sends authorization when triggering a job', async () => {
    setAuthToken('atk_test_token');
    let capturedHeaders = new Headers();
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      capturedHeaders = init?.headers as Headers;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'inst_1', job_id: 'job_1', status: 'pending', trigger_type: 'api', execution_mode: 'single', created_at: 'now', updated_at: 'now' },
      }));
    }) as unknown as typeof fetch;

    await triggerJob('job_1');

    expect(capturedHeaders.get('authorization')).toBe('Bearer atk_test_token');
  });
});
