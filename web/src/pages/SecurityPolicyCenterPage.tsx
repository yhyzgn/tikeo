import { Alert, Card, Col, Descriptions, Row, Space, Statistic, Table, Tag, Typography } from 'antd';
import { useEffect, useState } from 'react';

import { getSecurityPosture, type SecurityPolicyDenial, type SecurityPostureCheck, type SecurityPostureResponse } from '../api/security';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

const statusColor: Record<string, string> = {
  ok: 'green',
  warning: 'gold',
  critical: 'red',
};

const statusText: Record<string, string> = {
  ok: '正常',
  warning: '需关注',
  critical: '高风险',
};

function renderStatus(status: string) {
  return <Tag color={statusColor[status] ?? 'default'}>{statusText[status] ?? status}</Tag>;
}

export function SecurityPolicyCenterPage() {
  const active = useRouteActive(ROUTE_META.security.path);
  const [posture, setPosture] = useState<SecurityPostureResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    if (!active) return undefined;
    void getSecurityPosture()
      .then((data) => {
        if (!mounted) return;
        setPosture(data);
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

  const script = posture?.scriptGovernance;
  const notifications = posture?.notificationSafety;
  const cluster = posture?.clusterTransport;

  return (
    <div className="page-stack security-policy-center">
      <section className="hero-panel security-policy-center__hero">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color={statusColor[posture?.overallStatus ?? 'ok'] ?? 'default'} className="soft-tag">
              Security Policy Center
            </Tag>
            <Typography.Title level={1}>安全策略中心</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            汇总脚本默认拒绝策略、发布签名门禁、通知目标脱敏、TLS/mTLS 与 Raft 内部传输令牌等已落地的真实安全控制点。
            当前页面只展示由服务端配置、数据库策略快照和审计日志推导出的证据，不展示占位数据。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary">
          <strong>{posture ? statusText[posture.overallStatus] ?? posture.overallStatus : '-'}</strong>
          <span>overall posture</span>
        </div>
      </section>

      {error ? <Alert type="error" showIcon message="安全策略态势加载失败" description={<span data-runtime-text>{error}</span>} /> : null}

      <Row gutter={[16, 16]}>
        <Col xs={12} lg={6}><Card><Statistic title="脚本总数" value={script?.totalScripts ?? 0} /></Card></Col>
        <Col xs={12} lg={6}><Card><Statistic title="默认拒绝脚本" value={script?.safeDefaultDenyScripts ?? 0} valueStyle={{ color: '#389e0d' }} /></Card></Col>
        <Col xs={12} lg={6}><Card><Statistic title="危险策略快照" value={script?.dangerousPolicyScripts ?? 0} valueStyle={{ color: script?.dangerousPolicyScripts ? '#cf1322' : '#389e0d' }} /></Card></Col>
        <Col xs={12} lg={6}><Card><Statistic title="已签名发布" value={script?.signedReleases ?? 0} /></Card></Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={14}>
          <Card title="策略检查">
            <Table<SecurityPostureCheck>
              rowKey="id"
              dataSource={posture?.checks ?? []}
              pagination={false}
              columns={[
                { title: '检查项', dataIndex: 'label', width: 220 },
                { title: '状态', dataIndex: 'status', width: 96, render: renderStatus },
                { title: '来源', dataIndex: 'source', width: 170, render: (value: string) => <Tag>{value}</Tag> },
                { title: '证据数', dataIndex: 'evidenceCount', width: 96 },
                { title: '详情', dataIndex: 'detail', render: (value: string) => <span data-runtime-text>{value}</span> },
              ]}
            />
          </Card>
        </Col>
        <Col xs={24} xl={10}>
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Card title="部署与传输前置条件">
              <Descriptions column={1} size="small">
                <Descriptions.Item label="HTTP Listener">{posture?.transport.http.listenerMode ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="Worker Tunnel">{posture?.transport.workerTunnel.listenerMode ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="Raft Token">{cluster?.raftTransportTokenConfigured ? <Tag color="green">已配置</Tag> : <Tag color="gold">未配置</Tag>}</Descriptions.Item>
                <Descriptions.Item label="Worker Tunnel TLS Ready">{cluster?.workerTunnelTlsReady ? <Tag color="green">ready</Tag> : <Tag>not-ready</Tag>}</Descriptions.Item>
              </Descriptions>
            </Card>
            <Card title="通知安全">
              <Descriptions column={1} size="small">
                <Descriptions.Item label="渠道总数">{notifications?.totalChannels ?? 0}</Descriptions.Item>
                <Descriptions.Item label="启用渠道">{notifications?.enabledChannels ?? 0}</Descriptions.Item>
                <Descriptions.Item label="已配置目标">{notifications?.configuredTargets ?? 0}</Descriptions.Item>
                <Descriptions.Item label="脱敏目标">{notifications?.redactedTargets ?? 0}</Descriptions.Item>
                <Descriptions.Item label="安全策略 JSON">{notifications?.channelsWithSafetyPolicy ?? 0}</Descriptions.Item>
              </Descriptions>
            </Card>
          </Space>
        </Col>
      </Row>

      <Card title="最近策略拒绝 / 发布门禁审计">
        <Table<SecurityPolicyDenial>
          rowKey="id"
          dataSource={posture?.recentDenials ?? []}
          pagination={false}
          columns={[
            { title: '资源', render: (_, row) => <span data-runtime-text>{row.resourceType}/{row.resourceId}</span> },
            { title: '动作', dataIndex: 'action', width: 140 },
            { title: '失败原因', dataIndex: 'failureReason', width: 260, render: (value: string) => <Tag color="red">{value}</Tag> },
            { title: '详情', dataIndex: 'detail', render: (value: string | null) => <span data-runtime-text>{value ?? '-'}</span> },
            { title: '时间', dataIndex: 'createdAt', width: 220 },
          ]}
        />
      </Card>
    </div>
  );
}
