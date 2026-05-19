import { Card, Empty, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';

import type { JobInstanceSummary, JobSummary } from '../api/client';

export interface InstancesPageProps {
  jobs: JobSummary[];
  instances: JobInstanceSummary[];
}

export function InstancesPage({ jobs, instances }: InstancesPageProps) {
  const jobName = new Map(jobs.map((job) => [job.id, job.name]));
  const columns: ColumnsType<JobInstanceSummary> = [
    { title: 'Instance', dataIndex: 'id', ellipsis: true },
    { title: 'Job', dataIndex: 'job_id', render: (value: string) => jobName.get(value) ?? value },
    { title: 'Status', dataIndex: 'status', render: (value: string) => <Tag color={value === 'pending' ? 'gold' : 'blue'}>{value}</Tag> },
    { title: 'Trigger', dataIndex: 'trigger_type' },
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
    </Card>
  );
}
