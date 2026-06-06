import { expect, test, type Page } from '@playwright/test';

type RoleSummary = {
  id: string;
  name: string;
  displayName: string;
  description: string;
  builtin: boolean;
  enabled: boolean;
  assignable: boolean;
  permissions: { resource: string; action: string }[];
  menuKeys: string[];
  uiActionKeys: string[];
  createdAt: string;
  updatedAt: string;
};

const ownerRole: RoleSummary = {
  id: 'role-owner',
  name: 'owner',
  displayName: 'owner',
  description: 'Site owner and bootstrap recovery role',
  builtin: true,
  enabled: true,
  assignable: false,
  permissions: [{ resource: 'roles', action: 'manage' }, { resource: 'roles', action: 'read' }],
  menuKeys: ['/dashboard', '/roles', '/users'],
  uiActionKeys: ['roles.create', 'roles.edit', 'roles.delete', 'roles.permissions.edit'],
  createdAt: '2026-06-07T00:00:00Z',
  updatedAt: '2026-06-07T00:00:00Z',
};

async function mockRbacApi(page: Page) {
  const roles: RoleSummary[] = [ownerRole];
  await page.route('**/api/v1/auth/me', (route) => route.fulfill({
    contentType: 'application/json',
    body: JSON.stringify({
      code: 0,
      message: 'success',
      data: {
        username: 'owner',
        roles: ['owner'],
        permissions: [{ resource: 'roles', action: 'manage' }, { resource: 'roles', action: 'read' }],
        bootstrap_admin: true,
        scope_limited: false,
        token_scopes: [],
        scope_bindings: [],
        menu_keys: ['/dashboard', '/roles', '/users'],
        ui_action_keys: ['roles.create', 'roles.edit', 'roles.delete', 'roles.permissions.edit'],
      },
    }),
  }));
  await page.route('**/api/v1/permissions/catalog', (route) => route.fulfill({
    contentType: 'application/json',
    body: JSON.stringify({ code: 0, message: 'success', data: [
      { id: 'perm-roles-read', resource: 'roles', action: 'read', description: 'Read roles and permission catalogs' },
      { id: 'perm-roles-manage', resource: 'roles', action: 'manage', description: 'Manage roles and permission matrices' },
    ] }),
  }));
  await page.route('**/api/v1/menu-permissions/catalog', (route) => route.fulfill({
    contentType: 'application/json',
    body: JSON.stringify({ code: 0, message: 'success', data: [
      { key: '/roles', label: '角色管理', group: 'governance', routePath: '/roles', requiredPermission: { resource: 'roles', action: 'read' } },
    ] }),
  }));
  await page.route('**/api/v1/ui-action-permissions/catalog', (route) => route.fulfill({
    contentType: 'application/json',
    body: JSON.stringify({ code: 0, message: 'success', data: [
      { key: 'roles.create', label: '创建角色', pageKey: '/roles', operation: 'create', dangerous: false, requiredPermission: { resource: 'roles', action: 'manage' } },
      { key: 'roles.edit', label: '编辑角色', pageKey: '/roles', operation: 'edit', dangerous: false, requiredPermission: { resource: 'roles', action: 'manage' } },
    ] }),
  }));
  await page.route('**/api/v1/roles', async (route) => {
    if (route.request().method() === 'POST') {
      const payload = route.request().postDataJSON() as { name: string; displayName: string; permissionIds?: string[]; menuKeys?: string[]; uiActionKeys?: string[] };
      roles.push({
        id: `role-${payload.name}`,
        name: payload.name,
        displayName: payload.displayName,
        description: '',
        builtin: false,
        enabled: true,
        assignable: true,
        permissions: [],
        menuKeys: payload.menuKeys ?? [],
        uiActionKeys: payload.uiActionKeys ?? [],
        createdAt: '2026-06-07T00:00:00Z',
        updatedAt: '2026-06-07T00:00:00Z',
      });
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify({ code: 0, message: 'success', data: roles.at(-1) }) });
      return;
    }
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify({ code: 0, message: 'success', data: roles }) });
  });
}

test('role management page renders owner protection and creates a managed role', async ({ page }) => {
  await mockRbacApi(page);
  await page.addInitScript(() => localStorage.setItem('tikee.auth.token', 'pw-owner-session'));

  await page.goto('/roles');

  await expect(page.getByText('Role management').first()).toBeVisible();
  const ownerRow = page.getByRole('row', { name: /owner/ });
  await expect(ownerRow.getByText('OWNER', { exact: true })).toBeVisible();
  await expect(ownerRow.getByRole('button', { name: 'Edit' })).toBeDisabled();

  await page.getByRole('button', { name: 'New role' }).click();
  await expect(page.getByRole('dialog', { name: 'Create role' })).toBeVisible();
  await page.getByLabel('Role key').fill('tenant-auditor');
  await page.getByLabel('Display name').fill('Tenant Auditor');
  await page.getByRole('button', { name: 'Create role' }).click();

  await expect(page.getByRole('cell', { name: 'Tenant Auditor' })).toBeVisible();
});
