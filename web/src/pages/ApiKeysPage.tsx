import { CopyOutlined, DeleteOutlined, KeyOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { Alert, Button, Card, Form, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import { useEffect, useState } from 'react';

import { createSdkApiKey, deleteSdkApiKey, listAppScopes, listNamespaces, listSdkApiKeys, type AppScopeSummary, type NamespaceSummary, type SdkApiKeySummary } from '../api/client';

const DEFAULT_SCOPES = ['jobs:read', 'jobs:write', 'instances:execute'];

export function ApiKeysPage() {
  const [keys, setKeys] = useState<SdkApiKeySummary[]>([]);
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [form] = Form.useForm();

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
    const created = await createSdkApiKey(values);
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
          message="API-Key 是后台手动签发给 SDK 的 app 作用域凭证，请使用 X-Tikee-API-Key 请求头；明文只在创建后显示一次。"
        />
        <Table<SdkApiKeySummary>
          rowKey="id"
          loading={loading}
          dataSource={keys}
          columns={[
            { title: '名称', dataIndex: 'name' },
            { title: 'Key 前缀', dataIndex: 'key_prefix' },
            { title: '范围', render: (_, item) => `${item.namespace}/${item.app}` },
            { title: 'Scopes', render: (_, item) => <Space wrap>{item.scopes.map((scope) => <Tag key={scope}>{scope}</Tag>)}</Space> },
            { title: '状态', dataIndex: 'status', render: (status) => <Tag color={status === 'active' ? 'green' : 'default'}>{status}</Tag> },
            { title: '最近使用', dataIndex: 'last_used_at', render: (value) => value ?? '-' },
            { title: '创建人', dataIndex: 'created_by' },
            { title: '操作', render: (_, item) => <Button danger size="small" icon={<DeleteOutlined />} disabled={item.status !== 'active'} onClick={() => void handleRevoke(item.id)}>吊销</Button> },
          ]}
        />
      </Card>

      <Modal title="签发 SDK API-Key" open={open} onOk={() => void handleCreate()} onCancel={() => setOpen(false)} okText="签发">
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
        </Form>
      </Modal>

      <Modal title="API-Key 只显示一次" open={createdKey !== null} onCancel={() => setCreatedKey(null)} footer={<Button onClick={() => setCreatedKey(null)}>我已保存</Button>}>
        <Alert type="warning" showIcon message="请立即复制保存，服务端不会保存明文。" style={{ marginBottom: 12 }} />
        <Typography.Text code copyable={{ text: createdKey ?? '', icon: <CopyOutlined /> }}>{createdKey}</Typography.Text>
      </Modal>
    </Space>
  );
}
