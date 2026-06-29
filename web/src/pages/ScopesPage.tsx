import { Alert, Button, Card, Col, Drawer, Form, Input, InputNumber, Row, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  createAppScope,
  createNamespace,
  createWorkerPool,
  createSecret,
  updateWorkerPoolQuota,
  deleteAppScope,
  deleteNamespace,
  deleteOidcIdentity,
  deleteWorkerPool,
  deleteSecret,
  upsertOidcIdentity,
  listAppScopes,
  listNamespaces,
  listOidcIdentities,
  listSecrets,
  listWorkerPools,
  type AppScopeSummary,
  type CreateAppScopeRequest,
  type CreateNamespaceRequest,
  type CreateWorkerPoolRequest,
  type CreateSecretRequest,
  type UpdateWorkerPoolQuotaRequest,
  type NamespaceSummary,
  type OidcIdentitySummary,
  type SecretReferenceRequest,
  type SecretSummary,
  type UpsertOidcIdentityRequest,
  type WorkerPoolSummary,
} from '../api/client';
import { GuardedButton, PermissionGate, useCan } from '../components/Permission';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';
import { persistentPagination, usePersistentTablePageSize } from '../utils/pagination';


interface CreateSecretFormValues {
  namespace: string;
  app: string;
  name: string;
  referenceKind: 'env' | 'vault' | 'secret';
  envName?: string;
  vaultPath?: string;
  vaultKey?: string;
  secretProvider?: string;
  secretId?: string;
  secretKey?: string;
}


function parseSecretReference(value: string): SecretReferenceRequest | null {
  try {
    const parsed = JSON.parse(value) as SecretReferenceRequest;
    if (parsed.kind === 'env' && 'name' in parsed) return parsed;
    if (parsed.kind === 'vault' && 'path' in parsed && 'key' in parsed) return parsed;
    if (parsed.kind === 'secret' && 'provider' in parsed && 'id' in parsed) return parsed;
  } catch {
    return null;
  }
  return null;
}

function renderSecretReference(value: string): ReactNode {
  const reference = parseSecretReference(value);
  if (!reference) {
    return <Typography.Text type="danger">引用格式无效</Typography.Text>;
  }
  if (reference.kind === 'env') {
    return <Space size={6} wrap><Tag color="green">环境变量</Tag><Typography.Text code>{reference.name}</Typography.Text></Space>;
  }
  if (reference.kind === 'vault') {
    return <Space size={6} wrap><Tag color="gold">Vault</Tag><Typography.Text code>{reference.path}</Typography.Text><Tag>{reference.key}</Tag></Space>;
  }
  return <Space size={6} wrap><Tag color="geekblue">外部 Secret</Tag><Typography.Text code>{reference.provider}</Typography.Text><Typography.Text code>{reference.id}</Typography.Text>{reference.key ? <Tag>{reference.key}</Tag> : null}</Space>;
}

function toCreateSecretRequest(values: CreateSecretFormValues): CreateSecretRequest {
  const base = { namespace: values.namespace, app: values.app, name: values.name };
  if (values.referenceKind === 'vault') {
    return { ...base, reference: { kind: 'vault', path: values.vaultPath ?? '', key: values.vaultKey ?? '' } };
  }
  if (values.referenceKind === 'secret') {
    return { ...base, reference: { kind: 'secret', provider: values.secretProvider ?? '', id: values.secretId ?? '', key: values.secretKey ?? null } };
  }
  return { ...base, reference: { kind: 'env', name: values.envName ?? '' } };
}

