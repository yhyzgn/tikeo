import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const jobsSource = readFileSync(new URL('../JobsPage.tsx', import.meta.url), 'utf8');
const instancesSource = readFileSync(new URL('../InstancesPage.tsx', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const shellSource = readFileSync(new URL('../../components/AppShell.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('operation button layout and theme binding', () => {
  test('keeps table row actions expanded instead of hiding them in dropdown menus', () => {
    expect(jobsSource).toContain('className="table-action-strip"');
    expect(jobsSource).toContain('单机执行');
    expect(jobsSource).toContain('广播');
    expect(jobsSource).toContain('调度建议');
    expect(jobsSource).toContain('版本');
    expect(jobsSource).toContain('编辑');
    expect(jobsSource).toContain('删除');
    expect(jobsSource).not.toContain('<Dropdown');
    expect(instancesSource).toContain('查看日志');
    expect(instancesSource).toContain('<Space size={4}>');
  });

  test('binds all Ant Design primary and link action buttons to the custom site color', () => {
    expect(appSource).toContain('colorPrimary: primaryColor');
    expect(appSource).toContain("document.documentElement.style.setProperty('--app-primary-color', primaryColor)");
    expect(shellSource).toContain('ColorPicker');
    expect(styles).toContain('.ant-btn-primary:not(:disabled)');
    expect(styles).toContain('.ant-btn-color-primary.ant-btn-variant-solid:not(:disabled)');
    expect(styles).toContain('background: var(--app-primary-color) !important');
    expect(styles).toContain('border-color: var(--app-primary-color) !important');
    expect(styles).toContain('.ant-btn-link:not(.ant-btn-dangerous)');
    expect(styles).toContain('color: var(--app-primary-color) !important');
    expect(styles).toContain('.table-action-strip .ant-btn-link:not(.ant-btn-dangerous)');
  });
});
