import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const shellSource = readFileSync(new URL('./AppShell.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../routes.tsx', import.meta.url), 'utf8');

describe('app shell navigation grouping', () => {
  test('builds a two-level sidebar menu from route groups', () => {
    expect(routesSource).toContain('MENU_GROUPS');
    expect(routesSource).toContain('任务编排');
    expect(routesSource).toContain('执行资源');
    expect(routesSource).toContain('治理配置');
    expect(routesSource).toContain('观测审计');
    expect(shellSource).toContain('children');
    expect(shellSource).toContain('activeGroupKey');
    expect(shellSource).toContain('routeOpenKeys');
    expect(shellSource).toContain('openKeys={openKeys}');
    expect(shellSource).toContain('onOpenChange={(keys) => setOpenKeys(keys.map(String))}');
    expect(shellSource).toContain('[...current, ...nextKeys]');
    expect(shellSource).not.toContain('MENU_GROUPS.map((group) => `group:${group.key}`)');
    expect(shellSource).not.toContain("route.group === 'main'");
    expect(shellSource).toContain('<Sider width={304}');
    expect(shellSource).not.toContain('breakpoint=');
    expect(shellSource).not.toContain('collapsedWidth=');
  });
});
