import { useEffect, useMemo, useState, type PointerEvent } from 'react';
import { Alert, Button, Card, Col, Form, Input, List, Row, Segmented, Select, Space, Tag, Timeline, Typography, message } from 'antd';
import {
  advanceWorkflowInstance,
  createWorkflow,
  listWorkflowShards,
  materializeNextWorkflowNode,
  recoverWorkflowNode,
  dryRunWorkflow,
  getAuthToken,
  listWorkflows,
  runWorkflow,
  validateWorkflow,
  workflowEventStreamUrl,
  type InstanceEventSummary,
  type WorkflowDefinition,
  type WorkflowDryRunResponse,
  type WorkflowEdgeSpec,
  type WorkflowInstanceSummary,
  type WorkflowNodeSpec,
  type WorkflowShardSummary,
  type WorkflowSummary,
} from '../api/client';

const DEFAULT_WORKFLOW: WorkflowDefinition = {
  nodes: [
    { key: 'extract', name: 'Extract', kind: 'job', job_id: 'job_extract' },
    { key: 'map-users', name: 'Map users', kind: 'map', map_items: [{ shard: 1 }, { shard: 2 }] },
    { key: 'reduce', name: 'Reduce', kind: 'map_reduce', map_items: [{ shard: 1 }, { shard: 2 }] },
  ],
  edges: [
    { from: 'extract', to: 'map-users', condition: 'on_success' },
    { from: 'map-users', to: 'reduce', condition: 'on_success' },
  ],
};
const DEFAULT_DEFINITION = JSON.stringify(DEFAULT_WORKFLOW, null, 2);

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

