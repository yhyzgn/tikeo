import { describe, expect, test } from 'bun:test';

import { formatWorkerDisplayId } from '../instances/workerDisplay';

describe('worker display id formatting', () => {
  test('preserves the worker prefix and abbreviates the stable id hash to first and last eight chars', () => {
    expect(formatWorkerDisplayId('wrk-stable-35daa63ce09a98ed4a17baaef99460be52e9a5f4d78754ed66aee7ca66f045bb'))
      .toBe('wrk-stable-35daa63c....66f045bb');
  });

  test('leaves short worker labels unchanged', () => {
    expect(formatWorkerDisplayId('暂无 worker')).toBe('暂无 worker');
    expect(formatWorkerDisplayId('worker-a')).toBe('worker-a');
  });
});
