import { Button, Card, Drawer, Empty, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useState } from 'react';

import { listInstanceLogs, type JobInstanceLogSummary, type JobInstanceSummary, type JobSummary } from '../api/client';

export interface InstancesPageProps {
  jobs: JobSummary[];
  instances: JobInstanceSummary[];
}

export function InstancesPage({ jobs, instances }: InstancesPageProps) {
  const jobName = new Map(jobs.map((job) => [job.id, job.name]));
  const [logDrawerOpen, setLogDrawerOpen] = useState(false);
  const [selectedInstance, setSelectedInstance] = useState<JobInstanceSummary | null>(null);
  const [logs, setLogs] = useState<JobInstanceLogSummary[]>([]);
  const [logsLoading, setLogsLoading] = useState(false);

  const openLogs = async (instance: JobInstanceSummary) => {
    setSelectedInstance(instance);
    setLogDrawerOpen(true);
    setLogsLoading(true);
    try {
      const page = await listInstanceLogs(instance.id);
      setLogs(page.items);
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : '日志加载失败');
    } finally {
      setLogsLoading(false);
    }
  };

  const columns: ColumnsType<JobInstanceSummary> = [
    { title: 'Instance', dataIndex: 'id', ellipsis: true },
    { title: 'Job', dataIndex: 'job_id', render: (value: string) => jobName.get(value) ?? value },
    { title: 'Status', dataIndex: 'status', render: (value: string) => <Tag color={value === 'pending' ? 'gold' : 'blue'}>{value}</Tag> },
    { title: 'Trigger', dataIndex: 'trigger_type' },
    { title: 'Created At', dataIndex: 'created_at' },
    {
      title: 'Logs',
      render: (_, instance) => <Button type="link" onClick={() => void openLogs(instance)}>View Logs</Button>,
    },
  ];

  const logColumns: ColumnsType<JobInstanceLogSummary> = [
    { title: '#', dataIndex: 'sequence', width: 80 },
    { title: 'Level', dataIndex: 'level', width: 100, render: (value: string) => <Tag>{value}</Tag> },
    { title: 'Worker', dataIndex: 'worker_id', ellipsis: true },
    { title: 'Message', dataIndex: 'message' },
    { title: 'Created At', dataIndex: 'created_at' },
  ];

  return (
    <Card title="Instances">
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
        title={selectedInstance ? `Instance Logs: ${selectedInstance.id}` : 'Instance Logs'}
        open={logDrawerOpen}
        onClose={() => setLogDrawerOpen(false)}
      >
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
