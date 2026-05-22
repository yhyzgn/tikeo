import { useEffect, useMemo, useRef, useState, type PointerEvent } from 'react';
import { Alert, Button, Card, Form, Input, List, Popconfirm, Segmented, Select, Space, Tag, Timeline, Typography, message } from 'antd';
import {
  advanceWorkflowInstance,
  createWorkflow,
  getWorkflow,
  listWorkflowShards,
  materializeNextWorkflowNode,
  ApiClientError,
  recoverWorkflowNode,
  dryRunWorkflow,
  getAuthToken,
  listJobs,
  listWorkflows,
  normalizeWorkflowDefinition,
  runWorkflow,
  updateWorkflow,
  validateWorkflow,
  workflowEventStreamUrl,
  type InstanceEventSummary,
  type JobSummary,
  type WorkflowDefinition,
  type WorkflowDryRunResponse,
  type WorkflowEdgeSpec,
  type WorkflowInstanceSummary,
  type WorkflowNodeSpec,
  type WorkflowShardSummary,
  type WorkflowSummary,
} from '../api/client';
import { PermissionGate, useCan } from '../components/Permission';
import { useUrlQueryState } from '../hooks/useUrlQueryState';
import { useNavigate, useParams } from 'react-router-dom';

const DEFAULT_WORKFLOW: WorkflowDefinition = {
  nodes: [
    { key: 'extract', name: 'Extract', kind: 'job', job_id: 'job_extract', config: { ui: { x: 80, y: 120 } } },
    { key: 'map-users', name: 'Map users', kind: 'map', map_items: [{ shard: 1 }, { shard: 2 }], config: { ui: { x: 360, y: 80 } } },
    { key: 'reduce', name: 'Reduce', kind: 'map_reduce', map_items: [{ shard: 1 }, { shard: 2 }], config: { ui: { x: 650, y: 160 } } },
  ],
  edges: [
    { from: 'extract', to: 'map-users', condition: 'always' },
    { from: 'map-users', to: 'reduce', condition: 'always' },
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

const NODE_CATALOG = [
  { kind: 'start', label: 'Start', limits: { in: 0, out: 4 } },
  { kind: 'end', label: 'End', limits: { in: 8, out: 0 } },
  { kind: 'job', label: 'Job', limits: { in: 8, out: 8 } },
  { kind: 'script', label: 'Script', limits: { in: 8, out: 8 } },
  { kind: 'http', label: 'HTTP', limits: { in: 8, out: 8 } },
  { kind: 'condition', label: 'Condition', limits: { in: 8, out: 2 } },
  { kind: 'parallel', label: 'Parallel', limits: { in: 8, out: 16 } },
  { kind: 'join', label: 'Join', limits: { in: 16, out: 4 } },
  { kind: 'delay', label: 'Delay', limits: { in: 8, out: 8 } },
  { kind: 'approval', label: 'Approval', limits: { in: 8, out: 4 } },
  { kind: 'notification', label: 'Notify', limits: { in: 8, out: 8 } },
  { kind: 'map', label: 'Map', limits: { in: 8, out: 8 } },
  { kind: 'map_reduce', label: 'MapReduce', limits: { in: 16, out: 4 } },
  { kind: 'sub_workflow', label: 'SubFlow', limits: { in: 8, out: 4 } },
] as const;

const NODE_LIMITS: Record<string, { in: number; out: number }> = Object.fromEntries(
  NODE_CATALOG.map((node) => [node.kind, node.limits]),
);

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
    if (node.config) lines.push(`    config: ${JSON.stringify(node.config)}`);
  }
  lines.push('edges:');
  for (const edge of definition.edges) {
    lines.push(`  - from: ${edge.from}`);
    lines.push(`    to: ${edge.to}`);
    lines.push(`    condition: ${edge.condition ?? 'always'}`);
  }
  return lines.join('\n');
}

function nodeKind(node: WorkflowNodeSpec): string {
  return node.kind ?? 'job';
}

function nodeLimits(node: WorkflowNodeSpec) {
  return NODE_LIMITS[nodeKind(node)] ?? { in: 8, out: 8 };
}

