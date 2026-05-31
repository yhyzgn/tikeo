import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const shellSource = readFileSync(new URL('../../components/AppShell.tsx', import.meta.url), 'utf8');
const themeSource = readFileSync(new URL('../../theme.ts', import.meta.url), 'utf8');
const stylesSource = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('runtime theme mode', () => {
  test('persists light/dark mode and binds Ant Design algorithm', () => {
    expect(themeSource).toContain('THEME_MODE_STORAGE_KEY');
    expect(themeSource).toContain('normalizeThemeMode');
    expect(appSource).toContain('theme.darkAlgorithm');
    expect(appSource).toContain('document.documentElement.dataset.theme = mode');
    expect(shellSource).toContain('切换暗色模式');
    expect(shellSource).toContain('toggleMode');
    expect(stylesSource).toContain("html[data-theme='dark']");
  });
});


test('runtime theme mode > keeps form controls one step taller across the app', () => {
  expect(appSource).toContain('controlHeight: 36');
  expect(appSource).toContain('controlHeightSM: 28');
  expect(appSource).toContain('controlHeightLG: 44');
  expect(stylesSource).toContain('--app-control-horizontal-padding: 16px');
});
