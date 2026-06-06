import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const loginSource = readFileSync(new URL('../LoginPage.tsx', import.meta.url), 'utf8');
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
});
