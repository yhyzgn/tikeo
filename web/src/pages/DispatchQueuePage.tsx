import { ReloadOutlined } from '@ant-design/icons';
import { Button, Card, Col, Row, Statistic, Tag, Typography, message } from 'antd';
import { useCallback, useEffect, useState } from 'react';

import { getDispatchQueue, type QueueOverview } from '../api/client';
import { DispatchQueuePanel } from './workers/DispatchQueuePanel';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';
import { queueHealth } from './workers/workerPageModel';

export function DispatchQueuePage() {
  const [queue, setQueue] = useState<QueueOverview>({ pending: 0, running: 0, done: 0, failed: 0, items: [] });
  const [loading, setLoading] = useState(false);
  const active = useRouteActive(ROUTE_META.dispatchQueue.path);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setQueue(await getDispatchQueue());
    } catch (error) {
      message.error(error instanceof Error ? error.message : '加载调度队列失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { if (active) void refresh(); }, [active, refresh]);

  const health = queueHealth(queue);

  return (
    <div className="page-stack dispatch-queue-page">
      <section className="hero-panel dispatch-queue-hero">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag className="soft-tag" color="purple">Worker Ops · Dispatch</Tag>
            <Typography.Title level={1}>调度队列</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            独立查看待分发、运行中、完成和失败的调度队列项；Worker 集群页只展示执行节点拓扑。
          </Typography.Paragraph>
          <Button type="primary" icon={<ReloadOutlined />} loading={loading} onClick={refresh}>刷新调度队列</Button>
        </div>
        <Tag className={`worker-health worker-health--${health.tone}`}>{health.label}</Tag>
      </section>
      <Row gutter={[14, 14]}>
        <Col xs={12} md={6}><Card className="worker-stat-card"><Statistic title="待调度" value={queue.pending} /></Card></Col>
        <Col xs={12} md={6}><Card className="worker-stat-card"><Statistic title="运行中" value={queue.running} /></Card></Col>
        <Col xs={12} md={6}><Card className="worker-stat-card"><Statistic title="已完成" value={queue.done} /></Card></Col>
        <Col xs={12} md={6}><Card className="worker-stat-card worker-stat-card--danger"><Statistic title="失败" value={queue.failed} /></Card></Col>
      </Row>
      <DispatchQueuePanel queue={queue} loading={loading} />
    </div>
  );
}
