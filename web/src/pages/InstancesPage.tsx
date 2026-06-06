import { Alert, Button, Card, Drawer, Empty, Popconfirm, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useState } from 'react';

import {
  cancelInstance,
  getInstance,
  listInstanceAttempts,
  listInstanceLogs,
  listJobInstances,
  listJobs,
  type JobInstanceAttemptSummary,
  type JobInstanceLogSummary,
  type JobInstanceSummary,
  type JobSummary,
} from '../api/client';
import { WorkerLogTerminal, groupLogsByWorker } from '../components/logs/WorkerLogTerminal';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';
import { formatWorkerDisplayId } from './instances/workerDisplay';
import { persistentPagination, usePersistentTablePageSize } from '../utils/pagination';

const displayWorkerId = (instance: JobInstanceSummary) => instance.workerId ?? instance.latestLog?.workerId ?? '暂无 worker';

const getStatusColor = (status: string) => {
  switch (status) {
    case 'succeeded': return 'success';
    case 'failed': return 'error';
    case 'partial_failed': return 'warning';
    case 'running': return 'processing';
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

const renderBroadcastResults = (instance: JobInstanceSummary | null, attempts: JobInstanceAttemptSummary[], logs: JobInstanceLogSummary[]) => {
  if (instance?.executionMode !== 'broadcast') {
    return null;
  }
  return (
    <div className="instance-result-broadcast">
      <div className="instance-result-broadcast__header">
        <Typography.Text strong>广播节点结果</Typography.Text>
        <Typography.Text type="secondary">{attempts.length} 个执行节点</Typography.Text>
      </div>
      {attempts.length === 0 ? (
        <Typography.Text type="secondary">暂无广播子执行信息</Typography.Text>
      ) : (
        <div className="instance-result-broadcast__grid">
          {attempts.map((attempt) => {
            const workerLogCount = logs.filter((log) => log.workerId === attempt.workerId).length;
            const latestLog = latestWorkerLog(logs, attempt.workerId);
            const nodeResult = attempt.result;
            const nodeMessage = nodeResult?.message ?? latestLog?.message ?? '暂无节点结果';
            return (
              <div key={attempt.id} className="instance-result-broadcast__node">
                <div className="instance-result-broadcast__node-head">
                  <Typography.Text code title={attempt.workerId}>{formatWorkerDisplayId(attempt.workerId)}</Typography.Text>
                  <Tag color={getStatusColor(attempt.status)} className="instance-status-tag">{nodeResult ? (nodeResult.success ? 'succeeded' : 'failed') : attempt.status}</Tag>
                </div>
                <div className="instance-result-broadcast__node-meta">
                  <span>Updated</span>
                  <Typography.Text>{nodeResult?.completedAt ?? attempt.updatedAt}</Typography.Text>
                </div>
                <div className="instance-result-broadcast__node-meta">
                  <span>Result</span>
                  <Typography.Text>{nodeResult ? (nodeResult.success ? 'success' : 'failed') : 'pending'}</Typography.Text>
                </div>
                <div className="instance-result-broadcast__node-meta">
                  <span>Logs</span>
                  <Typography.Text>{workerLogCount} 条</Typography.Text>
                </div>
                <Typography.Paragraph className="instance-result-broadcast__latest-log" title={nodeMessage}>
                  {nodeMessage}
                </Typography.Paragraph>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

const renderExecutionResult = (instance: JobInstanceSummary | null, attempts: JobInstanceAttemptSummary[], logs: JobInstanceLogSummary[]) => {
  const result = instance?.result;
  if (!result) {
    return (
      <Card size="small" className="instance-result-card instance-result-card--empty" title="执行结果" style={{ marginTop: 16 }}>
        <div className="instance-result-empty">
          <Typography.Text strong>暂无执行结果</Typography.Text>
          <Typography.Text type="secondary">实例进入运行、重试或完成后会在这里展示 Worker 返回的具体结果。</Typography.Text>
        </div>
        {renderBroadcastResults(instance, attempts, logs)}
      </Card>
    );
  }

  const statusColor = result.success ? 'success' : 'error';
  const statusText = result.success ? '任务执行成功' : '任务执行失败';
  const messageText = result.message || 'Worker 未返回结果消息';

  return (
    <Card
      size="small"
      className={`instance-result-card instance-result-card--${result.success ? 'success' : 'failed'}`}
      title="执行结果"
      style={{ marginTop: 16 }}
    >
      <div className="instance-result-panel">
        <div className="instance-result-panel__summary">
          <div className="instance-result-panel__status">
            <span className="instance-result-panel__status-dot" />
            <div>
              <Typography.Text strong className="instance-result-panel__status-title">{statusText}</Typography.Text>
              <Typography.Text type="secondary" className="instance-result-panel__status-subtitle">Worker 返回的最终执行结果</Typography.Text>
            </div>
          </div>
          <Tag color={statusColor} className="instance-result-panel__tag">{String(result.success)}</Tag>
        </div>

        <div className="instance-result-panel__meta-grid">
          <div className="instance-result-panel__meta-item">
            <span>Worker</span>
            <Typography.Text code title={result.workerId}>{result.workerId || '-'}</Typography.Text>
          </div>
          <div className="instance-result-panel__meta-item">
            <span>Completed At</span>
            <Typography.Text className="instance-result-panel__time">{result.completedAt || '-'}</Typography.Text>
          </div>
          <div className="instance-result-panel__meta-item">
            <span>Status</span>
            <Tag color={statusColor} className="instance-status-tag">{result.success ? 'succeeded' : 'failed'}</Tag>
          </div>
        </div>

        <div className="instance-result-panel__message">
          <span>Message</span>
          <Typography.Paragraph className="instance-result-panel__message-body">
            {messageText}
          </Typography.Paragraph>
        </div>

        {renderBroadcastResults(instance, attempts, logs)}
      </div>
    </Card>
  );
};

export function InstancesPage() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);
  const [attemptsByInstance, setAttemptsByInstance] = useState<Map<string, JobInstanceAttemptSummary[]>>(new Map());
  const active = useRouteActive(ROUTE_META.instances.path);

  const load = useCallback(async () => {
    try {
      const jobPage = await listJobs();
      setJobs(jobPage.items);
      const instancePages = await Promise.all(jobPage.items.map((job) => listJobInstances(job.id)));
      const sortedInstances = instancePages
        .flatMap((page) => page.items)
        .sort((left, right) => right.createdAt.localeCompare(left.createdAt));
      setInstances(sortedInstances);
      const attemptPairs = await Promise.all(sortedInstances.map(async (instance) => {
        try {
          const attemptPage = await listInstanceAttempts(instance.id);
          return [instance.id, attemptPage.items] as const;
        } catch {
          return [instance.id, [] as JobInstanceAttemptSummary[]] as const;
        }
      }));
      setAttemptsByInstance(new Map(attemptPairs));
    } catch { /* silent */ }
  }, []);

  useEffect(() => { if (active) void load(); }, [active, load]);
  const jobName = new Map(jobs.map((job) => [job.id, job.name]));
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
        message.error(cause instanceof Error ? cause.message : '日志加载失败');
      }
    } finally {
      if (showLoading) {
        setLogsLoading(false);
      }
    }
  }, []);

  const openLogs = async (instance: JobInstanceSummary) => {
    setSelectedInstance(instance);
    setLogDrawerOpen(true);
    await loadLogs(instance);
  };

  useEffect(() => {
    if (!active || !logDrawerOpen || !selectedInstance) {
      return undefined;
    }
    const timer = window.setInterval(() => {
      void loadLogs(selectedInstance, false);
    }, 2_000);
    return () => window.clearInterval(timer);
  }, [active, loadLogs, logDrawerOpen, selectedInstance]);

  const cancelRunningInstance = async (instance: JobInstanceSummary) => {
    try {
      const updated = await cancelInstance(instance.id);
      message.success(`已取消实例 ${updated.id}`);
      await load();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : '取消失败');
    }
  };

  const copyInstanceId = async (instanceId: string) => {
    try {
      await navigator.clipboard.writeText(instanceId);
      message.success('实例 ID 已复制');
    } catch {
      message.error('实例 ID 复制失败');
    }
  };

  const copyWorkerId = async (workerId: string) => {
    try {
      await navigator.clipboard.writeText(workerId);
      message.success('执行节点已复制');
    } catch {
      message.error('执行节点复制失败');
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
          <Typography.Text ellipsis style={{ maxWidth: 188 }}>
            {instance.latestLog?.message ?? '暂无日志'}
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
          {['pending', 'dispatching', 'running'].includes(instance.status) ? (
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
    <Card className="clean-card" title="执行实例">
      {instances.length === 0 ? (
        <Empty description="还没有实例，请先在 Jobs 页面创建并触发任务" />
      ) : (
        <>
          <Typography.Paragraph type="secondary">实例详情 API 已可用：GET /api/v1/instances/&lt;instance&gt;</Typography.Paragraph>
          <Table rowKey="id" columns={columns} dataSource={instances} pagination={persistentPagination(pageSize, setPageSize)} scroll={{ x: 1_440 }} />
        </>
      )}
      <Drawer
        className="instance-log-drawer"
        width="60vw"
        title={selectedInstance ? `实例日志： ${selectedInstance.id}` : '实例日志'}
        open={logDrawerOpen}
        onClose={() => setLogDrawerOpen(false)}
      >
        <Card size="small" className="instance-log-section" title={selectedInstance?.executionMode === 'single' ? '执行器' : '广播子执行'}>
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
            locale={{ emptyText: selectedInstance?.executionMode === 'single' ? '暂无执行器信息' : '暂无广播子执行' }}
          />
        </Card>
        {renderExecutionResult(selectedInstance, attempts, logs)}
        <Space align="center" style={{ marginTop: 24, marginBottom: 12 }}>
          <Typography.Title level={5} style={{ margin: 0 }}>执行日志</Typography.Title>
          {selectedInstance ? (
            <Button size="small" onClick={() => void loadLogs(selectedInstance)} loading={logsLoading}>刷新</Button>
          ) : null}
        </Space>
        {governanceLogs.length > 0 ? (
          <Alert
            type="warning"
            showIcon
            message={`脚本执行治理事件 ${governanceLogs.length} 条`}
            description="已识别脚本 capability、runner、policy、digest、timeout、output 或 runtime 相关治理失败。"
            style={{ marginBottom: 12 }}
          />
        ) : null}
        {workerLogGroups.length === 0 ? (
          <Empty description={logsLoading ? '日志加载中...' : '暂无日志'} />
        ) : (
          <WorkerLogTerminal groups={workerLogGroups} />
        )}
      </Drawer>
    </Card>
  );
}
