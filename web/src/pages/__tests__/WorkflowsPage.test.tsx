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

  test('workflow canvas uses solid smooth moving data-flow dots', () => {
    expect(source).toContain('workflow-edge__line');
    expect(source).toContain('workflow-edge__flow-dot');
    expect(source).toContain('<animateMotion');
    expect(source).toContain('begin="-1.2s"');
    expect(source).not.toContain('workflow-edge__line" strokeDasharray');
    expect(source).not.toContain('strokeDasharray="28 76"');
    expect(styles).toContain('workflow-edge__flow-dot');
    expect(styles).toContain('filter: drop-shadow(0 0 6px');
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
    expect(source).toContain('jobExecutionLabel(jobById.get(selectedNode.jobId))');
    expect(source).toContain('onChange={(value) => updateNode(selectedNode.key, { jobId: value, processorName: undefined })}');
    expect(source).not.toContain('name=\"processorName\"');
    expect(source).not.toContain('label=\"Processor\"');
    expect(source).not.toContain('processorName: key');
  });
});


describe('workflow replay and definition diff affordances', () => {
  test('workflow list can load server-side replay snapshots', () => {
    expect(source).toContain('getWorkflowReplay');
    expect(source).toContain('WorkflowReplayResponse');
    expect(source).toContain('回放实例');
    expect(source).toContain('workflow replay');
    expect(source).toContain('workflow-replay-panel');
  });

  test('workflow replay renders an operator playback timeline', () => {
    expect(source).toContain('workflow-replay-player');
    expect(source).toContain('replayCursor');
    expect(source).toContain('replayPlaying');
    expect(source).toContain('replayPlaybackTimer');
    expect(source).toContain('playReplay');
    expect(source).toContain('pauseReplay');
    expect(source).toContain('resetReplay');
    expect(source).toContain('stepReplay');
    expect(source).toContain('currentReplayEvent');
    expect(source).toContain('workflow-replay-timeline');
    expect(source).toContain('workflow-replay-event--active');
    expect(source).toContain('Progress');
    expect(source).toContain('播放');
    expect(source).toContain('暂停');
    expect(source).toContain('上一步');
    expect(source).toContain('下一步');
  });

  test('workflow editor exposes a first-class definition diff view', () => {
    expect(source).toContain('buildLineDiff');
    expect(source).toContain('WorkflowDefinitionDiff');
    expect(source).toContain('定义 Diff');
    expect(source).toContain('workflow definition diff');
    expect(styles).toContain('.workflow-definition-diff');
    expect(styles).toContain('workflow-definition-diff__line--added');
    expect(styles).toContain('workflow-definition-diff__line--removed');
  });
});


describe('workflow HTTP node governance controls', () => {
  test('exposes allowlist denylist retry and circuit breaker fields', () => {
    expect(source).toContain('deniedHosts');
    expect(source).toContain('deniedCidrs');
    expect(source).toContain('allowedHosts');
    expect(source).toContain('maxRetries');
    expect(source).toContain('retryBackoffMs');
    expect(source).toContain('circuitBreaker');
    expect(source).toContain('熔断阈值');
  });
});
