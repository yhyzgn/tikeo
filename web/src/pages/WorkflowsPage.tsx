import { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Col, Form, Input, List, Row, Segmented, Space, Tag, Timeline, Typography, message } from 'antd';
import {
  advanceWorkflowInstance,
  createWorkflow,
  dryRunWorkflow,
  getAuthToken,
  listWorkflows,
  runWorkflow,
  validateWorkflow,
  workflowEventStreamUrl,
  type InstanceEventSummary,
  type WorkflowDefinition,
  type WorkflowDryRunResponse,
  type WorkflowInstanceSummary,
  type WorkflowSummary,
} from '../api/client';

const DEFAULT_DEFINITION = JSON.stringify({
  nodes: [
    { key: 'extract', name: 'Extract', kind: 'job', job_id: 'job_extract' },
    { key: 'map-users', name: 'Map users', kind: 'map', map_items: [{ shard: 1 }, { shard: 2 }] },
    { key: 'reduce', name: 'Reduce', kind: 'map_reduce', map_items: [{ shard: 1 }, { shard: 2 }] },
  ],
  edges: [
    { from: 'extract', to: 'map-users', condition: 'on_success' },
    { from: 'map-users', to: 'reduce', condition: 'on_success' },
  ],
}, null, 2);

const STATUS_COLORS: Record<string, string> = {
  active: 'blue',
  pending: 'gold',
  waiting: 'default',
  queued: 'geekblue',
  running: 'processing',
  succeeded: 'success',
  failed: 'error',
  skipped: 'warning',
};

function parseDefinition(raw: string): WorkflowDefinition {
  return JSON.parse(raw) as WorkflowDefinition;
}

function definitionToYaml(definition: WorkflowDefinition): string {
  const lines = ['nodes:'];
  for (const node of definition.nodes) {
    lines.push(`  - key: ${node.key}`);
    if (node.name) lines.push(`    name: ${node.name}`);
    if (node.kind) lines.push(`    kind: ${node.kind}`);
    if (node.job_id) lines.push(`    job_id: ${node.job_id}`);
    if (node.child_workflow_id) lines.push(`    child_workflow_id: ${node.child_workflow_id}`);
    if (node.map_items) lines.push(`    map_items: ${JSON.stringify(node.map_items)}`);
  }
  lines.push('edges:');
  for (const edge of definition.edges) {
    lines.push(`  - from: ${edge.from}`);
    lines.push(`    to: ${edge.to}`);
    lines.push(`    condition: ${edge.condition ?? 'on_success'}`);
  }
  return lines.join('\n');
}

