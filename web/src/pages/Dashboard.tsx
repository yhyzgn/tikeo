import { Card, Col, Row, Statistic, Typography } from 'antd';

import type { JobInstanceSummary, JobSummary } from '../api/client';

export interface DashboardProps {
  jobs: JobSummary[];
  instances: JobInstanceSummary[];
}

export function Dashboard({ jobs, instances }: DashboardProps) {
  const enabledJobs = jobs.filter((job) => job.enabled).length;
  const pendingInstances = instances.filter((instance) => instance.status === 'pending').length;

  return (
    <div className="page-stack">
      <Typography.Title level={2}>Dashboard</Typography.Title>
      <Row gutter={[16, 16]}>
        <Col xs={24} md={8}>
          <Card><Statistic title="Jobs" value={jobs.length} /></Card>
        </Col>
        <Col xs={24} md={8}>
          <Card><Statistic title="Enabled Jobs" value={enabledJobs} /></Card>
        </Col>
        <Col xs={24} md={8}>
          <Card><Statistic title="Pending Instances" value={pendingInstances} /></Card>
        </Col>
      </Row>
      <Card title="当前阶段说明">
        后端已支持 Job 创建、API 手动触发和实例查询；Worker 真实任务派发与 CRON / Fixed Rate tick loop 后续接入。
      </Card>
    </div>
  );
}
