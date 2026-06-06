import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const loginSource = readFileSync(new URL('../LoginPage.tsx', import.meta.url), 'utf8');
const setupSource = readFileSync(new URL('../SuperAdminSetupPage.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('login page entry experience', () => {
  test('presents username-or-email password login copy', () => {
    expect(loginSource).toContain('用户名或邮箱');
    expect(loginSource).toContain('请输入用户名或邮箱');
    expect(loginSource).toContain('autoComplete="username"');
  });

  test('uses the branded split-panel login composition', () => {
    expect(loginSource).toContain('login-page__visual');
    expect(loginSource).toContain('login-page__card');
    expect(loginSource).toContain('login-page__trust-list');
    expect(styles).toContain('.login-page__visual');
    expect(styles).toContain('.login-page__card');
    expect(styles).toContain('.login-page__trust-list');
    expect(styles).toContain('html[data-theme="dark"] .login-page');
  });

  test('uses the branded split-panel setup composition', () => {
    expect(setupSource).toContain('login-page__visual setup-page__visual');
    expect(setupSource).toContain('login-page__card login-card setup-card');
    expect(setupSource).toContain('首次部署初始化');
    expect(setupSource).toContain('创建 Owner 并进入站点');
    expect(setupSource).toContain('useI18n');
  });
});
