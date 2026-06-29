import { Alert, Card, Descriptions, Empty, Space, Spin, Table, Tag, Timeline, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';

import { getPublicJobInstanceTrace, type NotificationMessageTrace } from '../api/notifications';
import { useI18n } from '../i18n/I18nContext';

const levelColor: Record<string, string> = { error: 'red', warn: 'gold', warning: 'gold', info: 'blue', debug: 'default' };

export function PublicInstanceConsolePage() {
  const { id } = useParams();
  const { t } = useI18n();
  const [trace, setTrace] = useState<NotificationMessageTrace | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!id) {
      setError(t('缺少实例 ID'));
      setLoading(false);
      return;
    }
    setLoading(true);
    getPublicJobInstanceTrace(id)
      .then(setTrace)
      .catch((cause) => setError(cause instanceof Error ? cause.message : t('加载执行透传信息失败')))
      .finally(() => setLoading(false));
  }, [id, t]);

  if (loading) return <div className="public-console-page public-console-page--loading"><Spin description={t('正在加载执行控制台')} /></div>;
  if (error) return <div className="public-console-page"><Alert type="error" showIcon message={t('执行控制台加载失败')} description={<span data-runtime-text>{error}</span>} /></div>;
  if (!trace) return <div className="public-console-page"><Empty description={t('没有可展示的执行信息')} /></div>;

  return (
    <main className="public-console-page">
      <section className="public-console-page__hero">
        <Typography.Text className="public-console-page__eyebrow">{t('Tikeo 任务执行透传控制台')}</Typography.Text>
        <Typography.Title level={1} data-runtime-text>{trace.job?.name ?? trace.message.subject}</Typography.Title>
        <Typography.Paragraph data-runtime-text>{trace.message.body}</Typography.Paragraph>
        <Space wrap>
          <Tag color={trace.instance?.status === 'failed' ? 'red' : trace.instance?.status === 'succeeded' ? 'green' : 'blue'}>{trace.instance?.status ?? trace.message.eventType}</Tag>
          <Tag data-runtime-text>{trace.job?.namespace ?? '-'}/{trace.job?.app ?? '-'}</Tag>
          <Tag data-runtime-text>{trace.instance?.executionMode ?? '-'}</Tag>
        </Space>
      </section>

      <Card title={t('执行上下文')} className="public-console-page__card">
        <Descriptions column={{ xs: 1, sm: 2, md: 3 }} size="small">
          <Descriptions.Item label={t('实例 ID')}><span data-runtime-text>{trace.instance?.id ?? '-'}</span></Descriptions.Item>
          <Descriptions.Item label={t('任务 ID')}><span data-runtime-text>{trace.instance?.jobId ?? trace.message.resourceId}</span></Descriptions.Item>
          <Descriptions.Item label={t('触发类型')}><span data-runtime-text>{trace.instance?.triggerType ?? '-'}</span></Descriptions.Item>
          <Descriptions.Item label={t('Worker')}><span data-runtime-text>{trace.instance?.workerId ?? '-'}</span></Descriptions.Item>
          <Descriptions.Item label={t('开始时间')}>{trace.instance?.createdAt ?? '-'}</Descriptions.Item>
          <Descriptions.Item label={t('更新时间')}>{trace.instance?.updatedAt ?? '-'}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card title={t('投递记录')} className="public-console-page__card">
        <Table
          size="small"
          rowKey="id"
          pagination={false}
          dataSource={trace.attempts}
          columns={[
            { title: t('渠道'), dataIndex: 'provider' },
            { title: t('目标'), dataIndex: 'targetRedacted', render: (value: string) => <span data-runtime-text>{value}</span> },
            { title: t('状态'), dataIndex: 'retryState', render: (value) => <Tag>{String(value)}</Tag> },
            { title: 'HTTP', dataIndex: 'statusCode', render: (value) => value ?? '-' },
            { title: t('错误'), dataIndex: 'error', render: (value: string | null) => <span data-runtime-text>{value || '-'}</span> },
          ]}
        />
      </Card>

      <Card title={t('执行日志')} className="public-console-page__card">
        {trace.logs.excerpt.length ? (
          <Timeline items={trace.logs.excerpt.map((line) => ({
            color: levelColor[line.level.toLowerCase()] ?? 'gray',
            children: <div className="public-console-page__log" data-runtime-text><Tag>{line.level}</Tag><code>#{line.sequence}</code><span>{line.message}</span></div>,
          }))} />
        ) : <Empty description={t('暂无日志摘要')} />}
        {trace.logs.truncated ? <Alert type="info" showIcon message={t('日志仅展示最近 80 行，完整日志请在登录控制台内查看。')} /> : null}
      </Card>
    </main>
  );
}
