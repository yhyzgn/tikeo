import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../WorkflowsPage.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('workflow canvas fullscreen affordance', () => {
  test('DagPreview can toggle the editable canvas into fullscreen mode', () => {
    expect(source).toContain('isCanvasFullscreen');
    expect(source).toContain('切换全屏');
    expect(source).toContain('退出全屏');
    expect(source).toContain('workflow-dag-editor--fullscreen');
    expect(styles).toContain('.workflow-dag-editor--fullscreen');
    expect(styles).toContain('position: fixed');
  });
});
