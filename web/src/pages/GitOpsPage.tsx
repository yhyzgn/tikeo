import { BranchesOutlined, CopyOutlined, DiffOutlined, DownloadOutlined, ReloadOutlined } from '@ant-design/icons';
import { Alert, Button, Card, Col, Input, Row, Space, Table, Tag, Typography, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import { diffGitOpsManifest, exportGitOpsManifest, type GitOpsDiffChange, type GitOpsManifest } from '../api/client';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

const actionColor: Record<string, string> = {
  create: 'green',
  update: 'gold',
  delete: 'red',
  unchanged: 'default',
};

export function GitOpsPage() {
  const [manifest, setManifest] = useState<GitOpsManifest | null>(null);
  const [yaml, setYaml] = useState('');
  const [checksum, setChecksum] = useState('');
  const [desiredJson, setDesiredJson] = useState('');
  const [changes, setChanges] = useState<GitOpsDiffChange[]>([]);
  const [loading, setLoading] = useState(false);
  const [diffing, setDiffing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = useRouteActive(ROUTE_META.gitops.path);

  const resourceCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    manifest?.resources.forEach((resource) => {
      counts[resource.kind] = (counts[resource.kind] ?? 0) + 1;
    });
    return Object.entries(counts);
  }, [manifest]);

  const reload = async () => {
    setLoading(true);
    try {
      const exported = await exportGitOpsManifest({ format: 'yaml' });
      setManifest(exported.manifest);
      setYaml(exported.manifestYaml ?? JSON.stringify(exported.manifest, null, 2));
      setDesiredJson(JSON.stringify(exported.manifest, null, 2));
      setChecksum(exported.checksum);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (active) void reload();
  }, [active]);

  const copyYaml = async () => {
    await navigator.clipboard.writeText(yaml);
    message.success('Manifest YAML 已复制');
  };

  const downloadYaml = () => {
    const blob = new Blob([yaml], { type: 'text/yaml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'tikeo-manifest.yaml';
    link.click();
    URL.revokeObjectURL(url);
  };

  const runDiff = async () => {
    setDiffing(true);
    try {
      const desired = JSON.parse(desiredJson) as GitOpsManifest;
      const result = await diffGitOpsManifest(desired);
      setChanges(result.changes);
      message.success(`Diff 完成：${result.summary.update ?? 0} 更新 / ${result.summary.create ?? 0} 新增 / ${result.summary.delete ?? 0} 删除`);
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDiffing(false);
    }
  };

  return (
    <Space orientation="vertical" size={20} style={{ width: '100%' }}>
      <div>
        <Typography.Title level={2}>GitOps / IaC</Typography.Title>
        <Typography.Text type="secondary">导出 Job、Workflow、Script、Plugin、AlertRule 的声明式 Manifest，并对 desired JSON 做 drift diff。</Typography.Text>
      </div>
      {error ? <Alert type="error" showIcon message="GitOps Manifest 加载失败" description={error} /> : null}
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={14}>
          <Card
            title={<Space><BranchesOutlined />当前 Manifest</Space>}
            extra={<Space><Button icon={<ReloadOutlined />} loading={loading} onClick={reload}>刷新</Button><Button icon={<CopyOutlined />} onClick={() => void copyYaml()}>复制 YAML</Button><Button icon={<DownloadOutlined />} onClick={downloadYaml}>下载</Button></Space>}
          >
            <Alert type="info" showIcon style={{ marginBottom: 16 }} message="Manifest 是正式管理面能力，不是兼容导出：checksum 基于 canonical JSON，diff 以 kind/namespace/app/name 为资源键。" />
            <Space wrap style={{ marginBottom: 12 }}>
              <Tag color="blue">{checksum}</Tag>
              {resourceCounts.map(([kind, count]) => <Tag key={kind}>{kind}: {count}</Tag>)}
            </Space>
            <Input.TextArea value={yaml} readOnly rows={24} />
          </Card>
        </Col>
        <Col xs={24} lg={10}>
          <Card
            title={<Space><DiffOutlined />Desired Diff</Space>}
            extra={<Button type="primary" loading={diffing} onClick={() => void runDiff()}>执行 Diff</Button>}
          >
            <Typography.Paragraph type="secondary">把 Git 中的 desired Manifest JSON 粘贴到这里，服务端会和当前状态做 create/update/delete/unchanged 对比。</Typography.Paragraph>
            <Input.TextArea value={desiredJson} onChange={(event) => setDesiredJson(event.target.value)} rows={16} />
          </Card>
          <Card title="Diff 结果" style={{ marginTop: 16 }}>
            <Table<GitOpsDiffChange>
              rowKey="key"
              size="small"
              dataSource={changes}
              pagination={{ pageSize: 8 }}
              expandable={{ expandedRowRender: (item) => <Input.TextArea value={item.diff || '无变化'} readOnly rows={8} /> }}
              columns={[
                { title: 'Action', dataIndex: 'action', width: 110, render: (value: string) => <Tag color={actionColor[value] ?? 'default'}>{value}</Tag> },
                { title: 'Kind', dataIndex: 'kind', width: 120 },
                { title: 'Name', dataIndex: 'name', ellipsis: true },
              ]}
            />
          </Card>
        </Col>
      </Row>
    </Space>
  );
}
