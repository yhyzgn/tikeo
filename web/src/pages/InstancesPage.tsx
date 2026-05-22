import { Alert, Button, Card, Drawer, Empty, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useState } from 'react';

import {
  listInstanceAttempts,
  listInstanceLogs,
  listJobInstances,
  listJobs,
  type JobInstanceAttemptSummary,
  type JobInstanceLogSummary,
  type JobInstanceSummary,
  type JobSummary,
} from '../api/client';

export function InstancesPage() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);

  const load = useCallback(async () => {
    try {
      const jobPage = await listJobs();
      setJobs(jobPage.items);
      const instancePages = await Promise.all(jobPage.items.map((job) => listJobInstances(job.id)));
      setInstances(instancePages.flatMap((page) => page.items));
    } catch { /* silent */ }
  }, []);

  useEffect(() => { void load(); }, [load]);
  const jobName = new Map(jobs.map((job) => [job.id, job.name]));
  const [logDrawerOpen, setLogDrawerOpen] = useState(false);
  const [selectedInstance, setSelectedInstance] = useState<JobInstanceSummary | null>(null);
  const [logs, setLogs] = useState<JobInstanceLogSummary[]>([]);
  const [attempts, setAttempts] = useState<JobInstanceAttemptSummary[]>([]);
  const [logsLoading, setLogsLoading] = useState(false);

  const openLogs = async (instance: JobInstanceSummary) => {
    setSelectedInstance(instance);
    setLogDrawerOpen(true);
    setLogsLoading(true);
    try {
      const [logPage, attemptPage] = await Promise.all([
        listInstanceLogs(instance.id),
        listInstanceAttempts(instance.id),
      ]);
      setLogs(logPage.items);
      setAttempts(attemptPage.items);
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : '日志加载失败');
    } finally {
      setLogsLoading(false);
    }
  };

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

  const columns: ColumnsType<JobInstanceSummary> = [
    { title: 'Instance', dataIndex: 'id', ellipsis: true, width: 140 },
    { title: 'Job', dataIndex: 'job_id', render: (value: string) => <strong>{jobName.get(value) ?? value}</strong> },
    { title: 'Status', dataIndex: 'status', render: (value: string) => <Tag color={getStatusColor(value)} className="instance-status-tag">{value}</Tag> },
    { title: 'Trigger', dataIndex: 'trigger_type', render: (value: string) => <Tag>{value}</Tag> },
    { title: 'Mode', dataIndex: 'execution_mode', render: (value: string) => <Tag color={value === 'broadcast' ? 'purple' : 'default'} className="soft-tag">{value}</Tag> },
    { title: 'Created At', dataIndex: 'created_at', width: 180 },
    {
      title: 'Logs',
      width: 100,
      render: (_, instance) => <Button type="link" onClick={() => void openLogs(instance)}>查看日志</Button>,
    },
  ];

  const attemptColumns: ColumnsType<JobInstanceAttemptSummary> = [
    { title: 'Worker', dataIndex: 'worker_id', ellipsis: true },
    { title: 'Status', dataIndex: 'status', render: (value: string) => <Tag color={getStatusColor(value)} className="instance-status-tag">{value}</Tag> },
    { title: 'Updated At', dataIndex: 'updated_at', width: 180 },
  ];


  const governanceLogs = logs.filter((log) => log.governance_event === 'script_execution_governance');

  const renderLogMessage = (log: JobInstanceLogSummary) => {
    if (log.governance_event !== 'script_execution_governance') {
      return log.message;
    }
    return (
      <Space direction="vertical" size={2}>
        <Space wrap>
          <Tag color="volcano">script governance</Tag>
          {log.governance_failure_class ? <Tag color="red">{log.governance_failure_class}</Tag> : null}
        </Space>
        <Typography.Text>{log.governance_message ?? log.message}</Typography.Text>
      </Space>
    );
  };

  const logColumns: ColumnsType<JobInstanceLogSummary> = [
    { title: '#', dataIndex: 'sequence', width: 60 },
    { title: 'Level', dataIndex: 'level', width: 90, render: (value: string) => <Tag color={value === 'error' ? 'red' : value === 'warn' ? 'orange' : 'blue'}>{value}</Tag> },
    { title: 'Worker', dataIndex: 'worker_id', ellipsis: true, width: 120 },
    { title: 'Message', dataIndex: 'message', render: (_: string, log) => renderLogMessage(log) },
  ];

  return (
    <Card className="clean-card" title="执行实例">
      {instances.length === 0 ? (
        <Empty description="还没有实例，请先在 Jobs 页面创建并触发任务" />
      ) : (
        <>
          <Typography.Paragraph type="secondary">实例详情 API 已可用：GET /api/v1/instances/&lt;instance&gt;</Typography.Paragraph>
          <Table rowKey="id" columns={columns} dataSource={instances} pagination={{ pageSize: 8 }} />
        </>
      )}
      <Drawer
        width={760}
        title={selectedInstance ? `实例日志： ${selectedInstance.id}` : '实例日志'}
        open={logDrawerOpen}
        onClose={() => setLogDrawerOpen(false)}
      >
        <Typography.Title level={5}>广播子执行</Typography.Title>
        <Table
          rowKey="id"
          columns={attemptColumns}
          dataSource={attempts}
          pagination={false}
          locale={{ emptyText: '非广播实例或暂无子执行' }}
        />
        <Typography.Title level={5} style={{ marginTop: 24 }}>执行日志</Typography.Title>
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
          pagination={{ pageSize: 10 }}
          locale={{ emptyText: '暂无日志' }}
        />
      </Drawer>
    </Card>
  );
}
