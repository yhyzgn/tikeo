import { ReloadOutlined } from '@ant-design/icons';
import { Button, Card, Statistic, Tag, Typography } from 'antd';

import type { WorkerListResponse } from '../../api/client';
import type { ReactNode } from 'react';

interface WorkerClusterOverviewProps {
  workers: WorkerListResponse;
  loading: boolean;
  onRefresh: () => void;
  extraAction?: ReactNode;
}

export function WorkerClusterOverview({ workers, loading, onRefresh, extraAction }: WorkerClusterOverviewProps) {
  const scopeCount = new Set(workers.items.map((worker) => `${worker.namespace}/${worker.app}`)).size;
  const clusterCount = new Set(workers.items.map((worker) => `${worker.namespace}/${worker.app}/${worker.cluster}/${worker.region}`)).size;
  const masterCount = workers.items.filter((worker) => worker.master?.isMaster).length;

  return (
    <section className="hero-panel worker-cluster-hero">
      <div className="hero-panel__content">
        <div className="hero-panel__header">
          <Tag className="soft-tag" color="blue">Phase 3 · Worker Ops</Tag>
          <Typography.Title level={1}>Worker 集群</Typography.Title>
        </div>
        <Typography.Paragraph className="hero-panel__desc">
          面向运维的 Worker Mesh 拓扑视图：按命名空间、应用和集群组织在线节点，让应用、集群、主节点和从节点一目了然。
        </Typography.Paragraph>
        <div className="worker-cluster-hero__actions">
          <Button type="primary" icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>刷新集群状态</Button>
          {extraAction}
        </div>
      </div>
      <div className="worker-cluster-hero__summary-grid">
        <Card className="worker-mini-stat"><Statistic title="在线节点" value={workers.online} /></Card>
        <Card className="worker-mini-stat"><Statistic title="应用范围" value={scopeCount} /></Card>
        <Card className="worker-mini-stat"><Statistic title="集群" value={clusterCount} /></Card>
        <Card className="worker-mini-stat"><Statistic title="主节点" value={masterCount} /></Card>
      </div>
    </section>
  );
}
