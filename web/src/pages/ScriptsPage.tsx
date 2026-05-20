import { Button, Descriptions, Drawer, Form, Input, InputNumber, Modal, Popconfirm, Select, Space, Spin, Switch, Table, Tag, message } from 'antd';
import { useEffect, useState } from 'react';
import type { ScriptDiffResult, ScriptSummary, ScriptVersionSummary } from '../api/client';
import {
  createScript,
  deleteScript,
  diffScriptVersions,
  listScriptVersions,
  listScripts,
  updateScript,
} from '../api/client';
import { CodeEditor } from '../components/CodeEditor';

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

function DiffContent({ diff }: { diff: string }) {
  const lines = diff.split('\n');
  return (
    <pre
      style={{
        background: '#1e1e1e',
        color: '#d4d4d4',
        padding: 16,
        borderRadius: 6,
        overflow: 'auto',
        maxHeight: 400,
        fontSize: 13,
        lineHeight: 1.6,
        margin: 0,
      }}
    >
      {lines.map((line, i) => {
        let color = '#d4d4d4';
        if (line.startsWith('+') && !line.startsWith('+++')) {
          color = '#4ec9b0';
        } else if (line.startsWith('-') && !line.startsWith('---')) {
          color = '#f44747';
        } else if (line.startsWith('@@')) {
          color = '#569cd6';
        }
        return (
          <div key={i} style={{ color }}>
            {line}
          </div>
        );
      })}
    </pre>
  );
}

function PolicyDiffTable({ changes }: { changes: ScriptDiffResult['policy_diff'] }) {
  if (changes.length === 0) {
    return <div style={{ color: '#888' }}>无策略变更</div>;
  }
  return (
    <Table
      size="small"
      pagination={false}
      dataSource={changes}
      rowKey="field"
      columns={[
        { title: '字段', dataIndex: 'field', key: 'field', width: 200 },
        {
          title: '变更前',
          dataIndex: 'before',
          key: 'before',
          render: (v: string) => (
            <span style={{ color: '#f44747', fontFamily: 'monospace' }}>{v || '(空)'}</span>
          ),
        },
        {
          title: '变更后',
          dataIndex: 'after',
          key: 'after',
          render: (v: string) => (
            <span style={{ color: '#4ec9b0', fontFamily: 'monospace' }}>{v || '(空)'}</span>
          ),
        },
      ]}
    />
  );
}

export function ScriptsPage() {
  const [scripts, setScripts] = useState<ScriptSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();

  const [versionDrawerOpen, setVersionDrawerOpen] = useState(false);
  const [activeScript, setActiveScript] = useState<ScriptSummary | null>(null);
  const [versions, setVersions] = useState<ScriptVersionSummary[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);

  const [selectedV1, setSelectedV1] = useState<number | null>(null);
  const [selectedV2, setSelectedV2] = useState<number | null>(null);
  const [diffResult, setDiffResult] = useState<ScriptDiffResult | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

  const currentLanguage = Form.useWatch('language', form) ?? 'shell';

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

  const openVersionDrawer = async (script: ScriptSummary) => {
    setActiveScript(script);
    setVersionDrawerOpen(true);
    setDiffResult(null);
    setSelectedV1(null);
    setSelectedV2(null);
    setVersionsLoading(true);
    try {
      const list = await listScriptVersions(script.id);
      setVersions(list);
    } catch {
      message.error('加载版本历史失败');
    } finally {
      setVersionsLoading(false);
    }
  };

  const handleDiff = async () => {
    if (!activeScript || selectedV1 === null || selectedV2 === null) {
      return;
    }
    setDiffLoading(true);
    try {
      const result = await diffScriptVersions(activeScript.id, selectedV1, selectedV2);
      setDiffResult(result);
    } catch {
      message.error('加载版本对比失败');
    } finally {
      setDiffLoading(false);
    }
  };

  const versionOptions = versions.map((v) => ({
    value: v.version_number,
    label: `v${v.version_number} - ${v.created_by} (${new Date(v.created_at).toLocaleString()})`,
  }));

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
          <Button size="small" type="link" onClick={() => void openVersionDrawer(record)}>
            版本历史
          </Button>
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

      <Modal
        title="新建脚本"
        open={modalOpen}
        onOk={handleCreate}
        onCancel={() => { setModalOpen(false); form.resetFields(); }}
        width={700}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="language" label="语言" rules={[{ required: true, message: '请选择语言' }]} initialValue="shell">
            <Select options={LANGUAGE_OPTIONS} />
          </Form.Item>
          <Form.Item name="version" label="版本" initialValue="1.0.0">
            <Input />
          </Form.Item>
          <Form.Item name="content" label="脚本内容" rules={[{ required: true, message: '请输入脚本内容' }]}>
            <CodeEditor
              value={form.getFieldValue('content') ?? ''}
              onChange={(val) => form.setFieldValue('content', val)}
              language={currentLanguage}
            />
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

      <Drawer
        title={`版本历史 - ${activeScript?.name ?? ''}`}
        open={versionDrawerOpen}
        onClose={() => setVersionDrawerOpen(false)}
        width={720}
      >
        {versionsLoading ? (
          <Spin />
        ) : (
          <>
            <Table
              size="small"
              pagination={false}
              dataSource={versions}
              rowKey="id"
              columns={[
                { title: '版本号', dataIndex: 'version_number', key: 'version_number', width: 80 },
                {
                  title: '状态',
                  dataIndex: 'status',
                  key: 'status',
                  width: 80,
                  render: (v: string) => <Tag color={STATUS_COLORS[v] ?? 'default'}>{v}</Tag>,
                },
                { title: '创建者', dataIndex: 'created_by', key: 'created_by', width: 120 },
                {
                  title: '创建时间',
                  dataIndex: 'created_at',
                  key: 'created_at',
                  render: (v: string) => new Date(v).toLocaleString(),
                },
              ]}
            />

            <Descriptions
              title="版本对比"
              size="small"
              style={{ marginTop: 24 }}
              column={3}
            >
              <Descriptions.Item label="版本 A">
                <Select
                  style={{ width: 260 }}
                  placeholder="选择版本 A"
                  options={versionOptions}
                  value={selectedV1}
                  onChange={setSelectedV1}
                />
              </Descriptions.Item>
              <Descriptions.Item label="版本 B">
                <Select
                  style={{ width: 260 }}
                  placeholder="选择版本 B"
                  options={versionOptions}
                  value={selectedV2}
                  onChange={setSelectedV2}
                />
              </Descriptions.Item>
              <Descriptions.Item>
                <Button
                  type="primary"
                  onClick={() => void handleDiff()}
                  loading={diffLoading}
                  disabled={selectedV1 === null || selectedV2 === null}
                >
                  对比
                </Button>
              </Descriptions.Item>
            </Descriptions>

            {diffResult && (
              <div style={{ marginTop: 16 }}>
                <h4>代码变更</h4>
                <DiffContent diff={diffResult.content_diff} />

                <h4 style={{ marginTop: 24 }}>策略变更</h4>
                <PolicyDiffTable changes={diffResult.policy_diff} />
              </div>
            )}
          </>
        )}
      </Drawer>
    </div>
  );
}
