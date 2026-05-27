import { afterEach, describe, expect, mock, test } from 'bun:test';

import { ApiClientError, createAppScope, createJob, createNamespace, createWorkerPool, dryRunWorkflow, getAuthToken, listInstanceLogs, listJobs, listNamespaces, listWorkerPools, login, rotateSdkApiKey, setAuthErrorHandler, setAuthToken, triggerJob, updateWorkflow } from './client';

const originalFetch = globalThis.fetch;

function resetTokenStorage() {
  setAuthToken(null);
}

afterEach(() => {
  globalThis.fetch = originalFetch;
  resetTokenStorage();
  setAuthErrorHandler(null);
});

describe('api client envelope handling', () => {
  test('returns data when code is zero', async () => {
    const body = {
      code: 0,
      message: 'success',
      data: { items: [], nextPageToken: null },
    };
    globalThis.fetch = mock(async () => new Response(JSON.stringify(body))) as unknown as typeof fetch;

    await expect(listJobs()).resolves.toEqual({ items: [], nextPageToken: null });
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



  test('clears token and notifies handler on 401 envelope', async () => {
    setAuthToken('expired_token');
    let unauthorized = false;
    setAuthErrorHandler({ onUnauthorized: () => { unauthorized = true; } });
    globalThis.fetch = mock(async () => new Response(JSON.stringify({
      code: 40101,
      message: 'unauthorized',
      data: null,
    }), { status: 401 })) as unknown as typeof fetch;

    await expect(listJobs()).rejects.toMatchObject({ status: 401, code: 40101 });

    expect(getAuthToken()).toBeNull();
    expect(unauthorized).toBe(true);
  });

  test('notifies handler on 403 envelope without clearing token', async () => {
    setAuthToken('valid_token');
    let forbiddenMessage = '';
    setAuthErrorHandler({ onForbidden: (message) => { forbiddenMessage = message; } });
    globalThis.fetch = mock(async () => new Response(JSON.stringify({
      code: 40301,
      message: 'forbidden',
      data: null,
    }), { status: 403 })) as unknown as typeof fetch;

    await expect(listJobs()).rejects.toMatchObject({ status: 403, code: 40301 });

    expect(getAuthToken()).toBe('valid_token');
    expect(forbiddenMessage).toBe('forbidden');
  });

  test('loads instance logs through the envelope', async () => {
    const body = {
      code: 0,
      message: 'success',
      data: {
        items: [{ id: 'log_1', instanceId: 'inst_1', workerId: 'worker_1', level: 'warn', message: 'runtime missing', governanceEvent: 'script_execution_governance', governanceFailureClass: 'script_runtime_unavailable', governanceMessage: 'runtime missing', sequence: 1, createdAt: '2026-05-19T00:00:00Z' }],
        nextPageToken: null,
      },
    };
    globalThis.fetch = mock(async () => new Response(JSON.stringify(body))) as unknown as typeof fetch;

    await expect(listInstanceLogs('inst_1', { governanceOnly: true })).resolves.toEqual(body.data);
    expect(fetch).toHaveBeenCalledWith('/api/v1/instances/inst_1/logs?page_token=script_execution_governance', expect.any(Object));
  });

  test('normalizes legacy workflow edge conditions before workflow mutations', async () => {
    const bodies: unknown[] = [];
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      bodies.push(JSON.parse(String(init?.body)));
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          id: 'wf_1',
          name: 'legacy-flow',
          definition: { nodes: [], edges: [] },
          status: 'active',
          createdBy: 'usr-admin',
          createdAt: 'now',
          updatedAt: 'now',
        },
      }));
    }) as unknown as typeof fetch;

    await updateWorkflow('wf_1', {
      name: 'legacy-flow',
      definition: {
        nodes: [
          { key: 'hello', kind: 'job' },
          { key: 'report', kind: 'job' },
        ],
        edges: [{ from: 'hello', to: 'report', condition: 'success' as never }],
      },
    });

    expect(bodies.at(-1)).toMatchObject({
      definition: { edges: [{ condition: 'on_success' }] },
    });
  });

  test('normalizes legacy workflow edge conditions before dry-run validation', async () => {
    let body: unknown = null;
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      body = JSON.parse(String(init?.body));
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { validation: { valid: true, errors: [] }, startNodes: ['hello'], nodeCount: 2, edgeCount: 1 },
      }));
    }) as unknown as typeof fetch;

    await dryRunWorkflow({
      nodes: [
        { key: 'hello', kind: 'job' },
        { key: 'report', kind: 'job' },
      ],
      edges: [{ from: 'hello', to: 'report', condition: 'failed' as never }],
    });

    expect(body).toMatchObject({ edges: [{ condition: 'on_failure' }] });
  });

  test('stores login token and sends authorization for protected mutations', async () => {
    const calls: RequestInit[] = [];
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { token: 'atk_test_token', username: 'tikee_init', roles: ['admin'], permissions: [{ resource: 'users', action: 'manage' }] },
      }));
    }) as unknown as typeof fetch;

    await login({ username: 'tikee_init', password: 'Tikee@2026!' });

    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'job_1', namespace: 'default', app: 'default', name: 'demo', scheduleType: 'api', scheduleExpr: null, enabled: true },
      }));
    }) as unknown as typeof fetch;

    await createJob({ name: 'demo' });
    const headers = calls.at(-1)?.headers;
    expect(headers).toBeInstanceOf(Headers);
    expect((headers as Headers).get('authorization')).toBe('Bearer atk_test_token');
  });



  test('loads and creates tenant scope resources through management endpoints', async () => {
    const calls: Array<{ url: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : undefined });
      if (String(url).includes('/worker-pools')) {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: [{ id: 'wp_1', namespace: 'default', app: 'billing', name: 'critical', createdAt: 'now', updatedAt: 'now' }],
        }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: [{ id: 'ns_1', name: 'default', createdAt: 'now', updatedAt: 'now' }],
      }));
    }) as unknown as typeof fetch;

    await expect(listNamespaces()).resolves.toEqual([{ id: 'ns_1', name: 'default', createdAt: 'now', updatedAt: 'now' }]);
    await expect(listWorkerPools({ namespace: 'default', app: 'billing' })).resolves.toEqual([{ id: 'wp_1', namespace: 'default', app: 'billing', name: 'critical', createdAt: 'now', updatedAt: 'now' }]);

    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : undefined });
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { id: 'ok', name: 'ok', namespace: 'default', app: 'billing', createdAt: 'now', updatedAt: 'now' } }));
    }) as unknown as typeof fetch;

    await createNamespace({ name: 'payments' });
    await createAppScope({ namespace: 'payments', name: 'settlement' });
    await createWorkerPool({ namespace: 'payments', app: 'settlement', name: 'critical' });

    expect(calls.map((call) => call.url)).toContain('/api/v1/namespaces');
    expect(calls.map((call) => call.url)).toContain('/api/v1/worker-pools?namespace=default&app=billing');
    expect(calls.at(-1)?.body).toEqual({ namespace: 'payments', app: 'settlement', name: 'critical' });
  });

  test('rotates sdk api key through rotate endpoint', async () => {
    let capturedUrl = '';
    let capturedBody: unknown = null;
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      capturedUrl = String(url);
      capturedBody = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          api_key: 'tk-rotated',
          key: { id: 'sk_new', name: 'demo', key_prefix: 'tk-rot••••new', namespace: 'default', app: 'billing', scopes: ['jobs:read'], status: 'active', expires_at: null, last_used_at: null, created_by: 'admin', revoked_by: null, rotated_from: 'sk_old', created_at: 'now', updated_at: 'now' },
        },
      }));
    }) as unknown as typeof fetch;

    await expect(rotateSdkApiKey('sk_old', { scopes: ['jobs:read'], expires_at: null })).resolves.toMatchObject({ api_key: 'tk-rotated' });

    expect(capturedUrl).toBe('/api/v1/management/api-keys/sk_old/rotate');
    expect(capturedBody).toEqual({ scopes: ['jobs:read'], expires_at: null });
  });

  test('sends authorization when triggering a job', async () => {
    setAuthToken('atk_test_token');
    let capturedHeaders = new Headers();
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      capturedHeaders = init?.headers as Headers;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'inst_1', jobId: 'job_1', status: 'pending', triggerType: 'api', executionMode: 'single', createdAt: 'now', updatedAt: 'now' },
      }));
    }) as unknown as typeof fetch;

    await triggerJob('job_1');

    expect(capturedHeaders.get('authorization')).toBe('Bearer atk_test_token');
  });
});
