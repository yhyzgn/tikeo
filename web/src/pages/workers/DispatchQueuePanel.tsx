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
      title={<Space orientation="vertical" size={0}><span>Dispatch Queue</span><Typography.Text type="secondary">按状态聚焦积压、租约中任务和失败项</Typography.Text></Space>}
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
      <Space orientation="vertical" size={8} style={{ width: '100%' }}>
        <Space wrap align="center">
          <Typography.Text strong copyable data-runtime-text>{item.id}</Typography.Text>
          <Tag color={queueStatusColor(item.status)} data-runtime-text>{item.status}</Tag>
          <Tag data-runtime-text>attempt={item.attempt}</Tag>
          <Tag color="blue" data-runtime-text>priority={item.priority}</Tag>
        </Space>
        <div className="dispatch-queue-item__meta" data-runtime-text>
          <span>job={item.jobInstanceId ?? '-'}</span>
          <span>workflow_node={item.workflowNodeInstanceId ?? '-'}</span>
          <span>selector={item.workerSelector ?? 'any'}</span>
          <span>runAfter={new Date(item.runAfter).toLocaleString()}</span>
        </div>
      </Space>
    </List.Item>
  );
}
