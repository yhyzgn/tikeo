import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const logoSource = readFileSync(new URL('../../components/TikeoLogo.tsx', import.meta.url), 'utf8');
const shellSource = readFileSync(new URL('../../components/AppShell.tsx', import.meta.url), 'utf8');
const loginSource = readFileSync(new URL('../LoginPage.tsx', import.meta.url), 'utf8');
const setupSource = readFileSync(new URL('../SuperAdminSetupPage.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('tikeo animated logo', () => {
  test('defines a modern task-flow logo component', () => {
    expect(logoSource).toContain('export function TikeoLogo');
    expect(logoSource).toContain('tikeo-logo__flow');
    expect(logoSource).toContain('tikeo-logo__arrow');
    expect(logoSource).toContain('tikeo-logo__node');
    expect(logoSource).toContain('aria-label="tikeo task orchestration logo"');
    expect(logoSource).toContain('viewBox="4 4 56 56"');
  });

  test('uses the animated logo in shell and auth entry pages', () => {
    expect(shellSource).toContain('<TikeoLogo size={64} />');
    expect(loginSource).toContain('<TikeoLogo size={96} showWordmark />');
    expect(setupSource).toContain('<TikeoLogo size={96} showWordmark />');
  });

  test('styles logo motion and dark-mode compatibility', () => {
    expect(styles).toContain('@keyframes tikeo-logo-flow');
    expect(styles).toContain('@keyframes tikeo-logo-node-pulse');
    expect(styles).toContain('@keyframes tikeo-logo-arrow');
    expect(styles).toContain('.tikeo-logo__flow');
    expect(styles).toContain('html[data-theme="dark"] .tikeo-logo');
    expect(styles).toContain('--tikeo-logo-accent');
    expect(styles).toContain('--tikeo-logo-node-fill');
    expect(styles).toContain('.app-shell__brand .tikeo-logo__mark');
  });
});