function makeNode(kind: string, index: number): WorkflowNodeSpec {
  const key = `${kind.replace('_', '-')}-${index}`;
  const ui = { x: 90 + index * 44, y: 100 + index * 34 };
  if (kind === 'map' || kind === 'map_reduce') {
    return { key, name: key, kind, processor_name: key, map_items: [{ shard: 1 }, { shard: 2 }], config: { ui, mode: kind === 'map' ? 'fan-out' : 'fan-out-reduce' } };
  }
  if (kind === 'sub_workflow') {
    return { key, name: key, kind, child_workflow_id: 'wf_child', config: { ui } };
  }
  if (kind === 'job') {
    return { key, name: key, kind, job_id: '', processor_name: key, config: { ui } };
  }
  const configByKind: Record<string, Record<string, unknown>> = {
    script: { language: 'rhai', sandbox: 'isolated', source: '' },
    http: { method: 'GET', url: '', timeout_ms: 30000 },
    condition: { expression: 'context.success == true', true_edge: 'on_success', false_edge: 'on_failure' },
    parallel: { strategy: 'fan-out' },
    join: { quorum: 'all' },
    delay: { seconds: '60' },
    approval: { approvers: 'role:ops', timeout: '24h' },
    notification: { channel: 'webhook', target: '', template: '' },
  };
  return { key, name: key, kind, config: { ui, ...(configByKind[kind] ?? {}) } };
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

type EdgeConditionOption = { label: string; value: string; color: string };

const EDGE_CONDITION_OPTIONS: Record<string, EdgeConditionOption[]> = {
  condition: [
    { label: '条件成立 true', value: 'on_success', color: '#16a34a' },
    { label: '条件不成立 false', value: 'on_failure', color: '#ef4444' },
  ],
  approval: [
    { label: '审批通过 approved', value: 'on_success', color: '#16a34a' },
    { label: '审批拒绝 rejected', value: 'on_failure', color: '#ef4444' },
    { label: '审批完成 always', value: 'always', color: '#8b5cf6' },
  ],
  parallel: [
    { label: '并行分支 branch', value: 'always', color: '#8b5cf6' },
  ],
  join: [
    { label: '汇聚完成 joined', value: 'on_success', color: '#2563eb' },
    { label: '汇聚失败 failed', value: 'on_failure', color: '#ef4444' },
  ],
  http: [
    { label: 'HTTP 成功 2xx', value: 'on_success', color: '#16a34a' },
    { label: 'HTTP 失败 non-2xx', value: 'on_failure', color: '#ef4444' },
    { label: '请求完成 always', value: 'always', color: '#8b5cf6' },
  ],
  script: [
    { label: '脚本成功', value: 'on_success', color: '#16a34a' },
    { label: '脚本失败', value: 'on_failure', color: '#ef4444' },
    { label: '脚本结束', value: 'always', color: '#8b5cf6' },
  ],
};

const DEFAULT_EDGE_CONDITIONS: EdgeConditionOption[] = [
  { label: '始终 always', value: 'always', color: '#8b5cf6' },
  { label: '成功时 on_success', value: 'on_success', color: '#2563eb' },
  { label: '失败时 on_failure', value: 'on_failure', color: '#ef4444' },
];

function edgeConditionOptionsFor(node?: WorkflowNodeSpec | null): EdgeConditionOption[] {
  return EDGE_CONDITION_OPTIONS[node ? nodeKind(node) : ''] ?? DEFAULT_EDGE_CONDITIONS;
}

function defaultEdgeConditionFor(node?: WorkflowNodeSpec | null): WorkflowEdgeSpec['condition'] {
  return (edgeConditionOptionsFor(node)[0]?.value ?? 'always') as WorkflowEdgeSpec['condition'];
}

function edgeConditionMeta(condition: string | null | undefined, fromNode?: WorkflowNodeSpec | null): EdgeConditionOption {
  const value = condition ?? defaultEdgeConditionFor(fromNode) ?? 'always';
  return edgeConditionOptionsFor(fromNode).find((option) => option.value === value)
    ?? DEFAULT_EDGE_CONDITIONS.find((option) => option.value === value)
    ?? { label: value, value, color: '#2563eb' };
}

function DagPreview({ definition, instance, jobs = [], editable = false, onChange }: { definition: WorkflowDefinition; instance?: WorkflowInstanceSummary | null; jobs?: JobSummary[]; editable?: boolean; onChange?: (definition: WorkflowDefinition) => void }) {
  const [dragging, setDragging] = useState<{ key: string; offsetX: number; offsetY: number } | null>(null);
  const [linkDrag, setLinkDrag] = useState<{ from: string; x: number; y: number } | null>(null);
  const [edgeDrag, setEdgeDrag] = useState<{ index: number; side: 'from' | 'to'; anchorKey: string; x: number; y: number } | null>(null);
  const [selectedEdgeIndex, setSelectedEdgeIndex] = useState<number | null>(null);
  const [selectedNodeKey, setSelectedNodeKey] = useState<string | null>(definition.nodes[0]?.key ?? null);
  const [isCanvasFullscreen, setIsCanvasFullscreen] = useState(false);
  const spaceRef = useRef<HTMLDivElement | null>(null);
  const statuses = new Map(instance?.nodes.map((node) => [node.node_key, node.status]) ?? []);
  const positions = new Map(definition.nodes.map((node, index) => [node.key, nodePosition(node, index)]));

  const selectedNode = definition.nodes.find((node) => node.key === selectedNodeKey) ?? null;
  const selectedEdge = selectedEdgeIndex === null ? null : definition.edges[selectedEdgeIndex] ?? null;
  const selectedEdgeOverlay = selectedEdge ? (() => {
    const from = positions.get(selectedEdge.from);
    const to = positions.get(selectedEdge.to);
    if (!from || !to) return null;
    return { x: Math.max(16, (from.x + 218 + to.x) / 2 - 110), y: Math.max(16, (from.y + to.y) / 2 + 22) };
  })() : null;
  const selectedEdgeHandles = selectedEdge ? (() => {
    const from = positions.get(selectedEdge.from);
    const to = positions.get(selectedEdge.to);
    if (!from || !to) return null;
    return { from: { x: from.x + 218, y: from.y + 70 }, to: { x: to.x, y: to.y + 70 } };
  })() : null;
  const jobOptions = jobs.map((job) => ({ label: `${job.name} · ${job.namespace}/${job.app}`, value: job.id }));

  const update = (next: WorkflowDefinition) => onChange?.(next);
  const toCanvasPoint = (event: PointerEvent<Element>) => {
    const rect = spaceRef.current?.getBoundingClientRect();
    if (!rect) return { x: event.clientX, y: event.clientY };
    return { x: event.clientX - rect.left, y: event.clientY - rect.top };
  };
  const updateNode = (key: string, patch: Partial<WorkflowNodeSpec>) => update({ ...definition, nodes: definition.nodes.map((node) => node.key === key ? { ...node, ...patch } : node) });
  const renameNodeKey = (oldKey: string, nextKey: string) => {
    setSelectedNodeKey(nextKey);
    update({
      nodes: definition.nodes.map((node) => node.key === oldKey ? { ...node, key: nextKey, name: node.name === oldKey ? nextKey : node.name } : node),
      edges: definition.edges.map((edge) => ({
        ...edge,
        from: edge.from === oldKey ? nextKey : edge.from,
        to: edge.to === oldKey ? nextKey : edge.to,
      })),
    });
  };
  const updateNodeConfig = (key: string, patch: Record<string, unknown>) => update({
    ...definition,
    nodes: definition.nodes.map((node) => {
      if (node.key !== key) return node;
      const config = (typeof node.config === 'object' && node.config !== null ? node.config : {}) as Record<string, unknown>;
      return { ...node, config: { ...config, ...patch } };
    }),
  });
  const updateMapItems = (key: string, raw: string) => {
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (!Array.isArray(parsed)) { message.warning('map_items 必须是 JSON 数组'); return; }
      updateNode(key, { map_items: parsed });
    } catch {
      message.warning('map_items 不是合法 JSON');
    }
  };
  const removeNode = (key: string) => {
    update({ nodes: definition.nodes.filter((node) => node.key !== key), edges: definition.edges.filter((edge) => edge.from !== key && edge.to !== key) });
    if (selectedNodeKey === key) setSelectedNodeKey(definition.nodes.find((node) => node.key !== key)?.key ?? null);
  };
  const removeEdgeAt = (index: number) => {
    update({ ...definition, edges: definition.edges.filter((_, edgeIndex) => edgeIndex !== index) });
    setSelectedEdgeIndex(null);
  };
  const addNode = (kind: string) => {
    const node = makeNode(kind, definition.nodes.length + 1);
    update({ ...definition, nodes: [...definition.nodes, node] });
    setSelectedNodeKey(node.key);
  };
  const changeEdge = (index: number, patch: Partial<WorkflowEdgeSpec>) => update({ ...definition, edges: definition.edges.map((edge, edgeIndex) => edgeIndex === index ? { ...edge, ...patch } : edge) });
  const connectNodes = (from: string, to: string) => {
    if (from === to) { message.warning('不能连接到自身'); return; }
    const fromNode = definition.nodes.find((node) => node.key === from);
    const toNode = definition.nodes.find((node) => node.key === to);
    if (!fromNode || !toNode) return;
    const fromCount = definition.edges.filter((edge) => edge.from === from).length;
    const toCount = definition.edges.filter((edge) => edge.to === to).length;
    const fromLimit = nodeLimits(fromNode).out;
    const toLimit = nodeLimits(toNode).in;
    if (fromCount >= fromLimit) { message.warning(`${from} 的输出最多 ${fromLimit} 条`); return; }
    if (toCount >= toLimit) { message.warning(`${to} 的输入最多 ${toLimit} 条`); return; }
    if (definition.edges.some((edge) => edge.from === from && edge.to === to)) { message.info('这条连线已存在'); return; }
    update({ ...definition, edges: [...definition.edges, { from, to, condition: defaultEdgeConditionFor(fromNode) }] });
  };
  const pointerDown = (node: WorkflowNodeSpec, event: PointerEvent<HTMLDivElement>) => {
    if (!editable || (event.target as HTMLElement).closest('button,.workflow-node-port')) return;
    const position = positions.get(node.key) ?? { x: 0, y: 0 };
    const point = toCanvasPoint(event);
    setDragging({ key: node.key, offsetX: point.x - position.x, offsetY: point.y - position.y });
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const pointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!editable) return;
    if (linkDrag) {
      setLinkDrag({ ...linkDrag, ...toCanvasPoint(event) });
      return;
    }
    if (edgeDrag) {
      setEdgeDrag({ ...edgeDrag, ...toCanvasPoint(event) });
      return;
    }
    if (!dragging) return;
    const point = toCanvasPoint(event);
    const nextX = Math.max(18, point.x - dragging.offsetX);
    const nextY = Math.max(18, point.y - dragging.offsetY);
    update({ ...definition, nodes: definition.nodes.map((node) => node.key === dragging.key ? withNodePosition(node, nextX, nextY) : node) });
  };
  const startEdgeReconnect = (index: number, side: 'from' | 'to', event: PointerEvent<Element>) => {
    if (!editable) return;
    event.preventDefault();
    event.stopPropagation();
    const edge = definition.edges[index];
    if (!edge) return;
    setSelectedEdgeIndex(index);
    setSelectedNodeKey(null);
    setDragging(null);
    setEdgeDrag({ index, side, anchorKey: side === 'from' ? edge.to : edge.from, ...toCanvasPoint(event) });
  };
  const finishEdgeReconnect = (key: string, event: PointerEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    if (!editable || !edgeDrag) return;
    const edge = definition.edges[edgeDrag.index];
    if (!edge) return;
    const nextFromNode = edgeDrag.side === 'from' ? definition.nodes.find((node) => node.key === key) : definition.nodes.find((node) => node.key === edge.from);
    const nextEdge: WorkflowEdgeSpec = edgeDrag.side === 'from' ? { ...edge, from: key, condition: defaultEdgeConditionFor(nextFromNode) } : { ...edge, to: key };
    if (nextEdge.from === nextEdge.to) { message.warning('不能连接到自身'); setEdgeDrag(null); return; }
    update({ ...definition, edges: definition.edges.map((item, index) => index === edgeDrag.index ? nextEdge : item) });
    setSelectedEdgeIndex(edgeDrag.index);
    setEdgeDrag(null);
  };
  const startLinkDrag = (key: string, event: PointerEvent<HTMLButtonElement>) => {
    if (!editable) return;
    event.preventDefault();
    event.stopPropagation();
    setDragging(null);
    setLinkDrag({ from: key, ...toCanvasPoint(event) });
  };
  const finishLinkDrag = (key: string, event: PointerEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    if (!editable) return;
    if (edgeDrag) {
      finishEdgeReconnect(key, event);
      return;
    }
    if (!linkDrag) return;
    connectNodes(linkDrag.from, key);
    setLinkDrag(null);
  };


  useEffect(() => {
    if (!isCanvasFullscreen) return undefined;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsCanvasFullscreen(false);
    };
    document.body.classList.add('workflow-canvas-fullscreen-open');
    window.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.classList.remove('workflow-canvas-fullscreen-open');
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [isCanvasFullscreen]);

  const canvasWidth = Math.max(980, ...definition.nodes.map((node, index) => (positions.get(node.key)?.x ?? index * 220) + 280));
  const canvasHeight = Math.max(560, ...definition.nodes.map((node, index) => (positions.get(node.key)?.y ?? index * 100) + 210));

  return (
    <div className={`workflow-dag-editor ${isCanvasFullscreen ? 'workflow-dag-editor--fullscreen' : ''}`}>
      {editable ? (
        <Space wrap className="workflow-dag-toolbar">
          {NODE_CATALOG.map((node) => <Button key={node.kind} onClick={() => addNode(node.kind)}>+ {node.label}</Button>)}
          {linkDrag ? <Tag color="blue">从 {linkDrag.from} 拖线中：松到目标输入端口完成</Tag> : null}
          {edgeDrag ? <Tag color="purple">正在调整连线{edgeDrag.side === 'from' ? '起点' : '终点'}：松到目标端口完成</Tag> : null}
          <Button onClick={() => setIsCanvasFullscreen((current) => !current)}>{isCanvasFullscreen ? '退出全屏' : '切换全屏'}</Button>
        </Space>
      ) : null}
      <div className={`workflow-node-canvas ${editable ? 'workflow-node-canvas--editable' : 'workflow-node-canvas--readonly'} ${linkDrag || edgeDrag ? 'workflow-node-canvas--linking' : ''}`} style={{ height: isCanvasFullscreen ? undefined : Math.min(720, canvasHeight + 40) }} onPointerMove={pointerMove} onPointerDown={(event) => { if (event.target === event.currentTarget) { setSelectedEdgeIndex(null); } }} onPointerUp={() => { setDragging(null); setLinkDrag(null); setEdgeDrag(null); }}>
        <div ref={spaceRef} className="workflow-node-canvas__space" style={{ width: canvasWidth, height: canvasHeight }} onPointerDown={(event) => { if (event.target === event.currentTarget) setSelectedEdgeIndex(null); }}>
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
              const fromNode = definition.nodes.find((node) => node.key === edge.from);
              const meta = edgeConditionMeta(edge.condition, fromNode);
              const selected = selectedEdgeIndex === index;
              const path = `M ${x1} ${y1} C ${x1 + mid} ${y1}, ${x2 - mid} ${y2}, ${x2} ${y2}`;
              return (
                <g key={`${edge.from}-${edge.to}-${index}`} className={`workflow-edge ${selected ? 'workflow-edge--selected' : ''}`}>
                  {editable ? <path className="workflow-edge__hit" d={path} stroke="transparent" strokeWidth="16" fill="none" onPointerDown={(event) => { event.preventDefault(); event.stopPropagation(); setSelectedEdgeIndex(index); setSelectedNodeKey(null); }} /> : null}
                  <path d={path} stroke={selected ? '#0ea5e9' : meta.color} strokeWidth={selected ? '3.5' : '2.5'} fill="none" markerEnd="url(#workflow-arrow)" />
                  <text className="workflow-edge__label" x={(x1 + x2) / 2} y={(y1 + y2) / 2 - 10} fill={meta.color}>{meta.value}</text>
                  {selected ? (
                    <>
                      <circle className="workflow-edge__handle-ghost" cx={x1} cy={y1} r="7" />
                      <circle className="workflow-edge__handle-ghost" cx={x2} cy={y2} r="7" />
                    </>
                  ) : null}
                </g>
              );
            })}
            {edgeDrag ? (() => {
              const edge = definition.edges[edgeDrag.index];
              const anchor = positions.get(edgeDrag.anchorKey);
              if (!edge || !anchor) return null;
              const anchorX = edgeDrag.side === 'from' ? anchor.x : anchor.x + 218;
              const anchorY = anchor.y + 70;
              const x1 = edgeDrag.side === 'from' ? edgeDrag.x : anchorX;
              const y1 = edgeDrag.side === 'from' ? edgeDrag.y : anchorY;
              const x2 = edgeDrag.side === 'from' ? anchorX : edgeDrag.x;
              const y2 = edgeDrag.side === 'from' ? anchorY : edgeDrag.y;
              const mid = Math.max(80, Math.abs(x2 - x1) / 2);
              return <path className="workflow-node-canvas__temp-edge" d={`M ${x1} ${y1} C ${x1 + mid} ${y1}, ${x2 - mid} ${y2}, ${x2} ${y2}`} stroke="#8b5cf6" strokeWidth="2.5" fill="none" markerEnd="url(#workflow-arrow)" />;
            })() : null}
            {linkDrag ? (() => {
              const from = positions.get(linkDrag.from);
              if (!from) return null;
              const x1 = from.x + 218;
              const y1 = from.y + 70;
              const x2 = linkDrag.x;
              const y2 = linkDrag.y;
              const mid = Math.max(80, Math.abs(x2 - x1) / 2);
              return <path className="workflow-node-canvas__temp-edge" d={`M ${x1} ${y1} C ${x1 + mid} ${y1}, ${x2 - mid} ${y2}, ${x2} ${y2}`} stroke="#0ea5e9" strokeWidth="2.5" fill="none" markerEnd="url(#workflow-arrow)" />;
            })() : null}
          </svg>
          {editable && selectedEdgeIndex !== null && selectedEdgeHandles ? (
            <>
              <button
                className="workflow-edge-rehandle workflow-edge-rehandle--from"
                type="button"
                style={{ left: selectedEdgeHandles.from.x, top: selectedEdgeHandles.from.y }}
                title="拖动调整连线起点"
                onPointerDown={(event) => startEdgeReconnect(selectedEdgeIndex, 'from', event)}
              />
              <button
                className="workflow-edge-rehandle workflow-edge-rehandle--to"
                type="button"
                style={{ left: selectedEdgeHandles.to.x, top: selectedEdgeHandles.to.y }}
                title="拖动调整连线终点"
                onPointerDown={(event) => startEdgeReconnect(selectedEdgeIndex, 'to', event)}
              />
            </>
          ) : null}
          {editable && selectedEdge && selectedEdgeOverlay ? (() => {
            const fromNode = definition.nodes.find((node) => node.key === selectedEdge.from);
            const options = edgeConditionOptionsFor(fromNode);
            const value = selectedEdge.condition ?? defaultEdgeConditionFor(fromNode);
            return (
              <Card size="small" className="workflow-edge-popover" style={{ left: selectedEdgeOverlay.x, top: selectedEdgeOverlay.y }} onPointerDown={(event) => event.stopPropagation()}>
                <Space direction="vertical" size={8}>
                  <Typography.Text strong>{selectedEdge.from} → {selectedEdge.to}</Typography.Text>
                  <Select size="small" value={value} style={{ width: 180 }} options={options.map((option) => ({ label: option.label, value: option.value }))} onChange={(nextValue) => changeEdge(selectedEdgeIndex ?? 0, { condition: nextValue as WorkflowEdgeSpec['condition'] })} />
                  <Space>
                    <Button size="small" danger onClick={() => selectedEdgeIndex !== null && removeEdgeAt(selectedEdgeIndex)}>删除连线</Button>
                    <Typography.Text type="secondary">拖动两端圆点可重连</Typography.Text>
                  </Space>
                </Space>
              </Card>
            );
          })() : null}
          {definition.nodes.map((node, index) => {
            const position = positions.get(node.key) ?? { x: 0, y: 0 };
            const status = statuses.get(node.key) ?? 'design';
            const incoming = definition.edges.filter((edge) => edge.to === node.key);
            const outgoing = definition.edges.filter((edge) => edge.from === node.key);
            const limits = nodeLimits(node);
            return (
              <div className={`workflow-node-card ${editable ? 'workflow-node-card--editable' : ''} ${editable && selectedNodeKey === node.key ? 'workflow-node-card--selected' : ''} ${linkDrag?.from === node.key ? 'workflow-node-card--linking' : ''}`} key={node.key} style={{ left: position.x, top: position.y }} onPointerDown={(event) => pointerDown(node, event)} onClick={() => { if (editable) { setSelectedNodeKey(node.key); setSelectedEdgeIndex(null); } }}>
                {editable && limits.in > 0 ? <button className="workflow-node-port workflow-node-port--input" type="button" onPointerUp={(event) => finishLinkDrag(node.key, event)} onPointerDown={(event) => event.stopPropagation()} title={`输入端口：${incoming.length}/${limits.in}`} /> : null}
                {editable && limits.out > 0 ? <button className="workflow-node-port workflow-node-port--output" type="button" onPointerUp={(event) => finishEdgeReconnect(node.key, event)} onPointerDown={(event) => startLinkDrag(node.key, event)} title={`输出端口：${outgoing.length}/${limits.out}`} /> : null}
                <div className="workflow-node-card__header">
                  <span className="workflow-node-card__index">{index + 1}</span>
                  <span className="workflow-node-card__title">{node.name ?? node.key}</span>
                  <Tag color={STATUS_COLORS[status] ?? 'default'}>{status}</Tag>
                </div>
                <div className="workflow-node-card__body">
                  <Tag color="cyan">{nodeKind(node)}</Tag>
                  <Typography.Text className="workflow-node-card__key">{node.key}</Typography.Text>
                  {node.job_id ? <Typography.Text type="secondary">job: {jobs.find((job) => job.id === node.job_id)?.name ?? node.job_id}</Typography.Text> : null}
                  {node.processor_name ? <Typography.Text type="secondary">processor: {node.processor_name}</Typography.Text> : null}
                  {node.child_workflow_id ? <Typography.Text type="secondary">child: {node.child_workflow_id}</Typography.Text> : null}
                  {nodeKind(node) === 'condition' ? <Typography.Text type="secondary">条件分支</Typography.Text> : null}
                  {nodeKind(node) === 'parallel' ? <Typography.Text type="secondary">并行分发</Typography.Text> : null}
                  {nodeKind(node) === 'approval' ? <Typography.Text type="secondary">人工审批</Typography.Text> : null}
                </div>
                <div className="workflow-node-card__ports">
                  <span>in {incoming.length}/{limits.in}</span>
                  <span>out {outgoing.length}/{limits.out}</span>
                </div>
                {editable ? <Button size="small" danger className="workflow-node-card__delete" onClick={() => removeNode(node.key)}>删除</Button> : null}
              </div>
            );
          })}
        </div>
      </div>

      {editable && selectedNode ? (
        <Card size="small" title={`节点属性 · ${selectedNode.key}`} className="workflow-node-inspector" extra={<Tag color="cyan">{nodeKind(selectedNode)}</Tag>}>
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <Space wrap>
              <Input addonBefore="Key" value={selectedNode.key} style={{ width: 260 }} onChange={(event) => renameNodeKey(selectedNode.key, event.target.value)} />
              <Input addonBefore="名称" value={selectedNode.name ?? ''} style={{ width: 260 }} onChange={(event) => updateNode(selectedNode.key, { name: event.target.value })} />
            </Space>

            {nodeKind(selectedNode) === 'job' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>绑定调度任务</Typography.Text>
                <Select
                  showSearch
                  placeholder="选择一个已创建 Job"
                  value={selectedNode.job_id ?? undefined}
                  options={jobOptions}
                  optionFilterProp="label"
                  style={{ width: '100%' }}
                  onChange={(value) => updateNode(selectedNode.key, { job_id: value })}
                />
                <Input addonBefore="Processor" placeholder="SDK processor name" value={selectedNode.processor_name ?? ''} onChange={(event) => updateNode(selectedNode.key, { processor_name: event.target.value })} />
                <Typography.Text type="secondary">Job 节点会在物化时创建对应 job_instance，并按 Processor 路由到 SDK 处理器；为空时回退 Job 绑定。</Typography.Text>
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'script' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>动态脚本节点</Typography.Text>
                <Space wrap>
                  <Select value={(selectedNode.config as { language?: string } | undefined)?.language ?? 'rhai'} style={{ width: 160 }} options={['rhai', 'python', 'javascript', 'shell'].map((value) => ({ value, label: value }))} onChange={(value) => updateNodeConfig(selectedNode.key, { language: value })} />
                  <Input addonBefore="Sandbox" value={(selectedNode.config as { sandbox?: string } | undefined)?.sandbox ?? 'isolated'} style={{ width: 260 }} onChange={(event) => updateNodeConfig(selectedNode.key, { sandbox: event.target.value })} />
                </Space>
                <Input.TextArea rows={4} placeholder="脚本内容或脚本版本引用" value={(selectedNode.config as { source?: string } | undefined)?.source ?? ''} onChange={(event) => updateNodeConfig(selectedNode.key, { source: event.target.value })} />
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'http' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>HTTP 调用节点</Typography.Text>
                <Space wrap>
                  <Select value={(selectedNode.config as { method?: string } | undefined)?.method ?? 'GET'} style={{ width: 120 }} options={['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].map((value) => ({ value, label: value }))} onChange={(value) => updateNodeConfig(selectedNode.key, { method: value })} />
                  <Input placeholder="https://service.internal/api" value={(selectedNode.config as { url?: string } | undefined)?.url ?? ''} style={{ width: 420 }} onChange={(event) => updateNodeConfig(selectedNode.key, { url: event.target.value })} />
                </Space>
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'condition' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>条件分支节点</Typography.Text>
                <Input.TextArea rows={3} placeholder="例如：context.amount > 1000" value={(selectedNode.config as { expression?: string } | undefined)?.expression ?? ''} onChange={(event) => updateNodeConfig(selectedNode.key, { expression: event.target.value })} />
                <Typography.Text type="secondary">建议将两条出边分别设置为 on_success / on_failure，表达式通过为 success，不通过为 failure。</Typography.Text>
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'parallel' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>并行分发节点</Typography.Text>
                <Input addonBefore="策略" value={(selectedNode.config as { strategy?: string } | undefined)?.strategy ?? 'fan-out'} onChange={(event) => updateNodeConfig(selectedNode.key, { strategy: event.target.value })} />
                <Typography.Text type="secondary">该节点可连接多条输出边，用于同时推进多个后继分支。</Typography.Text>
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'join' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>并行汇聚节点</Typography.Text>
                <Input addonBefore="Quorum" value={(selectedNode.config as { quorum?: string } | undefined)?.quorum ?? 'all'} onChange={(event) => updateNodeConfig(selectedNode.key, { quorum: event.target.value })} />
                <Typography.Text type="secondary">当前推进规则要求所有满足条件的前置边完成后才会进入该节点。</Typography.Text>
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'delay' ? (
              <Input addonBefore="延迟秒数" value={(selectedNode.config as { seconds?: string } | undefined)?.seconds ?? '60'} onChange={(event) => updateNodeConfig(selectedNode.key, { seconds: event.target.value })} />
            ) : null}

            {nodeKind(selectedNode) === 'approval' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>人工审批节点</Typography.Text>
                <Input addonBefore="审批人" placeholder="alice,bob 或 role:ops" value={(selectedNode.config as { approvers?: string } | undefined)?.approvers ?? ''} onChange={(event) => updateNodeConfig(selectedNode.key, { approvers: event.target.value })} />
                <Input addonBefore="超时" placeholder="24h" value={(selectedNode.config as { timeout?: string } | undefined)?.timeout ?? '24h'} onChange={(event) => updateNodeConfig(selectedNode.key, { timeout: event.target.value })} />
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'notification' ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>通知节点</Typography.Text>
                <Space wrap>
                  <Select value={(selectedNode.config as { channel?: string } | undefined)?.channel ?? 'webhook'} style={{ width: 160 }} options={['webhook', 'email', 'lark', 'dingtalk'].map((value) => ({ value, label: value }))} onChange={(value) => updateNodeConfig(selectedNode.key, { channel: value })} />
                  <Input placeholder="目标地址 / 群 / 收件人" value={(selectedNode.config as { target?: string } | undefined)?.target ?? ''} style={{ width: 360 }} onChange={(event) => updateNodeConfig(selectedNode.key, { target: event.target.value })} />
                </Space>
                <Input.TextArea rows={3} placeholder="通知模板" value={(selectedNode.config as { template?: string } | undefined)?.template ?? ''} onChange={(event) => updateNodeConfig(selectedNode.key, { template: event.target.value })} />
              </Space>
            ) : null}

            {(nodeKind(selectedNode) === 'map' || nodeKind(selectedNode) === 'map_reduce') ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Typography.Text strong>分片输入 map_items</Typography.Text>
                <Input addonBefore="Processor" placeholder="SDK processor name" value={selectedNode.processor_name ?? ''} onChange={(event) => updateNode(selectedNode.key, { processor_name: event.target.value })} />
                <Input.TextArea key={`map-items-${selectedNode.key}`} rows={4} defaultValue={JSON.stringify(selectedNode.map_items ?? [], null, 2)} onBlur={(event) => updateMapItems(selectedNode.key, event.target.value)} />
              </Space>
            ) : null}

            {nodeKind(selectedNode) === 'sub_workflow' ? (
              <Input addonBefore="子工作流 ID" value={selectedNode.child_workflow_id ?? ''} onChange={(event) => updateNode(selectedNode.key, { child_workflow_id: event.target.value })} />
            ) : null}
          </Space>
        </Card>
      ) : null}
    </div>
  );
}