function stringifyDefinition(definition: WorkflowDefinition): string {
  return JSON.stringify(definition, null, 2);
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

function makeNode(kind: string, index: number): WorkflowNodeSpec {
  const key = `${kind.replace('_', '-')}-${index}`;
  if (kind === 'map' || kind === 'map_reduce') {
    return { key, name: key, kind, map_items: [{ shard: 1 }, { shard: 2 }] };
  }
  if (kind === 'sub_workflow') {
    return { key, name: key, kind, child_workflow_id: 'wf_child' };
  }
  return { key, name: key, kind: 'job', job_id: `job_${key.replaceAll('-', '_')}` };
}

function nodePosition(node: WorkflowNodeSpec, index: number) {
  const config = (typeof node.config === 'object' && node.config !== null ? node.config : {}) as { ui?: { x?: number; y?: number } };
  return {
    x: typeof config.ui?.x === 'number' ? config.ui.x : 70 + index * 250,
    y: typeof config.ui?.y === 'number' ? config.ui.y : 80 + (index % 2) * 150,
  };
}

function withNodePosition(node: WorkflowNodeSpec, x: number, y: number): WorkflowNodeSpec {
  const config = (typeof node.config === 'object' && node.config !== null ? node.config : {}) as Record<string, unknown>;
  return { ...node, config: { ...config, ui: { x, y } } };
}

function edgeColor(condition?: string | null) {
  if (condition === 'on_failure') return '#ef4444';
  if (condition === 'always') return '#8b5cf6';
  return '#2563eb';
}

function DagPreview({ definition, instance, editable = false, onChange }: { definition: WorkflowDefinition; instance?: WorkflowInstanceSummary | null; editable?: boolean; onChange?: (definition: WorkflowDefinition) => void }) {
  const [dragging, setDragging] = useState<{ key: string; offsetX: number; offsetY: number } | null>(null);
  const [pendingEdgeFrom, setPendingEdgeFrom] = useState<string | null>(null);
  const statuses = new Map(instance?.nodes.map((node) => [node.node_key, node.status]) ?? []);
  const positions = new Map(definition.nodes.map((node, index) => [node.key, nodePosition(node, index)]));

  const update = (next: WorkflowDefinition) => onChange?.(next);
  const removeNode = (key: string) => update({ nodes: definition.nodes.filter((node) => node.key !== key), edges: definition.edges.filter((edge) => edge.from !== key && edge.to !== key) });
  const removeEdge = (edge: WorkflowEdgeSpec) => update({ ...definition, edges: definition.edges.filter((item) => !(item.from === edge.from && item.to === edge.to && item.condition === edge.condition)) });
  const addNode = (kind: string) => {
    const nextNode = withNodePosition(makeNode(kind, definition.nodes.length + 1), 80 + definition.nodes.length * 38, 90 + definition.nodes.length * 34);
    update({ ...definition, nodes: [...definition.nodes, nextNode] });
  };
  const addEdge = () => {
    if (definition.nodes.length < 2) return;
    const from = definition.nodes.at(-2)?.key;
    const to = definition.nodes.at(-1)?.key;
    if (!from || !to) return;
    update({ ...definition, edges: [...definition.edges, { from, to, condition: 'on_success' }] });
  };
  const changeEdge = (index: number, patch: Partial<WorkflowEdgeSpec>) => update({ ...definition, edges: definition.edges.map((edge, edgeIndex) => edgeIndex === index ? { ...edge, ...patch } : edge) });
  const pointerDown = (node: WorkflowNodeSpec, event: PointerEvent<HTMLDivElement>) => {
    if (!editable || (event.target as HTMLElement).closest('button,.workflow-node-port')) return;
    const position = positions.get(node.key) ?? { x: 0, y: 0 };
    setDragging({ key: node.key, offsetX: event.clientX - position.x, offsetY: event.clientY - position.y });
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const pointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!editable || !dragging) return;
    const nextX = Math.max(18, event.clientX - dragging.offsetX);
    const nextY = Math.max(18, event.clientY - dragging.offsetY);
    update({ ...definition, nodes: definition.nodes.map((node) => node.key === dragging.key ? withNodePosition(node, nextX, nextY) : node) });
  };
  const connectFrom = (key: string) => {
    if (!editable) return;
    setPendingEdgeFrom(key);
    message.info(`选择 ${key} 的输出端口，接下来点击目标节点输入端口完成连线`);
  };
  const connectTo = (key: string) => {
    if (!editable || !pendingEdgeFrom || pendingEdgeFrom === key) return;
    const exists = definition.edges.some((edge) => edge.from === pendingEdgeFrom && edge.to === key);
    if (!exists) update({ ...definition, edges: [...definition.edges, { from: pendingEdgeFrom, to: key, condition: 'on_success' }] });
    setPendingEdgeFrom(null);
  };

  const canvasWidth = Math.max(900, ...definition.nodes.map((node, index) => (positions.get(node.key)?.x ?? index * 220) + 260));
  const canvasHeight = Math.max(480, ...definition.nodes.map((node, index) => (positions.get(node.key)?.y ?? index * 100) + 180));

  return (
    <div className="workflow-dag-editor">
      {editable ? (
        <Space wrap className="workflow-dag-toolbar">
          <Button onClick={() => addNode('job')}>+ Job</Button>
          <Button onClick={() => addNode('map')}>+ Map</Button>
          <Button onClick={() => addNode('map_reduce')}>+ MapReduce</Button>
          <Button onClick={() => addNode('sub_workflow')}>+ 子工作流</Button>
          <Button onClick={addEdge} disabled={definition.nodes.length < 2}>连接最后两个节点</Button>
          {pendingEdgeFrom ? <Tag color="blue">正在从 {pendingEdgeFrom} 连线：点击目标输入端口</Tag> : null}
        </Space>
      ) : null}
      <div className="workflow-node-canvas" style={{ height: Math.min(620, canvasHeight + 40) }} onPointerMove={pointerMove} onPointerUp={() => setDragging(null)}>
        <div className="workflow-node-canvas__space" style={{ width: canvasWidth, height: canvasHeight }}>
          <svg className="workflow-node-canvas__edges" width={canvasWidth} height={canvasHeight}>
            <defs>
              <marker id="workflow-arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto" markerUnits="strokeWidth">
                <path d="M0,0 L0,6 L8,3 z" fill="#2563eb" />
              </marker>
            </defs>
            {definition.edges.map((edge, index) => {
              const from = positions.get(edge.from);
              const to = positions.get(edge.to);
              if (!from || !to) return null;
              const x1 = from.x + 218;
              const y1 = from.y + 70;
              const x2 = to.x;
              const y2 = to.y + 70;
              const mid = Math.max(80, Math.abs(x2 - x1) / 2);
              const color = edgeColor(edge.condition);
              return <path key={`${edge.from}-${edge.to}-${index}`} d={`M ${x1} ${y1} C ${x1 + mid} ${y1}, ${x2 - mid} ${y2}, ${x2} ${y2}`} stroke={color} strokeWidth="2.5" fill="none" markerEnd="url(#workflow-arrow)" />;
            })}
          </svg>
          {definition.nodes.map((node, index) => {
            const position = positions.get(node.key) ?? { x: 0, y: 0 };
            const status = statuses.get(node.key) ?? 'design';
            const incoming = definition.edges.filter((edge) => edge.to === node.key);
            const outgoing = definition.edges.filter((edge) => edge.from === node.key);
            return (
              <div className={`workflow-node-card ${editable ? 'workflow-node-card--editable' : ''}`} key={node.key} style={{ left: position.x, top: position.y }} onPointerDown={(event) => pointerDown(node, event)}>
                <button className="workflow-node-port workflow-node-port--input" type="button" onClick={() => connectTo(node.key)} title="输入端口" />
                <button className="workflow-node-port workflow-node-port--output" type="button" onClick={() => connectFrom(node.key)} title="输出端口" />
                <div className="workflow-node-card__header">
                  <span className="workflow-node-card__index">{index + 1}</span>
                  <span className="workflow-node-card__title">{node.name ?? node.key}</span>
                  <Tag color={STATUS_COLORS[status] ?? 'default'}>{status}</Tag>
                </div>
                <div className="workflow-node-card__body">
                  <Tag color="cyan">{node.kind ?? 'job'}</Tag>
                  <Typography.Text className="workflow-node-card__key">{node.key}</Typography.Text>
                  {node.job_id ? <Typography.Text type="secondary">job: {node.job_id}</Typography.Text> : null}
                  {node.child_workflow_id ? <Typography.Text type="secondary">child: {node.child_workflow_id}</Typography.Text> : null}
                </div>
                <div className="workflow-node-card__ports">
                  <span>in {incoming.length}</span>
                  <span>out {outgoing.length}</span>
                </div>
                {editable ? <Button size="small" danger className="workflow-node-card__delete" onClick={() => removeNode(node.key)}>删除</Button> : null}
              </div>
            );
          })}
        </div>
      </div>
      {editable ? (
        <Card size="small" title="边关系" className="workflow-edge-editor">
          <Space direction="vertical" style={{ width: '100%' }}>
            {definition.edges.map((edge, index) => (
              <Space wrap key={`${edge.from}-${edge.to}-${index}`}>
                <Select value={edge.from} style={{ width: 140 }} options={definition.nodes.map((node) => ({ label: node.key, value: node.key }))} onChange={(value) => changeEdge(index, { from: value })} />
                <Typography.Text>→</Typography.Text>
                <Select value={edge.to} style={{ width: 140 }} options={definition.nodes.map((node) => ({ label: node.key, value: node.key }))} onChange={(value) => changeEdge(index, { to: value })} />
                <Select value={edge.condition ?? 'on_success'} style={{ width: 140 }} options={['on_success', 'on_failure', 'always'].map((value) => ({ label: value, value }))} onChange={(value) => changeEdge(index, { condition: value as WorkflowEdgeSpec['condition'] })} />
                <Button size="small" danger onClick={() => removeEdge(edge)}>删除边</Button>
              </Space>
            ))}
          </Space>
        </Card>
      ) : null}
    </div>
  );
}

