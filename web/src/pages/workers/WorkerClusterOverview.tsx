import { ReloadOutlined } from '@ant-design/icons';
import { Button, Card, Col, Progress, Row, Statistic, Tag, Typography } from 'antd';

import type { QueueOverview, WorkerListResponse } from '../../api/client';
import { queueHealth } from './workerPageModel';

interface WorkerClusterOverviewProps {
  workers: WorkerListResponse;
  queue: QueueOverview;
  loading: boolean;
  onRefresh: () => void;
}

export function WorkerClusterOverview({ workers, queue, loading, onRefresh }: WorkerClusterOverviewProps) {
  const health = queueHealth(queue);
  const totalQueue = queue.pending + queue.running + queue.done + queue.failed;
  const activeQueue = queue.pending + queue.running;
  const queuePressure = totalQueue === 0 ? 0 : Math.round((activeQueue / totalQueue) * 100);

  return (
    <section className="hero-panel worker-cluster-hero">
      <div className="hero-panel__content">
        <div className="hero-panel__header">
          <Tag className="soft-tag" color="blue">Phase 3 · Worker Ops</Tag>
          <Typography.Title level={1}>Worker 集群</Typography.Title>
        </div>
        <Typography.Paragraph className="hero-panel__desc">
          面向运维的 Worker Mesh 驾驶舱：快速判断在线容量、队列压力、Worker 能力覆盖和待处理 dispatch queue。
        </Typography.Paragraph>
        <div className="worker-cluster-hero__actions">
          <Button type="primary" icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>刷新集群状态</Button>
          <Tag className={`worker-health worker-health--${health.tone}`}>{health.label}</Tag>
        </div>
      </div>
      <div className="worker-cluster-hero__summary-grid">
        <Card className="worker-mini-stat"><Statistic title="Online Workers" value={workers.online} /></Card>
        <Card className="worker-mini-stat"><Statistic title="Active Queue" value={activeQueue} /></Card>
        <Card className="worker-mini-stat worker-mini-stat--wide">
          <Typography.Text type="secondary">Queue Pressure</Typography.Text>
          <Progress percent={queuePressure} size="small" status={health.tone === 'blocked' ? 'exception' : 'active'} />
        </Card>
      </div>
    </section>
  );
}

export function WorkerQueueStats({ queue }: { queue: QueueOverview }) {
  return (
    <Row gutter={[14, 14]}>
      <Col xs={12} md={6}><Card className="worker-stat-card"><Statistic title="Pending" value={queue.pending} /></Card></Col>
      <Col xs={12} md={6}><Card className="worker-stat-card"><Statistic title="Running" value={queue.running} /></Card></Col>
      <Col xs={12} md={6}><Card className="worker-stat-card"><Statistic title="Done" value={queue.done} /></Card></Col>
      <Col xs={12} md={6}><Card className="worker-stat-card worker-stat-card--danger"><Statistic title="Failed" value={queue.failed} /></Card></Col>
    </Row>
  );
}
