import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const clientSource = readFileSync(new URL('../../api/security.ts', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../SecurityPolicyCenterPage.tsx', import.meta.url), 'utf8');

describe('Security Policy Center page', () => {
  test('is a real governance route backed by security:read RBAC instead of coming soon', () => {
    expect(routesSource).toContain('security:');
    expect(routesSource).toContain("path: '/security'");
    expect(routesSource).toContain("resource: 'security'");
    expect(routesSource).toContain("group: 'governance'");
    expect(routesSource).not.toContain('security-next');
    expect(appSource).toContain('SecurityPolicyCenterPage');
    expect(appSource).toContain('ROUTE_META.security.path');
  });

  test('loads source-backed posture endpoint and renders no placeholder examples', () => {
    expect(clientSource).toContain('/api/v1/security/posture');
    expect(clientSource).toContain('SecurityPostureResponse');
    expect(pageSource).toContain('getSecurityPosture');
    expect(pageSource).toContain('scriptGovernance');
    expect(pageSource).toContain('notificationSafety');
    expect(pageSource).toContain('clusterTransport');
    expect(pageSource).toContain('transport?.http');
    expect(pageSource).toContain('transport?.workerTunnel');
    expect(pageSource).toContain('recentDenials');
    expect(pageSource).toContain('不是单纯看板');
    expect(pageSource).toContain('可操作治理入口');
    expect(pageSource).toContain('刷新态势');
    expect(pageSource).toContain('进入脚本管理');
    expect(pageSource).toContain('进入通知中心');
    expect(pageSource).toContain('进入角色管理');
    expect(pageSource).toContain('进入 API-Key');
    expect(pageSource).not.toContain('mock');
    expect(pageSource).not.toContain('TODO');
  });
});
