import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const stylesSource = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');
const shellSource = readFileSync(new URL('../../components/AppShell.tsx', import.meta.url), 'utf8');

describe('responsive console shell', () => {
  test('keeps shell, toolbars, tables, and drawers usable on mobile widths', () => {
    expect(shellSource).toContain('breakpoint="lg"');
    expect(shellSource).toContain('collapsedWidth="0"');
    expect(stylesSource).toContain('@media (max-width: 767px)');
    expect(stylesSource).toContain('.app-shell__user');
    expect(stylesSource).toContain('.ant-table-wrapper');
    expect(stylesSource).toContain('overflow-x: auto');
    expect(stylesSource).toContain('.ant-drawer-content-wrapper');
    expect(stylesSource).toContain('max-width: 100vw');
  });
});
