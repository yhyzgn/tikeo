import { Alert, Button, Card, Col, Drawer, Empty, Input, Popconfirm, Row, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import {
  cancelInstance,
  getInstance,
  instanceListStreamUrl,
  instanceLogStreamUrl,
  listInstanceAttempts,
  listInstanceLogs,
  listJobInstances,
  listJobs,
  type JobInstanceAttemptSummary,
  type JobInstanceLogSummary,
  type JobInstanceResult,
  type JobInstanceSummary,
  type JobSummary,
} from '../api/client';
import { WorkerLogTerminal, groupLogsByWorker } from '../components/logs/WorkerLogTerminal';
import { useRouteActive } from '../hooks/useRouteActivation';
import { useI18n, type LocaleCode } from '../i18n/I18nContext';
import { ROUTE_META } from '../routes';
import { formatWorkerDisplayId } from './instances/workerDisplay';
import { persistentPagination, usePersistentTablePageSize } from '../utils/pagination';


const ACTIVE_INSTANCE_STATUSES = ['pending', 'dispatching', 'running', 'retrying'];
const INSTANCE_STATUS_OPTIONS = ['pending', 'dispatching', 'running', 'retrying', 'succeeded', 'failed', 'partial_failed', 'cancelled'];
const INSTANCE_TRIGGER_OPTIONS = ['api', 'schedule', 'manual', 'cron', 'fixed_rate', 'webhook', 'unknown'];
const INSTANCE_MODE_OPTIONS = ['single', 'broadcast'];

type InstanceFilters = {
  status: string;
  jobId: string;
  triggerType: string;
  executionMode: string;
  workerId: string;
  keyword: string;
};

const EMPTY_INSTANCE_FILTERS: InstanceFilters = {
  status: '',
  jobId: '',
  triggerType: '',
  executionMode: '',
  workerId: '',
  keyword: '',
};

function filtersFromSearchParams(params: URLSearchParams): InstanceFilters {
  return {
    status: params.get('status') ?? '',
    jobId: params.get('jobId') ?? '',
    triggerType: params.get('triggerType') ?? '',
    executionMode: params.get('executionMode') ?? '',
    workerId: params.get('workerId') ?? '',
    keyword: params.get('keyword') ?? '',
  };
}

function hasInstanceFilters(filters: InstanceFilters): boolean {
  return Object.values(filters).some((value) => value.trim() !== '');
}

function instanceMatchesFilters(
  instance: JobInstanceSummary,
  attempts: JobInstanceAttemptSummary[] | undefined,
  jobName: Map<string, string>,
  filters: InstanceFilters,
): boolean {
  if (filters.status) {
    const statuses = filters.status === 'active' ? ACTIVE_INSTANCE_STATUSES : [filters.status];
    if (!statuses.includes(instance.status)) return false;
  }
  if (filters.jobId && instance.jobId !== filters.jobId) return false;
  if (filters.triggerType && instance.triggerType !== filters.triggerType) return false;
  if (filters.executionMode && instance.executionMode !== filters.executionMode) return false;
  if (filters.workerId) {
    const workerNeedle = filters.workerId.toLowerCase();
    const workerValues = [instance.workerId, instance.latestLog?.workerId, ...(attempts ?? []).map((attempt) => attempt.workerId)]
      .filter(Boolean)
      .map((value) => String(value).toLowerCase());
    if (!workerValues.some((value) => value.includes(workerNeedle))) return false;
  }
  if (filters.keyword) {
    const keyword = filters.keyword.toLowerCase();
    const searchable = [
      instance.id,
      instance.jobId,
      jobName.get(instance.jobId),
      instance.status,
      instance.triggerType,
      instance.executionMode,
      instance.latestLog?.message,
      instance.workerId,
      instance.latestLog?.workerId,
    ].filter(Boolean).join(' ').toLowerCase();
    if (!searchable.includes(keyword)) return false;
  }
  return true;
}

function semanticFilterLabel(filters: InstanceFilters): string | null {
  if (filters.status === 'failed') return '失败实例';
  if (filters.status === 'active') return '活跃实例';
  if (filters.executionMode === 'broadcast') return '广播实例';
  if (filters.workerId) return '指定 Worker';
  if (filters.jobId) return '指定任务';
  return null;
}

const displayWorkerId = (instance: JobInstanceSummary) => instance.workerId ?? instance.latestLog?.workerId ?? '暂无 worker';

const getStatusColor = (status: string) => {
  switch (status) {
    case 'succeeded': return 'success';
    case 'failed': return 'error';
    case 'partial_failed': return 'warning';
    case 'running': return 'processing';
    case 'retrying': return 'warning';
    case 'pending': return 'gold';
    default: return 'default';
  }
};


const displayExecutionNodes = (
  instance: JobInstanceSummary,
  attempts: JobInstanceAttemptSummary[] | undefined,
  onCopyWorkerId: (workerId: string) => void,
) => {
  const workerIds = [...new Set((attempts ?? []).map((attempt) => attempt.workerId).filter(Boolean))];
  const nodes = workerIds.length > 0 ? workerIds : [displayWorkerId(instance)];

  return (
    <Space direction="vertical" size={2} className="instance-execution-node-list">
      {nodes.map((workerId) => (
        <Typography.Text
          key={workerId}
          code
          className="instance-copy-id"
          title="点击复制执行节点"
          style={{ maxWidth: 308 }}
          onClick={() => onCopyWorkerId(workerId)}
        >
          {formatWorkerDisplayId(workerId)}
        </Typography.Text>
      ))}
    </Space>
  );
};


const latestWorkerLog = (logs: JobInstanceLogSummary[], workerId: string) => [...logs].reverse().find((log) => log.workerId === workerId);

const formatNodeResultSummary = (total: number, completed: number, locale: LocaleCode) => (
  locale === 'en-US' ? `${total} execution nodes, ${completed} returned results` : `共 ${total} 个执行节点，${completed} 个已返回结果`
);

const formatExecutionNodeCount = (count: number, locale: LocaleCode) => (locale === 'en-US' ? `${count} execution nodes` : `${count} 个执行节点`);

const formatResultLogCount = (count: number, locale: LocaleCode) => (locale === 'en-US' ? `${count} logs` : `${count} 条`);

const formatInstanceLogTitle = (instanceId: string, locale: LocaleCode) => (locale === 'en-US' ? `Instance logs: ${instanceId}` : `实例日志： ${instanceId}`);

type ExecutionResultNode = {
  id: string;
  workerId: string;
  status: string;
  result: JobInstanceResult | null;
  updatedAt: string;
  logCount: number;
  latestMessage: string | null;
};

const buildExecutionResultNodes = (
  instance: JobInstanceSummary | null,
  attempts: JobInstanceAttemptSummary[],
  logs: JobInstanceLogSummary[],
): ExecutionResultNode[] => {
  if (!instance) {
    return [];
  }

  if (instance.executionMode === 'broadcast') {
    return attempts.map((attempt) => {
      const latestLog = latestWorkerLog(logs, attempt.workerId);
      return {
        id: attempt.id,
        workerId: attempt.workerId,
        status: attempt.result ? (attempt.result.success ? 'succeeded' : 'failed') : attempt.status,
        result: attempt.result ?? null,
        updatedAt: attempt.result?.completedAt ?? attempt.updatedAt,
        logCount: logs.filter((log) => log.workerId === attempt.workerId).length,
        latestMessage: latestLog?.message ?? null,
      };
    });
  }

  const workerId = instance.result?.workerId ?? instance.workerId ?? instance.latestLog?.workerId ?? '暂无 worker 日志';
  const latestLog = latestWorkerLog(logs, workerId) ?? instance.latestLog ?? null;
  return [{
    id: `${instance.id}-result`,
    workerId,
    status: instance.result ? (instance.result.success ? 'succeeded' : 'failed') : instance.status,
    result: instance.result ?? null,
    updatedAt: instance.result?.completedAt ?? instance.updatedAt,
    logCount: logs.filter((log) => log.workerId === workerId).length || instance.logCount || 0,
    latestMessage: latestLog?.message ?? null,
  }];
};

const renderExecutionResult = (instance: JobInstanceSummary | null, attempts: JobInstanceAttemptSummary[], logs: JobInstanceLogSummary[], t: (text: string) => string, locale: LocaleCode) => {
  const nodes = buildExecutionResultNodes(instance, attempts, logs);
  const completed = nodes.filter((node) => node.result).length;
  const failed = nodes.filter((node) => node.result && !node.result.success).length;
  const cardState = failed > 0 ? 'failed' : completed > 0 ? 'success' : 'pending';

  return (
    <Card
      size="small"
      className={`instance-result-card instance-result-card--${cardState}`}
      title={t('执行结果')}
      style={{ marginTop: 16 }}
    >
      <div className="instance-result-panel">
        <div className="instance-result-panel__summary">
          <div className="instance-result-panel__status">
            <span className="instance-result-panel__status-dot" />
            <div>
              <Typography.Text strong className="instance-result-panel__status-title">{t('节点执行结果')}</Typography.Text>
              <Typography.Text type="secondary" className="instance-result-panel__status-subtitle">
                {nodes.length > 0 ? formatNodeResultSummary(nodes.length, completed, locale) : t('暂无执行节点信息')}
              </Typography.Text>
            </div>
          </div>
          {failed > 0 ? (
            <Tag color="error" className="instance-result-panel__tag">{failed} failed</Tag>
          ) : (
            <Tag color={completed > 0 ? 'success' : 'processing'} className="instance-result-panel__tag">{completed}/{nodes.length}</Tag>
          )}
        </div>

        {nodes.length === 0 ? (
          <div className="instance-result-empty">
            <Typography.Text strong>{t('暂无执行节点信息')}</Typography.Text>
            <Typography.Text type="secondary">{t('实例开始分发后会在这里按节点展示执行结果。')}</Typography.Text>
          </div>
        ) : (
          <div className="instance-result-nodes">
            <div className="instance-result-nodes__header">
              <Typography.Text strong>{t(instance?.executionMode === 'broadcast' ? '广播节点结果' : '单节点结果')}</Typography.Text>
              <Typography.Text type="secondary">{formatExecutionNodeCount(nodes.length, locale)}</Typography.Text>
            </div>
            <div className="instance-result-nodes__list">
              {nodes.map((node) => {
                const messageText = node.result?.message ?? node.latestMessage ?? t('等待 Worker 返回结果');
                const resultText = node.result ? (node.result.success ? t('success') : t('failed')) : t('pending');
                return (
                  <div key={node.id} className="instance-result-nodes__node">
                    <div className="instance-result-nodes__node-main">
                      <div className="instance-result-nodes__node-head">
                        <Typography.Text code title={node.workerId}>{formatWorkerDisplayId(node.workerId)}</Typography.Text>
                        <Tag color={getStatusColor(node.status)} className="instance-status-tag">{node.status}</Tag>
                      </div>
                      <div className="instance-result-nodes__meta-row">
                        <div className="instance-result-nodes__node-meta">
                          <span>{t('Updated')}</span>
                          <Typography.Text>{node.updatedAt || '-'}</Typography.Text>
                        </div>
                        <div className="instance-result-nodes__node-meta">
                          <span>{t('Result')}</span>
                          <Typography.Text>{resultText}</Typography.Text>
                        </div>
                        <div className="instance-result-nodes__node-meta">
                          <span>{t('Logs')}</span>
                          <Typography.Text>{formatResultLogCount(node.logCount, locale)}</Typography.Text>
                        </div>
                      </div>
                    </div>
                    <div className="instance-result-nodes__message">
                      <span>{t('Message')}</span>
                      <Typography.Paragraph className="instance-result-panel__message-body" data-runtime-text title={messageText}>
                        {messageText}
                      </Typography.Paragraph>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </Card>
  );
};

type InstanceListStreamSnapshot = {
  jobs: JobSummary[];
  instances: JobInstanceSummary[];
  attempts: Array<{ instanceId: string; items: JobInstanceAttemptSummary[] }>;
};

export function InstancesPage() {
  const { locale, t } = useI18n();
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);
  const [attemptsByInstance, setAttemptsByInstance] = useState<Map<string, JobInstanceAttemptSummary[]>>(new Map());
  const [searchParams, setSearchParams] = useSearchParams();
  const filters = useMemo(() => filtersFromSearchParams(searchParams), [searchParams]);
  const active = useRouteActive(ROUTE_META.instances.path);

  const applyInstanceSnapshot = useCallback((snapshot: InstanceListStreamSnapshot) => {
    setJobs(snapshot.jobs);
    setInstances(snapshot.instances);
    setAttemptsByInstance(new Map(snapshot.attempts.map((entry) => [entry.instanceId, entry.items])));
  }, []);

  const load = useCallback(async (options?: { silent?: boolean }) => {
    try {
      const jobPage = await listJobs();
      const instancePages = await Promise.all(jobPage.items.map((job) => listJobInstances(job.id)));
      const sortedInstances = instancePages
        .flatMap((page) => page.items)
        .sort((left, right) => right.createdAt.localeCompare(left.createdAt));
      const attemptPairs = await Promise.all(sortedInstances.map(async (instance) => {
        try {
          const attemptPage = await listInstanceAttempts(instance.id);
          return [instance.id, attemptPage.items] as const;
        } catch {
          return [instance.id, [] as JobInstanceAttemptSummary[]] as const;
        }
      }));
      applyInstanceSnapshot({
        jobs: jobPage.items,
        instances: sortedInstances,
        attempts: attemptPairs.map(([instanceId, items]) => ({ instanceId, items })),
      });
    } catch (cause) {
      if (!options?.silent) {
        message.error(cause instanceof Error ? cause.message : t('实例加载失败'));
      }
    }
  }, [applyInstanceSnapshot, t]);

  useEffect(() => { if (active) void load(); }, [active, load]);
  useEffect(() => {
    if (!active) return undefined;
    const source = new EventSource(instanceListStreamUrl());
    source.addEventListener('instances.snapshot', (event) => {
      try {
        const snapshot = JSON.parse((event as MessageEvent).data) as InstanceListStreamSnapshot;
        applyInstanceSnapshot(snapshot);
      } catch {
        // Ignore malformed stream frames; the 3s fallback refresh and manual navigation remain available.
      }
    });
    const fallbackTimer = window.setInterval(() => { void load({ silent: true }); }, 3000);
    return () => {
      source.close();
      window.clearInterval(fallbackTimer);
    };
  }, [active, applyInstanceSnapshot, load]);
  const jobName = useMemo(() => new Map(jobs.map((job) => [job.id, job.name])), [jobs]);
  const filteredInstances = useMemo(() => instances.filter((instance) => instanceMatchesFilters(instance, attemptsByInstance.get(instance.id), jobName, filters)), [attemptsByInstance, filters, instances, jobName]);
  const filterEntryLabel = semanticFilterLabel(filters);
  const updateFilters = useCallback((patch: Partial<InstanceFilters>) => {
    const next = { ...filters, ...patch };
    setSearchParams((previous) => {
      const params = new URLSearchParams(previous);
      (Object.entries(next) as Array<[keyof InstanceFilters, string]>).forEach(([key, value]) => {
        const normalized = value.trim();
        if (normalized) params.set(key, normalized);
        else params.delete(key);
      });
      return params;
    }, { replace: true });
  }, [filters, setSearchParams]);
  const resetFilters = useCallback(() => {
    setSearchParams((previous) => {
      const params = new URLSearchParams(previous);
      (Object.keys(EMPTY_INSTANCE_FILTERS) as Array<keyof InstanceFilters>).forEach((key) => params.delete(key));
      return params;
    }, { replace: true });
  }, [setSearchParams]);
  const [logDrawerOpen, setLogDrawerOpen] = useState(false);
  const [selectedInstance, setSelectedInstance] = useState<JobInstanceSummary | null>(null);
  const [logs, setLogs] = useState<JobInstanceLogSummary[]>([]);
  const [attempts, setAttempts] = useState<JobInstanceAttemptSummary[]>([]);
  const [logsLoading, setLogsLoading] = useState(false);
  const [pageSize, setPageSize] = usePersistentTablePageSize();

  const loadLogs = useCallback(async (instance: JobInstanceSummary, showLoading = true) => {
    if (showLoading) {
      setLogsLoading(true);
    }
    try {
      const [logPage, attemptPage, freshInstance] = await Promise.all([
        listInstanceLogs(instance.id),
        listInstanceAttempts(instance.id),
        getInstance(instance.id),
      ]);
      setLogs(logPage.items);
      setAttempts(attemptPage.items);
      setSelectedInstance(freshInstance);
      setInstances((current) => current.map((item) => item.id === freshInstance.id ? freshInstance : item));
      setAttemptsByInstance((previous) => new Map(previous).set(instance.id, attemptPage.items));
    } catch (cause) {
      if (showLoading) {
        message.error(cause instanceof Error ? cause.message : t('日志加载失败'));
      }
    } finally {
      if (showLoading) {
        setLogsLoading(false);
      }
    }
  }, [t]);

  const openLogs = async (instance: JobInstanceSummary) => {
    setSelectedInstance(instance);
    setLogDrawerOpen(true);
    await loadLogs(instance);
  };

  useEffect(() => {
    if (!active || !logDrawerOpen || !selectedInstance) {
      return undefined;
    }
    const source = new EventSource(instanceLogStreamUrl(selectedInstance.id));
    source.addEventListener('instance.snapshot', (event) => {
      try {
        const snapshot = JSON.parse((event as MessageEvent).data) as {
          instance: JobInstanceSummary;
          attempts: JobInstanceAttemptSummary[];
        };
        setSelectedInstance(snapshot.instance);
        setAttempts(snapshot.attempts);
        setInstances((current) => current.map((item) => item.id === snapshot.instance.id ? snapshot.instance : item));
        setAttemptsByInstance((previous) => new Map(previous).set(snapshot.instance.id, snapshot.attempts));
      } catch {
        // Ignore malformed stream frames; the manual refresh button remains available.
      }
    });
    source.addEventListener('instance.log', (event) => {
      try {
        const log = JSON.parse((event as MessageEvent).data) as JobInstanceLogSummary;
        setLogs((current) => {
          if (current.some((item) => item.id === log.id)) return current;
          return [...current, log].sort((left, right) => left.sequence - right.sequence);
        });
      } catch {
        // Ignore malformed stream frames; the manual refresh button remains available.
      }
    });
    return () => source.close();
  }, [active, logDrawerOpen, selectedInstance?.id]);

  const cancelRunningInstance = async (instance: JobInstanceSummary) => {
    try {
      const updated = await cancelInstance(instance.id);
      message.success(locale === 'en-US' ? `Cancelled instance ${updated.id}` : `已取消实例 ${updated.id}`);
      await load();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : t('取消失败'));
    }
  };

  const copyInstanceId = async (instanceId: string) => {
    try {
      await navigator.clipboard.writeText(instanceId);
      message.success(t('实例 ID 已复制'));
    } catch {
      message.error(t('实例 ID 复制失败'));
    }
  };

  const copyWorkerId = async (workerId: string) => {
    try {
      await navigator.clipboard.writeText(workerId);
      message.success(t('执行节点已复制'));
    } catch {
      message.error(t('执行节点复制失败'));
    }
  };

  const columns: ColumnsType<JobInstanceSummary> = [
    {
      title: 'Instance',
      dataIndex: 'id',
      width: 220,
      render: (_, instance) => (
        <Typography.Text
          code
          className="instance-copy-id"
          title="点击复制实例 ID"
          onClick={() => void copyInstanceId(instance.id)}
        >
          {instance.id}
        </Typography.Text>
      ),
    },
    { title: 'Job', dataIndex: 'jobId', width: 220, render: (value: string) => <strong>{jobName.get(value) ?? value}</strong> },
    { title: 'Status', dataIndex: 'status', width: 120, render: (value: string) => <Tag color={getStatusColor(value)} className="instance-status-tag">{value}</Tag> },
    { title: 'Trigger', dataIndex: 'triggerType', width: 110, render: (value: string) => <Tag>{value}</Tag> },
    { title: 'Mode', dataIndex: 'executionMode', width: 120, render: (value: string) => <Tag color={value === 'broadcast' ? 'purple' : 'default'} className="soft-tag">{value}</Tag> },
    {
      title: '执行节点', key: 'executionNodes', width: 340,
      render: (_, instance) => displayExecutionNodes(instance, attemptsByInstance.get(instance.id), (workerId) => void copyWorkerId(workerId)),
    },
    { title: 'Created At', dataIndex: 'createdAt', width: 220 },
    {
      title: 'Latest Log',
      width: 320,
      render: (_, instance) => (
        <Space direction="vertical" size={2}>
          <Typography.Text ellipsis style={{ maxWidth: 188 }} data-runtime-text>
            {instance.latestLog?.message ?? t('暂无日志')}
          </Typography.Text>
          <Typography.Text type="secondary">{instance.logCount ?? 0} 条日志</Typography.Text>
        </Space>
      ),
    },
    {
      title: 'Actions',
      width: 140,
      render: (_, instance) => (
        <Space size={4}>
          <Button type="link" onClick={() => void openLogs(instance)}>查看日志</Button>
          {['pending', 'dispatching', 'running', 'retrying'].includes(instance.status) ? (
            <Popconfirm title="取消实例" description="取消后会关闭对应队列项，Worker 后续结果会被视为过期。" onConfirm={() => void cancelRunningInstance(instance)}>
              <Button type="link" danger>取消</Button>
            </Popconfirm>
          ) : null}
        </Space>
      ),
    },
  ];

  const attemptColumns: ColumnsType<JobInstanceAttemptSummary> = [
    { title: 'Worker', dataIndex: 'workerId', ellipsis: true },
    { title: 'Status', dataIndex: 'status', width: 110, render: (value: string) => <Tag color={getStatusColor(value)} className="instance-status-tag">{value}</Tag> },
    {
      title: 'Updated At',
      dataIndex: 'updatedAt',
      width: 360,
      render: (value: string) => <Typography.Text className="instance-log-attempt-time">{value}</Typography.Text>,
    },
  ];

  const workerLogGroups = groupLogsByWorker(logs);
  const governanceLogs = logs.filter((log) => log.governanceEvent === 'script_execution_governance');

  return (
    <Card className="clean-card instance-list-card" title="执行实例">
      <div className="instance-filter-panel">
        <div className="instance-filter-panel__header">
          <div>
            <Typography.Text strong>实例过滤</Typography.Text>
            <Typography.Paragraph type="secondary">
              {filterEntryLabel ? `当前入口：${filterEntryLabel}` : '可按状态、任务、触发来源、执行模式、Worker 或关键字过滤。'}
            </Typography.Paragraph>
          </div>
          <Space>
            <Tag color={hasInstanceFilters(filters) ? 'blue' : 'default'}>{filteredInstances.length}/{instances.length}</Tag>
            <Button onClick={resetFilters} disabled={!hasInstanceFilters(filters)}>重置</Button>
          </Space>
        </div>
        <Row gutter={[12, 12]}>
          <Col xs={24} md={8} xl={4}>
            <Select
              allowClear
              value={filters.status || undefined}
              placeholder="状态"
              style={{ width: '100%' }}
              onChange={(value) => updateFilters({ status: value ?? '' })}
              options={[{ value: 'active', label: 'active / 运行中' }, ...INSTANCE_STATUS_OPTIONS.map((value) => ({ value, label: value }))]}
            />
          </Col>
          <Col xs={24} md={8} xl={5}>
            <Select
              allowClear
              showSearch
              optionFilterProp="label"
              value={filters.jobId || undefined}
              placeholder="任务"
              style={{ width: '100%' }}
              onChange={(value) => updateFilters({ jobId: value ?? '' })}
              options={jobs.map((job) => ({ value: job.id, label: `${job.name} · ${job.namespace}/${job.app}` }))}
            />
          </Col>
          <Col xs={24} md={8} xl={4}>
            <Select
              allowClear
              value={filters.triggerType || undefined}
              placeholder="触发方式"
              style={{ width: '100%' }}
              onChange={(value) => updateFilters({ triggerType: value ?? '' })}
              options={INSTANCE_TRIGGER_OPTIONS.map((value) => ({ value, label: value }))}
            />
          </Col>
          <Col xs={24} md={8} xl={4}>
            <Select
              allowClear
              value={filters.executionMode || undefined}
              placeholder="执行模式"
              style={{ width: '100%' }}
              onChange={(value) => updateFilters({ executionMode: value ?? '' })}
              options={INSTANCE_MODE_OPTIONS.map((value) => ({ value, label: value }))}
            />
          </Col>
          <Col xs={24} md={8} xl={3}>
            <Input value={filters.workerId} placeholder="Worker" allowClear onChange={(event) => updateFilters({ workerId: event.target.value })} />
          </Col>
          <Col xs={24} md={8} xl={4}>
            <Input.Search value={filters.keyword} placeholder="实例 / 日志关键字" allowClear onChange={(event) => updateFilters({ keyword: event.target.value })} />
          </Col>
        </Row>
      </div>
      {instances.length === 0 ? (
        <Empty description="还没有实例，请先在 Jobs 页面创建并触发任务" />
      ) : (
        <>
          <Typography.Paragraph type="secondary">实例详情 API 已可用：GET /api/v1/instances/&lt;instance&gt;</Typography.Paragraph>
          <Table rowKey="id" columns={columns} dataSource={filteredInstances} pagination={persistentPagination(pageSize, setPageSize)} scroll={{ x: 1_440 }} locale={{ emptyText: hasInstanceFilters(filters) ? '没有匹配当前过滤条件的实例' : '暂无实例' }} />
        </>
      )}
      <Drawer
        className="instance-log-drawer"
        width="60vw"
        title={selectedInstance ? formatInstanceLogTitle(selectedInstance.id, locale) : t('实例日志')}
        open={logDrawerOpen}
        onClose={() => setLogDrawerOpen(false)}
      >
        <Card size="small" className="instance-log-section" title={t(selectedInstance?.executionMode === 'single' ? '执行器' : '广播子执行')}>
          <Table
            rowKey="id"
            columns={attemptColumns}
            dataSource={selectedInstance?.executionMode === 'single' ? [{
              id: `${selectedInstance.id}-executor`,
              instanceId: selectedInstance.id,
              workerId: selectedInstance.workerId ?? selectedInstance.latestLog?.workerId ?? '暂无 worker 日志',
              status: selectedInstance.status,
              createdAt: selectedInstance.createdAt,
              updatedAt: selectedInstance.updatedAt,
            }] : attempts}
            pagination={false}
            scroll={{ x: 860 }}
            locale={{ emptyText: t(selectedInstance?.executionMode === 'single' ? '暂无执行器信息' : '暂无广播子执行') }}
          />
        </Card>
        {renderExecutionResult(selectedInstance, attempts, logs, t, locale)}
        <Space align="center" style={{ marginTop: 24, marginBottom: 12 }}>
          <Typography.Title level={5} style={{ margin: 0 }}>{t('执行日志')}</Typography.Title>
          {selectedInstance ? (
            <Button size="small" onClick={() => void loadLogs(selectedInstance)} loading={logsLoading}>{t('刷新')}</Button>
          ) : null}
        </Space>
        {governanceLogs.length > 0 ? (
          <Alert
            type="warning"
            showIcon
            message={locale === 'en-US' ? `Script execution governance events: ${governanceLogs.length}` : `脚本执行治理事件 ${governanceLogs.length} 条`}
            description={t('已识别脚本 capability、runner、policy、digest、timeout、output 或 runtime 相关治理失败。')}
            style={{ marginBottom: 12 }}
          />
        ) : null}
        {workerLogGroups.length === 0 ? (
          <Empty description={logsLoading ? t('日志加载中...') : t('暂无日志')} />
        ) : (
          <WorkerLogTerminal groups={workerLogGroups} />
        )}
      </Drawer>
    </Card>
  );
}
