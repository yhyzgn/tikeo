import { afterEach, describe, expect, mock, test } from 'bun:test';

import { ApiClientError, listJobs } from './client';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
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
});
