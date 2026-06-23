import { Alert, Button, Card, Col, Descriptions, Row, Space, Statistic, Table, Tag, Typography } from 'antd';
import { useCallback, useEffect, useState } from 'react';

import { Link } from 'react-router-dom';

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
  const [loading, setLoading] = useState(false);

  const loadPosture = useCallback(async (mounted: () => boolean = () => true) => {
    setLoading(true);
    try {
      const data = await getSecurityPosture();
      if (!mounted()) return;
      setPosture(data);
      setError(null);
    } catch (cause: unknown) {
      if (!mounted()) return;
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (mounted()) setLoading(false);
    }
  }, []);

  useEffect(() => {
    let mounted = true;
    if (!active) return undefined;
    void loadPosture(() => mounted);
    return () => {
      mounted = false;
    };
  }, [active, loadPosture]);

  const script = posture?.scriptGovernance;
  const notifications = posture?.notificationSafety;
  const cluster = posture?.clusterTransport;
  const transport = posture?.transport;
  const httpTransport = transport?.http;
  const workerTunnelTransport = transport?.workerTunnel;

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
            这里不是单纯看板：发现风险后可直接跳转到脚本、通知、RBAC、API-Key 等治理模块处理；服务端传输安全仍需通过部署配置变更完成。
          </Typography.Paragraph>
          <Space wrap>
            <Button type="primary" loading={loading} onClick={() => void loadPosture()}>刷新态势</Button>
            <Button><Link to={ROUTE_META.scripts.path}>治理脚本策略</Link></Button>
            <Button><Link to={ROUTE_META.notifications.path}>治理通知渠道</Link></Button>
            <Button><Link to={ROUTE_META.roles.path}>治理角色权限</Link></Button>
          </Space>
        </div>
        <div className="hero-panel__summary">
          <strong>{posture ? statusText[posture.overallStatus] ?? posture.overallStatus : '-'}</strong>
          <span>overall posture</span>
        </div>
      </section>

      {error ? <Alert type="error" showIcon message="安全策略态势加载失败" description={<span data-runtime-text>{error}</span>} /> : null}

      <Card title="可操作治理入口">
        <Row gutter={[16, 16]}>
          <Col xs={24} md={12} xl={6}>
            <Card size="small" title="脚本安全">
              <Typography.Paragraph type="secondary">处理默认拒绝、发布签名、脚本权限和运行策略。</Typography.Paragraph>
              <Button><Link to={ROUTE_META.scripts.path}>进入脚本管理</Link></Button>
            </Card>
          </Col>
          <Col xs={24} md={12} xl={6}>
            <Card size="small" title="通知安全">
              <Typography.Paragraph type="secondary">维护渠道目标、敏感配置、模板和投递策略。</Typography.Paragraph>
              <Button><Link to={ROUTE_META.notifications.path}>进入通知中心</Link></Button>
            </Card>
          </Col>
          <Col xs={24} md={12} xl={6}>
            <Card size="small" title="访问控制">
              <Typography.Paragraph type="secondary">调整角色、菜单权限、UI 操作权限和安全策略授权。</Typography.Paragraph>
              <Button><Link to={ROUTE_META.roles.path}>进入角色管理</Link></Button>
            </Card>
          </Col>
          <Col xs={24} md={12} xl={6}>
            <Card size="small" title="API 凭据">
              <Typography.Paragraph type="secondary">治理 API-Key、租户应用访问边界和调用凭据。</Typography.Paragraph>
              <Button><Link to={ROUTE_META.apiKeys.path}>进入 API-Key</Link></Button>
            </Card>
          </Col>
        </Row>
      </Card>

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
              loading={loading}
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
                <Descriptions.Item label="HTTP Listener">{httpTransport?.listenerMode ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="Worker Tunnel">{workerTunnelTransport?.listenerMode ?? '-'}</Descriptions.Item>
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
