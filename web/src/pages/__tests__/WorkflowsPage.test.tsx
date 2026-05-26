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


describe('workflow editor avoids user-facing internal bindings', () => {
  test('uses selected jobs and workflows instead of manual processor or child ids', () => {
    expect(source).not.toContain("{ kind: 'script', label: 'Script'");
    expect(source).toContain('脚本不再作为独立工作流节点配置');
    expect(source).toContain('选择已创建工作流');
    expect(source).not.toContain('子工作流 ID');
    expect(source).toContain('执行器由所选调度任务绑定决定');
    expect(source).toContain('脚本沙箱绑定都不能在工作流节点里手动覆盖');
    expect(source).not.toContain('processorName: key');
  });
});
