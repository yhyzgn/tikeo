import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const viteConfig = readFileSync(new URL('../../../vite.config.ts', import.meta.url), 'utf8');

describe('production chunking', () => {
  test('keeps rc-component modules in one vendor group for production overlays', () => {
    const rcGroup = viteConfig.match(/name:\s*'vendor-rc-select-menu'[\s\S]*?priority:\s*37,\n\s*}/)?.[0] ?? '';

    expect(rcGroup).toContain('select|menu|tree|tree-select|cascader|trigger|tooltip|dropdown|virtual-list|overflow|util');
    expect(rcGroup).not.toContain('maxSize');
  });

  test('does not split Ant Design vendor groups with maxSize', () => {
    for (const groupName of ['vendor-rc-picker', 'vendor-rc-table', 'vendor-rc-select-menu', 'vendor-rc-components', 'vendor-ant-design-icons', 'vendor-ant-design-runtime', 'vendor-antd-internals', 'vendor-antd', 'vendor-antd-core']) {
      const group = viteConfig.match(new RegExp(`name:\\s*'${groupName}'[\\s\\S]*?priority:\\s*\\d+,\\n\\s*}`))?.[0] ?? '';
      expect(group).not.toContain('maxSize');
    }
  });
});
