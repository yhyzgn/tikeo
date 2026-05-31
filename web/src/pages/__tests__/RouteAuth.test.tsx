import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');

describe('route defaults and authenticated login bypass', () => {
  test('root domain explicitly redirects to the dashboard route', () => {
    expect(appSource).toContain('path="/"');
    expect(appSource).toContain('to={ROUTE_META.dashboard.path}');
  });

  test('login route bypasses the login page before rendering it when an auth token already exists', () => {
    expect(appSource).toContain('function LoginRoute({ bootstrap }');
    expect(appSource).toContain('getAuthToken() !== null');
    expect(appSource).toContain('element={<LoginRoute bootstrap={bootstrap} />}');
    expect(appSource).not.toContain('path="/login" element={<LoginPage />}');
    expect(appSource).toContain('bootstrap.registrationOpen');
    expect(appSource).toContain('to="/setup"');
  });
});
