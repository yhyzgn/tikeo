import {
  ApiOutlined,
  ArrowRightOutlined,
  ClockCircleOutlined,
  DeploymentUnitOutlined,
  FireOutlined,
  NodeIndexOutlined,
  SafetyCertificateOutlined,
  ThunderboltOutlined,
  TeamOutlined,
  WarningOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  FieldTimeOutlined,
  SyncOutlined,
} from '@ant-design/icons';
import { Button, Card, Col, Empty, Progress, Row, Space, Statistic, Table, Tag, Tooltip, Typography } from 'antd';
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { Link } from 'react-router-dom';

import {
  dispatchQueueStreamUrl,
  getAlertDeliveryQueueStatus,
  getClusterDiagnostics,
  getDispatchQueue,
  instanceListStreamUrl,
  listAuditLogs,
  listInstances,
  listJobs,
  listWorkers,
  workerStreamUrl,
  type AlertDeliveryQueueStatus,
  type AuditLogPage,
  type ClusterDiagnosticsResponse,
  type JobInstanceSummary,
  type JobSummary,
  type QueueOverview,
  type WorkerListResponse,
} from '../api/client';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

const DASHBOARD_INSTANCE_PAGE_SIZE = 100;

type InstanceListStreamSnapshot = {
  jobs: JobSummary[];
  instances: JobInstanceSummary[];
};

type WorkerStreamSnapshot = {
  workers: WorkerListResponse;
};

type DispatchQueueStreamSnapshot = QueueOverview;

interface TrendBucket {
  label: string;
  total: number;
  succeeded: number;
  failed: number;
}

interface StatusSlice {
  key: string;
  label: string;
  value: number;
  color: string;
}

interface MiniSlice {
  label: string;
  value: number;
  color: string;
}

interface ScopeSummary {
  key: string;
  count: number;
  masters: number;
  followers: number;
  clusters: number;
}

const STATUS_META: Record<string, { label: string; color: string }> = {
  pending: { label: '等待', color: '#f59e0b' },
  dispatching: { label: '派发中', color: 'var(--app-primary-color)' },
  running: { label: '运行中', color: '#6366f1' },
  retrying: { label: '重试', color: '#f97316' },
  succeeded: { label: '成功', color: '#10b981' },
  failed: { label: '失败', color: '#ef4444' },
  cancelled: { label: '取消', color: '#94a3b8' },
};

function effectiveInstanceStatus(instance: JobInstanceSummary): string {
  if (instance.result) return instance.result.success ? 'succeeded' : 'failed';
  return instance.status;
}

function formatTime(value: string | null | undefined): string {
  if (!value) return '-';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }).format(date);
}

function recentTrend(instances: JobInstanceSummary[]): TrendBucket[] {
  const now = Date.now();
  const buckets = Array.from({ length: 12 }, (_, index) => {
    const offset = 11 - index;
    const date = new Date(now - offset * 60 * 60 * 1000);
    return { label: `${String(date.getHours()).padStart(2, '0')}:00`, total: 0, succeeded: 0, failed: 0 };
  });
  for (const instance of instances) {
    const created = new Date(instance.createdAt).getTime();
    if (Number.isNaN(created)) continue;
    const diffHours = Math.floor((now - created) / (60 * 60 * 1000));
    if (diffHours < 0 || diffHours > 11) continue;
    const bucket = buckets[11 - diffHours];
    bucket.total += 1;
    const status = effectiveInstanceStatus(instance);
    if (status === 'succeeded') bucket.succeeded += 1;
    if (status === 'failed') bucket.failed += 1;
  }
  return buckets;
}

