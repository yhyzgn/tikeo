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
                <Typography.Text strong copyable data-runtime-text>{session.workerId}</Typography.Text>
                <Tag color={sessionStatusColor(session.status)} data-runtime-text>{session.status}</Tag>
                <Tag data-runtime-text>gen {session.generation}</Tag>
              </Space>
              <Typography.Text type="secondary" data-runtime-text>{session.logicalInstanceId}</Typography.Text>
              <Typography.Text type="secondary" data-runtime-text>reason={session.statusReason ?? '-'} · seq={session.lastSequence}</Typography.Text>
              {session.statusEvidence ? <Typography.Text type="secondary" data-runtime-text>{session.statusEvidence}</Typography.Text> : null}
            </Space>
          </List.Item>
        )}
      />
      <Timeline
        className="worker-event-timeline"
        items={history.events.slice(0, 8).map((event) => ({
          color: sessionStatusColor(event.reason ?? event.eventType),
          children: (
            <Space direction="vertical" size={0}>
              <Typography.Text strong data-runtime-text>{event.eventType}</Typography.Text>
              <Typography.Text type="secondary" data-runtime-text>{event.workerId} · {event.reason ?? 'no reason'}</Typography.Text>
              <Typography.Text type="secondary">{event.createdAt}</Typography.Text>
            </Space>
          ),
        }))}
      />
    </Card>
  );
}