export function WorkflowsPage() {
  const [items, setItems] = useState<WorkflowSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState<'visual' | 'json' | 'yaml'>('visual');
  const [draft, setDraft] = useState(DEFAULT_DEFINITION);
  const [dryRun, setDryRun] = useState<WorkflowDryRunResponse | null>(null);
  const [activeWorkflow, setActiveWorkflow] = useState<WorkflowSummary | null>(null);
  const [activeInstance, setActiveInstance] = useState<WorkflowInstanceSummary | null>(null);
  const [events, setEvents] = useState<InstanceEventSummary[]>([]);
  const [shards, setShards] = useState<WorkflowShardSummary[]>([]);
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

  const updateDefinition = (definition: WorkflowDefinition) => {
    const next = stringifyDefinition(definition);
    setDraft(next);
    form.setFieldValue('definition', next);
    setDryRun(null);
  };

  const submit = async (values: { name: string; definition: string }) => {
    let definition: WorkflowDefinition;
    try { definition = parseDefinition(values.definition); }
    catch { message.error('Workflow definition 必须是合法 JSON'); return; }
    const created = await createWorkflow({ name: values.name, definition });
    message.success('Workflow 已创建');
    setActiveWorkflow(created);
    form.resetFields();
    updateDefinition(DEFAULT_WORKFLOW);
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

  const materializeNext = async () => {
    const result = await materializeNextWorkflowNode();
    setActiveInstance(result.instance);
    setShards(result.shards);
    message.success(`已物化节点：${result.node.node_key}`);
  };

  const recoverFirstFailed = async () => {
    if (!activeInstance) return;
    const target = activeInstance.nodes.find((node) => node.status === 'failed');
    if (!target) { message.info('没有失败节点'); return; }
    const result = await recoverWorkflowNode(activeInstance.id, { node_key: target.node_key, action: 'retry', message: `retry ${target.node_key}` });
    setActiveInstance(result.instance);
    message.success(`已重试节点：${target.node_key}`);
  };

  const refreshShards = async () => {
    if (!activeInstance) return;
    setShards(await listWorkflowShards(activeInstance.id));
  };

  const switchMode = (nextMode: 'visual' | 'json' | 'yaml') => {
    if (nextMode === 'yaml' && previewDefinition) {
      setMode(nextMode);
      setDraft(definitionToYaml(previewDefinition));
      message.info('YAML 当前为只读预览；切回可视化/JSON 会恢复当前 JSON 定义');
      return;
    }
    if ((mode === 'yaml' || nextMode === 'visual') && !previewDefinition) {
      setDraft(DEFAULT_DEFINITION);
      form.setFieldValue('definition', DEFAULT_DEFINITION);
    }
    setMode(nextMode);
  };

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div className="hero-panel workflow-hero">
        <div className="hero-panel__content">
          <Tag className="soft-tag" color="blue">Phase 2 · Workflow Engine</Tag>
          <Typography.Title level={1}>工作流编排</Typography.Title>
          <Typography.Paragraph className="hero-panel__desc">
            支持 DAG 校验、拖拽式节点编排、条件边推进、Map / MapReduce / 子工作流节点建模，以及面向调试的事件流。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary"><strong>{items.length}</strong><span>flows</span></div>
      </div>

      <Row gutter={[18, 18]}>
        <Col xs={24} xl={11}>
          <Card title="创建 / Dry-run" extra={<Segmented value={mode} onChange={(value) => switchMode(value as 'visual' | 'json' | 'yaml')} options={[{ label: '可视化', value: 'visual' }, { label: 'JSON', value: 'json' }, { label: 'YAML 预览', value: 'yaml' }]} />}>
            <Form form={form} layout="vertical" initialValues={{ definition: DEFAULT_DEFINITION }} onFinish={submit}>
              <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input placeholder="daily-pipeline" /></Form.Item>
              {mode !== 'visual' ? (
                <Form.Item name="definition" label={mode === 'json' ? '定义 JSON' : '定义 YAML 预览'} rules={[{ required: true }]}>
                  <Input.TextArea rows={14} spellCheck={false} value={draft} onChange={(event) => { setDraft(event.target.value); setDryRun(null); }} readOnly={mode === 'yaml'} />
                </Form.Item>
              ) : <Form.Item name="definition" hidden><Input.TextArea value={draft} /></Form.Item>}
              <Space wrap>
                <Button type="primary" htmlType="submit" disabled={mode === 'yaml'}>创建工作流</Button>
                <Button onClick={dryRunDraft} disabled={mode === 'yaml'}>Dry-run 校验</Button>
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
          <Card title={mode === 'visual' ? '可视化拖拽编排' : 'DAG 可视化预览'}>
            {previewDefinition ? <DagPreview definition={previewDefinition} instance={activeInstance} editable={mode === 'visual'} onChange={updateDefinition} /> : <Alert type="warning" message="JSON 解析失败，无法预览" />}
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
            <Card title={`运行视图 · ${activeWorkflow.name}`} extra={<Space wrap><Button onClick={materializeNext}>物化下一节点</Button><Button onClick={completeFirstQueued} disabled={!activeInstance}>推进首个队列节点</Button><Button onClick={recoverFirstFailed} disabled={!activeInstance}>重试失败节点</Button><Button onClick={refreshShards} disabled={!activeInstance}>刷新 Shards</Button></Space>} >
              <DagPreview definition={activeWorkflow.definition} instance={activeInstance} />
              {shards.length > 0 ? <List size="small" style={{ marginTop: 16 }} dataSource={shards} renderItem={(shard) => <List.Item><Typography.Text>{shard.node_key}#{shard.shard_index} · {shard.status} · {JSON.stringify(shard.input)}</Typography.Text></List.Item>} /> : null}
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
