import { afterEach, describe, expect, mock, test } from 'bun:test';

import { ApiClientError, createAppScope, createCalendar, createJob, createNamespace, createPlugin, createSdkApiKey, createServiceAccount, createWorkerPool, deletePlugin, disableServiceAccount, dryRunWorkflow, getAuthToken, listInstanceLogs, getJobImpact, getJobSchedulingAdvice, getJobTopology, getWorkflowReplay, listJobVersions, listJobs, listNamespaces, listPlugins, listServiceAccounts, listWorkerPools, login, rollbackJob, setAuthErrorHandler, setAuthToken, triggerJob, triggerJobWebhookEvent, updateJob, updatePlugin, updateSdkApiKey, updateServiceAccount, updateWorkflow } from './client';

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
        data: { token: 'atk_test_token', username: 'bootstrap_admin', roles: ['admin'], permissions: [{ resource: 'users', action: 'manage' }] },
      }));
    }) as unknown as typeof fetch;

    await login({ username: 'bootstrap_admin', password: 'Tikee@2026!' });

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

  test('sends job create and update payloads in server camelCase contract', async () => {
    const calls: Array<{ method: string; url: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({
        method: init?.method ?? 'GET',
        url: String(url),
        body: init?.body ? JSON.parse(String(init.body)) : undefined,
      });
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          id: 'job_1',
          namespace: 'default',
          app: 'billing',
          name: 'script job',
          scheduleType: 'api',
          scheduleExpr: null,
          misfirePolicy: 'fire_once',
          scheduleStartAt: null,
          scheduleEndAt: null,
          scheduleCalendar: null,
          processorName: null,
          processorType: null,
          scriptId: 'scr_shell',
          enabled: true,
          canaryJobId: null,
          canaryPercent: 0,
          versionNumber: 1,
        },
      }));
    }) as unknown as typeof fetch;

    await createJob({
      namespace: 'default',
      app: 'billing',
      name: 'script job',
      scheduleType: 'api',
      processorName: null,
      processorType: null,
      scriptId: 'scr_shell',
      scheduleStartAt: null,
      scheduleEndAt: null,
      scheduleCalendar: { ref: 'cal_default' },
      enabled: true,
    });
    await updateJob('job_1', {
      name: 'plugin job',
      scheduleType: 'cron',
      scheduleExpr: '0 0 * * * * *',
      processorType: 'sql',
      processorName: 'billing.sql-sync',
      scriptId: null,
      enabled: false,
    });

    expect(calls.map((call) => `${call.method} ${call.url}`)).toEqual([
      'POST /api/v1/jobs',
      'PATCH /api/v1/jobs/job_1',
    ]);
    expect(calls[0].body).toEqual({
      namespace: 'default',
      app: 'billing',
      name: 'script job',
      scheduleType: 'api',
      processorName: null,
      processorType: null,
      scriptId: 'scr_shell',
      scheduleStartAt: null,
      scheduleEndAt: null,
      scheduleCalendar: { ref: 'cal_default' },
      enabled: true,
    });
    expect(calls[1].body).toEqual({
      name: 'plugin job',
      scheduleType: 'cron',
      scheduleExpr: '0 0 * * * * *',
      processorType: 'sql',
      processorName: 'billing.sql-sync',
      scriptId: null,
      enabled: false,
    });
  });

  test('manages plugin registry through CRUD endpoints', async () => {
    const calls: Array<{ method: string; url: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({
        method: init?.method ?? 'GET',
        url: String(url),
        body: init?.body ? JSON.parse(String(init.body)) : undefined,
      });
      if ((init?.method ?? 'GET') === 'GET') {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: [{
            id: 'plugin_ops',
            name: 'Ops Plugin',
            kind: 'mixed',
            processorTypes: [{ type: 'sql', label: 'SQL Processor', capability: 'sql', processorNames: ['billing.sql-sync'], description: null }],
            alertChannelTypes: [{ type: 'ops_webhook', label: 'Ops Webhook', targetKind: 'webhook', description: null, template: { body: { text: '{{message}}' } } }],
            enabled: true,
            createdAt: 'now',
            updatedAt: 'now',
          }],
        }));
      }
      if (init?.method === 'DELETE') {
        return new Response(JSON.stringify({ code: 0, message: 'success', data: {} }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          id: 'plugin_ops',
          name: 'Ops Plugin',
          kind: 'mixed',
          processorTypes: [{ type: 'sql', label: 'SQL Processor', capability: 'sql', processorNames: ['billing.sql-sync'], description: null }],
          alertChannelTypes: [{ type: 'ops_webhook', label: 'Ops Webhook', targetKind: 'webhook', description: null, template: { body: { text: '{{message}}' } } }],
          enabled: true,
          createdAt: 'now',
          updatedAt: 'now',
        },
      }));
    }) as unknown as typeof fetch;

    const payload = {
      name: 'Ops Plugin',
      kind: 'mixed',
      enabled: true,
      processorTypes: [{ type: 'sql', label: 'SQL Processor', capability: 'sql', processorNames: ['billing.sql-sync'], description: null }],
      alertChannelTypes: [{ type: 'ops_webhook', label: 'Ops Webhook', targetKind: 'webhook', description: null, template: { body: { text: '{{message}}' } } }],
    };

    await expect(listPlugins()).resolves.toHaveLength(1);
    await expect(createPlugin(payload)).resolves.toMatchObject({ id: 'plugin_ops' });
    await expect(updatePlugin('plugin_ops', { ...payload, enabled: false })).resolves.toMatchObject({ id: 'plugin_ops' });
    await expect(deletePlugin('plugin_ops')).resolves.toBeUndefined();

    expect(calls.map((call) => `${call.method} ${call.url}`)).toEqual([
      'GET /api/v1/plugins',
      'POST /api/v1/plugins',
      'PATCH /api/v1/plugins/plugin_ops',
      'DELETE /api/v1/plugins/plugin_ops',
    ]);
    expect(calls[1].body).toMatchObject({ processorTypes: [{ type: 'sql' }], alertChannelTypes: [{ type: 'ops_webhook' }] });
    expect(calls[2].body).toMatchObject({ enabled: false });
  });



  test('loads and creates tenant scope resources through management endpoints', async () => {
    const calls: Array<{ url: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : undefined });
      if (String(url).includes('/worker-pools')) {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: [{ id: 'wp_1', namespace: 'default', app: 'billing', name: 'critical', maxQueueDepth: 0, maxConcurrency: 0, createdAt: 'now', updatedAt: 'now' }],
        }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: [{ id: 'ns_1', name: 'default', createdAt: 'now', updatedAt: 'now' }],
      }));
    }) as unknown as typeof fetch;

    await expect(listNamespaces()).resolves.toEqual([{ id: 'ns_1', name: 'default', createdAt: 'now', updatedAt: 'now' }]);
    await expect(listWorkerPools({ namespace: 'default', app: 'billing' })).resolves.toEqual([{ id: 'wp_1', namespace: 'default', app: 'billing', name: 'critical', maxQueueDepth: 0, maxConcurrency: 0, createdAt: 'now', updatedAt: 'now' }]);

    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : undefined });
      return new Response(JSON.stringify({ code: 0, message: 'success', data: { id: 'ok', name: 'ok', namespace: 'default', app: 'billing', maxQueueDepth: 0, maxConcurrency: 0, createdAt: 'now', updatedAt: 'now' } }));
    }) as unknown as typeof fetch;

    await createNamespace({ name: 'payments' });
    await createAppScope({ namespace: 'payments', name: 'settlement' });
    await createWorkerPool({ namespace: 'payments', app: 'settlement', name: 'critical' });

    expect(calls.map((call) => call.url)).toContain('/api/v1/namespaces');
    expect(calls.map((call) => call.url)).toContain('/api/v1/worker-pools?namespace=default&app=billing');
    expect(calls.at(-1)?.body).toEqual({ namespace: 'payments', app: 'settlement', name: 'critical' });
  });

  test('creates calendars with typed date arrays and start/end window payloads', async () => {
    let capturedBody: unknown = null;
    globalThis.fetch = mock(async (_url: string | URL | Request, init?: RequestInit) => {
      capturedBody = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          id: 'cal_1',
          namespace: 'default',
          app: 'billing',
          name: 'cn-maintenance',
          timezone: 'Asia/Shanghai',
          excludedDates: ['2026-10-01'],
          holidays: ['2026-10-02'],
          maintenanceWindows: [{ start: '2026-06-01T01:00:00.000Z', end: '2026-06-01T02:00:00.000Z' }],
          freezeWindows: [{ start: '2026-06-02T01:00:00.000Z', end: '2026-06-02T02:00:00.000Z' }],
          createdBy: 'admin',
          createdAt: 'now',
          updatedAt: 'now',
        },
      }));
    }) as unknown as typeof fetch;

    await expect(createCalendar({
      namespace: 'default',
      app: 'billing',
      name: 'cn-maintenance',
      timezone: 'Asia/Shanghai',
      excludedDates: ['2026-10-01'],
      holidays: ['2026-10-02'],
      maintenanceWindows: [{ start: '2026-06-01T01:00:00.000Z', end: '2026-06-01T02:00:00.000Z' }],
      freezeWindows: [{ start: '2026-06-02T01:00:00.000Z', end: '2026-06-02T02:00:00.000Z' }],
    })).resolves.toMatchObject({ id: 'cal_1' });

    expect(capturedBody).toEqual({
      namespace: 'default',
      app: 'billing',
      name: 'cn-maintenance',
      timezone: 'Asia/Shanghai',
      excludedDates: ['2026-10-01'],
      holidays: ['2026-10-02'],
      maintenanceWindows: [{ start: '2026-06-01T01:00:00.000Z', end: '2026-06-01T02:00:00.000Z' }],
      freezeWindows: [{ start: '2026-06-02T01:00:00.000Z', end: '2026-06-02T02:00:00.000Z' }],
    });
  });

  test('manages service accounts and binds sdk api keys by existing id', async () => {
    const calls: Array<{ method: string; url: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({
        method: init?.method ?? 'GET',
        url: String(url),
        body: init?.body ? JSON.parse(String(init.body)) : undefined,
      });
      if ((init?.method ?? 'GET') === 'GET') {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: [{ id: 'sa_1', name: 'java-demo-sa', description: null, namespace: 'default', app: 'billing', workerPool: null, status: 'active', createdBy: 'admin', updatedBy: null, createdAt: 'now', updatedAt: 'now' }],
        }));
      }
      if (init?.method === 'DELETE') {
        return new Response(JSON.stringify({ code: 0, message: 'success', data: {} }));
      }
      if (String(url).endsWith('/api-keys')) {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: {
            api_key: 'tk-AbCdEf0123456789AbCdEf0123456789AbCdEf0123456789AbCdEf0123456789',
            key: { id: 'sk_1', name: 'demo-key', key_prefix: 'tk-AbCd••••6789', namespace: 'default', app: 'billing', service_account_id: 'sa_1', service_account_name: 'java-demo-sa', scopes: ['jobs:read'], status: 'active', expires_at: null, last_used_at: null, created_by: 'admin', revoked_by: null, rotated_from: null, created_at: 'now', updated_at: 'now' },
          },
        }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'sa_1', name: 'java-demo-sa', description: 'demo', namespace: 'default', app: 'billing', workerPool: null, status: 'active', createdBy: 'admin', updatedBy: null, createdAt: 'now', updatedAt: 'now' },
      }));
    }) as unknown as typeof fetch;

    await expect(listServiceAccounts()).resolves.toHaveLength(1);
    await expect(createServiceAccount({ name: 'java-demo-sa', description: 'demo', namespace: 'default', app: 'billing' })).resolves.toMatchObject({ id: 'sa_1' });
    await expect(updateServiceAccount('sa_1', { name: 'java-demo-sa', description: 'demo', namespace: 'default', app: 'billing', status: 'active' })).resolves.toMatchObject({ id: 'sa_1' });
    await expect(createSdkApiKey({ name: 'demo-key', namespace: 'default', app: 'billing', service_account_id: 'sa_1', scopes: ['jobs:read'], expires_at: null })).resolves.toMatchObject({ key: { service_account_id: 'sa_1' } });
    await expect(disableServiceAccount('sa_1')).resolves.toBeUndefined();

    expect(calls.map((call) => `${call.method} ${call.url}`)).toEqual([
      'GET /api/v1/management/service-accounts',
      'POST /api/v1/management/service-accounts',
      'PATCH /api/v1/management/service-accounts/sa_1',
      'POST /api/v1/management/api-keys',
      'DELETE /api/v1/management/service-accounts/sa_1',
    ]);
    expect(calls[3].body).toMatchObject({ service_account_id: 'sa_1' });
  });

  test('updates sdk api key metadata through patch endpoint', async () => {
    let capturedUrl = '';
    let capturedBody: unknown = null;
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      capturedUrl = String(url);
      capturedBody = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: {
          id: 'sk_old', name: 'demo', key_prefix: 'tk-demo••••old', namespace: 'default', app: 'billing', scopes: ['jobs:read'], status: 'active', expires_at: null, last_used_at: null, created_by: 'admin', revoked_by: null, rotated_from: null, created_at: 'now', updated_at: 'now',
        },
      }));
    }) as unknown as typeof fetch;

    await expect(updateSdkApiKey('sk_old', { name: 'demo-renamed', scopes: ['jobs:read'], expires_at: null })).resolves.toMatchObject({ id: 'sk_old' });

    expect(capturedUrl).toBe('/api/v1/management/api-keys/sk_old');
    expect(capturedBody).toEqual({ name: 'demo-renamed', scopes: ['jobs:read'], expires_at: null });
  });

  test('loads and rolls back job versions', async () => {
    const captured: string[] = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      captured.push(`${init?.method ?? 'GET'} ${String(url)} ${init?.body ?? ''}`);
      if (String(url).endsWith('/versions')) {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: { items: [{ id: 'jv_1', job_id: 'job_1', version_number: 1, name: 'demo', schedule_type: 'api', schedule_expr: null, processor_name: 'demo.echo', script_id: null, enabled: true, created_by: 'admin', change_reason: 'create', rolled_back_from_version: null, created_at: 'now' }], nextPageToken: null },
        }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'job_1', namespace: 'default', app: 'billing', name: 'demo', scheduleType: 'api', scheduleExpr: null, processorName: 'demo.echo', scriptId: null, enabled: true, versionNumber: 2 },
      }));
    }) as unknown as typeof fetch;

    await expect(listJobVersions('job_1')).resolves.toMatchObject({ items: [{ version_number: 1 }] });
    await expect(rollbackJob('job_1', 1)).resolves.toMatchObject({ id: 'job_1', versionNumber: 2 });

    expect(captured[0]).toBe('GET /api/v1/jobs/job_1/versions ');
    expect(captured[1]).toBe('POST /api/v1/jobs/job_1/rollback {"versionNumber":1}');
  });


  test('loads job topology graph', async () => {
    globalThis.fetch = mock(async () => new Response(JSON.stringify({
      code: 0,
      message: 'success',
      data: {
        nodes: [{ id: 'job_a', type: 'job', label: 'A', namespace: 'default', app: 'billing', metadata: {} }],
        edges: [{ id: 'edge_1', from: 'job_a', to: 'job_b', type: 'workflow_job_dependency', label: 'on_success', workflowId: 'wf_1', workflowName: 'Billing', condition: 'on_success', metadata: {} }],
        unresolved: [{ workflowId: 'wf_1', workflowName: 'Billing', nodeKey: 'missing', missingJobId: 'job_missing', reason: 'workflow node references missing job' }],
      },
    }))) as unknown as typeof fetch;

    await expect(getJobTopology()).resolves.toMatchObject({
      edges: [{ from: 'job_a', to: 'job_b' }],
      unresolved: [{ missingJobId: 'job_missing' }],
    });
    expect(fetch).toHaveBeenCalledWith('/api/v1/jobs/topology', expect.any(Object));
  });


  test('triggers job through inbound webhook event endpoint', async () => {
    let capturedUrl = '';
    let capturedBody: unknown = null;
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      capturedUrl = String(url);
      capturedBody = init?.body ? JSON.parse(String(init.body)) : null;
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { accepted: true, instanceId: 'inst_webhook', jobId: 'job_webhook', status: 'pending', triggerType: 'webhook' },
      }));
    }) as unknown as typeof fetch;

    await expect(triggerJobWebhookEvent('job_webhook', { source: 'gitlab', eventType: 'push', payload: { sha: 'abc123' } })).resolves.toMatchObject({ accepted: true, triggerType: 'webhook' });
    expect(capturedUrl).toBe('/api/v1/events/webhooks/job_webhook:trigger');
    expect(capturedBody).toEqual({ source: 'gitlab', eventType: 'push', payload: { sha: 'abc123' } });
  });


  test('loads scheduling advice for a job', async () => {
    globalThis.fetch = mock(async () => new Response(JSON.stringify({
      code: 0,
      message: 'success',
      data: { ready: true, severity: 'ok', reason: '1 eligible worker online', requiredCapability: "SDK processor 'demo.echo'", eligibleWorkers: ['worker-1'], recentInstances: 3, recentFailures: 0, history: { inspectedInstances: 3, completedInstances: 2, failedInstances: 0, averageDurationSeconds: 20, p50DurationSeconds: 10, p95DurationSeconds: 30, maxDurationSeconds: 30 }, prediction: { estimatedDurationSeconds: 30, recommendedConcurrency: 1, workerCapacity: { eligibleWorkerCount: 1, advertisedCpuCores: 4, advertisedMemoryMb: 8192 }, reasons: ['history uses 2 completed instance(s)'] } },
    }))) as unknown as typeof fetch;

    await expect(getJobSchedulingAdvice('job_advice')).resolves.toMatchObject({ ready: true, requiredCapability: "SDK processor 'demo.echo'", history: { p95DurationSeconds: 30 }, prediction: { estimatedDurationSeconds: 30 } });
    expect(fetch).toHaveBeenCalledWith('/api/v1/jobs/job_advice/scheduling-advice', expect.any(Object));
  });


  test('supports canary fields in job creation and trigger response', async () => {
    const calls: Array<{ url: string; body?: unknown }> = [];
    globalThis.fetch = mock(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : undefined });
      if (String(url).endsWith(':trigger')) {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: { id: 'inst_canary', jobId: 'job_canary', status: 'pending', triggerType: 'api', executionMode: 'single', createdAt: 'now', updatedAt: 'now', logCount: 0, latestLog: null, workerId: null, canaryRouting: { enabled: true, routed: true, originalJobId: 'job_main', routedJobId: 'job_canary', percent: 100 } },
        }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { id: 'job_main', namespace: 'default', app: 'billing', name: 'main', scheduleType: 'api', scheduleExpr: null, processorName: 'main', scriptId: null, enabled: true, versionNumber: 1, canaryJobId: 'job_canary', canaryPercent: 100 },
      }));
    }) as unknown as typeof fetch;

    await expect(createJob({ name: 'main', canaryJobId: 'job_canary', canaryPercent: 100 })).resolves.toMatchObject({ canaryJobId: 'job_canary', canaryPercent: 100 });
    await expect(triggerJob('job_main')).resolves.toMatchObject({ canaryRouting: { routed: true, routedJobId: 'job_canary' } });
    expect(calls[0].body).toMatchObject({ canaryJobId: 'job_canary', canaryPercent: 100 });
  });


  test('loads job impact analysis and workflow replay bundles', async () => {
    const urls: string[] = [];
    globalThis.fetch = mock(async (url: string | URL | Request) => {
      urls.push(String(url));
      if (String(url).includes('/impact')) {
        return new Response(JSON.stringify({
          code: 0,
          message: 'success',
          data: { targetJob: { id: 'job_mid', name: 'mid' }, referencingWorkflows: [{ id: 'wf_1', name: 'flow' }], upstreamJobs: [{ id: 'job_a', name: 'a' }], downstreamJobs: [{ id: 'job_b', name: 'b' }], riskSummary: { workflowCount: 1, upstreamCount: 1, downstreamCount: 1, unresolvedCount: 0, riskLevel: 'medium', reasons: ['referenced by 1 workflow(s)'] } },
        }));
      }
      return new Response(JSON.stringify({
        code: 0,
        message: 'success',
        data: { instance: { id: 'wfi_1' }, workflow: { id: 'wf_1' }, events: [{ id: 'evt_1', eventType: 'workflow_started' }], graph: { nodes: [{ id: 'run', type: 'workflow_node', label: 'run', namespace: null, app: null, metadata: { position: { x: 0, y: 0 }, layer: 0 } }], edges: [], unresolved: [] } },
      }));
    }) as unknown as typeof fetch;

    await expect(getJobImpact('job_mid')).resolves.toMatchObject({ riskSummary: { workflowCount: 1 } });
    await expect(getWorkflowReplay('wfi_1')).resolves.toMatchObject({ instance: { id: 'wfi_1' }, events: [{ id: 'evt_1' }] });
    expect(urls).toEqual(['/api/v1/jobs/job_mid/impact', '/api/v1/workflow-instances/wfi_1/replay']);
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
