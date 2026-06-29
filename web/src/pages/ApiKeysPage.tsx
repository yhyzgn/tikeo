import { DeleteOutlined, EditOutlined, KeyOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { Alert, Button, Card, DatePicker, Form, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import dayjs, { type Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';

import { createSdkApiKey, createServiceAccount, deleteSdkApiKey, disableServiceAccount, listAppScopes, listNamespaces, listSdkApiKeys, listServiceAccounts, listWorkerPools, updateSdkApiKey, updateServiceAccount, type AppScopeSummary, type NamespaceSummary, type SdkApiKeySummary, type ServiceAccountSummary, type WorkerPoolSummary } from '../api/client';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

const DEFAULT_SCOPES = ['jobs:read', 'jobs:write', 'instances:execute'];

const SectionTitle = ({ title, desc }: { title: string; desc: string }) => (
  <Space orientation="vertical" size={0}>
    <Typography.Text strong>{title}</Typography.Text>
    <Typography.Text type="secondary" className="api-key-section-desc">{desc}</Typography.Text>
  </Space>
);

interface ApiKeyFormValues {
  name: string;
  namespace: string;
  app: string;
  serviceAccountId: string;
  scopes: string[];
  expiresAt?: Dayjs | null;
}

interface EditFormValues {
  name: string;
  scopes: string[];
  expiresAt?: Dayjs | null;
}

export function ApiKeysPage() {
  const [keys, setKeys] = useState<SdkApiKeySummary[]>([]);
  const [serviceAccounts, setServiceAccounts] = useState<ServiceAccountSummary[]>([]);
  const [editingServiceAccount, setEditingServiceAccount] = useState<ServiceAccountSummary | null>(null);
  const [serviceAccountOpen, setServiceAccountOpen] = useState(false);
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [workerPools, setWorkerPools] = useState<WorkerPoolSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [editingKey, setEditingKey] = useState<SdkApiKeySummary | null>(null);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [form] = Form.useForm<ApiKeyFormValues>();
  const [editForm] = Form.useForm<EditFormValues>();
  const [serviceAccountForm] = Form.useForm<{ name: string; description?: string; namespace: string; app: string; workerPool?: string; status?: string }>();
  const serviceAccountNamespace = Form.useWatch('namespace', serviceAccountForm);
  const serviceAccountApp = Form.useWatch('app', serviceAccountForm);
  const active = useRouteActive(ROUTE_META.apiKeys.path);

  const reload = async () => {
    setLoading(true);
    try {
      const [nextKeys, nextServiceAccounts, nextNamespaces, nextApps, nextWorkerPools] = await Promise.all([listSdkApiKeys(), listServiceAccounts(), listNamespaces(), listAppScopes(), listWorkerPools()]);
      setKeys(nextKeys);
      setServiceAccounts(nextServiceAccounts);
      setNamespaces(nextNamespaces);
      setApps(nextApps);
      setWorkerPools(nextWorkerPools);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (active) void reload();
  }, [active]);

  const handleCreate = async () => {
    const values = await form.validateFields();
    const created = await createSdkApiKey({
      name: values.name,
      namespace: values.namespace,
      app: values.app,
      service_account_id: values.serviceAccountId,
      scopes: values.scopes,
      expires_at: values.expiresAt?.toISOString() ?? null,
    });
    setCreatedKey(created.api_key);
    setOpen(false);
    form.resetFields();
    await reload();
  };

  const openEditModal = (item: SdkApiKeySummary) => {
    setEditingKey(item);
    editForm.setFieldsValue({
      name: item.name,
      scopes: item.scopes,
      expiresAt: item.expires_at ? dayjs(item.expires_at) : null,
    });
  };

  const handleEdit = async () => {
    if (!editingKey) return;
    const values = await editForm.validateFields();
    await updateSdkApiKey(editingKey.id, {
      name: values.name,
      scopes: values.scopes,
      expires_at: values.expiresAt?.toISOString() ?? null,
    });
    setEditingKey(null);
    editForm.resetFields();
    message.success('API-Key 已更新');
    await reload();
  };

  const handleRevoke = async (id: string) => {
    await deleteSdkApiKey(id);
    message.success('API-Key 已吊销');
    await reload();
  };

  const openCreateServiceAccount = () => {
    setEditingServiceAccount(null);
    serviceAccountForm.resetFields();
    serviceAccountForm.setFieldsValue({ namespace: 'default', app: 'default', status: 'active' });
    setServiceAccountOpen(true);
  };

  const openEditServiceAccount = (item: ServiceAccountSummary) => {
    setEditingServiceAccount(item);
    serviceAccountForm.setFieldsValue({
      name: item.name,
      description: item.description ?? undefined,
      namespace: item.namespace,
      app: item.app,
      workerPool: item.workerPool ?? undefined,
      status: item.status,
    });
    setServiceAccountOpen(true);
  };

  const handleSaveServiceAccount = async () => {
    const values = await serviceAccountForm.validateFields();
    const payload = {
      name: values.name,
      description: values.description ?? null,
      namespace: values.namespace,
      app: values.app,
      workerPool: values.workerPool ?? null,
    };
    if (editingServiceAccount) {
      await updateServiceAccount(editingServiceAccount.id, { ...payload, status: values.status ?? 'active' });
      message.success('Service Account 已更新');
    } else {
      await createServiceAccount(payload);
      message.success('Service Account 已创建');
    }
    setServiceAccountOpen(false);
    setEditingServiceAccount(null);
    serviceAccountForm.resetFields();
    await reload();
  };

  const handleDisableServiceAccount = async (id: string) => {
    await disableServiceAccount(id);
    message.success('Service Account 已禁用，关联 API-Key 已吊销');
    await reload();
  };

  const copyCreatedKey = async () => {
    if (!createdKey) return;
    await navigator.clipboard.writeText(createdKey);
    message.success('完整 API-Key 已复制');
  };

  const appOptionsForServiceAccount = useMemo(() => apps
    .filter((item) => !serviceAccountNamespace || item.namespace === serviceAccountNamespace)
    .map((item) => ({ value: item.name, label: item.name }))
    .sort((left, right) => left.label.localeCompare(right.label)), [apps, serviceAccountNamespace]);
  const workerPoolOptionsForServiceAccount = useMemo(() => workerPools
    .filter((item) => item.namespace === serviceAccountNamespace && item.app === serviceAccountApp)
    .map((item) => ({ value: item.name, label: `${item.name} · 并发 ${item.maxConcurrency > 0 ? item.maxConcurrency : '不限'} / 队列 ${item.maxQueueDepth > 0 ? item.maxQueueDepth : '不限'}` }))
    .sort((left, right) => left.value.localeCompare(right.value)), [serviceAccountApp, serviceAccountNamespace, workerPools]);

  return (
    <Space orientation="vertical" size={20} style={{ width: '100%' }}>
      <Card
        title={<Space><KeyOutlined />SDK Management API-Key</Space>}
        extra={<Button icon={<ReloadOutlined />} onClick={reload}>刷新列表</Button>}
      >
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
          message="本页分为两栏：Service Account 是 app 作用域机器身份；API-Key 是绑定到 Service Account 的访问凭证。先维护机器身份，再给它签发一个或多个 Key。"
        />

        <Space orientation="vertical" size={16} style={{ width: '100%' }}>
          <Card
            size="small"
            className="api-key-section-card"
            title={<SectionTitle title="Service Accounts（机器身份）" desc="定义 namespace/app 与可选执行池作用域；禁用后会吊销关联 API-Key。" />}
            extra={<Button icon={<PlusOutlined />} onClick={openCreateServiceAccount}>新建 Service Account</Button>}
          >
            <Table<ServiceAccountSummary>
              rowKey="id"
              loading={loading}
              dataSource={serviceAccounts}
              pagination={false}
              size="small"
              scroll={{ x: 980 }}
              columns={[
                { title: 'Service Account', width: 260, render: (_, item) => <Space orientation="vertical" size={0}><Typography.Text strong>{item.name}</Typography.Text><Typography.Text type="secondary">{item.id}</Typography.Text></Space> },
                { title: '作用域', width: 180, render: (_, item) => `${item.namespace}/${item.app}` },
                { title: '执行池', dataIndex: 'workerPool', width: 140, render: (value) => value ?? '不限' },
                { title: '状态', dataIndex: 'status', width: 100, render: (status) => <Tag color={status === 'active' ? 'green' : 'default'}>{status}</Tag> },
                { title: '描述', dataIndex: 'description', width: 180, render: (value) => value ?? '-' },
                {
                  title: '操作',
                  fixed: 'right',
                  width: 180,
                  render: (_, item) => (
                    <Space>
                      <Button size="small" icon={<EditOutlined />} onClick={() => openEditServiceAccount(item)}>编辑</Button>
                      <Button danger size="small" icon={<DeleteOutlined />} disabled={item.status !== 'active'} onClick={() => void handleDisableServiceAccount(item.id)}>禁用</Button>
                    </Space>
                  ),
                },
              ]}
            />
          </Card>

          <Card
            size="small"
            className="api-key-section-card"
            title={<SectionTitle title="API-Keys（访问凭证）" desc="绑定到一个 Service Account；用于 SDK 调用，明文只在签发时显示一次。" />}
            extra={<Button type="primary" icon={<PlusOutlined />} onClick={() => setOpen(true)}>签发 API-Key</Button>}
          >
            <Table<SdkApiKeySummary>
              rowKey="id"
              loading={loading}
              dataSource={keys}
              size="small"
              scroll={{ x: 1160 }}
              columns={[
                { title: '名称', dataIndex: 'name', width: 180 },
                {
                  title: 'API-Key',
                  dataIndex: 'key_prefix',
                  width: 260,
                  render: (value: string) => <Typography.Text className="api-key-masked-text">{value}</Typography.Text>,
                },
                { title: '范围', width: 180, render: (_, item) => `${item.namespace}/${item.app}` },
                { title: 'Service Account', width: 240, render: (_, item) => <Space orientation="vertical" size={0}><Typography.Text>{item.service_account_name}</Typography.Text><Typography.Text type="secondary">{item.service_account_id}</Typography.Text></Space> },
                { title: 'Scopes', width: 260, render: (_, item) => <Space wrap>{item.scopes.map((scope) => <Tag key={scope}>{scope}</Tag>)}</Space> },
                { title: '状态', dataIndex: 'status', width: 100, render: (status) => <Tag color={status === 'active' ? 'green' : 'default'}>{status}</Tag> },
                { title: '有效期', dataIndex: 'expires_at', width: 190, render: (value) => value ?? '永久有效' },
                { title: '最近使用', dataIndex: 'last_used_at', width: 190, render: (value) => value ?? '-' },
                { title: '创建人', dataIndex: 'created_by', width: 140 },
                {
                  title: '操作',
                  fixed: 'right',
                  width: 170,
                  render: (_, item) => (
                    <Space>
                      <Button size="small" icon={<EditOutlined />} disabled={item.status !== 'active'} onClick={() => openEditModal(item)}>编辑</Button>
                      <Button danger size="small" icon={<DeleteOutlined />} disabled={item.status !== 'active'} onClick={() => void handleRevoke(item.id)}>吊销</Button>
                    </Space>
                  ),
                },
              ]}
            />
          </Card>
        </Space>
      </Card>


      <Modal title={editingServiceAccount ? '编辑 Service Account' : '新建 Service Account'} width={760} open={serviceAccountOpen} onOk={() => void handleSaveServiceAccount()} onCancel={() => setServiceAccountOpen(false)} okText="保存">
        <Alert type="info" showIcon message="Service Account 是机器身份；禁用后会吊销其关联 API-Key。" style={{ marginBottom: 16 }} />
        <Form form={serviceAccountForm} layout="vertical" initialValues={{ namespace: 'default', app: 'default', status: 'active' }}>
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入 Service Account 名称' }]}><Input placeholder="java-demo-service-account" /></Form.Item>
          <Form.Item name="description" label="描述"><Input.TextArea rows={2} placeholder="用途说明，例如 Java demo 管理接口调用" /></Form.Item>
          <Form.Item name="namespace" label="Namespace" rules={[{ required: true }]}>
            <Select options={namespaces.map((item) => ({ value: item.name, label: item.name }))} showSearch onChange={() => serviceAccountForm.setFieldsValue({ app: undefined, workerPool: undefined })} />
          </Form.Item>
          <Form.Item name="app" label="App" rules={[{ required: true }]}>
            <Select options={appOptionsForServiceAccount} showSearch disabled={!serviceAccountNamespace} onChange={() => serviceAccountForm.setFieldsValue({ workerPool: undefined })} />
          </Form.Item>
          <Form.Item name="workerPool" label="执行池（可选）" extra="留空表示不限执行池；选择后该机器身份和关联 API-Key 只能操作同一执行池内的资源。">
            <Select allowClear showSearch options={workerPoolOptionsForServiceAccount} disabled={!serviceAccountNamespace || !serviceAccountApp} placeholder={serviceAccountNamespace && serviceAccountApp ? '选择执行池，留空表示不限执行池' : '先选择 Namespace 和 App'} />
          </Form.Item>
          {editingServiceAccount ? <Form.Item name="status" label="状态" rules={[{ required: true }]}><Select options={[{ value: 'active', label: 'active' }, { value: 'disabled', label: 'disabled' }]} /></Form.Item> : null}
        </Form>
      </Modal>

      <Modal title="签发 SDK API-Key" width={760} open={open} onOk={() => void handleCreate()} onCancel={() => setOpen(false)} okText="签发">
        <Alert type="warning" showIcon message="明文 API-Key 创建后只显示一次；有效期留空则永久有效。" style={{ marginBottom: 16 }} />
        <Form form={form} layout="vertical" initialValues={{ namespace: 'default', app: 'default', scopes: DEFAULT_SCOPES }}>
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}><Input placeholder="java-demo-management" /></Form.Item>
          <Form.Item name="namespace" label="Namespace" rules={[{ required: true }]}>
            <Select options={namespaces.map((item) => ({ value: item.name, label: item.name }))} showSearch />
          </Form.Item>
          <Form.Item name="app" label="App" rules={[{ required: true }]}>
            <Select options={apps.map((item) => ({ value: item.name, label: `${item.namespace}/${item.name}` }))} showSearch />
          </Form.Item>
          <Form.Item name="serviceAccountId" label="Service Account" rules={[{ required: true, message: '请选择 Service Account' }]}>
            <Select
              showSearch
              placeholder="选择已有 Service Account"
              options={serviceAccounts.filter((item) => item.status === 'active').map((item) => ({ value: item.id, label: `${item.namespace}/${item.app} · ${item.name}` }))}
            />
          </Form.Item>
          <Form.Item name="scopes" label="权限 scopes" rules={[{ required: true }]}>
            <Select mode="tags" options={DEFAULT_SCOPES.map((scope) => ({ value: scope }))} />
          </Form.Item>
          <Form.Item name="expiresAt" label="有效期">
            <DatePicker showTime style={{ width: '100%' }} placeholder="留空则永久有效" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal title="编辑 API-Key" width={760} open={editingKey !== null} onOk={() => void handleEdit()} onCancel={() => setEditingKey(null)} okText="保存">
        <Alert type="info" showIcon message="这里只更新名称、权限 scopes 和有效期，不会重新生成 Key，现有 SDK 配置无需替换。" style={{ marginBottom: 16 }} />
        <Typography.Paragraph type="secondary">{editingKey ? `${editingKey.name} · ${editingKey.namespace}/${editingKey.app}` : ''}</Typography.Paragraph>
        <Form form={editForm} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input placeholder="java-demo-management" />
          </Form.Item>
          <Form.Item name="scopes" label="权限 scopes" rules={[{ required: true }]}>
            <Select mode="tags" options={DEFAULT_SCOPES.map((scope) => ({ value: scope }))} />
          </Form.Item>
          <Form.Item name="expiresAt" label="有效期">
            <DatePicker showTime style={{ width: '100%' }} placeholder="留空则永久有效" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="API-Key 只显示一次"
        width={860}
        open={createdKey !== null}
        closable={false}
        maskClosable={false}
        keyboard={false}
        footer={<Button type="primary" onClick={() => setCreatedKey(null)}>我已复制并保存</Button>}
      >
        <Alert
          className="api-key-once-alert"
          type="warning"
          showIcon
          message={<strong>请现在手动复制并保存完整 API-Key</strong>}
          description="这是唯一一次展示明文。关闭弹窗后服务端无法找回明文，列表也只会显示脱敏值；丢失后只能吊销并重新签发。"
        />
        <div
          className="api-key-secret-box"
          aria-label="点击复制新签发的完整 API-Key"
          role="button"
          tabIndex={0}
          title="点击复制完整 API-Key"
          onClick={() => void copyCreatedKey()}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              void copyCreatedKey();
            }
          }}
        >
          <Typography.Text className="api-key-secret-text">{createdKey ?? ''}</Typography.Text>
        </div>
        <Typography.Paragraph className="api-key-copy-hint">点击上方完整 Key 可一键复制；保存到安全位置后，再关闭此弹窗。</Typography.Paragraph>
      </Modal>
    </Space>
  );
}
