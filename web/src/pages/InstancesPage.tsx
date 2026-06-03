import { Alert, Button, Card, Drawer, Empty, Popconfirm, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useState } from 'react';

import {
  cancelInstance,
  listInstanceAttempts,
  listInstanceLogs,
  listJobInstances,
  listJobs,
  type JobInstanceAttemptSummary,
  type JobInstanceLogSummary,
  type JobInstanceSummary,
  type JobSummary,
} from '../api/client';
import { persistentPagination, usePersistentTablePageSize } from '../utils/pagination';

export function InstancesPage() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);

  const load = useCallback(async () => {
    try {
      const jobPage = await listJobs();
      setJobs(jobPage.items);
      const instancePages = await Promise.all(jobPage.items.map((job) => listJobInstances(job.id)));
      setInstances(instancePages
        .flatMap((page) => page.items)
        .sort((left, right) => right.createdAt.localeCompare(left.createdAt)));
    } catch { /* silent */ }
  }, []);

  useEffect(() => { void load(); }, [load]);
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
      const [logPage, attemptPage] = await Promise.all([
        listInstanceLogs(instance.id),
        listInstanceAttempts(instance.id),
      ]);
      setLogs(logPage.items);
      setAttempts(attemptPage.items);
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
    if (!logDrawerOpen || !selectedInstance) {
      return undefined;
    }
    const timer = window.setInterval(() => {
      void loadLogs(selectedInstance, false);
    }, 2_000);
    return () => window.clearInterval(timer);
  }, [loadLogs, logDrawerOpen, selectedInstance]);

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

  const cancelRunningInstance = async (instance: JobInstanceSummary) => {
    try {
      const updated = await cancelInstance(instance.id);
      message.success(`已取消实例 ${updated.id}`);
      await load();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : '取消失败');
    }
  };

  const columns: ColumnsType<JobInstanceSummary> = [
    { title: 'Instance', dataIndex: 'id', ellipsis: true, width: 140 },
    { title: 'Job', dataIndex: 'jobId', render: (value: string) => <strong>{jobName.get(value) ?? value}</strong> },
    { title: 'Status', dataIndex: 'status', render: (value: string) => <Tag color={getStatusColor(value)} className="instance-status-tag">{value}</Tag> },
    { title: 'Trigger', dataIndex: 'triggerType', render: (value: string) => <Tag>{value}</Tag> },
    { title: 'Mode', dataIndex: 'executionMode', render: (value: string) => <Tag color={value === 'broadcast' ? 'purple' : 'default'} className="soft-tag">{value}</Tag> },
    { title: 'Created At', dataIndex: 'createdAt', width: 180 },
    {
      title: 'Latest Log',
      width: 280,
      render: (_, instance) => (
        <Space direction="vertical" size={2}>
          <Typography.Text ellipsis style={{ maxWidth: 260 }}>
            {instance.latestLog?.message ?? '暂无日志'}
          </Typography.Text>
          <Typography.Text type="secondary">{instance.logCount ?? 0} 条日志</Typography.Text>
        </Space>
      ),
    },
    {
      title: 'Actions',
      width: 180,
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
    { title: 'Status', dataIndex: 'status', render: (value: string) => <Tag color={getStatusColor(value)} className="instance-status-tag">{value}</Tag> },
    { title: 'Updated At', dataIndex: 'updatedAt', width: 180 },
  ];


  const governanceLogs = logs.filter((log) => log.governanceEvent === 'script_execution_governance');

  const renderLogMessage = (log: JobInstanceLogSummary) => {
    if (log.governanceEvent !== 'script_execution_governance') {
      return log.message;
    }
    return (
      <Space direction="vertical" size={2}>
        <Space wrap>
          <Tag color="volcano">script governance</Tag>
          {log.governanceFailureClass ? <Tag color="red">{log.governanceFailureClass}</Tag> : null}
        </Space>
        <Typography.Text>{log.governanceMessage ?? log.message}</Typography.Text>
      </Space>
    );
  };

  const logColumns: ColumnsType<JobInstanceLogSummary> = [
    { title: '#', dataIndex: 'sequence', width: 60 },
    { title: 'Level', dataIndex: 'level', width: 90, render: (value: string) => <Tag color={value === 'error' ? 'red' : value === 'warn' ? 'orange' : 'blue'}>{value}</Tag> },
    { title: 'Worker', dataIndex: 'workerId', ellipsis: true, width: 120 },
    { title: 'Message', dataIndex: 'message', render: (_: string, log) => renderLogMessage(log) },
  ];

  return (
    <Card className="clean-card" title="执行实例">
      {instances.length === 0 ? (
        <Empty description="还没有实例，请先在 Jobs 页面创建并触发任务" />
      ) : (
        <>
          <Typography.Paragraph type="secondary">实例详情 API 已可用：GET /api/v1/instances/&lt;instance&gt;</Typography.Paragraph>
          <Table rowKey="id" columns={columns} dataSource={instances} pagination={persistentPagination(pageSize, setPageSize)} />
        </>
      )}
      <Drawer
        width={900}
        title={selectedInstance ? `实例日志： ${selectedInstance.id}` : '实例日志'}
        open={logDrawerOpen}
        onClose={() => setLogDrawerOpen(false)}
      >
        <Typography.Title level={5}>{selectedInstance?.executionMode === 'single' ? '执行器' : '广播子执行'}</Typography.Title>
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
          locale={{ emptyText: selectedInstance?.executionMode === 'single' ? '暂无执行器信息' : '暂无广播子执行' }}
        />
        <Space align="center" style={{ marginTop: 24, marginBottom: 8 }}>
          <Typography.Title level={5} style={{ margin: 0 }}>执行日志</Typography.Title>
          {selectedInstance ? (
            <Button size="small" onClick={() => void loadLogs(selectedInstance)}>刷新</Button>
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
        <Table
          rowKey="id"
          loading={logsLoading}
          columns={logColumns}
          dataSource={logs}
          pagination={persistentPagination(pageSize, setPageSize)}
          locale={{ emptyText: '暂无日志' }}
        />
      </Drawer>
    </Card>
  );
}
