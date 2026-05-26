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
});