function DagPreview({ definition, instance }: { definition: WorkflowDefinition; instance?: WorkflowInstanceSummary | null }) {
  const statuses = new Map(instance?.nodes.map((node) => [node.node_key, node.status]) ?? []);
  return (
    <div className="workflow-dag">
      {definition.nodes.map((node, index) => {
        const outgoing = definition.edges.filter((edge) => edge.from === node.key);
        const status = statuses.get(node.key) ?? 'design';
        return (
          <div className="workflow-dag__node" key={node.key}>
            <div className="workflow-dag__badge">{index + 1}</div>
            <div className="workflow-dag__content">
              <Space wrap>
                <Typography.Text strong>{node.name ?? node.key}</Typography.Text>
                <Tag color="cyan">{node.kind ?? 'job'}</Tag>
                <Tag color={STATUS_COLORS[status] ?? 'default'}>{status}</Tag>
              </Space>
              <Typography.Text type="secondary" className="workflow-dag__meta">
                {node.key}{node.job_id ? ` · job ${node.job_id}` : ''}{node.child_workflow_id ? ` · child ${node.child_workflow_id}` : ''}
              </Typography.Text>
              {outgoing.length > 0 ? (
                <div className="workflow-dag__edges">
                  {outgoing.map((edge) => <Tag key={`${edge.from}-${edge.to}`} color="blue">→ {edge.to} · {edge.condition ?? 'on_success'}</Tag>)}
                </div>
              ) : <Typography.Text type="secondary">终止节点</Typography.Text>}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function WorkflowsPage() {
  const [items, setItems] = useState<WorkflowSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState<'json' | 'yaml'>('json');
  const [draft, setDraft] = useState(DEFAULT_DEFINITION);
  const [dryRun, setDryRun] = useState<WorkflowDryRunResponse | null>(null);
  const [activeWorkflow, setActiveWorkflow] = useState<WorkflowSummary | null>(null);
  const [activeInstance, setActiveInstance] = useState<WorkflowInstanceSummary | null>(null);
  const [events, setEvents] = useState<InstanceEventSummary[]>([]);
  const [form] = Form.useForm<{ name: string; definition: string }>();

  const fetchItems = async () => {
    setLoading(true);
    try { setItems(await listWorkflows()); } finally { setLoading(false); }
  };

  useEffect(() => { void fetchItems(); }, []);

  useEffect(() => {
    if (!activeInstance) return undefined;
    const token = getAuthToken();
    const url = token ? `${workflowEventStreamUrl(activeInstance.id)}?token=${encodeURIComponent(token)}` : workflowEventStreamUrl(activeInstance.id);
    const source = new EventSource(url);
    source.onmessage = (event) => {
      try { setEvents((current) => [...current, JSON.parse(event.data) as InstanceEventSummary]); }
      catch { setEvents((current) => [...current, { id: crypto.randomUUID(), instance_id: activeInstance.id, instance_type: 'workflow', event_type: 'message', message: event.data, payload: null, created_at: new Date().toISOString() }]); }
    };
    ['workflow.started', 'workflow.succeeded', 'workflow.failed', 'workflow.node.succeeded', 'workflow.node.failed'].forEach((name) => {
      source.addEventListener(name, (event) => {
        const payload = (event as MessageEvent).data;
        try { setEvents((current) => [...current, JSON.parse(payload) as InstanceEventSummary]); }
        catch { /* ignore malformed server-sent event */ }
      });
    });
    return () => source.close();
  }, [activeInstance]);

  const previewDefinition = useMemo(() => {
    try { return parseDefinition(draft); } catch { return null; }
  }, [draft]);

  const submit = async (values: { name: string; definition: string }) => {
    let definition: WorkflowDefinition;
    try { definition = parseDefinition(values.definition); }
    catch { message.error('Workflow definition 必须是合法 JSON'); return; }
    const created = await createWorkflow({ name: values.name, definition });
    message.success('Workflow 已创建');
    setActiveWorkflow(created);
    form.resetFields();
    setDraft(DEFAULT_DEFINITION);
    form.setFieldValue('definition', DEFAULT_DEFINITION);
    await fetchItems();
  };

  const dryRunDraft = async () => {
    try {
      const definition = parseDefinition(draft);
      setDryRun(await dryRunWorkflow(definition));
      message.success('Dry-run 已完成');
    } catch (error) {
      message.error(error instanceof Error ? error.message : 'Dry-run 失败');
    }
  };

  const validate = async (item: WorkflowSummary) => {
    const result = await validateWorkflow(item.id);
    if (result.valid) message.success('DAG 校验通过');
    else message.error(result.errors.join('; '));
    setActiveWorkflow(item);
  };

  const run = async (item: WorkflowSummary) => {
    const instance = await runWorkflow(item.id);
    setActiveWorkflow(item);
    setActiveInstance(instance);
    setEvents([]);
    message.success(`Workflow instance queued: ${instance.id}`);
  };

  const completeFirstQueued = async () => {
    if (!activeInstance) return;
    const target = activeInstance.nodes.find((node) => node.status === 'queued' || node.status === 'running');
    if (!target) { message.info('没有可推进节点'); return; }
    const result = await advanceWorkflowInstance(activeInstance.id, { node_key: target.node_key, status: 'succeeded', message: `manual success for ${target.node_key}` });
    setActiveInstance(result.instance);
    message.success(result.completed ? 'Workflow 已完成' : `已推进，入队节点：${result.queued_nodes.join(', ') || '无'}`);
  };

  const switchMode = (nextMode: 'json' | 'yaml') => {
    setMode(nextMode);
    if (nextMode === 'yaml' && previewDefinition) {
      setDraft(definitionToYaml(previewDefinition));
      message.info('YAML 当前为只读预览；创建/ dry-run 仍以 JSON 模式提交');
    } else if (nextMode === 'json') {
      setDraft(DEFAULT_DEFINITION);
    }
  };

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div className="hero-panel workflow-hero">
        <div className="hero-panel__content">
          <Tag className="soft-tag" color="blue">Phase 2 · Workflow Engine</Tag>
          <Typography.Title level={1}>工作流编排</Typography.Title>
          <Typography.Paragraph className="hero-panel__desc">
            支持 DAG 校验、条件边推进、Map / MapReduce / 子工作流节点建模，以及面向调试的事件流和可视化基础能力。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary"><strong>{items.length}</strong><span>flows</span></div>
      </div>

      <Row gutter={[18, 18]}>
        <Col xs={24} xl={11}>
          <Card title="创建 / Dry-run" extra={<Segmented value={mode} onChange={(value) => switchMode(value as 'json' | 'yaml')} options={[{ label: 'JSON', value: 'json' }, { label: 'YAML 预览', value: 'yaml' }]} />}>
            <Form form={form} layout="vertical" initialValues={{ definition: DEFAULT_DEFINITION }} onFinish={submit}>
              <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input placeholder="daily-pipeline" /></Form.Item>
              <Form.Item name="definition" label={mode === 'json' ? '定义 JSON' : '定义 YAML 预览'} rules={[{ required: true }]}>
                <Input.TextArea rows={14} spellCheck={false} value={draft} onChange={(event) => setDraft(event.target.value)} readOnly={mode === 'yaml'} />
              </Form.Item>
              <Space wrap>
                <Button type="primary" htmlType="submit" disabled={mode !== 'json'}>创建工作流</Button>
                <Button onClick={dryRunDraft} disabled={mode !== 'json'}>Dry-run 校验</Button>
              </Space>
            </Form>
            {dryRun ? (
              <Alert
                style={{ marginTop: 16 }}
                type={dryRun.validation.valid ? 'success' : 'error'}
                message={dryRun.validation.valid ? 'Dry-run 通过' : 'Dry-run 失败'}
                description={`start: ${dryRun.start_nodes.join(', ') || '-'} · nodes: ${dryRun.node_count} · edges: ${dryRun.edge_count}${dryRun.validation.errors.length ? ` · ${dryRun.validation.errors.join('; ')}` : ''}`}
              />
            ) : null}
          </Card>
        </Col>
        <Col xs={24} xl={13}>
          <Card title="DAG 可视化预览">
            {previewDefinition ? <DagPreview definition={previewDefinition} instance={activeInstance} /> : <Alert type="warning" message="JSON 解析失败，无法预览" />}
          </Card>
        </Col>
      </Row>

      <Card title="工作流列表" extra={<Button onClick={fetchItems}>刷新</Button>}>
        <List
          loading={loading}
          dataSource={items}
          locale={{ emptyText: '暂无工作流' }}
          renderItem={(item) => (
            <List.Item actions={[<Button key="validate" onClick={() => validate(item)}>校验</Button>, <Button key="run" type="primary" onClick={() => run(item)}>运行</Button>] }>
              <List.Item.Meta
                title={<Space><span>{item.name}</span><Tag color={STATUS_COLORS[item.status] ?? 'blue'}>{item.status}</Tag></Space>}
                description={<span>{item.id} · nodes: {item.definition.nodes.length} · edges: {item.definition.edges.length}</span>}
              />
            </List.Item>
          )}
        />
      </Card>

      {activeWorkflow ? (
        <Row gutter={[18, 18]}>
          <Col xs={24} lg={14}>
            <Card title={`运行视图 · ${activeWorkflow.name}`} extra={<Button onClick={completeFirstQueued} disabled={!activeInstance}>推进首个队列节点</Button>}>
              <DagPreview definition={activeWorkflow.definition} instance={activeInstance} />
            </Card>
          </Col>
          <Col xs={24} lg={10}>
            <Card title="实例事件流">
              {activeInstance ? <Typography.Text type="secondary">{activeInstance.id} · {activeInstance.status}</Typography.Text> : <Typography.Text type="secondary">运行工作流后展示 SSE 事件</Typography.Text>}
              <Timeline style={{ marginTop: 18 }} items={events.map((event) => ({ color: event.event_type.includes('failed') ? 'red' : 'blue', children: <span>{event.created_at} · {event.event_type} · {event.message}</span> }))} />
            </Card>
          </Col>
        </Row>
      ) : null}
    </Space>
  );
}
