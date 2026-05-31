import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../ScopesPage.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('tenant scope management page', () => {
  test('exposes a governed route and menu entry for tenant scopes', () => {
    expect(routesSource).toContain('scopes');
    expect(routesSource).toContain('/scopes');
    expect(routesSource).toContain("resource: 'tenants'");
    expect(appSource).toContain('ScopesPage');
    expect(appSource).toContain('ROUTE_META.scopes.path');
  });

  test('builds namespace app and worker-pool management around focused cards', () => {
    expect(pageSource).toContain('listNamespaces');
    expect(pageSource).toContain('createNamespace');
    expect(pageSource).toContain('createAppScope');
    expect(pageSource).toContain('createWorkerPool');
    expect(pageSource).toContain('deleteNamespace');
    expect(pageSource).toContain('deleteAppScope');
    expect(pageSource).toContain('deleteWorkerPool');
    expect(pageSource).toContain('listOidcIdentities');
    expect(pageSource).toContain('upsertOidcIdentity');
    expect(pageSource).toContain('deleteOidcIdentity');
    expect(pageSource).toContain('createSecret');
    expect(pageSource).toContain('handleSecretCreate');
    expect(pageSource).toContain("drawer === 'secret'");
    expect(pageSource).toContain('新建 Secret 引用');
    expect(pageSource).toContain('OIDC tenant/app/role 绑定');
    expect(pageSource).toContain('命名空间');
    expect(pageSource).toContain('应用');
    expect(pageSource).toContain('Worker Pool');
    expect(pageSource).toContain('confirmTitle="删除命名空间"');
    expect(pageSource).toContain('confirmTitle="删除应用"');
    expect(pageSource).toContain('confirmTitle="删除 Worker Pool"');
    expect(styles).toContain('.scope-management-page');
  });
});
