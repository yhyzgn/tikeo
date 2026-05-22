import { afterEach, describe, expect, mock, test } from 'bun:test';

import { ApiClientError, createJob, dryRunWorkflow, getAuthToken, listInstanceLogs, listJobs, login, setAuthErrorHandler, setAuthToken, triggerJob, updateWorkflow } from './client';

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
        items: [{ id: 'log_1', instance_id: 'inst_1', worker_id: 'worker_1', level: 'warn', message: 'runtime missing', governance_event: 'script_execution_governance', governance_failure_class: 'script_runtime_unavailable', governance_message: 'runtime missing', sequence: 1, created_at: '2026-05-19T00:00:00Z' }],
        next_page_token: null,
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
          created_by: 'usr-admin',
          created_at: 'now',
          updated_at: 'now',
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
        data: { validation: { valid: true, errors: [] }, start_nodes: ['hello'], node_count: 2, edge_count: 1 },
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
