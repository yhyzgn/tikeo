import { FullscreenExitOutlined, FullscreenOutlined } from '@ant-design/icons';
import { Button, Card, Empty, Space, Spin, Tag, Typography } from 'antd';
import { useState } from 'react';

import type { JobTopologyEdge, JobTopologyNode, JobTopologyResponse } from '../../api/client';

interface TopologyCanvasProps {
  topology: JobTopologyResponse | null;
  loading?: boolean;
  selectedJobId: string | null;
  onSelectJob: (jobId: string) => void;
}

interface CanvasNode {
  node: JobTopologyNode;
  position: Point;
}

interface Point {
  x: number;
  y: number;
}

interface NodeBox {
  id: string;
  left: number;
  right: number;
  top: number;
  bottom: number;
}

const NODE_WIDTH = 148;
const NODE_HEIGHT = 52;
const NODE_PADDING = 18;
const FALLBACK_X_STEP = 190;
const FALLBACK_Y = 120;

export function TopologyCanvas({ topology, loading = false, selectedJobId, onSelectJob }: TopologyCanvasProps) {
  const [fullscreen, setFullscreen] = useState(false);
  const nodes = topology?.nodes ?? [];
  const edges = topology?.edges ?? [];
  if (!topology || nodes.length === 0) return <Card size="small" title="拓扑图形画布"><Empty description={loading ? '加载拓扑数据...' : '暂无拓扑数据'} /></Card>;

  const positioned = nodes.map((node, index) => ({ node, position: nodePosition(node, index) }));
  const nodeById = new Map(positioned.map((item) => [item.node.id, item]));
  const boxes = positioned.map(nodeBox);
  const width = Math.max(760, ...positioned.map((item) => item.position.x + NODE_WIDTH + 80));
  const height = Math.max(360, ...positioned.map((item) => item.position.y + NODE_HEIGHT + 80));

  return (
    <Card
      size="small"
      className={fullscreen ? 'topology-canvas-card topology-canvas-card--fullscreen' : 'topology-canvas-card'}
      title="拓扑图形画布"
      extra={<Button icon={fullscreen ? <FullscreenExitOutlined /> : <FullscreenOutlined />} onClick={() => setFullscreen((value) => !value)}>{fullscreen ? '退出全屏' : '全屏'}</Button>}
      styles={{ body: { overflow: 'auto', padding: 12, minHeight: fullscreen ? 'calc(100vh - 96px)' : 420 } }}
    >
      <Typography.Paragraph type="secondary" style={{ marginBottom: 12 }}>
        点击 Job 节点加载跨工作流影响分析；画布同时展示 workflow_job_dependency 与 workflow_job_ref。
      </Typography.Paragraph>
      <Spin spinning={loading}>
        <svg width={width} height={height} role="img" aria-label="任务拓扑图形画布">
          <defs>
            <marker id="topology-arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto" markerUnits="strokeWidth">
              <path d="M0,0 L0,6 L9,3 z" fill="#8c8c8c" />
            </marker>
          </defs>
          {edges.map((edge) => renderEdge(edge, nodeById, boxes))}
          {positioned.map(({ node, position }) => renderNode(node, position, selectedJobId, onSelectJob))}
        </svg>
      </Spin>
    </Card>
  );
}

function renderEdge(edge: JobTopologyEdge, nodeById: Map<string, CanvasNode>, boxes: NodeBox[]) {
  const from = nodeById.get(edge.from);
  const to = nodeById.get(edge.to);
  if (!from || !to) return null;
  const points = routeOrthogonalEdge(from, to, boxes);
  const path = pointsToPath(points);
  const labelPoint = points[Math.max(1, Math.floor(points.length / 2))];
  const stroke = edge.type === 'workflow_job_dependency' ? '#1677ff' : '#8c8c8c';
  const dashed = edge.type === 'workflow_job_ref';
  return (
    <g key={edge.id} className="topology-edge">
      <path className="topology-flow-line" d={path} fill="none" stroke={stroke} strokeWidth={1.9} markerEnd="url(#topology-arrow)" strokeDasharray={dashed ? '7 5' : '10 8'} />
      <path className="topology-flow-pulse" d={path} fill="none" stroke={stroke} strokeWidth={3.2} strokeLinecap="round" strokeDasharray="1 70" />
      {edge.condition ? <text x={labelPoint.x + 4} y={labelPoint.y - 6} fontSize="11" fill="#595959">{edge.condition}</text> : null}
    </g>
  );
}

function routeOrthogonalEdge(from: CanvasNode, to: CanvasNode, boxes: NodeBox[]) {
  const start = { x: from.position.x + NODE_WIDTH / 2, y: from.position.y + NODE_HEIGHT };
  const end = { x: to.position.x + NODE_WIDTH / 2, y: to.position.y };
  const direct = [start, end];
  const blockers = boxes.filter((box) => box.id !== from.node.id && box.id !== to.node.id);
  if (!pathIntersectsNodeBox(direct, blockers)) return direct;

  const midY = pickClearHorizontalY(start, end, blockers);
  const routed = [start, { x: start.x, y: midY }, { x: end.x, y: midY }, end];
  if (!pathIntersectsNodeBox(routed, blockers)) return routed;

  const sideX = pickClearVerticalX(start, end, blockers);
  return [start, { x: start.x, y: start.y + NODE_PADDING }, { x: sideX, y: start.y + NODE_PADDING }, { x: sideX, y: end.y - NODE_PADDING }, { x: end.x, y: end.y - NODE_PADDING }, end];
}