function statusSlices(instances: JobInstanceSummary[]): StatusSlice[] {
  const counts = new Map<string, number>();
  for (const instance of instances) {
    const status = effectiveInstanceStatus(instance);
    counts.set(status, (counts.get(status) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort((left, right) => right[1] - left[1])
    .map(([status, value]) => ({ key: status, label: STATUS_META[status]?.label ?? status, value, color: STATUS_META[status]?.color ?? '#64748b' }));
}

function miniSlices(items: MiniSlice[]): MiniSlice[] {
  return items.filter((item) => item.value > 0);
}

function MiniDistribution({ slices, emptyText = '暂无数据' }: { slices: MiniSlice[]; emptyText?: string }) {
  const total = slices.reduce((sum, slice) => sum + slice.value, 0);
  if (total === 0) return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={emptyText} />;
  return (
    <div className="dashboard-mini-distribution">
      <div className="dashboard-mini-distribution__bar">
        {slices.map((slice) => (
          <Tooltip key={slice.label} title={`${slice.label}: ${slice.value}`}>
            <span style={{ '--slice-color': slice.color, '--slice-width': `${Math.max(5, (slice.value / total) * 100)}%` } as CSSProperties} />
          </Tooltip>
        ))}
      </div>
      <div className="dashboard-mini-distribution__legend">
        {slices.map((slice) => <span key={slice.label}><i style={{ background: slice.color }} />{slice.label} · {slice.value}</span>)}
      </div>
    </div>
  );
}

function TopList({ items, emptyText }: { items: Array<{ label: string; value: number; hint?: string }>; emptyText: string }) {
  if (items.length === 0) return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={emptyText} />;
  const max = Math.max(1, ...items.map((item) => item.value));
  return (
    <div className="dashboard-top-list">
      {items.map((item) => (
        <div className="dashboard-top-list__item" key={item.label}>
          <div>
            <strong>{item.label}</strong>
            {item.hint ? <span>{item.hint}</span> : null}
          </div>
          <em>{item.value}</em>
          <b style={{ '--row-width': `${Math.max(8, (item.value / max) * 100)}%` } as CSSProperties} />
        </div>
      ))}
    </div>
  );
}

function riskSignals(params: {
  failedInstances: number;
  pendingInstances: number;
  onlineWorkers: number;
  queue: QueueOverview | null;
  alertQueue: AlertDeliveryQueueStatus | null;
  clusterStatus: string;
}) {
  const signals: Array<{ label: string; value: string; tone: 'green' | 'gold' | 'red' | 'blue' }> = [];
  signals.push({ label: '失败实例', value: String(params.failedInstances), tone: params.failedInstances > 0 ? 'red' : 'green' });
  signals.push({ label: '活跃实例', value: String(params.pendingInstances), tone: params.pendingInstances > 0 ? 'blue' : 'green' });
  signals.push({ label: '队列积压', value: String((params.queue?.pending ?? 0) + (params.queue?.running ?? 0)), tone: (params.queue?.pending ?? 0) > 0 ? 'gold' : 'green' });
  signals.push({ label: '通知死信', value: String(params.alertQueue?.dead_letter ?? 0), tone: (params.alertQueue?.dead_letter ?? 0) > 0 ? 'red' : 'green' });
  signals.push({ label: '在线容量', value: String(params.onlineWorkers), tone: params.onlineWorkers > 0 ? 'green' : 'gold' });
  signals.push({ label: '集群状态', value: params.clusterStatus, tone: params.clusterStatus === 'ready' || params.clusterStatus === 'leader' ? 'green' : 'gold' });
  return signals;
}



function instancesPath(filters: Record<string, string>): string {
  const params = new URLSearchParams(filters);
  const query = params.toString();
  return query ? `${ROUTE_META.instances.path}?${query}` : ROUTE_META.instances.path;
}

function recommendedActions(params: {
  failedInstances: number;
  queueBacklog: number;
  onlineWorkers: number;
  alertDeadLetters: number;
  clusterStatus: string;
}): Array<{ key: string; label: string; detail: string; to: string; tone: 'ok' | 'warn' | 'danger' }> {
  const actions: Array<{ key: string; label: string; detail: string; to: string; tone: 'ok' | 'warn' | 'danger' }> = [];
  if (params.failedInstances > 0) {
    actions.push({ key: 'failed', label: '复查失败实例', detail: `${params.failedInstances} 个实例需要定位日志或重试策略`, to: instancesPath({ status: 'failed' }), tone: 'danger' });
  }
  if (params.queueBacklog > 0) {
    actions.push({ key: 'queue', label: '观察派发队列', detail: `${params.queueBacklog} 个队列项仍在等待或执行`, to: ROUTE_META.dispatchQueue.path, tone: 'warn' });
  }
  if (params.onlineWorkers === 0) {
    actions.push({ key: 'workers', label: '恢复 Worker 容量', detail: '当前没有在线 Worker，任务不会被实际消费', to: ROUTE_META.workers.path, tone: 'danger' });
  }
  if (params.alertDeadLetters > 0) {
    actions.push({ key: 'alerts', label: '处理通知死信', detail: `${params.alertDeadLetters} 条通知进入死信队列`, to: ROUTE_META.notifications.path, tone: 'warn' });
  }
  if (!['ready', 'leader'].includes(params.clusterStatus)) {
    actions.push({ key: 'ha', label: '检查集群状态', detail: `当前状态为 ${params.clusterStatus}，建议确认网关与 Raft 节点`, to: ROUTE_META.workers.path, tone: 'warn' });
  }
  if (actions.length === 0) {
    actions.push({ key: 'healthy', label: '系统处于稳定态', detail: '可继续观察趋势，或进入任务配置做容量和通知演练', to: ROUTE_META.jobs.path, tone: 'ok' });
  }
  return actions.slice(0, 4);
}

function scheduleTagClass(scheduleType: string): string {
  if (scheduleType === 'cron') return 'dashboard-plan-tag dashboard-plan-tag--cron';
  if (scheduleType === 'api') return 'dashboard-plan-tag dashboard-plan-tag--api';
  if (scheduleType === 'fixed_rate') return 'dashboard-plan-tag dashboard-plan-tag--rate';
  return 'dashboard-plan-tag dashboard-plan-tag--other';
}

function realtimeModeMeta(mode: 'connecting' | 'live' | 'fallback'): { label: string; detail: string; className: string } {
  if (mode === 'live') return { label: 'SSE 实时', detail: '实例、Worker、队列事件实时推送；慢变化指标 15 秒补偿刷新', className: 'dashboard-realtime-chip--live' };
  if (mode === 'fallback') return { label: '轮询兜底', detail: 'SSE 连接异常，临时使用 3 秒轮询恢复数据一致性', className: 'dashboard-realtime-chip--fallback' };
  return { label: '连接中', detail: '正在建立 SSE 流，首屏数据由 HTTP 快照填充', className: 'dashboard-realtime-chip--connecting' };
}

function schedulePlans(jobs: JobSummary[]): JobSummary[] {
  const weight = (job: JobSummary) => {
    if (!job.enabled) return 30;
    if (job.scheduleType === 'api') return 20;
    if (job.scheduleType === 'cron') return 1;
    return 5;
  };
  return [...jobs].sort((left, right) => weight(left) - weight(right) || left.name.localeCompare(right.name)).slice(0, 8);
}


function scheduleTone(job: JobSummary): { label: string; color: string; width: number } {
  if (!job.enabled) return { label: '停用', color: '#94a3b8', width: 22 };
  if (job.scheduleType === 'cron') return { label: '周期调度', color: '#2563eb', width: 88 };
  if (job.scheduleType === 'fixed_rate') return { label: '固定频率', color: 'var(--dashboard-muted-accent)', width: 68 };
  if (job.scheduleType === 'api') return { label: 'API 触发', color: '#7c3aed', width: 44 };
  return { label: job.scheduleType, color: '#14b8a6', width: 58 };
}

function scheduleExpression(job: JobSummary): string {
  if (!job.enabled) return '已停用，不参与自动调度';
  if (job.scheduleType === 'api') return '外部 API / SDK 手动触发';
  return job.scheduleExpr ?? '未配置表达式';
}

function SchedulePlanMap({ jobs }: { jobs: JobSummary[] }) {
  if (jobs.length === 0) return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无任务计划" />;
  return (
    <div className="dashboard-plan-map" aria-label="任务计划轨道">
      {jobs.slice(0, 6).map((job, index) => {
        const tone = scheduleTone(job);
        return (
          <div className="dashboard-plan-map__row" key={job.id}>
            <div className="dashboard-plan-map__meta">
              <strong>{job.name}</strong>
              <span>{tone.label} · {scheduleExpression(job)}</span>
            </div>
            <Tooltip title={`${job.namespace}/${job.app} · ${job.processorName ?? job.processorType ?? job.scriptId ?? '未绑定处理器'}`}>
              <div className="dashboard-plan-map__rail">
                <span
                  className="dashboard-plan-map__bar"
                  style={{ '--bar-color': tone.color, '--bar-width': `${tone.width}%`, '--bar-delay': `${index * 9}%` } as CSSProperties}
                />
              </div>
            </Tooltip>
          </div>
        );
      })}
    </div>
  );
}

function TrendBars({ buckets }: { buckets: TrendBucket[] }) {
  const max = Math.max(1, ...buckets.map((bucket) => bucket.total));
  return (
    <div className="dashboard-trend" aria-label="最近 12 小时执行趋势">
      {buckets.map((bucket) => {
        const height = Math.max(8, Math.round((bucket.total / max) * 120));
        const successHeight = bucket.total ? Math.round((bucket.succeeded / bucket.total) * height) : 0;
        const failHeight = bucket.total ? Math.round((bucket.failed / bucket.total) * height) : 0;
        return (
          <div className="dashboard-trend__bucket" key={bucket.label} title={`${bucket.label} total=${bucket.total}`}>
            <div className="dashboard-trend__bar" style={{ height }}>
              <span className="dashboard-trend__segment dashboard-trend__segment--failed" style={{ height: failHeight }} />
              <span className="dashboard-trend__segment dashboard-trend__segment--success" style={{ height: successHeight }} />
            </div>
            <span className="dashboard-trend__label">{bucket.label}</span>
          </div>
        );
      })}
    </div>
  );
}

function StatusDonut({ slices }: { slices: StatusSlice[] }) {
  const total = slices.reduce((sum, slice) => sum + slice.value, 0);
  if (total === 0) {
    return (
      <div className="dashboard-instance-status-empty">
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无实例" />
        <Typography.Text type="secondary">触发任务后，这里会展示成功、失败、运行中与重试实例的实时占比。</Typography.Text>
      </div>
    );
  }
  let offset = 25;
  const gradient = slices.map((slice) => {
    const start = offset;
    const span = (slice.value / total) * 100;
    offset += span;
    return `${slice.color} ${start}% ${offset}%`;
  }).join(', ');
  const succeeded = slices.find((slice) => slice.key === 'succeeded')?.value ?? 0;
  const failed = slices.find((slice) => slice.key === 'failed')?.value ?? 0;
  const active = slices
    .filter((slice) => ['pending', 'dispatching', 'running', 'retrying'].includes(slice.key))
    .reduce((sum, slice) => sum + slice.value, 0);
  const terminal = Math.max(0, total - active);
  const successRate = Math.round((succeeded / total) * 100);
  const failureRate = Math.round((failed / total) * 100);
  const leading = slices[0];
  const healthTone = failed > 0 ? 'danger' : active > 0 ? 'active' : 'healthy';
  const healthText = failed > 0 ? '需要处理' : active > 0 ? '执行中' : '运行健康';
  return (
    <div className={`dashboard-instance-status dashboard-instance-status--${healthTone}`}>
      <div className="dashboard-instance-status__topline">
        <Tag color={failed > 0 ? 'red' : active > 0 ? 'blue' : 'green'}>{healthText}</Tag>
        <Typography.Text type="secondary">主状态：{leading.label} · {Math.round((leading.value / total) * 100)}%</Typography.Text>
      </div>
      <div className="dashboard-instance-status__hero">
        <div className="dashboard-donut" style={{ background: `conic-gradient(${gradient})` }}>
          <div className="dashboard-donut__inner">
            <strong>{total}</strong>
            <span>实例总量</span>
          </div>
        </div>
        <div className="dashboard-instance-status__kpis">
          <Link to={instancesPath({ status: 'succeeded' })} className="dashboard-instance-status__kpi dashboard-instance-status__kpi--success">
            <span>成功率</span>
            <strong>{successRate}%</strong>
            <em>{succeeded} 成功</em>
          </Link>
          <Link to={instancesPath({ status: 'failed' })} className="dashboard-instance-status__kpi dashboard-instance-status__kpi--danger">
            <span>失败率</span>
            <strong>{failureRate}%</strong>
            <em>{failed} 失败</em>
          </Link>
          <Link to={instancesPath({ status: 'active' })} className="dashboard-instance-status__kpi dashboard-instance-status__kpi--active">
            <span>活跃实例</span>
            <strong>{active}</strong>
            <em>待派发 / 运行 / 重试</em>
          </Link>
        </div>
      </div>
      <div className="dashboard-status-lanes" aria-label="实例状态分布明细">
        {slices.map((slice) => {
          const percent = Math.round((slice.value / total) * 100);
          return (
            <Link to={instancesPath({ status: slice.key })} className="dashboard-status-lane" key={slice.key}>
              <span className="dashboard-status-lane__head"><i style={{ background: slice.color }} />{slice.label}<strong>{slice.value}</strong></span>
              <span className="dashboard-status-lane__rail"><b style={{ '--lane-color': slice.color, '--lane-width': `${Math.max(4, percent)}%` } as CSSProperties} /></span>
              <em>{percent}%</em>
            </Link>
          );
        })}
      </div>
      <div className="dashboard-instance-status__footer">
        <span>终态实例 {terminal}</span>
        <span>活跃实例 {active}</span>
        <span>失败实例 {failed}</span>
      </div>
    </div>
  );
}

export function Dashboard() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);
  const [workers, setWorkers] = useState<WorkerListResponse | null>(null);
  const [clusterDiagnostics, setClusterDiagnostics] = useState<ClusterDiagnosticsResponse | null>(null);
  const [queue, setQueue] = useState<QueueOverview | null>(null);
  const [alertQueue, setAlertQueue] = useState<AlertDeliveryQueueStatus | null>(null);
  const [auditLogs, setAuditLogs] = useState<AuditLogPage | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [realtimeMode, setRealtimeMode] = useState<'connecting' | 'live' | 'fallback'>('connecting');
  const streamHealthyRef = useRef(false);
  const active = useRouteActive(ROUTE_META.dashboard.path);

  const load = useCallback(async () => {
    try {
      const [jobPage, instancePage, workerPage, diagnostics, queueOverview, alertStatus, audits] = await Promise.all([
        listJobs(),
        listInstances({ pageSize: DASHBOARD_INSTANCE_PAGE_SIZE }),
        listWorkers(),
        getClusterDiagnostics().catch(() => null),
        getDispatchQueue().catch(() => null),
        getAlertDeliveryQueueStatus().catch(() => null),
        listAuditLogs({ page_size: 8 }).catch(() => null),
      ]);
      setJobs(jobPage.items);
      setInstances(instancePage.items);
      setWorkers(workerPage);
      setClusterDiagnostics(diagnostics);
      setQueue(queueOverview);
      setAlertQueue(alertStatus);
      setAuditLogs(audits);
      setLastUpdated(new Date());
    } catch { /* silent */ }
  }, []);


  useEffect(() => { if (active) void load(); }, [active, load]);

  useEffect(() => {
    if (!active) return undefined;
    streamHealthyRef.current = false;
    setRealtimeMode('connecting');
    const healthyStreams = new Set<'instances' | 'workers' | 'queue'>();
    const markLive = (stream: 'instances' | 'workers' | 'queue') => {
      healthyStreams.add(stream);
      streamHealthyRef.current = healthyStreams.size === 3;
      setRealtimeMode(streamHealthyRef.current ? 'live' : 'connecting');
      setLastUpdated(new Date());
    };
    const markFallback = (stream: 'instances' | 'workers' | 'queue') => {
      healthyStreams.delete(stream);
      streamHealthyRef.current = false;
      setRealtimeMode('fallback');
    };
    const refreshSlowSignals = () => {
      void Promise.all([
        getClusterDiagnostics().then(setClusterDiagnostics).catch(() => null),
        getAlertDeliveryQueueStatus().then(setAlertQueue).catch(() => null),
        listAuditLogs({ page_size: 8 }).then(setAuditLogs).catch(() => null),
      ]).then(() => setLastUpdated(new Date()));
    };

    const instanceSource = new EventSource(instanceListStreamUrl({ pageSize: DASHBOARD_INSTANCE_PAGE_SIZE }));
    instanceSource.addEventListener('instances.snapshot', (event) => {
      try {
        const snapshot = JSON.parse((event as MessageEvent).data) as InstanceListStreamSnapshot;
        setJobs(snapshot.jobs);
        setInstances(snapshot.instances);
        markLive('instances');
      } catch {
        // Ignore malformed stream frames; periodic fallback refresh keeps the dashboard current.
      }
    });
    instanceSource.onopen = () => markLive('instances');
    instanceSource.onerror = () => markFallback('instances');

    const workerSource = new EventSource(workerStreamUrl());
    workerSource.addEventListener('workers.snapshot', (event) => {
      try {
        const snapshot = JSON.parse((event as MessageEvent).data) as WorkerStreamSnapshot;
        setWorkers(snapshot.workers);
        markLive('workers');
      } catch {
        // Ignore malformed stream frames; periodic fallback refresh keeps the dashboard current.
      }
    });
    workerSource.onopen = () => markLive('workers');
    workerSource.onerror = () => markFallback('workers');

    const queueSource = new EventSource(dispatchQueueStreamUrl());
    queueSource.addEventListener('dispatchQueue.snapshot', (event) => {
      try {
        setQueue(JSON.parse((event as MessageEvent).data) as DispatchQueueStreamSnapshot);
        markLive('queue');
      } catch {
        // Ignore malformed stream frames; periodic fallback refresh keeps the dashboard current.
      }
    });
    queueSource.onopen = () => markLive('queue');
    queueSource.onerror = () => markFallback('queue');

    const fallbackTimer = window.setInterval(() => {
      if (!streamHealthyRef.current) void load();
    }, 3000);
    const slowSignalTimer = window.setInterval(refreshSlowSignals, 15000);
    return () => {
      instanceSource.close();
      workerSource.close();
      queueSource.close();
      window.clearInterval(fallbackTimer);
      window.clearInterval(slowSignalTimer);
    };
  }, [active, load]);

  const enabledJobs = jobs.filter((job) => job.enabled).length;
  const pendingInstances = instances.filter((instance) => ['pending', 'dispatching', 'running', 'retrying'].includes(effectiveInstanceStatus(instance))).length;
  const failedInstances = instances.filter((instance) => effectiveInstanceStatus(instance) === 'failed').length;
  const succeededInstances = instances.filter((instance) => effectiveInstanceStatus(instance) === 'succeeded').length;
  const broadcastInstances = instances.filter((instance) => instance.executionMode === 'broadcast').length;
  const onlineWorkers = workers?.online ?? 0;
  const trend = useMemo(() => recentTrend(instances), [instances]);
  const slices = useMemo(() => statusSlices(instances), [instances]);
  const plans = useMemo(() => schedulePlans(jobs), [jobs]);
  const scheduleMix = useMemo(() => miniSlices([
    { label: 'Cron', value: jobs.filter((job) => job.scheduleType === 'cron').length, color: '#2563eb' },
    { label: 'API', value: jobs.filter((job) => job.scheduleType === 'api').length, color: '#7c3aed' },
    { label: '固定频率', value: jobs.filter((job) => job.scheduleType === 'fixed_rate').length, color: 'var(--dashboard-muted-accent)' },
    { label: '其他', value: jobs.filter((job) => !['cron', 'api', 'fixed_rate'].includes(job.scheduleType)).length, color: '#14b8a6' },
  ]), [jobs]);
  const triggerMix = useMemo(() => {
    const counts = new Map<string, number>();
    for (const instance of instances) counts.set(instance.triggerType || 'unknown', (counts.get(instance.triggerType || 'unknown') ?? 0) + 1);
    const palette = ['#2563eb', '#7c3aed', '#f97316', '#14b8a6', '#64748b'];
    return [...counts.entries()].map(([label, value], index) => ({ label, value, color: palette[index % palette.length] }));
  }, [instances]);
  const workerScopes = useMemo<ScopeSummary[]>(() => {
    const map = new Map<string, { workers: number; masters: number; clusters: Set<string> }>();
    for (const worker of workers?.items ?? []) {
      const key = `${worker.namespace}/${worker.app}`;
      const current = map.get(key) ?? { workers: 0, masters: 0, clusters: new Set<string>() };
      current.workers += 1;
      if (worker.master?.isMaster) current.masters += 1;
      current.clusters.add(`${worker.cluster}/${worker.region}`);
      map.set(key, current);
    }
    return [...map.entries()]
      .map(([key, value]) => ({ key, count: value.workers, masters: value.masters, followers: Math.max(0, value.workers - value.masters), clusters: value.clusters.size }))
      .sort((left, right) => right.count - left.count)
      .slice(0, 5);
  }, [workers]);
  const capabilityLeaders = useMemo(() => {
    const counts = new Map<string, number>();
    for (const worker of workers?.items ?? []) {
      const capabilities = [
        ...(worker.structuredCapabilities?.tags ?? []),
        ...(worker.structuredCapabilities?.normalProcessors?.map((processor) => `Normal:${processor.name}`) ?? []),
        ...(worker.structuredCapabilities?.scriptRunners.map((runner) => `Script:${runner.language}`) ?? []),
        ...(worker.structuredCapabilities?.pluginProcessors.flatMap((plugin) => (plugin.processors?.map((processor) => `Plugin:${plugin.type}:${processor.name}`) ?? plugin.processorNames.map((processor) => `Plugin:${plugin.type}:${processor}`))) ?? []),
      ];
      for (const capability of capabilities) counts.set(capability, (counts.get(capability) ?? 0) + 1);
    }
    return [...counts.entries()].sort((left, right) => right[1] - left[1]).slice(0, 6).map(([label, value]) => ({ label, value }));
  }, [workers]);
  const successRate = instances.length ? Math.round((succeededInstances / instances.length) * 100) : 100;
  const workerCoverage = jobs.length ? Math.min(100, Math.round((onlineWorkers / Math.max(enabledJobs, 1)) * 100)) : 100;
  const clusterStatus = clusterDiagnostics?.smartGateway?.status ?? clusterDiagnostics?.status?.role ?? 'unknown';
  const smartGateway = clusterDiagnostics?.smartGateway;
  const nodeCount = clusterDiagnostics?.nodes.length ?? clusterDiagnostics?.status?.nodes ?? 0;
  const queueBacklog = (queue?.pending ?? 0) + (queue?.running ?? 0);
  const alertDeliveryRate = alertQueue?.total_attempts ? Math.round((alertQueue.delivered / alertQueue.total_attempts) * 100) : 100;
  const riskItems = riskSignals({ failedInstances, pendingInstances, onlineWorkers, queue, alertQueue, clusterStatus });
  const actions = recommendedActions({ failedInstances, queueBacklog, onlineWorkers, alertDeadLetters: alertQueue?.dead_letter ?? 0, clusterStatus });
  const realtime = realtimeModeMeta(realtimeMode);
  const recentAudits = auditLogs?.items ?? [];

  return (
    <div className="page-stack dashboard-page">
      <section className="hero-panel dashboard-hero">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color="blue" className="soft-tag">Live Scheduler Cockpit</Tag>
            <Typography.Title level={1}>调度驾驶舱</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            聚合任务计划、实例趋势、Worker 在线容量与集群调度健康。数据来自任务/实例/Worker SSE 流，并以 3 秒轮询兜底刷新。
          </Typography.Paragraph>
          <Space wrap>
            <Button type="primary"><Link to={ROUTE_META.jobs.path}>创建或触发任务</Link></Button>
            <Button><Link to={ROUTE_META.instances.path}>查看实例日志</Link></Button>
            <Button><Link to={ROUTE_META.workers.path}>检查 Worker</Link></Button>
            <Tooltip title={realtime.detail}>
              <span className={`dashboard-realtime-chip ${realtime.className}`}>
                <SyncOutlined spin={realtimeMode === 'connecting'} /> {realtime.label}
              </span>
            </Tooltip>
          </Space>
        </div>
        <div className="dashboard-hero__control-tower" aria-label="调度健康雷达">
          <div className="dashboard-radar">
            <span style={{ '--score': `${successRate}%` } as CSSProperties}>成功率</span>
            <span style={{ '--score': `${workerCoverage}%` } as CSSProperties}>容量</span>
            <span style={{ '--score': `${failedInstances ? 52 : 100}%` } as CSSProperties}>风险</span>
          </div>
          <strong>{successRate}%</strong>
          <span>execution success</span>
        </div>
      </section>

      <Row gutter={[16, 16]} className="dashboard-brief-row">
        <Col xs={24} xl={14}>
          <Card className="clean-card dashboard-brief-card" title="运营摘要" extra={<span className="dashboard-muted">{realtime.label}</span>}>
            <div className="dashboard-brief-grid">
              <div className="dashboard-brief-item dashboard-brief-item--strong"><CheckCircleOutlined /><span>成功率</span><strong>{successRate}%</strong></div>
              <div className="dashboard-brief-item"><TeamOutlined /><span>Worker 覆盖</span><strong>{workerCoverage}%</strong></div>
              <div className="dashboard-brief-item"><NodeIndexOutlined /><span>队列积压</span><strong>{queueBacklog}</strong></div>
              <div className="dashboard-brief-item"><FieldTimeOutlined /><span>最近刷新</span><strong>{lastUpdated ? formatTime(lastUpdated.toISOString()) : '-'}</strong></div>
            </div>
          </Card>
        </Col>
        <Col xs={24} xl={10}>
          <Card className="clean-card dashboard-action-brief" title="建议动作">
            <div className="dashboard-action-brief__list">
              {actions.map((action) => (
                <Link className={`dashboard-action-brief__item dashboard-action-brief__item--${action.tone}`} to={action.to} key={action.key}>
                  {action.tone === 'ok' ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />}
                  <span><strong>{action.label}</strong><em>{action.detail}</em></span>
                  <ArrowRightOutlined className="dashboard-link-icon" />
                </Link>
              ))}
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[20, 20]}>
        <Col xs={24} sm={12} xl={4}><Card className="metric-card"><Statistic prefix={<ThunderboltOutlined />} title="任务总数" value={jobs.length} /></Card></Col>
        <Col xs={24} sm={12} xl={4}><Card className="metric-card"><Statistic prefix={<ApiOutlined />} title="启用任务" value={enabledJobs} /></Card></Col>
        <Col xs={24} sm={12} xl={4}><Link className="dashboard-metric-link" to={instancesPath({ status: 'active' })}><Card className="metric-card"><Statistic prefix={<ClockCircleOutlined />} title="活跃实例" value={pendingInstances} /></Card></Link></Col>
        <Col xs={24} sm={12} xl={4}><Card className="metric-card"><Statistic prefix={<TeamOutlined />} title="在线 Worker" value={onlineWorkers} /></Card></Col>
        <Col xs={24} sm={12} xl={4}><Link className="dashboard-metric-link" to={instancesPath({ executionMode: 'broadcast' })}><Card className="metric-card"><Statistic prefix={<DeploymentUnitOutlined />} title="广播实例" value={broadcastInstances} /></Card></Link></Col>
        <Col xs={24} sm={12} xl={4}><Link className="dashboard-metric-link" to={instancesPath({ status: 'failed' })}><Card className="metric-card"><Statistic prefix={<WarningOutlined />} title="失败实例" value={failedInstances} valueStyle={{ color: failedInstances ? '#ef4444' : '#10b981' }} /></Card></Link></Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} md={12} xl={6}>
          <Card className="clean-card dashboard-signal-card" title="队列压力">
            <Statistic title="待派发 + 运行中" value={queueBacklog} prefix={<NodeIndexOutlined />} />
            <MiniDistribution slices={miniSlices([
              { label: 'Pending', value: queue?.pending ?? 0, color: '#f59e0b' },
              { label: 'Running', value: queue?.running ?? 0, color: '#6366f1' },
              { label: 'Done', value: queue?.done ?? 0, color: '#10b981' },
              { label: 'Failed', value: queue?.failed ?? 0, color: '#ef4444' },
            ])} emptyText="暂无队列记录" />
          </Card>
        </Col>
        <Col xs={24} md={12} xl={6}>
          <Card className="clean-card dashboard-signal-card" title="通知投递">
            <Statistic title="投递成功率" value={alertDeliveryRate} suffix="%" prefix={<ApiOutlined />} valueStyle={{ color: alertQueue?.dead_letter ? '#ef4444' : '#10b981' }} />
            <MiniDistribution slices={miniSlices([
              { label: 'Delivered', value: alertQueue?.delivered ?? 0, color: '#10b981' },
              { label: 'Retry', value: alertQueue?.retry_pending ?? 0, color: '#f59e0b' },
              { label: 'Dead', value: alertQueue?.dead_letter ?? 0, color: '#ef4444' },
              { label: 'Failed', value: alertQueue?.failed ?? 0, color: '#fb7185' },
            ])} emptyText="暂无投递记录" />
          </Card>
        </Col>
        <Col xs={24} md={12} xl={6}>
          <Card className="clean-card dashboard-signal-card" title="HA / 网关">
            <Statistic title="Server 节点" value={nodeCount} prefix={<SafetyCertificateOutlined />} />
            <div className="dashboard-gateway-grid">
              <Tag color={clusterStatus === 'ready' || clusterStatus === 'leader' ? 'green' : 'gold'}>{clusterStatus}</Tag>
              <span>本地 Worker：{smartGateway?.localGatewayWorkers ?? 0}</span>
              <span>远端 Worker：{smartGateway?.remoteGatewayWorkers ?? 0}</span>
              <span>Outbox：{smartGateway?.outboxTotal ?? 0}</span>
            </div>
          </Card>
        </Col>
        <Col xs={24} md={12} xl={6}>
          <Card className="clean-card dashboard-signal-card" title="审计活动">
            <Statistic title="最近审计事件" value={auditLogs?.total ?? recentAudits.length} prefix={<SafetyCertificateOutlined />} />
            <div className="dashboard-audit-strip">
              {recentAudits.slice(0, 4).map((item) => <Tag key={item.id} color={item.result === 'success' ? 'green' : 'red'}>{item.action}</Tag>)}
              {recentAudits.length === 0 ? <Typography.Text type="secondary">暂无审计记录</Typography.Text> : null}
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={15}>
          <Card className="clean-card dashboard-chart-card" title="最近 12 小时执行趋势" extra={<span className="dashboard-muted">SSE + 3s fallback</span>}>
            <TrendBars buckets={trend} />
          </Card>
        </Col>
        <Col xs={24} xl={9}>
          <Card className="clean-card dashboard-chart-card" title="实例状态分布">
            <StatusDonut slices={slices} />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card className="clean-card dashboard-breakdown-card" title="任务类型分布">
            <MiniDistribution slices={scheduleMix} emptyText="暂无任务" />
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card className="clean-card dashboard-breakdown-card" title="触发方式分布">
            <MiniDistribution slices={triggerMix} emptyText="暂无实例" />
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card className="clean-card dashboard-breakdown-card" title="风险信号">
            <div className="dashboard-risk-grid">
              {riskItems.map((item) => <Tag key={item.label} color={item.tone}>{item.label}：{item.value}</Tag>)}
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={15}>
          <Card className="clean-card dashboard-plan-card" title="任务计划图" extra={<Link to={ROUTE_META.jobs.path}>查看全部 <ArrowRightOutlined /></Link>}>
            <SchedulePlanMap jobs={plans} />
            <Table<JobSummary>
              rowKey="id"
              size="small"
              pagination={false}
              dataSource={plans}
              className="dashboard-plan-table"
              columns={[
                { title: '任务', dataIndex: 'name', render: (value: string, row) => <Space direction="vertical" size={0}><Link className="dashboard-plan-link" to={ROUTE_META.jobs.path}>{value}</Link><Typography.Text type="secondary">{row.namespace}/{row.app}</Typography.Text></Space> },
                { title: '计划', dataIndex: 'scheduleType', width: 240, render: (value: string, row) => <span className={scheduleTagClass(value)}>{value}{row.scheduleExpr ? ` · ${row.scheduleExpr}` : ''}</span> },
                { title: '处理器', width: 180, render: (_, row) => <span data-runtime-text>{row.processorName ?? row.processorType ?? row.scriptId ?? '-'}</span> },
                { title: '状态', dataIndex: 'enabled', width: 90, render: (value: boolean) => value ? <Tag color="green">启用</Tag> : <Tag>停用</Tag> },
              ]}
            />
          </Card>
        </Col>
        <Col xs={24} xl={9}>
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Card className="clean-card" title="调度健康">
              <Space direction="vertical" size={14} style={{ width: '100%' }}>
                <div><Typography.Text strong>成功率</Typography.Text><Progress percent={successRate} status={failedInstances ? 'exception' : 'success'} /></div>
                <div><Typography.Text strong>Worker 覆盖</Typography.Text><Progress percent={workerCoverage} /></div>
                <div><Typography.Text strong>通知投递</Typography.Text><Progress percent={alertDeliveryRate} status={alertQueue?.dead_letter ? 'exception' : 'success'} /></div>
                <div><Typography.Text strong>集群状态</Typography.Text><div><Tag color={clusterStatus === 'ready' || clusterStatus === 'leader' ? 'green' : 'gold'}>{clusterStatus}</Tag></div></div>
              </Space>
            </Card>
            <Card className="clean-card" title="快速入口">
              <div className="dashboard-action-grid">
                <Link to={ROUTE_META.dispatchQueue.path}><NodeIndexOutlined /> 调度队列 <ArrowRightOutlined className="dashboard-link-icon" /></Link>
                <Link to={ROUTE_META.security.path}><FireOutlined /> 安全态势 <ArrowRightOutlined className="dashboard-link-icon" /></Link>
                <Link to={ROUTE_META.notifications.path}><ApiOutlined /> 通知中心 <ArrowRightOutlined className="dashboard-link-icon" /></Link>
                <Link to={ROUTE_META.audit.path}><ClockCircleOutlined /> 审计日志 <ArrowRightOutlined className="dashboard-link-icon" /></Link>
              </div>
            </Card>
          </Space>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={8}>
          <Card className="clean-card dashboard-ops-card" title="Worker Mesh 分布" extra={<Link to={ROUTE_META.workers.path}>查看 Worker <ArrowRightOutlined /></Link>}>
            <TopList items={workerScopes.map((scope) => ({ label: scope.key, value: scope.count, hint: `${scope.clusters} 集群 · ${scope.masters} 主节点 · ${scope.followers} 从节点` }))} emptyText="暂无在线 Worker" />
          </Card>
        </Col>
        <Col xs={24} xl={8}>
          <Card className="clean-card dashboard-ops-card" title="能力覆盖 Top 6">
            <TopList items={capabilityLeaders} emptyText="暂无能力标签" />
          </Card>
        </Col>
        <Col xs={24} xl={8}>
          <Card className="clean-card dashboard-ops-card" title="最近审计" extra={<Link to={ROUTE_META.audit.path}>查看审计 <ArrowRightOutlined /></Link>}>
            <div className="dashboard-audit-list">
              {recentAudits.slice(0, 6).map((item) => (
                <div key={item.id} className="dashboard-audit-list__item">
                  <Tag color={item.result === 'success' ? 'green' : 'red'}>{item.result}</Tag>
                  <div>
                    <strong>{item.action}</strong>
                    <span>{item.actor} · {item.resource_type}</span>
                  </div>
                </div>
              ))}
              {recentAudits.length === 0 ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无审计记录" /> : null}
            </div>
          </Card>
        </Col>
      </Row>

      <div className="dashboard-footer-note">最近刷新：{lastUpdated ? formatTime(lastUpdated.toISOString()) : '-'}</div>
    </div>
  );
}
