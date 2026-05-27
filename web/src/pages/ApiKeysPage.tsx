import { CopyOutlined, DeleteOutlined, KeyOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { Alert, Button, Card, DatePicker, Form, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import { useEffect, useState } from 'react';

import { createSdkApiKey, deleteSdkApiKey, listAppScopes, listNamespaces, listSdkApiKeys, type AppScopeSummary, type NamespaceSummary, type SdkApiKeySummary } from '../api/client';

const DEFAULT_SCOPES = ['jobs:read', 'jobs:write', 'instances:execute'];

interface ApiKeyFormValues {
  name: string;
  namespace: string;
  app: string;
  scopes: string[];
  expiresAt?: { toISOString: () => string } | null;
}

export function ApiKeysPage() {
  const [keys, setKeys] = useState<SdkApiKeySummary[]>([]);
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [form] = Form.useForm<ApiKeyFormValues>();

  const reload = async () => {
    setLoading(true);
    try {
      const [nextKeys, nextNamespaces, nextApps] = await Promise.all([listSdkApiKeys(), listNamespaces(), listAppScopes()]);
      setKeys(nextKeys);
      setNamespaces(nextNamespaces);
      setApps(nextApps);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reload();
  }, []);

  const handleCreate = async () => {
    const values = await form.validateFields();
    const created = await createSdkApiKey({
      name: values.name,
      namespace: values.namespace,
      app: values.app,
      scopes: values.scopes,
      expires_at: values.expiresAt?.toISOString() ?? null,
    });
    setCreatedKey(created.api_key);
    setOpen(false);
    form.resetFields();
    await reload();
  };

  const handleRevoke = async (id: string) => {
    await deleteSdkApiKey(id);
    message.success('API-Key 已吊销');
    await reload();
  };

  const copyText = async (value: string, label: string) => {
    await navigator.clipboard.writeText(value);
    message.success(`已复制${label}`);
  };

  return (
    <Space direction="vertical" size={20} style={{ width: '100%' }}>
      <Card
        title={<Space><KeyOutlined />SDK Management API-Key</Space>}
        extra={<Space><Button icon={<ReloadOutlined />} onClick={reload}>刷新</Button><Button type="primary" icon={<PlusOutlined />} onClick={() => setOpen(true)}>签发 API-Key</Button></Space>}
      >
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
          message="API-Key 是后台手动签发给 SDK 的 app 作用域凭证，请使用 X-Tikee-API-Key 请求头；明文只在创建后显示一次，列表只能复制脱敏后的展示值。"
        />
        <Table<SdkApiKeySummary>
          rowKey="id"
          loading={loading}
          dataSource={keys}
          scroll={{ x: 1100 }}
          columns={[
            { title: '名称', dataIndex: 'name', width: 180 },
            {
              title: 'Key 前缀',
              dataIndex: 'key_prefix',
              width: 180,
              render: (value: string) => <Typography.Text code copyable={{ text: value }}>{value}</Typography.Text>,
            },
            { title: '范围', width: 180, render: (_, item) => `${item.namespace}/${item.app}` },
            { title: 'Scopes', width: 260, render: (_, item) => <Space wrap>{item.scopes.map((scope) => <Tag key={scope}>{scope}</Tag>)}</Space> },
            { title: '状态', dataIndex: 'status', width: 100, render: (status) => <Tag color={status === 'active' ? 'green' : 'default'}>{status}</Tag> },
            { title: '有效期', dataIndex: 'expires_at', width: 190, render: (value) => value ?? '永久有效' },
            { title: '最近使用', dataIndex: 'last_used_at', width: 190, render: (value) => value ?? '-' },
            { title: '创建人', dataIndex: 'created_by', width: 140 },
            {
              title: '操作',
              fixed: 'right',
              width: 160,
              render: (_, item) => (
                <Space>
                  <Button size="small" icon={<CopyOutlined />} onClick={() => void copyText(item.key_prefix, '脱敏 Key')}>复制</Button>
                  <Button danger size="small" icon={<DeleteOutlined />} disabled={item.status !== 'active'} onClick={() => void handleRevoke(item.id)}>吊销</Button>
                </Space>
              ),
            },
          ]}
        />
      </Card>

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
          <Form.Item name="scopes" label="权限 scopes" rules={[{ required: true }]}>
            <Select mode="tags" options={DEFAULT_SCOPES.map((scope) => ({ value: scope }))} />
          </Form.Item>
          <Form.Item name="expiresAt" label="有效期">
            <DatePicker showTime style={{ width: '100%' }} placeholder="留空则永久有效" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal title="API-Key 只显示一次" width={860} open={createdKey !== null} onCancel={() => setCreatedKey(null)} footer={<Button onClick={() => setCreatedKey(null)}>我已保存</Button>}>
        <Alert type="warning" showIcon message="请立即复制保存，服务端不会保存明文；关闭后只能在列表看到脱敏前缀。" style={{ marginBottom: 12 }} />
        <Input.TextArea readOnly autoSize={{ minRows: 3, maxRows: 5 }} value={createdKey ?? ''} />
        <Button style={{ marginTop: 12 }} type="primary" icon={<CopyOutlined />} onClick={() => void copyText(createdKey ?? '', 'API-Key')}>复制完整 API-Key</Button>
      </Modal>
    </Space>
  );
}
