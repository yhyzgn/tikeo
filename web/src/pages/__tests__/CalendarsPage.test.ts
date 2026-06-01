import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { toWindowPayloads } from '../CalendarsPage';

const source = readFileSync(new URL('../CalendarsPage.tsx', import.meta.url), 'utf8');

function fakeDate(value: string) {
  return { toISOString: () => value };
}

describe('calendar window editor', () => {
  test('uses edit actions and range-picker rows instead of free-form JSON textareas', () => {
    expect(source).toContain('EditOutlined');
    expect(source).toContain('更新 Calendar');
    expect(source).toContain('Form.List name="maintenanceWindows"');
    expect(source).toContain('Form.List name="freezeWindows"');
    expect(source).toContain('DatePicker.RangePicker showTime');
    expect(source).toContain('添加维护窗口');
    expect(source).toContain('添加冻结窗口');
    expect(source).not.toContain('TextArea');
    expect(source).not.toContain('JSON.stringify');
  });

  test('serializes each selected range row into a typed start/end window payload', () => {
    const payload = toWindowPayloads([
      { range: [fakeDate('2026-06-01T01:00:00.000Z'), fakeDate('2026-06-01T02:00:00.000Z')] as never },
      {},
    ]);

    expect(payload).toEqual([
      { start: '2026-06-01T01:00:00.000Z', end: '2026-06-01T02:00:00.000Z' },
    ]);
  });
});
