import { describe, expect, test } from 'bun:test';

import { formatLogTimestamp } from './logTime';

describe('terminal log timestamp formatting', () => {
  test('renders ISO-like local timestamps with milliseconds and timezone offset', () => {
    expect(formatLogTimestamp('2026-06-06T11:05:35.722+08:00')).toBe('2026-06-06T11:05:35.722+08:00');
  });
});