function pickClearHorizontalY(start: Point, end: Point, blockers: NodeBox[]) {
  const candidates = [
    (start.y + end.y) / 2,
    Math.min(start.y, end.y) - NODE_PADDING * 2,
    Math.max(start.y, end.y) + NODE_PADDING * 2,
    ...blockers.map((box) => box.top - NODE_PADDING),
    ...blockers.map((box) => box.bottom + NODE_PADDING),
  ];
  return candidates.find((y) => !segmentIntersectsAnyBox({ x: start.x, y }, { x: end.x, y }, blockers)) ?? candidates[0];
}

function pickClearVerticalX(start: Point, end: Point, blockers: NodeBox[]) {
  const candidates = [
    Math.min(start.x, end.x) - NODE_WIDTH,
    Math.max(start.x, end.x) + NODE_WIDTH,
    ...blockers.map((box) => box.left - NODE_PADDING),
    ...blockers.map((box) => box.right + NODE_PADDING),
  ];
  return candidates.find((x) => !segmentIntersectsAnyBox({ x, y: start.y }, { x, y: end.y }, blockers)) ?? candidates[0];
}

function pathIntersectsNodeBox(points: Point[], boxes: NodeBox[]) {
  return points.slice(1).some((point, index) => segmentIntersectsAnyBox(points[index], point, boxes));
}

function segmentIntersectsAnyBox(start: Point, end: Point, boxes: NodeBox[]) {
  return boxes.some((box) => intersectsNodeBox(start, end, box));
}

function intersectsNodeBox(start: Point, end: Point, box: NodeBox) {
  const horizontal = start.y === end.y;
  const vertical = start.x === end.x;
  if (!horizontal && !vertical) return lineBoundsOverlapBox(start, end, box);
  if (horizontal) return between(start.y, box.top, box.bottom) && rangesOverlap(start.x, end.x, box.left, box.right);
  return between(start.x, box.left, box.right) && rangesOverlap(start.y, end.y, box.top, box.bottom);
}

function lineBoundsOverlapBox(start: Point, end: Point, box: NodeBox) {
  return rangesOverlap(start.x, end.x, box.left, box.right) && rangesOverlap(start.y, end.y, box.top, box.bottom);
}

function rangesOverlap(a: number, b: number, min: number, max: number) {
  return Math.max(Math.min(a, b), min) <= Math.min(Math.max(a, b), max);
}

function between(value: number, min: number, max: number) {
  return value >= min && value <= max;
}

function pointsToPath(points: Point[]) {
  const [first, ...rest] = points;
  return [`M ${first.x} ${first.y}`, ...rest.map((point) => `L ${point.x} ${point.y}`)].join(' ');
}

function nodeBox(item: CanvasNode): NodeBox {
  return {
    id: item.node.id,
    left: item.position.x - NODE_PADDING,
    right: item.position.x + NODE_WIDTH + NODE_PADDING,
    top: item.position.y - NODE_PADDING,
    bottom: item.position.y + NODE_HEIGHT + NODE_PADDING,
  };
}

function renderNode(node: JobTopologyNode, position: Point, selectedJobId: string | null, onSelectJob: (jobId: string) => void) {
  const isJob = node.type === 'job';
  const selected = selectedJobId === node.id;
  const fill = isJob ? '#e6f4ff' : '#f6ffed';
  const stroke = selected ? '#fa8c16' : isJob ? '#1677ff' : '#52c41a';
  return (
    <g key={node.id} className="topology-node" transform={`translate(${position.x},${position.y})`} onClick={() => { if (isJob) onSelectJob(node.id); }} style={{ cursor: isJob ? 'pointer' : 'default' }}>
      <rect width={NODE_WIDTH} height={NODE_HEIGHT} rx="8" fill={fill} stroke={stroke} strokeWidth={selected ? 2.5 : 1.5} />
      <text x="12" y="22" fontSize="13" fontWeight={600} fill="#262626" data-runtime-text>{truncate(node.label, 17)}</text>
      <text x="12" y="40" fontSize="11" fill="#595959" data-runtime-text>{node.type}</text>
    </g>
  );
}

function nodePosition(node: JobTopologyNode, index: number) {
  const raw = node.metadata.position;
  if (isPosition(raw)) return raw;
  const layer = typeof node.metadata.layer === 'number' ? node.metadata.layer : 0;
  return { x: 80 + (index * FALLBACK_X_STEP), y: FALLBACK_Y + (layer * 180) };
}

function isPosition(value: unknown): value is Point {
  return typeof value === 'object' && value !== null && typeof (value as { x?: unknown }).x === 'number' && typeof (value as { y?: unknown }).y === 'number';
}

function truncate(value: string, max: number) {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

export function ImpactJobTags({ jobs, empty }: { jobs: Array<{ id: string; name: string }>; empty: string }) {
  if (jobs.length === 0) return <Typography.Text type="secondary">{empty}</Typography.Text>;
  return <Space wrap>{jobs.map((job) => <Tag key={job.id} color="blue" data-runtime-text>{job.name}</Tag>)}</Space>;
}