export function WorkflowsPage() {
  const canManageWorkflows = useCan('workflows', 'manage');
  const canExecuteWorkflows = useCan('workflows', 'execute');
  const { query, setQuery } = useUrlQueryState({ page: 1, page_size: 8, keyword: '', status: '' });
  const [items, setItems] = useState<WorkflowSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeWorkflow, setActiveWorkflow] = useState<WorkflowSummary | null>(null);
  const [activeInstance, setActiveInstance] = useState<WorkflowInstanceSummary | null>(null);
  const [events, setEvents] = useState<InstanceEventSummary[]>([]);
  const [shards, setShards] = useState<WorkflowShardSummary[]>([]);
  const [expandedWorkflowId, setExpandedWorkflowId] = useState<string | null>(null);
  const navigate = useNavigate();

  const filteredItems = useMemo(() => items.filter((item) => {
    const keyword = String(query.keyword ?? '').trim().toLowerCase();
    const status = String(query.status ?? '').trim();
    const matchesKeyword = keyword === '' || [item.name, item.id, item.created_by].some((value) => value.toLowerCase().includes(keyword));
    const matchesStatus = status === '' || item.status === status;
    return matchesKeyword && matchesStatus;
  }), [items, query.keyword, query.status]);

  const pagedItems = useMemo(() => {
    const page = Number(query.page) || 1;
    const pageSize = Number(query.page_size) || 8;
    return filteredItems.slice((page - 1) * pageSize, page * pageSize);
  }, [filteredItems, query.page, query.page_size]);

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

  const validate = async (item: WorkflowSummary) => {
    const result = await validateWorkflow(item.id);
    if (result.valid) message.success('DAG 校验通过');
    else message.error(result.errors.join('; '));
  };

  const openRunView = (item: WorkflowSummary) => {
    setActiveWorkflow((current) => {
      if (current?.id !== item.id) {
        setActiveInstance(null);
        setEvents([]);
        setShards([]);
      }
      return item;
    });
    setExpandedWorkflowId((current) => current === item.id ? null : item.id);
  };

  const run = async (item: WorkflowSummary) => {
    const instance = await runWorkflow(item.id);
    setActiveWorkflow(item);
    setActiveInstance(instance);
    setEvents([]);
    setExpandedWorkflowId(item.id);
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
    try {
      const result = await materializeNextWorkflowNode();
      setActiveInstance(result.instance);
      setShards(result.shards);
      message.success(`已准备节点执行：${result.node.node_key}`);
    } catch (error) {
      if (error instanceof ApiClientError && error.message.includes('no queued workflow node')) {
        message.info('当前没有等待准备的节点：请先运行工作流，或先推进已有运行中节点。');
        return;
      }
      message.error(error instanceof Error ? error.message : '准备下一节点失败');
    }
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

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div className="hero-panel workflow-hero">
        <div className="hero-panel__content">
          <Tag className="soft-tag" color="blue">Phase 2 · Workflow Engine</Tag>
          <Typography.Title level={1}>工作流列表</Typography.Title>
          <Typography.Paragraph className="hero-panel__desc">统一查看、运行、校验工作流；运行视图和事件流按条目手风琴展开。</Typography.Paragraph>
        </div>
        <div className="hero-panel__summary"><strong>{items.length}</strong><span>flows</span></div>
      </div>

      <Card title="工作流列表" extra={<Space wrap><PermissionGate resource="workflows" action="manage"><Button onClick={() => navigate('/workflows/new')} type="primary">新增工作流</Button></PermissionGate><Input allowClear placeholder="搜索工作流" value={String(query.keyword ?? '')} onChange={(event) => setQuery({ keyword: event.target.value, page: 1 })} style={{ width: 200 }} /><Select allowClear placeholder="状态" value={query.status || undefined} onChange={(value) => setQuery({ status: value ?? '', page: 1 })} style={{ width: 130 }} options={[{ value: 'enabled' }, { value: 'disabled' }]} /><Button onClick={fetchItems}>刷新</Button></Space>}>
        <List
          loading={loading}
          dataSource={pagedItems}
          pagination={{ pageSize: Number(query.page_size) || 8, current: Number(query.page) || 1, total: filteredItems.length, onChange: (page, pageSize) => setQuery({ page, page_size: pageSize }) }}
          locale={{ emptyText: '暂无工作流' }}
          renderItem={(item) => (
            <div className="workflow-list-entry">
              <List.Item
                actions={[
                  <Button key="view" onClick={() => openRunView(item)}>{expandedWorkflowId === item.id ? '收起运行视图' : '运行视图'}</Button>,
                  canManageWorkflows ? <Button key="edit" onClick={() => navigate(`/workflows/${encodeURIComponent(item.id)}/edit`)}>编辑</Button> : null,
                  <Button key="validate" onClick={() => validate(item)}>校验</Button>,
                  canExecuteWorkflows ? <Popconfirm key="run" title="确认运行工作流？" description={`将创建新的工作流实例：${item.name}`} onConfirm={() => run(item)}><Button type="primary">运行</Button></Popconfirm> : null,
                ]}
              >
                <List.Item.Meta
                  title={<Space><span>{item.name}</span><Tag color={STATUS_COLORS[item.status] ?? 'blue'}>{item.status}</Tag></Space>}
                  description={<span>{item.id} · nodes: {item.definition.nodes.length} · edges: {item.definition.edges.length}</span>}
                />
              </List.Item>
              {expandedWorkflowId === item.id ? (
                <div className="workflow-inline-run-panel">
                  <Space direction="vertical" size={16} style={{ width: '100%' }}>
                    <Card size="small" title="运行视图" extra={<Space wrap>{canExecuteWorkflows ? <Popconfirm title="准备下一节点执行？" description="将把 queued 工作流节点物化为实际执行项。" onConfirm={materializeNext}><Button>准备下一节点执行</Button></Popconfirm> : null}{canExecuteWorkflows ? <Popconfirm title="标记当前节点成功？" description="该人工推进会改变工作流实例状态。" onConfirm={completeFirstQueued}><Button disabled={!activeInstance}>标记当前节点成功</Button></Popconfirm> : null}{canExecuteWorkflows ? <Popconfirm title="重试失败节点？" description="将对第一个失败节点执行 retry 恢复操作。" onConfirm={recoverFirstFailed}><Button disabled={!activeInstance}>重试失败节点</Button></Popconfirm> : null}<Button onClick={refreshShards} disabled={!activeInstance}>刷新 Shards</Button></Space>}>
                      <DagPreview definition={item.definition} instance={activeWorkflow?.id === item.id ? activeInstance : null} />
                      {activeWorkflow?.id === item.id && shards.length > 0 ? <List size="small" style={{ marginTop: 16 }} dataSource={shards} renderItem={(shard) => <List.Item><Typography.Text>{shard.node_key}#{shard.shard_index} · {shard.status} · {JSON.stringify(shard.input)}</Typography.Text></List.Item>} /> : null}
                    </Card>
                    <Card size="small" title="实例事件流">
                      {activeWorkflow?.id === item.id && activeInstance ? <Typography.Text type="secondary">{activeInstance.id} · {activeInstance.status}</Typography.Text> : <Typography.Text type="secondary">运行工作流后展示 SSE 事件</Typography.Text>}
                      <Timeline style={{ marginTop: 18 }} items={(activeWorkflow?.id === item.id ? events : []).map((event) => ({ color: event.event_type.includes('failed') ? 'red' : 'blue', children: <span>{event.created_at} · {event.event_type} · {event.message}</span> }))} />
                    </Card>
                  </Space>
                </div>
              ) : null}
            </div>
          )}
        />
      </Card>
    </Space>
  );
}

export function WorkflowEditorPage() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [previewMode, setPreviewMode] = useState<'visual' | 'json' | 'yaml'>('visual');
  const [draft, setDraft] = useState(DEFAULT_DEFINITION);
  const [dryRun, setDryRun] = useState<WorkflowDryRunResponse | null>(null);
  const [form] = Form.useForm<{ name: string }>();
  const navigate = useNavigate();
  const params = useParams();
  const workflowId = params.id;
  const isEdit = Boolean(workflowId);

  const fetchEditorData = async () => {
    setLoading(true);
    try {
      const jobPage = await listJobs();
      setJobs(jobPage.items);
      if (workflowId) {
        const workflow = await getWorkflow(workflowId);
        form.setFieldsValue({ name: workflow.name });
        setDraft(stringifyDefinition(normalizeWorkflowDefinition(workflow.definition)));
      }
    } finally { setLoading(false); }
  };

  useEffect(() => { void fetchEditorData(); }, [workflowId]);

  const previewDefinition = useMemo(() => {
    try { return parseDefinition(draft); } catch { return null; }
  }, [draft]);
  const yamlPreview = previewDefinition ? definitionToYaml(previewDefinition) : '';

  const updateDefinition = (definition: WorkflowDefinition) => {
    setDraft(stringifyDefinition(definition));
    setDryRun(null);
  };

  const submit = async (values: { name: string }) => {
    let definition: WorkflowDefinition;
    try { definition = parseDefinition(draft); }
    catch { message.error('Workflow definition 必须是合法 JSON'); return; }
    if (workflowId) {
      await updateWorkflow(workflowId, { name: values.name, definition });
      message.success('Workflow 已更新');
    } else {
      await createWorkflow({ name: values.name, definition });
      message.success('Workflow 已创建');
    }
    navigate('/workflows');
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

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div className="hero-panel workflow-hero workflow-editor-hero">
        <div className="hero-panel__content">
          <Button className="workflow-back-button" onClick={() => navigate('/workflows')}>← 返回工作流列表</Button>
          <Tag className="soft-tag" color="blue">Phase 2 · Workflow Engine</Tag>
          <Typography.Title level={1}>{isEdit ? '编辑工作流' : '新增工作流'}</Typography.Title>
          <Typography.Paragraph className="hero-panel__desc">
            使用节点画布编辑 DAG、节点属性、端口连线和边条件；保存后回到工作流列表统一运行和查看实例。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary"><strong>{previewDefinition?.nodes.length ?? 0}</strong><span>nodes</span></div>
      </div>

      <Card
        title="可视化节点画布"
        loading={loading}
        extra={<Space wrap><Segmented value={previewMode} onChange={(value) => setPreviewMode(value as 'visual' | 'json' | 'yaml')} options={[{ label: '画布', value: 'visual' }, { label: 'JSON', value: 'json' }, { label: 'YAML', value: 'yaml' }]} /><Button onClick={dryRunDraft}>Dry-run</Button></Space>}
      >
        <Form form={form} layout="inline" onFinish={submit} className="workflow-create-inline" initialValues={{ name: '' }}>
          <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input placeholder="daily-pipeline" style={{ width: 260 }} /></Form.Item>
          <Form.Item><Button type="primary" htmlType="submit">{isEdit ? '保存工作流' : '创建工作流'}</Button></Form.Item>
          {dryRun ? <Alert type={dryRun.validation.valid ? 'success' : 'error'} message={dryRun.validation.valid ? 'Dry-run 通过' : 'Dry-run 失败'} description={`start: ${dryRun.start_nodes.join(', ') || '-'} · nodes: ${dryRun.node_count} · edges: ${dryRun.edge_count}${dryRun.validation.errors.length ? ` · ${dryRun.validation.errors.join('; ')}` : ''}`} /> : null}
        </Form>
        {previewDefinition && previewMode === 'visual' ? <DagPreview definition={previewDefinition} jobs={jobs} editable onChange={updateDefinition} /> : null}
        {previewMode === 'json' ? <Input.TextArea className="workflow-definition-preview" rows={18} spellCheck={false} value={draft} onChange={(event) => { setDraft(event.target.value); setDryRun(null); }} /> : null}
        {previewMode === 'yaml' ? <Input.TextArea className="workflow-definition-preview" rows={18} spellCheck={false} value={yamlPreview || 'JSON 解析失败，无法生成 YAML'} readOnly /> : null}
        {!previewDefinition && previewMode === 'visual' ? <Alert type="warning" message="JSON 解析失败，无法预览画布" /> : null}
      </Card>
    </Space>
  );
}
