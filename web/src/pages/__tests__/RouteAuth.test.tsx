import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const loginSource = readFileSync(new URL('../LoginPage.tsx', import.meta.url), 'utf8');

describe('route defaults and authenticated login bypass', () => {
  test('root domain explicitly redirects to the dashboard route', () => {
    expect(appSource).toContain('path="/"');
    expect(appSource).toContain('to={ROUTE_META.dashboard.path}');
  });

  test('login page skips itself when an auth token already exists', () => {
    expect(loginSource).toContain('getAuthToken');
    expect(loginSource).toContain('useEffect');
    expect(loginSource).toContain('navigate(ROUTE_META.dashboard.path');
  });
});
