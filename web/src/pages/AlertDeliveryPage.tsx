import { Alert, Card, Col, Row, Space, Statistic, Table, Tag, Typography } from 'antd';
import { useEffect, useState } from 'react';

import { getAlertDeliveryQueueStatus, type AlertDeliveryQueueStatus, type AlertDeliveryAttemptSummary } from '../api/client';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

const stateColor: Record<string, string> = {
  delivered: 'green',
  retry_pending: 'gold',
  dead_letter: 'red',
  retry_consumed: 'blue',
};

export function AlertDeliveryPage() {
  const [status, setStatus] = useState<AlertDeliveryQueueStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const active = useRouteActive(ROUTE_META.alerts.path);

  useEffect(() => {
    let mounted = true;
    if (!active) return undefined;
    void getAlertDeliveryQueueStatus()
      .then((data) => {
        if (!mounted) return;
        setStatus(data);
        setError(null);
      })
      .catch((cause: unknown) => {
        if (!mounted) return;
        setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      mounted = false;
    };
  }, [active]);

  return (
    <Space orientation="vertical" size={20} style={{ width: '100%' }}>
      <div>
        <Typography.Title level={2}>告警投递</Typography.Title>
        <Typography.Text type="secondary">查看生产告警投递 retry / DLQ 状态，Provider target 已脱敏。</Typography.Text>
      </div>
      {error ? <Alert type="error" showIcon message="告警投递状态加载失败" description={<span data-runtime-text>{error}</span>} /> : null}
      <Row gutter={[16, 16]}>
        <Col xs={12} md={6}><Card><Statistic title="总尝试" value={status?.total_attempts ?? 0} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title="待重试" value={status?.retry_pending ?? 0} styles={{ content: { color: '#d48806' } }} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title="DLQ" value={status?.dead_letter ?? 0} styles={{ content: { color: '#cf1322' } }} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title="已投递" value={status?.delivered ?? 0} styles={{ content: { color: '#389e0d' } }} /></Card></Col>
      </Row>
      <Card title="最近 DLQ">
        <Table<AlertDeliveryAttemptSummary>
          rowKey="id"
          dataSource={status?.recent_dead_letters ?? []}
          pagination={false}
          columns={[
            { title: 'Provider', dataIndex: 'provider' },
            { title: 'Target', dataIndex: 'target', render: (value: string) => <span data-runtime-text>{value}</span> },
            { title: 'Attempt', dataIndex: 'attempt', width: 96 },
            { title: 'State', dataIndex: 'retry_state', render: (value: string) => <Tag color={stateColor[value] ?? 'default'}>{value}</Tag> },
            { title: 'Error', dataIndex: 'error', ellipsis: true, render: (value: string | null) => <span data-runtime-text>{value ?? '-'}</span> },
            { title: 'Created', dataIndex: 'createdAt' },
          ]}
        />
      </Card>
    </Space>
  );
}
