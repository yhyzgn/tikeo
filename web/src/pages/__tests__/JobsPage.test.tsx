import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../JobsPage.tsx', import.meta.url), 'utf8');


describe('job schedule form governance', () => {
  test('uses structured schedule fields instead of one free-form expression for every schedule type', () => {
    expect(source).toContain('API 手动触发任务不会配置调度表达式');
    expect(source).toContain('fixedRateValue');
    expect(source).toContain('fixedRateUnit');
    expect(source).toContain('Cron 表达式');
    expect(source).not.toContain('cron 或 fixed_rate 表达式，可留空');
  });

  test('distinguishes SDK processors from sandbox script workers', () => {
    expect(source).toContain('SDK Processor');
    expect(source).toContain('脚本执行器（沙箱自动执行）');
    expect(source).toContain('具体脚本');
    expect(source).toContain("capabilityValues('processor:')");
    expect(source).toContain("capabilityValues('script:')");
    expect(source).toContain('SDK Processor 不能选择 script:* 执行器');
    expect(source).toContain('只展示与所选脚本执行器语言匹配的已审批脚本');
    expect(source).toContain('选择已审批脚本');
    expect(source).toContain('demo.echo');
    expect(source).not.toContain('执行器类型');
    expect(source).not.toContain('script:${script.id}');
    expect(source).not.toContain("label: 'Script'");
  });
});
