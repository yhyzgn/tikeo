import { Alert, Button, Card, Descriptions, Drawer, Empty, Space, Table, Tabs, Tag, Typography } from 'antd';
import { useEffect, useState } from 'react';

import { getNotificationMessageTrace, type NotificationDeliveryAttemptSummary, type NotificationMessageSummary, type NotificationMessageTrace, type NotificationTraceLogLine } from '../../api/notifications';
import { useI18n } from '../../i18n/I18nContext';
import { formatJson } from './jsonUtils';

type Props = {
  message: NotificationMessageSummary | null;
  open: boolean;
  onClose: () => void;
};

const stateColor: Record<string, string> = { delivered: 'green', retry_pending: 'gold', dead_letter: 'red', retry_consumed: 'blue', pending: 'gold', failed: 'red' };

export function NotificationMessageDetailDrawer({ message, open, onClose }: Props) {
  const { t } = useI18n();
  const [trace, setTrace] = useState<NotificationMessageTrace | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!message || !open) return;
    setLoading(true);
    setError(null);
    getNotificationMessageTrace(message.id)
      .then(setTrace)
      .catch((err) => setError(err instanceof Error ? err.message : t('通知消息加载失败')))
      .finally(() => setLoading(false));
  }, [message, open, t]);

  return (
    <Drawer title={message ? `${t('通知消息详情')} · ${message.eventType}` : t('通知消息详情')} open={open} onClose={onClose} width={1040} destroyOnClose>
      {error ? <Alert type="error" showIcon message={<span data-runtime-text>{error}</span>} /> : null}
      <Tabs
        items={[
          {
            key: 'overview',
            label: t('概览'),
            children: trace ? <Overview trace={trace} t={t} /> : <Empty description={loading ? t('加载中') : t('暂无数据')} />,
          },
          {
            key: 'delivery',
            label: t('投递'),
            children: <Delivery attempts={trace?.attempts ?? []} t={t} />,
          },
          {
            key: 'logs',
            label: t('执行日志透传'),
            children: <Logs trace={trace} t={t} />,
          },
          {
            key: 'payload',
            label: 'Payload',
            children: <pre className="json-preview" data-runtime-text>{formatJson(trace?.message.payloadJson ?? '{}')}</pre>,
          },
        ]}
      />
    </Drawer>
  );
}

function Overview({ trace, t }: { trace: NotificationMessageTrace; t: (value: string) => string }) {
  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
      <Descriptions bordered size="small" column={1} items={[
        { key: 'message', label: 'messageId', children: <Typography.Text copyable code>{trace.message.id}</Typography.Text> },
        { key: 'status', label: t('状态'), children: <Tag color={stateColor[trace.message.status] ?? 'default'}>{trace.message.status}</Tag> },
        { key: 'event', label: t('事件'), children: trace.message.eventType },
        { key: 'resource', label: t('资源'), children: `${trace.message.resourceType}:${trace.message.resourceId}` },
        { key: 'subject', label: t('主题'), children: <span data-runtime-text>{trace.message.subject}</span> },
        { key: 'body', label: t('内容'), children: <span data-runtime-text>{trace.message.body}</span> },
        { key: 'job', label: 'Job', children: trace.job ? `${trace.job.namespace}/${trace.job.app}/${trace.job.name}` : '-' },
        { key: 'instance', label: 'Instance', children: trace.instance ? <Typography.Text copyable code>{trace.instance.id}</Typography.Text> : '-' },
        { key: 'policy', label: 'Policy', children: trace.policy ? <Typography.Text copyable code>{trace.policy.id}</Typography.Text> : '-' },
      ]} />
    </Space>
  );
}

function Delivery({ attempts, t }: { attempts: NotificationDeliveryAttemptSummary[]; t: (value: string) => string }) {
  if (!attempts.length) return <Empty description={t('暂无投递记录')} />;
  return (
    <Table<NotificationDeliveryAttemptSummary>
      rowKey="id"
      dataSource={attempts}
      pagination={false}
      columns={[
        { title: t('提供方'), dataIndex: 'provider' },
        { title: t('目标'), dataIndex: 'targetRedacted', ellipsis: true, render: (value: string) => <span data-runtime-text>{value}</span> },
        { title: t('尝试次数'), dataIndex: 'attempt', width: 96 },
        { title: t('状态'), dataIndex: 'retryState', render: (value: string) => <Tag color={stateColor[value] ?? 'default'}>{value}</Tag> },
        { title: 'HTTP', dataIndex: 'statusCode', width: 90, render: (value) => value ?? '-' },
        { title: t('错误'), dataIndex: 'error', ellipsis: true, render: (value) => <span data-runtime-text>{value ?? '-'}</span> },
        { title: t('创建时间'), dataIndex: 'createdAt' },
      ]}
    />
  );
}

function Logs({ trace, t }: { trace: NotificationMessageTrace | null; t: (value: string) => string }) {
  if (!trace?.instance) return <Empty description={t('该消息没有关联执行实例')} />;
  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Alert type="info" showIcon message={trace.logs.url ? `${t('完整日志')}: ${trace.logs.url}` : t('该消息没有关联执行实例')} action={trace.logs.url ? <Button size="small" href={trace.logs.url}>{t('打开')}</Button> : null} />
      <Card size="small" title={trace.logs.truncated ? t('最近日志摘要（已截断）') : t('执行日志摘要')}>
        {trace.logs.excerpt.length ? (
          <Table<NotificationTraceLogLine>
            rowKey={(row) => `${row.workerId}-${row.sequence}`}
            dataSource={trace.logs.excerpt}
            pagination={false}
            size="small"
            columns={[
              { title: '#', dataIndex: 'sequence', width: 80 },
              { title: t('级别'), dataIndex: 'level', width: 100, render: (value) => <Tag>{value}</Tag> },
              { title: 'Worker', dataIndex: 'workerId', width: 180, ellipsis: true },
              { title: t('日志'), dataIndex: 'message', render: (value: string) => <span data-runtime-text>{value}</span> },
              { title: t('创建时间'), dataIndex: 'createdAt', width: 210 },
            ]}
          />
        ) : <Empty description={t('暂无日志')} />}
      </Card>
    </Space>
  );
}
