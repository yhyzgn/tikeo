import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const clientSource = readFileSync(new URL('../../api/client.ts', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../PluginsPage.tsx', import.meta.url), 'utf8');
const jobsSource = readFileSync(new URL('../JobsPage.tsx', import.meta.url), 'utf8');


describe('plugin system management page', () => {
  test('wires plugin registry route and API client', () => {
    expect(routesSource).toContain('plugins:');
    expect(routesSource).toContain('/plugins');
    expect(routesSource).toContain('插件系统');
    expect(appSource).toContain('PluginsPage');
    expect(clientSource).toContain('/api/v1/plugins');
  });

  test('exposes custom processor and alert channel management copy', () => {
    expect(pageSource).toContain('自定义处理器类型');
    expect(pageSource).toContain('自定义告警通道');
    expect(pageSource).toContain('pluginProcessors.type');
    expect(pageSource).not.toContain('plugin-processor:');
    expect(pageSource).toContain('targetKind');
  });

  test('uses guided dropdowns instead of free-form technical fields in plugin drawer', () => {
    expect(pageSource).toContain('PROCESSOR_TYPE_OPTIONS');
    expect(pageSource).toContain('ALERT_CHANNEL_OPTIONS');
    expect(pageSource).toContain('ALERT_TEMPLATE_OPTIONS');
    expect(pageSource).toContain('结构化匹配字段');
    expect(pageSource).toContain('任务处理器名候选');
    expect(pageSource).toContain('billing.sql-sync');
    expect(pageSource).not.toContain('name="processorCapability" label="Worker 能力"><Input');
    expect(pageSource).not.toContain('name="alertType" label="Channel Type"><Input');
  });

  test('job form can select plugin processor type', () => {
    expect(jobsSource).toContain('插件处理器');
    expect(jobsSource).toContain('processorType');
    expect(jobsSource).toContain('listPlugins');
  });

  test('job plugin executor derives processor name from selected plugin option', () => {
    expect(jobsSource).toContain('applyPluginProcessorSelection');
    expect(jobsSource).toContain('pluginProcessorNameOptions');
    expect(jobsSource).toContain('selected?.processor.processorNames');
    expect(jobsSource).toContain('请先在插件管理中维护任务处理器名候选');
    expect(jobsSource).toContain('任务处理器名必须来自插件管理中的候选项');
    expect(jobsSource).not.toContain('selected.plugin.kind');
    expect(jobsSource).not.toContain('selected.plugin.name.toLowerCase');
    expect(jobsSource).not.toContain('if (currentValue?.trim()) names.add');
    expect(jobsSource).toContain('任务处理器名');
    expect(jobsSource).not.toContain('label="插件 Processor Name"');
  });
});
