import { Button, Form, Input, InputNumber, Modal, Popconfirm, Select, Space, Switch, Table, Tag, message } from 'antd';
import { useEffect, useState } from 'react';
import type { ScriptSummary } from '../api/client';
import { createScript, deleteScript, listScripts, updateScript } from '../api/client';

const LANGUAGE_OPTIONS = [
  { value: 'shell', label: 'Shell' },
  { value: 'python', label: 'Python' },
  { value: 'node', label: 'Node.js' },
  { value: 'powershell', label: 'PowerShell' },
  { value: 'rhai', label: 'Rhai' },
  { value: 'wasm', label: 'WASM' },
];

const STATUS_COLORS: Record<string, string> = {
  draft: 'orange',
  approved: 'green',
  disabled: 'red',
};

export function ScriptsPage() {
  const [scripts, setScripts] = useState<ScriptSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();

  const load = async () => {
    setLoading(true);
    try {
      const page = await listScripts();
      setScripts(page.items);
    } catch {
      message.error('加载脚本列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      await createScript({
        ...values,
        allow_network: values.allow_network ?? false,
      });
      message.success('脚本创建成功');
      setModalOpen(false);
      form.resetFields();
      void load();
    } catch {
      message.error('创建脚本失败');
    }
  };

  const handleStatusChange = async (id: string, status: string) => {
    try {
      await updateScript(id, { status });
      message.success('状态已更新');
      void load();
    } catch {
      message.error('状态更新失败');
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteScript(id);
      message.success('脚本已删除');
      void load();
    } catch {
      message.error('删除失败');
    }
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '语言', dataIndex: 'language', key: 'language', render: (v: string) => v.toUpperCase() },
    { title: '版本', dataIndex: 'version', key: 'version' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (v: string) => <Tag color={STATUS_COLORS[v] ?? 'default'}>{v}</Tag>,
    },
    { title: '网络', dataIndex: 'allow_network', key: 'allow_network', render: (v: boolean) => v ? '允许' : '禁止' },
    {
      title: '超时(秒)',
      dataIndex: 'timeout_seconds',
      key: 'timeout_seconds',
      render: (v: number | null) => v ?? '-',
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: unknown, record: ScriptSummary) => (
        <Space>
          {record.status === 'draft' && (
            <Button size="small" type="link" onClick={() => void handleStatusChange(record.id, 'approved')}>审批</Button>
          )}
          {record.status !== 'disabled' && (
            <Button size="small" type="link" danger onClick={() => void handleStatusChange(record.id, 'disabled')}>禁用</Button>
          )}
          <Popconfirm title="确定删除？" onConfirm={() => void handleDelete(record.id)}>
            <Button size="small" type="link" danger>删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Button type="primary" onClick={() => setModalOpen(true)}>新建脚本</Button>
      </div>
      <Table rowKey="id" dataSource={scripts} columns={columns} loading={loading} pagination={false} />
      <Modal title="新建脚本" open={modalOpen} onOk={handleCreate} onCancel={() => { setModalOpen(false); form.resetFields(); }} width={600}>
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="language" label="语言" rules={[{ required: true, message: '请选择语言' }]}>
            <Select options={LANGUAGE_OPTIONS} />
          </Form.Item>
          <Form.Item name="version" label="版本" initialValue="1.0.0">
            <Input />
          </Form.Item>
          <Form.Item name="content" label="脚本内容" rules={[{ required: true, message: '请输入脚本内容' }]}>
            <Input.TextArea rows={6} />
          </Form.Item>
          <Form.Item name="timeout_seconds" label="超时(秒)">
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="max_memory_bytes" label="内存限制(字节)">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="allow_network" label="允许网络" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="allowed_env_vars" label="允许的环境变量">
            <Select mode="tags" placeholder="输入变量名后回车" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
