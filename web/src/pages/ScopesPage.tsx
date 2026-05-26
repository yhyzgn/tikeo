import { Alert, Button, Card, Col, Drawer, Form, Input, Row, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  createAppScope,
  createNamespace,
  createWorkerPool,
  deleteAppScope,
  deleteNamespace,
  deleteOidcIdentity,
  deleteWorkerPool,
  upsertOidcIdentity,
  listAppScopes,
  listNamespaces,
  listOidcIdentities,
  listWorkerPools,
  type AppScopeSummary,
  type CreateAppScopeRequest,
  type CreateNamespaceRequest,
  type CreateWorkerPoolRequest,
  type NamespaceSummary,
  type OidcIdentitySummary,
  type UpsertOidcIdentityRequest,
  type WorkerPoolSummary,
} from '../api/client';
import { GuardedButton, PermissionGate, useCan } from '../components/Permission';

export function ScopesPage() {
  const canManageScopes = useCan('tenants', 'manage');
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [workerPools, setWorkerPools] = useState<WorkerPoolSummary[]>([]);
  const [oidcIdentities, setOidcIdentities] = useState<OidcIdentitySummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [namespaceForm] = Form.useForm<CreateNamespaceRequest>();
  const [appForm] = Form.useForm<CreateAppScopeRequest>();
  const [poolForm] = Form.useForm<CreateWorkerPoolRequest>();
  const [oidcForm] = Form.useForm<UpsertOidcIdentityRequest>();
  const [drawer, setDrawer] = useState<'namespace' | 'app' | 'pool' | 'oidc' | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [namespaceData, appData, poolData, oidcData] = await Promise.all([
        listNamespaces(),
        listAppScopes(),
        listWorkerPools(),
        listOidcIdentities(),
      ]);
      setNamespaces(namespaceData);
      setApps(appData);
      setWorkerPools(poolData);
      setOidcIdentities(oidcData);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '加载租户范围失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const namespaceOptions = useMemo(() => namespaces.map((item) => ({ value: item.name, label: item.name })), [namespaces]);
  const appOptions = useMemo(() => apps.map((item) => ({ value: item.name, label: `${item.namespace}/${item.name}` })), [apps]);
  const workerPoolOptions = useMemo(() => workerPools.map((item) => ({ value: item.name, label: `${item.namespace}/${item.app}/${item.name}` })), [workerPools]);

  const handleNamespaceCreate = async (values: CreateNamespaceRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理租户范围'); return; }
    await createNamespace(values);
    namespaceForm.resetFields();
    setDrawer(null);
    message.success('命名空间已创建');
    await refresh();
  };

  const handleAppCreate = async (values: CreateAppScopeRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理租户范围'); return; }
    await createAppScope(values);
    appForm.resetFields();
    setDrawer(null);
    message.success('应用已创建');
    await refresh();
  };

  const handleWorkerPoolCreate = async (values: CreateWorkerPoolRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理租户范围'); return; }
    await createWorkerPool(values);
    poolForm.resetFields();
    setDrawer(null);
    message.success('Worker Pool 已创建');
    await refresh();
  };

  const handleOidcIdentityUpsert = async (values: UpsertOidcIdentityRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理 OIDC 映射'); return; }
    await upsertOidcIdentity(values);
    oidcForm.resetFields();
    setDrawer(null);
    message.success('OIDC 映射已保存');
    await refresh();
  };

  const handleNamespaceDelete = async (id: string) => {
    await deleteNamespace(id);
    message.success('命名空间已删除');
    await refresh();
  };

  const handleAppDelete = async (id: string) => {
    await deleteAppScope(id);
    message.success('应用已删除');
    await refresh();
  };

  const handleWorkerPoolDelete = async (id: string) => {
    await deleteWorkerPool(id);
    message.success('Worker Pool 已删除');
    await refresh();
  };

  const handleOidcIdentityDelete = async (id: string) => {
    await deleteOidcIdentity(id);
    message.success('OIDC 映射已删除');
    await refresh();
  };

  const namespaceColumns: ColumnsType<NamespaceSummary> = [
    { title: '命名空间', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '创建时间', dataIndex: 'createdAt' },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除命名空间" confirmDescription="仅空命名空间可删除；含应用、Worker Pool 或任务时后端会拒绝。" onConfirm={() => void handleNamespaceDelete(record.id)}>删除</GuardedButton> },
  ];

  const appColumns: ColumnsType<AppScopeSummary> = [
    { title: '命名空间', dataIndex: 'namespace', render: (value: string) => <Tag color="blue">{value}</Tag> },
    { title: '应用', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除应用" confirmDescription="仅空应用可删除；含 Worker Pool 或任务时后端会拒绝。" onConfirm={() => void handleAppDelete(record.id)}>删除</GuardedButton> },
  ];

  const poolColumns: ColumnsType<WorkerPoolSummary> = [
    { title: '命名空间', dataIndex: 'namespace', render: (value: string) => <Tag color="blue">{value}</Tag> },
    { title: '应用', dataIndex: 'app', render: (value: string) => <Tag color="purple">{value}</Tag> },
    { title: 'Worker Pool', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 140, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除 Worker Pool" confirmDescription="删除后不会影响在线 Worker，会移除该持久化元数据。" onConfirm={() => void handleWorkerPoolDelete(record.id)}>删除</GuardedButton> },
  ];

  const oidcColumns: ColumnsType<OidcIdentitySummary> = [
    { title: 'Issuer', dataIndex: 'issuer', ellipsis: true },
    { title: 'Subject', dataIndex: 'subject', render: (value: string) => <Typography.Text code>{value}</Typography.Text> },
    { title: '本地用户', dataIndex: 'username', render: (value: string) => <strong>{value}</strong> },
    { title: 'Scope', render: (_, record) => (
      <Space size={4} wrap>
        <Tag color="blue">{record.namespace ?? '*'}</Tag>
        <Tag color="purple">{record.app ?? '*'}</Tag>
        <Tag color="geekblue">{record.worker_pool ?? '*'}</Tag>
      </Space>
    ) },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除 OIDC 映射" confirmDescription="删除后该外部 subject 将无法换取本地 tikee session。" onConfirm={() => void handleOidcIdentityDelete(record.id)}>删除</GuardedButton> },
  ];

  return (
    <div className="page-stack scope-management-page">
      <section className="hero-panel scope-management-hero">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag className="soft-tag" color="blue">Tenant Scope</Tag>
            <Typography.Title level={3}>租户范围</Typography.Title>
          </div>
          <Typography.Text className="hero-panel__desc scope-management-hero__desc" ellipsis title="管理 namespace、app、Worker Pool 与 OIDC subject 映射；未映射的外部身份无法换取本地 session，已映射身份按 scope binding 收窄权限。">管理 namespace、app、Worker Pool 与 OIDC subject 映射；未映射的外部身份无法换取本地 session，已映射身份按 scope binding 收窄权限。</Typography.Text>
        </div>
        <div className="hero-panel__actions">
          <Button onClick={() => void refresh()} loading={loading}>刷新</Button>
        </div>
      </section>

      <Card className="clean-card" title="租户资源" extra={<PermissionGate resource="tenants" action="manage"><Space wrap className="card-toolbar"><Button type="primary" onClick={() => setDrawer('namespace')}>新建命名空间</Button><Button onClick={() => setDrawer('app')}>新建应用</Button><Button onClick={() => setDrawer('pool')}>新建 Worker Pool</Button><Button onClick={() => setDrawer('oidc')}>新建 OIDC 映射</Button></Space></PermissionGate>}>
        <Typography.Text type="secondary">所有新建/绑定操作通过右侧抽屉完成，列表区域只负责检索、查看和删除。</Typography.Text>
      </Card>

      <Drawer title="创建命名空间" open={drawer === 'namespace'} onClose={() => { setDrawer(null); namespaceForm.resetFields(); }} width={420} destroyOnClose>
        <Form form={namespaceForm} layout="vertical" onFinish={(values) => void handleNamespaceCreate(values)}>
          <Form.Item name="name" label="命名空间" rules={[{ required: true, message: '请输入命名空间' }]}><Input placeholder="default / payments" /></Form.Item>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建命名空间</Button></PermissionGate>
        </Form>
      </Drawer>
      <Drawer title="创建应用" open={drawer === 'app'} onClose={() => { setDrawer(null); appForm.resetFields(); }} width={420} destroyOnClose>
        <Form form={appForm} layout="vertical" onFinish={(values) => void handleAppCreate(values)}>
          <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
          <Form.Item name="name" label="应用" rules={[{ required: true, message: '请输入应用名' }]}><Input placeholder="billing / settlement" /></Form.Item>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建应用</Button></PermissionGate>
        </Form>
      </Drawer>
      <Drawer title="创建 Worker Pool" open={drawer === 'pool'} onClose={() => { setDrawer(null); poolForm.resetFields(); }} width={420} destroyOnClose>
        <Form form={poolForm} layout="vertical" onFinish={(values) => void handleWorkerPoolCreate(values)}>
          <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
          <Form.Item name="app" label="应用" rules={[{ required: true, message: '请选择应用' }]}><Select options={appOptions} placeholder="选择 app" /></Form.Item>
          <Form.Item name="name" label="Worker Pool" rules={[{ required: true, message: '请输入 Worker Pool' }]}><Input placeholder="critical / batch" /></Form.Item>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建 Worker Pool</Button></PermissionGate>
        </Form>
      </Drawer>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={8}><Card className="clean-card" title="命名空间"><Table rowKey="id" loading={loading} columns={namespaceColumns} dataSource={namespaces} pagination={{ pageSize: 6 }} size="small" /></Card></Col>
        <Col xs={24} xl={8}><Card className="clean-card" title="应用"><Table rowKey="id" loading={loading} columns={appColumns} dataSource={apps} pagination={{ pageSize: 6 }} size="small" /></Card></Col>
        <Col xs={24} xl={8}><Card className="clean-card" title="Worker Pool"><Table rowKey="id" loading={loading} columns={poolColumns} dataSource={workerPools} pagination={{ pageSize: 6 }} size="small" /></Card></Col>
      </Row>

      <Drawer title="保存 OIDC 映射" open={drawer === 'oidc'} onClose={() => { setDrawer(null); oidcForm.resetFields(); }} width={720} destroyOnClose>
        <Alert type="info" showIcon style={{ marginBottom: 16 }} message="Fail-closed OIDC 映射" description="只有显式配置 issuer + subject 到本地用户的映射后，OIDC callback 才会签发本地 tikee session；namespace/app/Worker Pool 会进入 scope binding。" />
        <Form form={oidcForm} layout="vertical" onFinish={(values) => void handleOidcIdentityUpsert(values)}>
          <Row gutter={[12, 0]}>
            <Col xs={24} lg={12}><Form.Item name="issuer" label="Issuer" rules={[{ required: true, message: '请输入 issuer' }]}><Input placeholder="https://idp.example.com/realms/tikee" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="subject" label="Subject" rules={[{ required: true, message: '请输入 subject' }]}><Input placeholder="OIDC sub claim" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="username" label="本地用户" rules={[{ required: true, message: '请输入本地用户名' }]}><Input placeholder="oidc.alice" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="namespace" label="Namespace scope"><Select allowClear options={namespaceOptions} placeholder="不选表示任意 namespace" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="app" label="App scope"><Select allowClear options={appOptions} placeholder="不选表示任意 app" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="worker_pool" label="Worker Pool scope"><Select allowClear options={workerPoolOptions} placeholder="不选表示任意 worker pool" /></Form.Item></Col>
          </Row>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>保存 OIDC 映射</Button></PermissionGate>
        </Form>
      </Drawer>

      <Card className="clean-card" title="OIDC tenant/app/role 绑定" extra={<PermissionGate resource="tenants" action="manage"><Space className="card-toolbar"><Button onClick={() => setDrawer('oidc')}>新建映射</Button></Space></PermissionGate>}>
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Alert type="info" showIcon message="Fail-closed OIDC 映射" description="外部身份必须显式映射后才可换取本地 session；scope binding 会限制 namespace/app/Worker Pool。" />
          <Table rowKey="id" loading={loading} columns={oidcColumns} dataSource={oidcIdentities} pagination={{ pageSize: 6 }} size="small" />
        </Space>
      </Card>
    </div>
  );
}
