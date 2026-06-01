import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../GitOpsPage.tsx', import.meta.url), 'utf8');
const routeSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');

describe('GitOps IaC page', () => {
  test('exposes manifest export and desired diff as first-class page actions', () => {
    expect(routeSource).toContain('gitops');
    expect(routeSource).toContain("path: '/gitops'");
    expect(source).toContain('exportGitOpsManifest({ format: \'yaml\' })');
    expect(source).toContain('diffGitOpsManifest(desired)');
    expect(source).toContain('当前 Manifest');
    expect(source).toContain('Desired Diff');
    expect(source).toContain('执行 Diff');
    expect(source).toContain('checksum 基于 canonical JSON');
    expect(source).toContain('create/update/delete/unchanged');
  });

  test('does not present Terraform provider verification as completed in the Web page', () => {
    expect(source).not.toContain('Terraform Provider 已完成');
    expect(source).not.toContain('provider build passed');
    expect(source).not.toContain('plan/apply 已验证');
  });
});
