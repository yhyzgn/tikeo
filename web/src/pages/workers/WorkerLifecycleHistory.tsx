import { Card, List, Segmented, Space, Tag, Timeline, Typography } from 'antd';
import { useMemo, useState } from 'react';

import type { WorkerLifecycleHistoryResponse, WorkerSessionHistorySummary } from '../../api/client';
import { groupWorkerSessionsByLayer, sessionStatusColor } from './workerPageModel';

interface WorkerLifecycleHistoryProps {
  history: WorkerLifecycleHistoryResponse;
  loading: boolean;
}

const layerLabels = {
  active: '在线',
  degraded: '异常/待确认',
  history: '历史',
} as const;

export function WorkerLifecycleHistory({ history, loading }: WorkerLifecycleHistoryProps) {
  const [layer, setLayer] = useState<keyof typeof layerLabels>('active');
  const grouped = useMemo(() => groupWorkerSessionsByLayer(history.sessions), [history.sessions]);
  const sessions = grouped[layer];

  return (
    <Card
      className="worker-ops-card worker-history-card"
      title={<Space direction="vertical" size={0}><span>Worker 生命周期</span><Typography.Text type="secondary">按在线、异常与历史分层排查 session 代际</Typography.Text></Space>}
      extra={<Tag color="purple">events {history.events.length}</Tag>}
    >
      <Segmented
        className="worker-history-layer-switch"
        value={layer}
        onChange={(value) => setLayer(value as keyof typeof layerLabels)}
        options={(Object.keys(layerLabels) as Array<keyof typeof layerLabels>).map((key) => ({
          label: `${layerLabels[key]} ${grouped[key].length}`,
          value: key,
        }))}
      />
      <List<WorkerSessionHistorySummary>
        className="worker-history-list"
        loading={loading}
        dataSource={sessions}
        locale={{ emptyText: '暂无该分层 Worker session' }}
        renderItem={(session) => (
          <List.Item>
            <Space direction="vertical" size={4} className="worker-history-list__item">
              <Space wrap>
                <Typography.Text strong copyable>{session.worker_id}</Typography.Text>
                <Tag color={sessionStatusColor(session.status)}>{session.status}</Tag>
                <Tag>gen {session.generation}</Tag>
              </Space>
              <Typography.Text type="secondary">{session.logical_instance_id}</Typography.Text>
              <Typography.Text type="secondary">reason={session.status_reason ?? '-'} · seq={session.last_sequence}</Typography.Text>
              {session.status_evidence ? <Typography.Text type="secondary">{session.status_evidence}</Typography.Text> : null}
            </Space>
          </List.Item>
        )}
      />
      <Timeline
        className="worker-event-timeline"
        items={history.events.slice(0, 8).map((event) => ({
          color: sessionStatusColor(event.reason ?? event.event_type),
          children: (
            <Space direction="vertical" size={0}>
              <Typography.Text strong>{event.event_type}</Typography.Text>
              <Typography.Text type="secondary">{event.worker_id} · {event.reason ?? 'no reason'}</Typography.Text>
              <Typography.Text type="secondary">{event.created_at}</Typography.Text>
            </Space>
          ),
        }))}
      />
    </Card>
  );
}
