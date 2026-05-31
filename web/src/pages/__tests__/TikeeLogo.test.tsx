import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const logoSource = readFileSync(new URL('../../components/TikeeLogo.tsx', import.meta.url), 'utf8');
const shellSource = readFileSync(new URL('../../components/AppShell.tsx', import.meta.url), 'utf8');
const loginSource = readFileSync(new URL('../LoginPage.tsx', import.meta.url), 'utf8');
const setupSource = readFileSync(new URL('../SuperAdminSetupPage.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('tikee animated logo', () => {
  test('defines a modern task-flow logo component', () => {
    expect(logoSource).toContain('export function TikeeLogo');
    expect(logoSource).toContain('tikee-logo__flow');
    expect(logoSource).toContain('tikee-logo__arrow');
    expect(logoSource).toContain('tikee-logo__node');
    expect(logoSource).toContain('aria-label="tikee task orchestration logo"');
  });

  test('uses the animated logo in shell and auth entry pages', () => {
    expect(shellSource).toContain('<TikeeLogo size={44} />');
    expect(loginSource).toContain('<TikeeLogo size={64} showWordmark />');
    expect(setupSource).toContain('<TikeeLogo size={64} showWordmark />');
  });

  test('styles logo motion and dark-mode compatibility', () => {
    expect(styles).toContain('@keyframes tikee-logo-flow');
    expect(styles).toContain('@keyframes tikee-logo-node-pulse');
    expect(styles).toContain('@keyframes tikee-logo-arrow');
    expect(styles).toContain('.tikee-logo__flow');
    expect(styles).toContain('html[data-theme="dark"] .tikee-logo');
  });
});
