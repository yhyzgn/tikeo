import { Alert, Button, Card, Descriptions, Drawer, Form, Input, InputNumber, Modal, Select, Space, Spin, Switch, Table, Tag, Typography, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { diffLines } from 'diff';
import type { ScriptDiffResult, ScriptExecutionPolicy, ScriptSummary, ScriptVersionSummary } from '../api/client';
import { GuardedButton, PermissionGate, useCan } from '../components/Permission';
import {
  createScript,
  deleteScript,
  diffScriptVersions,
  getScript,
  listScriptVersions,
  listScripts,
  publishScript,
  rollbackScript,
  updateScript,
} from '../api/client';
import { CodeEditor } from '../components/CodeEditor';
import { useUrlQueryState } from '../hooks/useUrlQueryState';
import { TABLE_PAGE_SIZE_OPTIONS, usePersistentTablePageSize } from '../utils/pagination';

const LANGUAGE_OPTIONS = [
  { value: 'shell', label: 'Shell' },
  { value: 'python', label: 'Python' },
  { value: 'js', label: 'JavaScript' },
  { value: 'ts', label: 'TypeScript' },
  { value: 'powershell', label: 'PowerShell' },
  { value: 'rhai', label: 'Rhai' },
  { value: 'wasm', label: 'WASM' },
];

const SANDBOX_OPTIONS = [
  { value: 'auto', label: 'Auto（自适应，默认）' },
  { value: 'wasmtime', label: 'Wasmtime' },
  { value: 'wasmedge', label: 'WasmEdge' },
  { value: 'srt', label: 'Anthropic Sandbox Runtime (srt)' },
  { value: 'deno', label: 'Deno' },
  { value: 'v8', label: 'V8 isolate' },
  { value: 'docker', label: 'Docker' },
  { value: 'podman', label: 'Podman' },
  { value: 'custom', label: 'Custom' },
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

const DEFAULT_SCRIPT_POLICY: ScriptExecutionPolicy = {
  resources: { timeout_ms: 30_000, max_memory_bytes: 64 * 1024 * 1024, max_output_bytes: 1024 * 1024 },
  network: { enabled: false, allowed_hosts: [] },
  filesystem: { read_only_paths: [], writable_paths: [] },
  secrets: { refs: [] },
  env_vars: [],
  sandbox: { backend: 'auto' },
};

function policyFromForm(values: Record<string, unknown>): ScriptExecutionPolicy {
  return {
    resources: {
      timeout_ms: Number(values.policy_timeout_ms ?? DEFAULT_SCRIPT_POLICY.resources.timeout_ms / 1000) * 1000,
      max_memory_bytes: Number(values.policy_max_memory_bytes ?? DEFAULT_SCRIPT_POLICY.resources.max_memory_bytes),
      max_output_bytes: Number(values.policy_max_output_bytes ?? DEFAULT_SCRIPT_POLICY.resources.max_output_bytes),
    },
    network: { enabled: false, allowed_hosts: [] },
    filesystem: { read_only_paths: [], writable_paths: [] },
    secrets: { refs: [] },
    env_vars: Array.isArray(values.policy_env_vars) ? values.policy_env_vars as string[] : [],
    sandbox: { backend: String(values.policy_sandbox_backend ?? 'auto') },
  };
}

function policyToForm(policy?: ScriptExecutionPolicy) {
  const p = policy ?? DEFAULT_SCRIPT_POLICY;
  return {
    policy_timeout_ms: Math.floor(p.resources.timeout_ms / 1000),
    policy_max_memory_bytes: p.resources.max_memory_bytes,
    policy_max_output_bytes: p.resources.max_output_bytes,
    policy_env_vars: p.env_vars,
    policy_sandbox_backend: p.sandbox?.backend ?? 'auto',
  };
}

function policySummary(policy?: ScriptExecutionPolicy): string {
  const p = policy ?? DEFAULT_SCRIPT_POLICY;
  return `timeout=${p.resources.timeout_ms}ms, memory=${p.resources.max_memory_bytes}B, output=${p.resources.max_output_bytes}B, network=${p.network.enabled ? 'allow' : 'deny'}, fs=${p.filesystem.read_only_paths.length + p.filesystem.writable_paths.length}, secrets=${p.secrets.refs.length}, sandbox=${p.sandbox?.backend ?? 'auto'}`;
}

function scriptCapabilitySummary(script?: ScriptSummary): string {
  if (!script) return '-';
  if (script.language === 'wasm') {
    return '直接 WASM 模块模式：Worker 需声明 script:wasm 能力并启用 Wasmtime 沙箱执行器；此模式用于原生 WASI/WASM 插件';
  }
  return `Worker 需声明 script:${script.language} 语言能力；默认 sandbox=auto：可编译到 WASM 时优先 Wasmtime，原生命令/二进制优先 srt，JS/TS 逻辑优先 Deno，未匹配时回退 Wasmtime；也可手动指定 wasmtime/wasmedge/srt/deno/v8/docker/podman/custom`;
}

function shortDigest(value?: string | null): string {
  return value ? `${value.slice(0, 12)}…${value.slice(-8)}` : '-';
}

function defaultFuel(script: ScriptSummary): string {
  return script.language === 'wasm' ? '10000000' : '-';
}

function buildUnifiedDiff(oldText: string, newText: string): string {
  const changes = diffLines(oldText, newText);
  const lines: string[] = ['--- 原始内容', '+++ 修改内容'];
  for (const part of changes) {
    const prefix = part.added ? '+' : part.removed ? '-' : ' ';
    for (const line of part.value.replace(/\n$/, '').split('\n')) {
      lines.push(`${prefix}${line}`);
    }
  }
  return lines.join('\n');
}

function buildPolicyDiff(
  original: Record<string, unknown>,
  modified: Record<string, unknown>,
): { field: string; before: string; after: string }[] {
  const fields: Array<{ label: string; key: string; format?: (v: unknown) => string }> = [
    { label: '名称', key: 'name' },
    { label: '语言', key: 'language' },
    { label: '版本', key: 'version' },
    { label: '超时(秒)', key: 'timeout_seconds' },
    { label: '内存限制(字节)', key: 'max_memory_bytes' },
    { label: '允许网络', key: 'allow_network', format: (v) => (v ? '允许' : '禁止') },
    { label: '允许的环境变量', key: 'allowed_env_vars', format: (v) => (Array.isArray(v) ? v.join(', ') : String(v ?? '')) },
    { label: '策略超时(秒)', key: 'policy_timeout_ms' },
    { label: '策略内存限制(字节)', key: 'policy_max_memory_bytes' },
    { label: '策略输出限制(字节)', key: 'policy_max_output_bytes' },
    { label: '策略环境变量', key: 'policy_env_vars', format: (v) => (Array.isArray(v) ? v.join(', ') : String(v ?? '')) },
  ];
  const changes: { field: string; before: string; after: string }[] = [];
  for (const f of fields) {
    const fmt = f.format ?? ((v: unknown) => String(v ?? ''));
    const oldVal = fmt(original[f.key]);
    const newVal = fmt(modified[f.key]);
    if (oldVal !== newVal) {
      changes.push({ field: f.label, before: oldVal || '(空)', after: newVal || '(空)' });
    }
  }
  return changes;
}

function errorMessage(prefix: string, err: unknown): string {
  const reason = err instanceof Error ? err.message : '';
  return reason ? `${prefix}: ${reason}` : prefix;
}

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
  const navigate = useNavigate();
  const canManageScripts = useCan('scripts', 'manage');
  const [pageSize, setPageSize] = usePersistentTablePageSize();
  const queryDefaults = useMemo(() => ({ page: 1, page_size: pageSize, keyword: '', language: '', status: '' }), [pageSize]);
  const { query, setQuery } = useUrlQueryState(queryDefaults);
  const [scripts, setScripts] = useState<ScriptSummary[]>([]);
  const [loading, setLoading] = useState(false);

  // Create drawer
  const [createDrawerOpen, setCreateDrawerOpen] = useState(false);
  const [form] = Form.useForm();
  const currentLanguage = Form.useWatch('language', form) ?? 'shell';

  // View detail drawer
  const [detailDrawerOpen, setDetailDrawerOpen] = useState(false);
  const [detailScript, setDetailScript] = useState<ScriptSummary | null>(null);
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
    } catch (err) {
      message.error(errorMessage('加载脚本列表失败', err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  // Create
  const handleCreate = async () => {
    if (!canManageScripts) { message.error('当前账号无权限管理脚本'); return; }
    try {
      const values = await form.validateFields();
      await createScript({
        ...values,
        allow_network: values.allow_network ?? false,
        policy: policyFromForm(values),
      });
      message.success('脚本创建成功');
      setCreateDrawerOpen(false);
      form.resetFields();
      void load();
    } catch (err) {
      message.error(errorMessage('创建脚本失败', err));
    }
  };

  // Status transitions
  const handleStatusChange = async (id: string, status: string) => {
    try {
      await updateScript(id, { status });
      message.success('状态已更新');
      void load();
    } catch (err) {
      message.error(errorMessage('状态更新失败', err));
    }
  };

  const handlePublish = async (script: ScriptSummary) => {
    if (!canManageScripts) { message.error('当前账号无权限管理脚本'); return; }
    try {
      await publishScript(script.id);
      message.success('发布指针已更新到最新版本');
      void load();
    } catch (err) {
      message.error(errorMessage('发布失败', err));
    }
  };

  const handleRollback = async (script: ScriptSummary) => {
    if (!canManageScripts) { message.error('当前账号无权限管理脚本'); return; }
    try {
      const versionList = await listScriptVersions(script.id);
      const older = versionList
        .filter((version) => version.version_number !== script.released_version_number)
        .sort((a, b) => b.version_number - a.version_number)[0];
      if (!older) {
        message.warning('没有可回滚的历史版本');
        return;
      }
      await rollbackScript(script.id, older.version_number);
      message.success(`已回滚发布指针到版本 #${older.version_number}`);
      void load();
    } catch (err) {
      message.error(errorMessage('回滚失败', err));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteScript(id);
      message.success('脚本已删除');
      void load();
    } catch (err) {
      message.error(errorMessage('删除失败', err));
    }
  };

  // View detail
  const openDetailDrawer = async (script: ScriptSummary) => {
    setDetailDrawerOpen(true);
    setDetailLoading(true);
    try {
      const full = await getScript(script.id);
      setDetailScript({ ...full });
    } catch (err) {
      setDetailScript(script);
      message.error(errorMessage('加载脚本详情失败', err));
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
    } catch (err) {
      message.error(errorMessage('加载版本历史失败', err));
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
    } catch (err) {
      message.error(errorMessage('加载版本对比失败', err));
    } finally {
      setDiffLoading(false);
    }
  };

  const versionOptions = versions.map((v) => ({
    value: v.version_number,
    label: `v${v.version_number} - ${v.createdBy} (${new Date(v.createdAt).toLocaleString()})`,
  }));

  const filteredScripts = useMemo(() => scripts.filter((script) => {
    const keyword = String(query.keyword ?? '').trim().toLowerCase();
    const language = String(query.language ?? '').trim();
    const status = String(query.status ?? '').trim();
    const matchesKeyword = keyword === '' || [script.name, script.id, script.createdBy].some((value) => value.toLowerCase().includes(keyword));
    const matchesLanguage = language === '' || script.language === language;
    const matchesStatus = status === '' || script.status === status;
    return matchesKeyword && matchesLanguage && matchesStatus;
  }), [scripts, query.keyword, query.language, query.status]);

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '语言', dataIndex: 'language', key: 'language', render: (v: string) => v.toUpperCase() },
    { title: '版本', dataIndex: 'version', key: 'version' },
    { title: '发布版本', dataIndex: 'released_version_number', key: 'released_version_number', render: (v: number | null) => v ? `#${v}` : <Tag color="orange">未发布</Tag> },
    {
      title: '发布签名',
      dataIndex: 'release_signature',
      key: 'release_signature',
      render: (v: ScriptSummary['release_signature']) => v ? <Tag color="green">已验证</Tag> : <Tag>未配置</Tag>,
    },
    {
      title: '发布 Grants',
      dataIndex: 'release_grants',
      key: 'release_grants',
      render: (v: ScriptSummary['release_grants']) => v ? <Tag color="purple">已验证</Tag> : <Tag>未启用</Tag>,
    },
    {
      title: 'SHA-256',
      dataIndex: 'content_sha256',
      key: 'content_sha256',
      render: (v: string) => <Typography.Text code>{shortDigest(v)}</Typography.Text>,
    },
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
          <GuardedButton resource="scripts" action="manage" size="small" type="link" onClick={() => navigate(`/scripts/${record.id}/edit`)}>
            编辑
          </GuardedButton>
          <Button size="small" type="link" onClick={() => void openVersionDrawer(record)}>
            版本历史
          </Button>
          <GuardedButton
            resource="scripts"
            action="manage"
            size="small"
            type="link"
            confirmTitle="发布脚本"
            confirmDescription="确认将可执行发布指针移动到最新不可变版本？"
            onConfirm={() => void handlePublish(record)}
          >
            发布
          </GuardedButton>
          <GuardedButton
            resource="scripts"
            action="manage"
            size="small"
            type="link"
            confirmTitle="回滚发布指针"
            confirmDescription="确认回滚到最近一个非当前发布版本？"
            onConfirm={() => void handleRollback(record)}
          >
            回滚
          </GuardedButton>
          {record.status === 'draft' && (
            <GuardedButton
              resource="scripts"
              action="manage"
              size="small"
              type="link"
              confirmTitle="提交审批"
              confirmDescription="确认提交审批？审批通过后脚本将可用于生产环境。"
              onConfirm={() => void handleStatusChange(record.id, 'approved')}
            >
              提交审批
            </GuardedButton>
          )}
          {record.status === 'approved' && (
            <>
              <GuardedButton
                resource="scripts"
                action="manage"
                size="small"
                type="link"
                danger
                confirmTitle="禁用脚本"
                confirmDescription="确认禁用？禁用后脚本将无法执行。"
                onConfirm={() => void handleStatusChange(record.id, 'disabled')}
              >
                禁用
              </GuardedButton>
              <GuardedButton
                resource="scripts"
                action="manage"
                size="small"
                type="link"
                confirmTitle="回退草稿"
                confirmDescription="确认回退为草稿状态？"
                onConfirm={() => void handleStatusChange(record.id, 'draft')}
              >
                回退草稿
              </GuardedButton>
            </>
          )}
          {record.status === 'disabled' && (
            <GuardedButton
              resource="scripts"
              action="manage"
              size="small"
              type="link"
              confirmTitle="重新启用"
              confirmDescription="确认重新启用此脚本？"
              onConfirm={() => void handleStatusChange(record.id, 'approved')}
            >
              重新启用
            </GuardedButton>
          )}
          <GuardedButton
            resource="scripts"
            action="manage"
            size="small"
            type="link"
            danger
            confirmTitle="确定删除脚本？"
            confirmDescription="删除脚本会影响后续任务绑定与版本追踪，请确认。"
            onConfirm={() => void handleDelete(record.id)}
          >
            删除
          </GuardedButton>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Space wrap className="card-toolbar">
          <PermissionGate resource="scripts" action="manage"><Button type="primary" onClick={() => { form.setFieldsValue(policyToForm()); setCreateDrawerOpen(true); }}>新建脚本</Button></PermissionGate>
          <Input allowClear placeholder="搜索脚本/创建人" value={String(query.keyword ?? '')} onChange={(event) => setQuery({ keyword: event.target.value, page: 1 })} style={{ width: 220 }} />
          <Select allowClear placeholder="语言" value={query.language || undefined} onChange={(value) => setQuery({ language: value ?? '', page: 1 })} style={{ width: 150 }} options={LANGUAGE_OPTIONS} />
          <Select allowClear placeholder="状态" value={query.status || undefined} onChange={(value) => setQuery({ status: value ?? '', page: 1 })} style={{ width: 130 }} options={Object.entries(STATUS_LABELS).map(([value, label]) => ({ value, label }))} />
        </Space>
      </div>
      <Table rowKey="id" dataSource={filteredScripts} columns={columns} loading={loading} pagination={{ pageSize: Number(query.page_size) || pageSize, current: Number(query.page) || 1, showSizeChanger: true, pageSizeOptions: TABLE_PAGE_SIZE_OPTIONS.map(String), onChange: (page, nextPageSize) => { setPageSize(nextPageSize); setQuery({ page, page_size: nextPageSize }); } }} />

      {/* Create Drawer */}
      <Drawer
        title="新建脚本"
        open={createDrawerOpen}
        onClose={() => { setCreateDrawerOpen(false); form.resetFields(); }}
        width={760}
        destroyOnClose
        extra={<Space wrap className="card-toolbar"><Button onClick={() => { setCreateDrawerOpen(false); form.resetFields(); }}>取消</Button><Button type="primary" onClick={() => void handleCreate()}>创建脚本</Button></Space>}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="language" label="语言" rules={[{ required: true, message: '请选择语言' }]} initialValue="shell">
            <Select options={LANGUAGE_OPTIONS} />
          </Form.Item>
          {currentLanguage === 'wasm' && (
            <Alert
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
              message="直接 WASM 模块策略"
              description="language=wasm 用于原生 WASI/WASM 插件；Shell/Python/JS/TS/PowerShell 等脚本语言默认使用 sandbox=auto 自适应后端。模块会携带 SHA-256 摘要；默认 runtime=wasmtime、entrypoint=_start、fuel=10000000、禁止网络，签名字段预留。"
            />
          )}
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
          <Alert
            type="info"
            showIcon
            style={{ marginBottom: 16 }}
            message="动态脚本策略（默认拒绝危险能力）"
            description="当前阶段仅允许资源限制与环境变量白名单；网络、文件系统与 Secret 访问仍由后续审批/策略引擎开放。"
          />
          <Form.Item name="allowed_env_vars" label="允许的环境变量">
            <Select mode="tags" placeholder="输入变量名后回车" />
          </Form.Item>
          <Form.Item name="policy_timeout_ms" label="策略超时(秒)" initialValue={30}>
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="policy_max_memory_bytes" label="策略内存限制(字节)" initialValue={64 * 1024 * 1024}>
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="policy_max_output_bytes" label="策略输出限制(字节)" initialValue={1024 * 1024}>
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="policy_env_vars" label="策略环境变量白名单">
            <Select mode="tags" placeholder="输入变量名后回车" />
          </Form.Item>
          <Form.Item name="policy_sandbox_backend" label="沙箱后端" initialValue="auto">
            <Select options={SANDBOX_OPTIONS} />
          </Form.Item>
        </Form>
      </Drawer>

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
              <Descriptions.Item label="内容 SHA-256"><Typography.Text code copyable>{detailScript.content_sha256}</Typography.Text></Descriptions.Item>
              <Descriptions.Item label="发布版本">{detailScript.released_version_number ? `#${detailScript.released_version_number}` : '未发布'}</Descriptions.Item>
              <Descriptions.Item label="发布版本 ID"><Typography.Text code copyable>{detailScript.released_version_id ?? '-'}</Typography.Text></Descriptions.Item>
              <Descriptions.Item label="签名审批单">{detailScript.release_signature?.approval_ticket ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="签名校验时间">{detailScript.release_signature?.verified_at ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="签名校验人">{detailScript.release_signature?.verified_by ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="发布签名"><Typography.Text code copyable>{detailScript.release_signature?.signature ?? '-'}</Typography.Text></Descriptions.Item>
              <Descriptions.Item label="Grant 校验人">{detailScript.release_grants?.verified_by ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="Grant 校验时间">{detailScript.release_grants?.verified_at ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="Grant URL">{detailScript.release_grants?.url.join(', ') || '-'}</Descriptions.Item>
              <Descriptions.Item label="Grant Secret">{detailScript.release_grants?.secret.join(', ') || '-'}</Descriptions.Item>
              <Descriptions.Item label="状态">
                <Tag color={STATUS_COLORS[detailScript.status] ?? 'default'}>{STATUS_LABELS[detailScript.status] ?? detailScript.status}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="超时(秒)">{detailScript.timeout_seconds ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="内存限制(字节)">{detailScript.max_memory_bytes ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="允许网络">{detailScript.allow_network ? '允许' : '禁止'}</Descriptions.Item>
              <Descriptions.Item label="默认沙箱后端">{detailScript.language === 'wasm' ? 'wasmtime / direct WASM module' : 'auto / adaptive script sandbox'}</Descriptions.Item>
              <Descriptions.Item label="WASM Entrypoint">{detailScript.language === 'wasm' ? '_start' : '-'}</Descriptions.Item>
              <Descriptions.Item label="WASM Fuel">{defaultFuel(detailScript)}</Descriptions.Item>
              <Descriptions.Item label="模块签名">{detailScript.language === 'wasm' ? '预留，当前未启用' : '-'}</Descriptions.Item>
              <Descriptions.Item label="Worker 执行能力" span={2}>{scriptCapabilitySummary(detailScript)}</Descriptions.Item>
              <Descriptions.Item label="执行策略" span={2}>{policySummary(detailScript.policy)}</Descriptions.Item>
              <Descriptions.Item label="策略环境变量">
                {detailScript.policy.env_vars.length > 0 ? detailScript.policy.env_vars.join(', ') : '-'}
              </Descriptions.Item>
              <Descriptions.Item label="允许的环境变量">
                {detailScript.allowed_env_vars && detailScript.allowed_env_vars.length > 0
                  ? detailScript.allowed_env_vars.join(', ')
                  : '-'}
              </Descriptions.Item>
              <Descriptions.Item label="创建者">{detailScript.createdBy}</Descriptions.Item>
              <Descriptions.Item label="创建时间">{new Date(detailScript.createdAt).toLocaleString()}</Descriptions.Item>
              <Descriptions.Item label="更新时间" span={2}>{new Date(detailScript.updatedAt).toLocaleString()}</Descriptions.Item>
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
                {
                  title: '版本号',
                  dataIndex: 'version_number',
                  key: 'version_number',
                  width: 100,
                  render: (v: number) => (
                    <Space size={4}>
                      <span>#{v}</span>
                      {activeScript?.released_version_number === v && <Tag color="green">已发布</Tag>}
                    </Space>
                  ),
                },
                {
                  title: 'SHA-256',
                  dataIndex: 'content_sha256',
                  key: 'content_sha256',
                  width: 180,
                  render: (v: string) => <Typography.Text code copyable>{shortDigest(v)}</Typography.Text>,
                },
                {
                  title: '状态',
                  dataIndex: 'status',
                  key: 'status',
                  width: 80,
                  render: (v: string) => <Tag color={STATUS_COLORS[v] ?? 'default'}>{STATUS_LABELS[v] ?? v}</Tag>,
                },
                { title: '创建者', dataIndex: 'createdBy', key: 'createdBy', width: 120 },
                {
                  title: '创建时间',
                  dataIndex: 'createdAt',
                  key: 'createdAt',
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


export function ScriptEditorPage() {
  const canManageScripts = useCan('scripts', 'manage');
  const navigate = useNavigate();
  const params = useParams();
  const scriptId = params.id;
  const [form] = Form.useForm();
  const editLanguage = Form.useWatch('language', form) ?? 'shell';
  const [editingScript, setEditingScript] = useState<ScriptSummary | null>(null);
  const [originalScript, setOriginalScript] = useState<Record<string, unknown> | null>(null);
  const [hasVersions, setHasVersions] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [diffPreviewOpen, setDiffPreviewOpen] = useState(false);
  const [editContentDiff, setEditContentDiff] = useState('');
  const [editPolicyDiff, setEditPolicyDiff] = useState<{ field: string; before: string; after: string }[]>([]);

  useEffect(() => {
    if (!scriptId) return;
    setLoading(true);
    void (async () => {
      try {
        const detail = await getScript(scriptId);
        const formValues = {
          name: detail.name,
          language: detail.language,
          version: detail.version,
          content: detail.content,
          timeout_seconds: detail.timeout_seconds,
          max_memory_bytes: detail.max_memory_bytes,
          allow_network: detail.allow_network,
          allowed_env_vars: detail.allowed_env_vars,
          ...policyToForm(detail.policy),
        };
        setEditingScript(detail);
        setOriginalScript({ ...formValues });
        form.setFieldsValue(formValues);
        try {
          const versionList = await listScriptVersions(scriptId);
          setHasVersions(versionList.length > 0);
        } catch {
          setHasVersions(false);
        }
      } catch (err) {
        message.error(errorMessage('加载脚本详情失败', err));
      } finally {
        setLoading(false);
      }
    })();
  }, [form, scriptId]);

  const handleEditPreview = async () => {
    if (!canManageScripts) { message.error('当前账号无权限管理脚本'); return; }
    if (!editingScript || !originalScript) return;
    try {
      const values = await form.validateFields();
      setEditContentDiff(buildUnifiedDiff((originalScript.content as string) ?? '', values.content ?? ''));
      setEditPolicyDiff(buildPolicyDiff(originalScript, values));
      setDiffPreviewOpen(true);
    } catch {
      // form validation failed — errors shown on fields
    }
  };

  const handleEditConfirm = async () => {
    if (!editingScript) return;
    try {
      const values = await form.validateFields();
      setSaving(true);
      await updateScript(editingScript.id, {
        name: values.name,
        language: values.language,
        version: values.version,
        content: values.content,
        timeout_seconds: values.timeout_seconds,
        max_memory_bytes: values.max_memory_bytes,
        allow_network: values.allow_network,
        allowed_env_vars: values.allowed_env_vars,
        policy: policyFromForm(values),
      });
      message.success('脚本更新成功');
      setDiffPreviewOpen(false);
      navigate('/scripts');
    } catch (err) {
      message.error(errorMessage('更新脚本失败', err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div className="hero-panel script-editor-hero">
        <div className="hero-panel__content">
          <Button className="workflow-back-button" onClick={() => navigate('/scripts')}>← 返回脚本列表</Button>
          <Tag className="soft-tag" color="purple">Script Governance</Tag>
          <Typography.Title level={1}>编辑脚本</Typography.Title>
          <Typography.Paragraph className="hero-panel__desc">
            在独立页面编辑脚本内容、执行限制和策略白名单；保存前会展示内容与策略 diff，确认后生成新的不可变版本快照。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary"><strong>{editingScript?.language.toUpperCase() ?? '-'}</strong><span>runtime</span></div>
      </div>

      <Spin spinning={loading}>
        <Form form={form} layout="vertical" className="script-editor-page">
          <Card
            className="script-editor-page__main"
            title={<Space><span>{editingScript?.name ?? '脚本'}</span>{editingScript ? <Tag color={STATUS_COLORS[editingScript.status] ?? 'default'}>{STATUS_LABELS[editingScript.status] ?? editingScript.status}</Tag> : null}</Space>}
            extra={<Space wrap className="card-toolbar"><Button onClick={() => navigate('/scripts')}>取消</Button><Button type="primary" loading={saving} onClick={() => void handleEditPreview()}>预览变更</Button></Space>}
          >
            {hasVersions ? (
              <Alert type="warning" showIcon style={{ marginBottom: 16 }} message="此脚本存在历史版本，更新后将生成新版本记录。" />
            ) : null}
            {editLanguage === 'wasm' ? (
              <Alert
                type="info"
                showIcon
                style={{ marginBottom: 16 }}
                message="直接 WASM 模块策略"
                description="language=wasm 用于原生 WASI/WASM 插件；普通脚本语言默认使用 sandbox=auto 自适应后端。更新后将生成新的不可变版本快照与 SHA-256 摘要；默认 runtime=wasmtime、entrypoint=_start、fuel=10000000、禁止网络，签名字段预留。"
              />
            ) : null}
            <div className="script-editor-page__grid">
              <div className="script-editor-page__code">
                <Form.Item name="content" label="脚本内容" rules={[{ required: true, message: '请输入脚本内容' }]}>
                  <CodeEditor value={form.getFieldValue('content') ?? ''} onChange={(val) => form.setFieldValue('content', val)} language={editLanguage} />
                </Form.Item>
              </div>
              <div className="script-editor-page__side">
                <Card size="small" title="基本信息">
                  <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}><Input /></Form.Item>
                  <Form.Item name="language" label="语言" rules={[{ required: true, message: '请选择语言' }]}><Select options={LANGUAGE_OPTIONS} /></Form.Item>
                  <Form.Item name="version" label="版本"><Input /></Form.Item>
                  {editingScript ? <Typography.Text type="secondary">SHA-256：<Typography.Text code>{shortDigest(editingScript.content_sha256)}</Typography.Text></Typography.Text> : null}
                </Card>
                <Card size="small" title="运行限制">
                  <Form.Item name="timeout_seconds" label="超时(秒)"><InputNumber min={1} style={{ width: '100%' }} /></Form.Item>
                  <Form.Item name="max_memory_bytes" label="内存限制(字节)"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
                  <Form.Item name="allow_network" label="允许网络" valuePropName="checked"><Switch /></Form.Item>
                  <Form.Item name="allowed_env_vars" label="允许的环境变量"><Select mode="tags" placeholder="输入变量名后回车" /></Form.Item>
                </Card>
                <Card size="small" title="执行策略">
                  <Alert type="info" showIcon style={{ marginBottom: 16 }} message="默认拒绝危险能力" description="当前阶段仅允许资源限制与环境变量白名单；网络、文件系统与 Secret 访问仍由后续审批/策略引擎开放。" />
                  <Form.Item name="policy_timeout_ms" label="策略超时(秒)"><InputNumber min={1} style={{ width: '100%' }} /></Form.Item>
                  <Form.Item name="policy_max_memory_bytes" label="策略内存限制(字节)"><InputNumber min={1} style={{ width: '100%' }} /></Form.Item>
                  <Form.Item name="policy_max_output_bytes" label="策略输出限制(字节)"><InputNumber min={1} style={{ width: '100%' }} /></Form.Item>
                  <Form.Item name="policy_env_vars" label="策略环境变量白名单"><Select mode="tags" placeholder="输入变量名后回车" /></Form.Item>
                  <Form.Item name="policy_sandbox_backend" label="沙箱后端"><Select options={SANDBOX_OPTIONS} /></Form.Item>
                </Card>
              </div>
            </div>
          </Card>
        </Form>
      </Spin>

      <Modal
        title="变更预览"
        open={diffPreviewOpen}
        onCancel={() => setDiffPreviewOpen(false)}
        width={900}
        footer={[
          <Button key="back" onClick={() => setDiffPreviewOpen(false)}>返回编辑</Button>,
          <Button key="confirm" type="primary" loading={saving} onClick={() => void handleEditConfirm()}>确认保存</Button>,
        ]}
      >
        {editPolicyDiff.length === 0 && editContentDiff.split('\n').filter((line) => line.startsWith('+') || line.startsWith('-')).length <= 2 ? (
          <div style={{ color: '#888', textAlign: 'center', padding: 24 }}>未检测到变更</div>
        ) : (
          <>
            {editPolicyDiff.length > 0 ? <><h4>策略变更</h4><PolicyDiffTable changes={editPolicyDiff} /></> : null}
            <h4 style={{ marginTop: editPolicyDiff.length > 0 ? 24 : 0 }}>代码变更</h4>
            <DiffContent diff={editContentDiff} />
          </>
        )}
      </Modal>
    </Space>
  );
}
