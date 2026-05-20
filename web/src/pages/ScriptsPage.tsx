import { Button, Descriptions, Drawer, Form, Input, InputNumber, Modal, Popconfirm, Select, Space, Spin, Switch, Table, Tag, message } from 'antd';
import { useEffect, useState } from 'react';
import type { ScriptDiffResult, ScriptSummary, ScriptVersionSummary } from '../api/client';
import {
  createScript,
  deleteScript,
  diffScriptVersions,
  getScript,
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

const STATUS_LABELS: Record<string, string> = {
  draft: '草稿',
  approved: '已审批',
  disabled: '已禁用',
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

type ScriptDetail = ScriptSummary & { content?: string };

export function ScriptsPage() {
  const [scripts, setScripts] = useState<ScriptSummary[]>([]);
  const [loading, setLoading] = useState(false);

  // Create modal
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();
  const currentLanguage = Form.useWatch('language', form) ?? 'shell';

  // Edit modal
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editingScript, setEditingScript] = useState<ScriptDetail | null>(null);
  const [editForm] = Form.useForm();
  const editLanguage = Form.useWatch('language', editForm) ?? 'shell';
  const [editHasVersions, setEditHasVersions] = useState(false);

  // View detail drawer
  const [detailDrawerOpen, setDetailDrawerOpen] = useState(false);
  const [detailScript, setDetailScript] = useState<ScriptDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // Version history drawer
  const [versionDrawerOpen, setVersionDrawerOpen] = useState(false);
  const [activeScript, setActiveScript] = useState<ScriptSummary | null>(null);
  const [versions, setVersions] = useState<ScriptVersionSummary[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);
  const [selectedV1, setSelectedV1] = useState<number | null>(null);
  const [selectedV2, setSelectedV2] = useState<number | null>(null);
  const [diffResult, setDiffResult] = useState<ScriptDiffResult | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

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

  // Create
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

  // Edit
  const openEditModal = async (script: ScriptSummary) => {
    setEditLoading(true);
    try {
      const full = await getScript(script.id);
      const detail: ScriptDetail = { ...script, content: (full as ScriptSummary & { content?: string }).content ?? '' };
      setEditingScript(detail);
      editForm.setFieldsValue({
        name: detail.name,
        language: detail.language,
        version: detail.version,
        content: detail.content,
        timeout_seconds: detail.timeout_seconds,
        max_memory_bytes: detail.max_memory_bytes,
        allow_network: detail.allow_network,
        allowed_env_vars: detail.allowed_env_vars,
      });
      // Check if script has versions for diff hint
      try {
        const vList = await listScriptVersions(script.id);
        setEditHasVersions(vList.length > 0);
      } catch {
        setEditHasVersions(false);
      }
      setEditModalOpen(true);
    } catch {
      message.error('加载脚本详情失败');
    } finally {
      setEditLoading(false);
    }
  };

  const [editLoading, setEditLoading] = useState(false);

  const handleEdit = async () => {
    if (!editingScript) return;
    try {
      const values = await editForm.validateFields();
      await updateScript(editingScript.id, {
        name: values.name,
        language: values.language,
        version: values.version,
        content: values.content,
        timeout_seconds: values.timeout_seconds,
        max_memory_bytes: values.max_memory_bytes,
        allow_network: values.allow_network,
        allowed_env_vars: values.allowed_env_vars,
      });
      message.success('脚本更新成功');
      setEditModalOpen(false);
      editForm.resetFields();
      setEditingScript(null);
      void load();
    } catch {
      message.error('更新脚本失败');
    }
  };

  // Status transitions
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

  // View detail
  const openDetailDrawer = async (script: ScriptSummary) => {
    setDetailDrawerOpen(true);
    setDetailLoading(true);
    try {
      const full = await getScript(script.id);
      setDetailScript({ ...full, content: (full as ScriptSummary & { content?: string }).content ?? '' });
    } catch {
      setDetailScript(script);
      message.error('加载脚本详情失败');
    } finally {
      setDetailLoading(false);
    }
  };

  // Version history
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
    if (!activeScript || selectedV1 === null || selectedV2 === null) return;
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
      render: (v: string) => <Tag color={STATUS_COLORS[v] ?? 'default'}>{STATUS_LABELS[v] ?? v}</Tag>,
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
        <Space size="small" wrap>
          <Button size="small" type="link" onClick={() => void openDetailDrawer(record)}>
            查看
          </Button>
          <Button size="small" type="link" onClick={() => void openEditModal(record)}>
            编辑
          </Button>
          <Button size="small" type="link" onClick={() => void openVersionDrawer(record)}>
            版本历史
          </Button>
          {record.status === 'draft' && (
            <Popconfirm
              title="提交审批"
              description="确认提交审批？审批通过后脚本将可用于生产环境。"
              onConfirm={() => void handleStatusChange(record.id, 'approved')}
            >
              <Button size="small" type="link">提交审批</Button>
            </Popconfirm>
          )}
          {record.status === 'approved' && (
            <>
              <Popconfirm
                title="禁用脚本"
                description="确认禁用？禁用后脚本将无法执行。"
                onConfirm={() => void handleStatusChange(record.id, 'disabled')}
              >
                <Button size="small" type="link" danger>禁用</Button>
              </Popconfirm>
              <Popconfirm
                title="回退草稿"
                description="确认回退为草稿状态？"
                onConfirm={() => void handleStatusChange(record.id, 'draft')}
              >
                <Button size="small" type="link">回退草稿</Button>
              </Popconfirm>
            </>
          )}
          {record.status === 'disabled' && (
            <Popconfirm
              title="重新启用"
              description="确认重新启用此脚本？"
              onConfirm={() => void handleStatusChange(record.id, 'approved')}
            >
              <Button size="small" type="link">重新启用</Button>
            </Popconfirm>
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

      {/* Create Modal */}
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

      {/* Edit Modal */}
      <Modal
        title={`编辑脚本 - ${editingScript?.name ?? ''}`}
        open={editModalOpen}
        onOk={handleEdit}
        onCancel={() => { setEditModalOpen(false); editForm.resetFields(); setEditingScript(null); }}
        width={700}
        confirmLoading={editLoading}
      >
        {editHasVersions && (
          <div style={{ marginBottom: 12, padding: '8px 12px', background: '#fffbe6', border: '1px solid #ffe58f', borderRadius: 6, fontSize: 13 }}>
            此脚本存在历史版本，更新后将生成新版本记录。
          </div>
        )}
        <Form form={editForm} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="language" label="语言" rules={[{ required: true, message: '请选择语言' }]}>
            <Select options={LANGUAGE_OPTIONS} />
          </Form.Item>
          <Form.Item name="version" label="版本">
            <Input />
          </Form.Item>
          <Form.Item name="content" label="脚本内容" rules={[{ required: true, message: '请输入脚本内容' }]}>
            <CodeEditor
              value={editForm.getFieldValue('content') ?? ''}
              onChange={(val) => editForm.setFieldValue('content', val)}
              language={editLanguage}
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

      {/* View Detail Drawer */}
      <Drawer
        title={`脚本详情 - ${detailScript?.name ?? ''}`}
        open={detailDrawerOpen}
        onClose={() => { setDetailDrawerOpen(false); setDetailScript(null); }}
        width={720}
      >
        {detailLoading ? (
          <Spin />
        ) : detailScript ? (
          <div>
            <Descriptions size="small" column={2} bordered>
              <Descriptions.Item label="名称">{detailScript.name}</Descriptions.Item>
              <Descriptions.Item label="语言">{detailScript.language.toUpperCase()}</Descriptions.Item>
              <Descriptions.Item label="版本">{detailScript.version}</Descriptions.Item>
              <Descriptions.Item label="状态">
                <Tag color={STATUS_COLORS[detailScript.status] ?? 'default'}>{STATUS_LABELS[detailScript.status] ?? detailScript.status}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="超时(秒)">{detailScript.timeout_seconds ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="内存限制(字节)">{detailScript.max_memory_bytes ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="允许网络">{detailScript.allow_network ? '允许' : '禁止'}</Descriptions.Item>
              <Descriptions.Item label="允许的环境变量">
                {detailScript.allowed_env_vars && detailScript.allowed_env_vars.length > 0
                  ? detailScript.allowed_env_vars.join(', ')
                  : '-'}
              </Descriptions.Item>
              <Descriptions.Item label="创建者">{detailScript.created_by}</Descriptions.Item>
              <Descriptions.Item label="创建时间">{new Date(detailScript.created_at).toLocaleString()}</Descriptions.Item>
              <Descriptions.Item label="更新时间" span={2}>{new Date(detailScript.updated_at).toLocaleString()}</Descriptions.Item>
            </Descriptions>
            <h4 style={{ marginTop: 24 }}>脚本内容</h4>
            <CodeEditor
              value={detailScript.content ?? ''}
              onChange={() => {}}
              language={detailScript.language}
              readOnly
            />
          </div>
        ) : null}
      </Drawer>

      {/* Version History Drawer */}
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
                  render: (v: string) => <Tag color={STATUS_COLORS[v] ?? 'default'}>{STATUS_LABELS[v] ?? v}</Tag>,
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

            <Descriptions title="版本对比" size="small" style={{ marginTop: 24 }} column={3}>
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
