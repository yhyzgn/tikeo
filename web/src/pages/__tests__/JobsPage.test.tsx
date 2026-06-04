import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../JobsPage.tsx', import.meta.url), 'utf8');
const topologyCanvasSource = readFileSync(new URL('../jobs/TopologyCanvas.tsx', import.meta.url), 'utf8');
const topologyPageSource = readFileSync(new URL('../JobTopologyPage.tsx', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const stylesSource = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');


describe('job schedule form governance', () => {
  test('uses structured schedule fields instead of one free-form expression for every schedule type', () => {
    expect(source).toContain('API 手动触发任务不会配置调度表达式');
    expect(source).toContain('fixedRateValue');
    expect(source).toContain('fixedRateUnit');
    expect(source).toContain('Cron 表达式');
    expect(source).not.toContain('cron 或 fixed_rate 表达式，可留空');
  });

  test('distinguishes 处理器 from sandbox script workers', () => {
    expect(source).toContain('处理器');
    expect(source).toContain('脚本（沙箱自动执行）');
    expect(source).toContain('具体脚本');
    expect(source).toContain('workerSdkProcessorNames');
    expect(source).toContain('选择已审批脚本');
    expect(source).toContain('Server 会按脚本语言匹配 Worker 注册的结构化 scriptRunners');
    expect(source).toContain('demo.echo');
    expect(source).not.toContain('scriptExecutor');
    expect(source).not.toContain('选择脚本执行器能力');
    expect(source).not.toContain('执行器类型');
    expect(source).not.toContain('script:');
    expect(source).not.toContain('script:${script.id}');
    expect(source).not.toContain("label: 'Script'");
  });

  test('uses formal drawers and structured executor/script selectors for create and edit', () => {
    expect(source).toContain('title="创建任务"');
    expect(source).toContain("title={editingJob ? `编辑任务 - ${editingJob.name}` : '编辑任务'}");
    expect(source.match(/width=\{900\}/g)?.length).toBeGreaterThanOrEqual(3);
    expect(source).toContain("{ value: 'sdk', label: '处理器' }");
    expect(source).toContain("{ value: 'plugin', label: '插件处理器' }");
    expect(source).toContain("{ value: 'script', label: '脚本（沙箱自动执行）' }");
    expect(source).toContain('normalizeExecutor');
    expect(source).toContain("if (values.executorKind === 'script') return { ...rest, processorName: null, processorType: null }");
    expect(source).toContain('validatePluginExecutor(values.processorType, values.processorName)');
    expect(source).toContain('return { ...rest, scriptId: null, processorType: null }');
    expect(source).not.toContain('script:${script.id}');
  });

  test('replays edit drawer values after remount and converts lifecycle date values', () => {
    expect(source).toContain('useEffect(() => {');
    expect(source).toContain('if (!editingJob) return;');
    expect(source).toContain('editForm.resetFields();');
    expect(source).toContain('scheduleStartAt: datePickerValue(editingJob.scheduleStartAt)');
    expect(source).toContain('scheduleEndAt: datePickerValue(editingJob.scheduleEndAt)');
    expect(source).toContain('scheduleStartAt: isoDateValue(scheduled.scheduleStartAt)');
    expect(source).toContain('scheduleEndAt: isoDateValue(scheduled.scheduleEndAt)');
  });

  test('exposes job version history and rollback UI copy', () => {
    expect(source).toContain('版本历史');
    expect(source).toContain('listJobVersions');
    expect(source).toContain('rollbackJob');
    expect(source).toContain('回滚会生成新的最新版本');
    expect(source).toContain('回滚到此版本');
    expect(source).toContain('v{job.versionNumber}');
  });
});

describe('job topology foundation', () => {
  test('exposes topology secondary page backed by topology api', () => {
    expect(source).toContain('任务拓扑');
    expect(topologyPageSource).toContain('getJobTopology');
    expect(topologyPageSource).toContain('workflow_job_dependency');
    expect(topologyPageSource).toContain('无法解析的引用');
  });
});


describe('job scheduling advice foundation', () => {
  test('exposes scheduling advice drawer backed by advice api', () => {
    expect(source).toContain('调度建议');
    expect(source).toContain('getJobSchedulingAdvice');
    expect(source).toContain('Required capability');
    expect(source).toContain('Eligible workers');
    expect(source).toContain('历史耗时');
    expect(source).toContain('资源预测');
    expect(source).toContain('recommendedConcurrency');
    expect(source).toContain('scheduling-advice-grid');
    expect(source).toContain('scheduling-advice-stat-card');
    expect(source).toContain('预测依据');
    expect(stylesSource).toContain('.scheduling-advice-grid');
    expect(stylesSource).toContain('.scheduling-advice-stat-card');
  });
});


describe('job canary routing foundation', () => {
  test('exposes canary fields and routed trigger feedback', () => {
    expect(source).toContain('灰度目标任务');
    expect(source).toContain('canaryPercent');
    expect(source).toContain('canary {job.canaryPercent}%');
    expect(source).toContain('命中灰度');
  });
});


describe('job topology canvas impact replay upgrade', () => {
  test('moves topology into a secondary page with canvas and impact analysis', () => {
    expect(routesSource).toContain('jobTopology');
    expect(routesSource).toContain('/jobs/topology');
    expect(appSource).toContain('ROUTE_META.jobTopology.path');
    expect(source).toContain('navigate(ROUTE_META.jobTopology.path)');
    expect(source).not.toContain('title="任务拓扑"');
    expect(topologyCanvasSource).toContain('拓扑图形画布');
    expect(topologyPageSource).toContain('getJobImpact');
    expect(topologyPageSource).toContain('跨工作流影响分析');
    expect(topologyPageSource).toContain('workflow-back-button');
    expect(topologyPageSource).toContain('← 返回任务列表');
    expect(topologyPageSource).toContain('upstreamJobs');
    expect(topologyPageSource).toContain('downstreamJobs');
  });

  test('supports fullscreen canvas affordance on topology page', () => {
    expect(topologyCanvasSource).toContain('<svg');
    expect(topologyCanvasSource).toContain('全屏');
    expect(topologyCanvasSource).toContain('退出全屏');
    expect(topologyCanvasSource).toContain('fullscreen');
    expect(stylesSource).toContain('.topology-canvas-card--fullscreen');
    expect(stylesSource).toContain('position: fixed');
  });

  test('routes edges around nodes and animates data flow', () => {
    expect(topologyCanvasSource).toContain('routeOrthogonalEdge');
    expect(topologyCanvasSource).toContain('intersectsNodeBox');
    expect(topologyCanvasSource).toContain('<path');
    expect(topologyCanvasSource).toContain('topology-flow-line');
    expect(topologyCanvasSource).toContain('topology-flow-pulse');
    expect(stylesSource).toContain('@keyframes topology-flow');
  });
});
