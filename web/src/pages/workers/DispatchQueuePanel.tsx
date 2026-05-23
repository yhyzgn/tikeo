import { Card, Empty, List, Segmented, Space, Tag, Typography } from 'antd';
import { useMemo, useState } from 'react';

import type { DispatchQueueSummary, QueueOverview } from '../../api/client';
import { filterQueueItems, queueStatusColor, type QueueStatusFilter } from './workerPageModel';

interface DispatchQueuePanelProps {
  queue: QueueOverview;
  loading: boolean;
}

const STATUS_OPTIONS: { label: string; value: QueueStatusFilter }[] = [
  { label: '全部', value: 'all' },
  { label: 'Pending', value: 'pending' },
  { label: 'Running', value: 'running' },
  { label: 'Done', value: 'done' },
  { label: 'Failed', value: 'failed' },
];

export function DispatchQueuePanel({ queue, loading }: DispatchQueuePanelProps) {
  const [status, setStatus] = useState<QueueStatusFilter>('all');
  const items = useMemo(() => filterQueueItems(queue.items, status), [queue.items, status]);

  return (
    <Card
      className="worker-ops-card dispatch-queue-card"
      title={<Space direction="vertical" size={0}><span>Dispatch Queue</span><Typography.Text type="secondary">按状态聚焦积压、租约中任务和失败项</Typography.Text></Space>}
      extra={<Tag color="purple">{items.length} items</Tag>}
    >
      <Segmented className="dispatch-queue-filter" value={status} onChange={(value) => setStatus(value as QueueStatusFilter)} options={STATUS_OPTIONS} />
      {items.length === 0 ? (
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={loading ? '正在加载队列' : '当前筛选下没有队列项'} />
      ) : (
        <List
          className="dispatch-queue-list"
          dataSource={items}
          renderItem={(item) => <DispatchQueueItem item={item} />}
        />
      )}
    </Card>
  );
}

function DispatchQueueItem({ item }: { item: DispatchQueueSummary }) {
  return (
    <List.Item className="dispatch-queue-item">
      <Space direction="vertical" size={8} style={{ width: '100%' }}>
        <Space wrap align="center">
          <Typography.Text strong copyable>{item.id}</Typography.Text>
          <Tag color={queueStatusColor(item.status)}>{item.status}</Tag>
          <Tag>attempt={item.attempt}</Tag>
          <Tag color="blue">priority={item.priority}</Tag>
        </Space>
        <div className="dispatch-queue-item__meta">
          <span>job={item.job_instance_id ?? '-'}</span>
          <span>workflow_node={item.workflow_node_instance_id ?? '-'}</span>
          <span>selector={item.worker_selector ?? 'any'}</span>
          <span>run_after={new Date(item.run_after).toLocaleString()}</span>
        </div>
      </Space>
    </List.Item>
  );
}
