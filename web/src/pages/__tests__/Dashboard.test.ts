import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../Dashboard.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('dashboard realtime overview', () => {
  test('subscribes to instance and worker SSE streams with a 3s fallback refresh', () => {
    expect(source).toContain('instanceListStreamUrl');
    expect(source).toContain('workerStreamUrl');
    expect(source).toContain('listWorkers');
    expect(source).toContain('new EventSource(instanceListStreamUrl())');
    expect(source).toContain('new EventSource(workerStreamUrl())');
    expect(source).toContain("instanceSource.addEventListener('instances.snapshot'");
    expect(source).toContain("workerSource.addEventListener('workers.snapshot'");
    expect(source).toContain('setWorkers(snapshot.workers);');
    expect(source).toContain('window.setInterval(() => { void load(); }, 3000)');
    expect(source).toContain('new EventSource(dispatchQueueStreamUrl())');
    expect(source).toContain("queueSource.addEventListener('dispatchQueue.snapshot'");
    expect(source).toContain('queueSource.close();');
    expect(source).toContain('window.clearInterval(fallbackTimer);');
  });

  test('renders cockpit charts, health metrics, and task schedule plan from live data', () => {
    expect(source).toContain('getClusterDiagnostics');
    expect(source).toContain('getDispatchQueue');
    expect(source).toContain('getAlertDeliveryQueueStatus');
    expect(source).toContain('listAuditLogs');
    expect(source).toContain('dispatchQueueStreamUrl');
    expect(source).toContain('function TrendBars');
    expect(source).toContain('function StatusDonut');
    expect(source).toContain('function SchedulePlanMap');
    expect(source).toContain('function recentTrend');
    expect(source).toContain('function schedulePlans');
    expect(source).toContain('调度驾驶舱');
    expect(source).toContain('最近 12 小时执行趋势');
    expect(source).toContain('实例状态分布');
    expect(source).toContain('任务计划图');
    expect(source).toContain('调度健康');
    expect(source).toContain('队列压力');
    expect(source).toContain('通知投递');
    expect(source).toContain('HA / 网关');
    expect(source).toContain('审计活动');
    expect(source).toContain('任务类型分布');
    expect(source).toContain('触发方式分布');
    expect(source).toContain('风险信号');
    expect(source).toContain('Worker Mesh 分布');
    expect(source).toContain('能力覆盖 Top 6');
    expect(source).toContain('最近审计');
    expect(source).toContain('快速入口');
    expect(source).toContain('SSE + 3s fallback');
  });

  test('ships dashboard-specific visual structure for trend bars, donut chart, radar, and action grid', () => {
    expect(styles).toContain('.dashboard-radar');
    expect(styles).toContain('.dashboard-trend');
    expect(styles).toContain('.dashboard-trend__segment--success');
    expect(styles).toContain('.dashboard-trend__segment--failed');
    expect(styles).toContain('.dashboard-donut');
    expect(source).toContain('conic-gradient');
    expect(styles).toContain('.dashboard-plan-map');
    expect(styles).toContain('.dashboard-plan-map__bar');
    expect(styles).toContain('.dashboard-mini-distribution');
    expect(styles).toContain('.dashboard-signal-card');
    expect(styles).toContain('.dashboard-risk-grid');
    expect(styles).toContain('.dashboard-top-list');
    expect(styles).toContain('.dashboard-audit-list');
    expect(styles).toContain('.dashboard-action-grid');
    expect(styles).toContain('@media (max-width: 991px)');
  });
});
