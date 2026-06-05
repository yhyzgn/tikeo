import { ApiOutlined, ClockCircleOutlined, DeploymentUnitOutlined, ThunderboltOutlined } from '@ant-design/icons';
import { Card, Col, Row, Statistic, Tag, Typography } from 'antd';
import { useCallback, useEffect, useState } from 'react';

import { listJobInstances, listJobs, type JobInstanceSummary, type JobSummary } from '../api/client';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

export function Dashboard() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);
  const active = useRouteActive(ROUTE_META.dashboard.path);

  const load = useCallback(async () => {
    try {
      const jobPage = await listJobs();
      setJobs(jobPage.items);
      const instancePages = await Promise.all(jobPage.items.map((job) => listJobInstances(job.id)));
      setInstances(instancePages.flatMap((page) => page.items));
    } catch { /* silent */ }
  }, []);

  useEffect(() => { if (active) void load(); }, [active, load]);

  const enabledJobs = jobs.filter((job) => job.enabled).length;
  const pendingInstances = instances.filter((instance) => instance.status === 'pending').length;
  const broadcastInstances = instances.filter((instance) => instance.executionMode === 'broadcast').length;

  return (
    <div className="page-stack">
      <section className="hero-panel">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color="blue" className="soft-tag">MVP Console</Tag>
            <Typography.Title level={1}>任务调度中枢</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            用统一控制台管理任务、触发执行、查看实例与日志。当前菜单只开放已实现能力，规划中模块暂以禁用项展示。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary">
          <strong>{jobs.length}</strong>
          <span>total jobs</span>
        </div>
      </section>

      <Row gutter={[20, 20]}>
        <Col xs={24} sm={12} xl={6}>
          <Card className="metric-card"><Statistic prefix={<ThunderboltOutlined />} title="任务总数" value={jobs.length} /></Card>
        </Col>
        <Col xs={24} sm={12} xl={6}>
          <Card className="metric-card"><Statistic prefix={<ApiOutlined />} title="启用任务" value={enabledJobs} /></Card>
        </Col>
        <Col xs={24} sm={12} xl={6}>
          <Card className="metric-card"><Statistic prefix={<ClockCircleOutlined />} title="等待实例" value={pendingInstances} /></Card>
        </Col>
        <Col xs={24} sm={12} xl={6}>
          <Card className="metric-card"><Statistic prefix={<DeploymentUnitOutlined />} title="广播实例" value={broadcastInstances} /></Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={14}>
          <Card className="clean-card" title="当前能力">
            <div className="capability-list">
              <span>任务创建</span>
              <span>API 手动触发</span>
              <span>单机 / 广播执行</span>
              <span>实例与日志查看</span>
              <span>开发认证</span>
            </div>
          </Card>
        </Col>
        <Col xs={24} lg={10}>
          <Card className="clean-card" title="菜单说明">
            Worker 集群、安全策略、审计日志等菜单是平台后续能力入口。因为对应功能尚未完成，目前先禁用，避免误导操作。
          </Card>
        </Col>
      </Row>
    </div>
  );
}