export function ScopesPage() {
  const canManageScopes = useCan('tenants', 'manage');
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [workerPools, setWorkerPools] = useState<WorkerPoolSummary[]>([]);
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [oidcIdentities, setOidcIdentities] = useState<OidcIdentitySummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [pageSize, setPageSize] = usePersistentTablePageSize();
  const [namespaceForm] = Form.useForm<CreateNamespaceRequest>();
  const [appForm] = Form.useForm<CreateAppScopeRequest>();
  const [poolForm] = Form.useForm<CreateWorkerPoolRequest>();
  const [secretForm] = Form.useForm<CreateSecretFormValues>();
  const [oidcForm] = Form.useForm<UpsertOidcIdentityRequest>();
  const secretReferenceKind = Form.useWatch('referenceKind', secretForm) ?? 'env';
  const oidcNamespace = Form.useWatch('namespace', oidcForm);
  const oidcApp = Form.useWatch('app', oidcForm);
  const [drawer, setDrawer] = useState<'namespace' | 'app' | 'pool' | 'secret' | 'oidc' | null>(null);
  const [quotaForm] = Form.useForm<UpdateWorkerPoolQuotaRequest>();
  const [quotaPool, setQuotaPool] = useState<WorkerPoolSummary | null>(null);
  const active = useRouteActive(ROUTE_META.scopes.path);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [namespaceData, appData, poolData, secretData, oidcData] = await Promise.all([
        listNamespaces(),
        listAppScopes(),
        listWorkerPools(),
        listSecrets(),
        listOidcIdentities(),
      ]);
      setNamespaces(namespaceData);
      setApps(appData);
      setWorkerPools(poolData);
      setSecrets(secretData);
      setOidcIdentities(oidcData);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '加载作用域失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { if (active) void refresh(); }, [active, refresh]);

  const namespaceOptions = useMemo(() => namespaces.map((item) => ({ value: item.name, label: item.name })), [namespaces]);
  const appOptions = useMemo(() => apps.map((item) => ({ value: item.name, label: `${item.namespace}/${item.name}` })), [apps]);
  const workerPoolOptions = useMemo(() => workerPools
    .filter((item) => (!oidcNamespace || item.namespace === oidcNamespace) && (!oidcApp || item.app === oidcApp))
    .map((item) => ({ value: item.name, label: `${item.namespace}/${item.app}/${item.name}` })), [oidcApp, oidcNamespace, workerPools]);

  const handleNamespaceCreate = async (values: CreateNamespaceRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理作用域'); return; }
    await createNamespace(values);
    namespaceForm.resetFields();
    setDrawer(null);
    message.success('命名空间已创建');
    await refresh();
  };

  const handleAppCreate = async (values: CreateAppScopeRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理作用域'); return; }
    await createAppScope(values);
    appForm.resetFields();
    setDrawer(null);
    message.success('应用已创建');
    await refresh();
  };

  const handleWorkerPoolCreate = async (values: CreateWorkerPoolRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理作用域'); return; }
    await createWorkerPool(values);
    poolForm.resetFields();
    setDrawer(null);
    message.success('执行池已创建');
    await refresh();
  };

  const handleSecretCreate = async (values: CreateSecretFormValues) => {
    if (!canManageScopes) { message.error('当前账号无权限管理 Secret 引用'); return; }
    await createSecret(toCreateSecretRequest(values));
    secretForm.resetFields();
    setDrawer(null);
    message.success('Secret 已创建');
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
    message.success('执行池已删除');
    await refresh();
  };

  const handleOidcIdentityDelete = async (id: string) => {
    await deleteOidcIdentity(id);
    message.success('OIDC 映射已删除');
    await refresh();
  };

  const openQuotaDrawer = (pool: WorkerPoolSummary) => {
    setQuotaPool(pool);
    quotaForm.setFieldsValue({ maxQueueDepth: pool.maxQueueDepth ?? 0, maxConcurrency: pool.maxConcurrency ?? 0 });
  };

  const handleQuotaUpdate = async (values: UpdateWorkerPoolQuotaRequest) => {
    if (!quotaPool) return;
    await updateWorkerPoolQuota(quotaPool.id, values);
    setQuotaPool(null);
    quotaForm.resetFields();
    message.success('执行池配额已更新');
    await refresh();
  };

  const namespaceColumns: ColumnsType<NamespaceSummary> = [
    { title: '命名空间', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '创建时间', dataIndex: 'createdAt' },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除命名空间" confirmDescription="仅空命名空间可删除；含应用、执行池或任务时后端会拒绝。" onConfirm={() => void handleNamespaceDelete(record.id)}>删除</GuardedButton> },
  ];

  const appColumns: ColumnsType<AppScopeSummary> = [
    { title: '命名空间', dataIndex: 'namespace', render: (value: string) => <Tag color="blue">{value}</Tag> },
    { title: '应用', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除应用" confirmDescription="仅空应用可删除；含执行池或任务时后端会拒绝。" onConfirm={() => void handleAppDelete(record.id)}>删除</GuardedButton> },
  ];

  const secretColumns: ColumnsType<SecretSummary> = [
    { title: '范围', render: (_, item) => <Space><Tag color="blue">{item.namespace}</Tag><Tag color="purple">{item.app}</Tag></Space> },
    { title: '名称', dataIndex: 'name' },
    { title: '引用', dataIndex: 'valueRef', render: renderSecretReference },
    { title: '创建人', dataIndex: 'createdBy' },
    { title: '操作', width: 100, render: (_, record) => <GuardedButton danger size="small" resource="tenants" action="manage" onClick={async () => { await deleteSecret(record.id); message.success('Secret 已删除'); await refresh(); }}>删除</GuardedButton> },
  ];

  const poolColumns: ColumnsType<WorkerPoolSummary> = [
    { title: '命名空间', dataIndex: 'namespace', render: (value: string) => <Tag color="blue">{value}</Tag> },
    { title: '应用', dataIndex: 'app', render: (value: string) => <Tag color="purple">{value}</Tag> },
    { title: '执行池', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '队列上限', dataIndex: 'maxQueueDepth', render: (value: number) => value > 0 ? value : '不限' },
    { title: '并发上限', dataIndex: 'maxConcurrency', render: (value: number) => value > 0 ? value : '不限' },
    { title: '更新时间', dataIndex: 'updatedAt' },
    { title: '操作', width: 180, render: (_, record) => <Space size={4}><Button type="link" size="small" onClick={() => openQuotaDrawer(record)}>配额</Button><GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除执行池" confirmDescription="删除后不会影响在线 Worker，会移除该执行池元数据；已按该池标记的 Worker 需要调整标签或重新注册。" onConfirm={() => void handleWorkerPoolDelete(record.id)}>删除</GuardedButton></Space> },
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
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除 OIDC 映射" confirmDescription="删除后该外部 subject 将无法换取本地 tikeo session。" onConfirm={() => void handleOidcIdentityDelete(record.id)}>删除</GuardedButton> },
  ];

  return (
    <div className="page-stack scope-management-page">
      <section className="hero-panel scope-management-hero">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag className="soft-tag" color="blue">Scope Management</Tag>
            <Typography.Title level={3}>作用域管理</Typography.Title>
          </div>
          <Typography.Text className="hero-panel__desc scope-management-hero__desc" ellipsis title="管理 namespace、app、执行池与 OIDC subject 映射；未映射的外部身份无法换取本地 session，已映射身份按 scope binding 收窄权限。">管理 namespace、app、执行池与 OIDC subject 映射；未映射的外部身份无法换取本地 session，已映射身份按 scope binding 收窄权限。</Typography.Text>
        </div>
        <div className="hero-panel__actions">
          <Button onClick={() => void refresh()} loading={loading}>刷新</Button>
        </div>
      </section>

      <Card className="clean-card" title="作用域资源" extra={<PermissionGate resource="tenants" action="manage"><Space wrap className="card-toolbar"><Button type="primary" onClick={() => setDrawer('namespace')}>新建命名空间</Button><Button onClick={() => setDrawer('app')}>新建应用</Button><Button onClick={() => setDrawer('pool')}>新建执行池</Button><Button onClick={() => setDrawer('secret')}>新建 Secret</Button><Button onClick={() => setDrawer('oidc')}>新建 OIDC 映射</Button></Space></PermissionGate>}>
        <Typography.Text type="secondary">作用域层级为 Namespace → App → 执行池。Namespace 表示环境、团队或业务边界；App 表示应用边界；执行池是 App 下的可选执行资源分组。</Typography.Text>
      </Card>

      <Alert
        type="info"
        showIcon
        message="执行池语义"
        description="执行池可对应一个 Worker 服务、一类运行时、一组机器资源或一个隔离队列；用于并发/队列配额、权限收窄、通知作用域、任务路由和运维定位。小规模部署可以留空，不配置执行池时仍按 Namespace/App 匹配。"
      />

      <Drawer title="创建命名空间" open={drawer === 'namespace'} onClose={() => { setDrawer(null); namespaceForm.resetFields(); }} width={760} destroyOnClose>
        <Form form={namespaceForm} layout="vertical" onFinish={(values) => void handleNamespaceCreate(values)}>
          <Form.Item name="name" label="命名空间" rules={[{ required: true, message: '请输入命名空间' }]}><Input placeholder="default / payments" /></Form.Item>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建命名空间</Button></PermissionGate>
        </Form>
      </Drawer>
      <Drawer title="创建应用" open={drawer === 'app'} onClose={() => { setDrawer(null); appForm.resetFields(); }} width={760} destroyOnClose>
        <Form form={appForm} layout="vertical" onFinish={(values) => void handleAppCreate(values)}>
          <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
          <Form.Item name="name" label="应用" rules={[{ required: true, message: '请输入应用名' }]}><Input placeholder="billing / settlement" /></Form.Item>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建应用</Button></PermissionGate>
        </Form>
      </Drawer>
      <Drawer title="创建执行池" open={drawer === 'pool'} onClose={() => { setDrawer(null); poolForm.resetFields(); }} width={760} destroyOnClose>
        <Form form={poolForm} layout="vertical" onFinish={(values) => void handleWorkerPoolCreate(values)}>
          <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
          <Form.Item name="app" label="应用" rules={[{ required: true, message: '请选择应用' }]}><Select options={appOptions} placeholder="选择 app" /></Form.Item>
          <Form.Item name="name" label="执行池" rules={[{ required: true, message: '请输入执行池' }]}><Input placeholder="critical / batch" /></Form.Item>
          <Typography.Paragraph type="secondary">执行池是 App 下的可选执行资源分组，可对应一个 Worker 服务、一类运行时、一组机器资源或一个隔离队列；创建后可配置队列上限与并发上限，0 表示不限。</Typography.Paragraph>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建执行池</Button></PermissionGate>
        </Form>
      </Drawer>

      <Drawer title="新建 Secret 引用" open={drawer === 'secret'} onClose={() => { setDrawer(null); secretForm.resetFields(); }} width={760} destroyOnClose>
        <Alert type="info" showIcon style={{ marginBottom: 16 }} message="这里只保存结构化 Secret 引用，不保存明文" description="请选择引用类型并填写对应字段，系统会保存规范化结构化引用。" />
        <Form form={secretForm} layout="vertical" initialValues={{ referenceKind: 'env' }} onFinish={(values) => void handleSecretCreate(values)}>
          <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
          <Form.Item name="app" label="应用" rules={[{ required: true, message: '请选择应用' }]}><Select options={appOptions} placeholder="选择 app" /></Form.Item>
          <Form.Item name="name" label="Secret 名称" rules={[{ required: true, message: '请输入 Secret 名称' }]}><Input placeholder="billing-db-password" /></Form.Item>
          <Form.Item name="referenceKind" label="引用类型" rules={[{ required: true, message: '请选择引用类型' }]}><Select options={[{ value: 'env', label: '环境变量' }, { value: 'vault', label: 'Vault 路径' }, { value: 'secret', label: '外部 Secret Provider' }]} /></Form.Item>
          {secretReferenceKind === 'env' ? <Form.Item name="envName" label="环境变量名" rules={[{ required: true, message: '请输入环境变量名' }]}><Input placeholder="BILLING_DB_PASSWORD" /></Form.Item> : null}
          {secretReferenceKind === 'vault' ? <><Form.Item name="vaultPath" label="Vault 路径" rules={[{ required: true, message: '请输入 Vault 路径' }]}><Input placeholder="kv/data/tikeo/billing" /></Form.Item><Form.Item name="vaultKey" label="Vault Key" rules={[{ required: true, message: '请输入 Vault Key' }]}><Input placeholder="db_password" /></Form.Item></> : null}
          {secretReferenceKind === 'secret' ? <><Form.Item name="secretProvider" label="Provider" rules={[{ required: true, message: '请输入 Provider' }]}><Input placeholder="aws-secrets-manager / k8s" /></Form.Item><Form.Item name="secretId" label="Secret ID" rules={[{ required: true, message: '请输入 Secret ID' }]}><Input placeholder="prod/billing/db" /></Form.Item><Form.Item name="secretKey" label="Secret Key"><Input placeholder="可选，例如 password" /></Form.Item></> : null}
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建 Secret</Button></PermissionGate>
        </Form>
      </Drawer>

      <Drawer title={quotaPool ? `执行池配额 - ${quotaPool.name}` : '执行池配额'} open={quotaPool !== null} onClose={() => { setQuotaPool(null); quotaForm.resetFields(); }} width={760} destroyOnClose>
        <Form form={quotaForm} layout="vertical" onFinish={(values) => void handleQuotaUpdate(values)}>
          <Form.Item name="maxQueueDepth" label="队列上限" extra="pending + running 总数上限；0 表示不限" rules={[{ required: true }]}><InputNumber min={0} precision={0} style={{ width: '100%' }} /></Form.Item>
          <Form.Item name="maxConcurrency" label="并发上限" extra="running 上限；0 表示不限" rules={[{ required: true }]}><InputNumber min={0} precision={0} style={{ width: '100%' }} /></Form.Item>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>保存配额</Button></PermissionGate>
        </Form>
      </Drawer>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={8}><Card className="clean-card" title="命名空间"><Table rowKey="id" loading={loading} columns={namespaceColumns} dataSource={namespaces} pagination={persistentPagination(pageSize, setPageSize)} size="small" /></Card></Col>
        <Col xs={24} xl={8}><Card className="clean-card" title="应用"><Table rowKey="id" loading={loading} columns={appColumns} dataSource={apps} pagination={persistentPagination(pageSize, setPageSize)} size="small" /></Card></Col>
        <Col xs={24} xl={8}><Card className="clean-card" title="执行池"><Table rowKey="id" loading={loading} columns={poolColumns} dataSource={workerPools} pagination={persistentPagination(pageSize, setPageSize)} size="small" /></Card></Col>
      </Row>

      <Card className="clean-card" title="Secret 引用" style={{ marginTop: 16 }} extra={<PermissionGate resource="tenants" action="manage"><Button onClick={() => setDrawer('secret')}>新建 Secret</Button></PermissionGate>}>
        <Alert type="info" showIcon style={{ marginBottom: 16 }} message="Secret 按 namespace/app 隔离" description="这里只保存结构化引用，不保存明文；创建、删除和使用会进入审计日志。" />
        <Table rowKey="id" loading={loading} columns={secretColumns} dataSource={secrets} pagination={persistentPagination(pageSize, setPageSize)} size="small" />
      </Card>

      <Drawer title="保存 OIDC 映射" open={drawer === 'oidc'} onClose={() => { setDrawer(null); oidcForm.resetFields(); }} width={980} destroyOnClose>
        <Alert type="info" showIcon style={{ marginBottom: 16 }} message="Fail-closed OIDC 映射" description="只有显式配置 issuer + subject 到本地用户的映射后，OIDC callback 才会签发本地 tikeo session；namespace/app/执行池会进入 scope binding。" />
        <Form form={oidcForm} layout="vertical" onFinish={(values) => void handleOidcIdentityUpsert(values)}>
          <Row gutter={[12, 0]}>
            <Col xs={24} lg={12}><Form.Item name="issuer" label="Issuer" rules={[{ required: true, message: '请输入 issuer' }]}><Input placeholder="https://idp.example.com/realms/tikeo" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="subject" label="Subject" rules={[{ required: true, message: '请输入 subject' }]}><Input placeholder="OIDC sub claim" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="username" label="本地用户" rules={[{ required: true, message: '请输入本地用户名' }]}><Input placeholder="oidc.alice" /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="namespace" label="Namespace scope"><Select allowClear options={namespaceOptions} placeholder="不选表示任意 namespace" onChange={() => oidcForm.setFieldsValue({ app: undefined, worker_pool: undefined })} /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="app" label="App scope"><Select allowClear options={appOptions} placeholder="不选表示任意 app" onChange={() => oidcForm.setFieldsValue({ worker_pool: undefined })} /></Form.Item></Col>
            <Col xs={24} lg={12}><Form.Item name="worker_pool" label="执行池 scope"><Select allowClear options={workerPoolOptions} placeholder="不选表示任意执行池" /></Form.Item></Col>
          </Row>
          <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>保存 OIDC 映射</Button></PermissionGate>
        </Form>
      </Drawer>

      <Card className="clean-card" title="OIDC 作用域/应用/角色绑定" extra={<PermissionGate resource="tenants" action="manage"><Space className="card-toolbar"><Button onClick={() => setDrawer('oidc')}>新建映射</Button></Space></PermissionGate>}>
        <Space orientation="vertical" size="middle" style={{ width: '100%' }}>
          <Alert type="info" showIcon message="Fail-closed OIDC 映射" description="外部身份必须显式映射后才可换取本地 session；scope binding 会限制 namespace/app/执行池。" />
          <Table rowKey="id" loading={loading} columns={oidcColumns} dataSource={oidcIdentities} pagination={persistentPagination(pageSize, setPageSize)} size="small" />
        </Space>
      </Card>
    </div>
  );
}
