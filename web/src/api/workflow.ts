import { API_BASE, request, type JobTopologyResponse } from './client';

export interface WorkflowNodeSpec {
  key: string;
  name?: string | null;
  kind?: 'job' | 'map' | 'map_reduce' | 'sub_workflow' | string | null;
  jobId?: string | null;
  processorName?: string | null;
  childWorkflowId?: string | null;
  mapItems?: unknown[] | null;
  config?: unknown;
}

export type WorkflowEdgeCondition = 'always' | 'on_success' | 'on_failure';

export interface WorkflowEdgeSpec {
  from: string;
  to: string;
  condition?: WorkflowEdgeCondition | null;
}

export interface WorkflowDefinition {
  nodes: WorkflowNodeSpec[];
  edges: WorkflowEdgeSpec[];
}

export interface WorkflowSummary {
  id: string;
  name: string;
  definition: WorkflowDefinition;
  status: string;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface WorkflowValidationResult {
  valid: boolean;
  errors: string[];
}

export interface WorkflowDryRunResponse {
  validation: WorkflowValidationResult;
  startNodes: string[];
  nodeCount: number;
  edgeCount: number;
}

export interface WorkflowNodeInstanceSummary {
  id: string;
  workflowInstanceId: string;
  nodeKey: string;
  status: string;
  jobInstanceId: string | null;
  childWorkflowInstanceId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface WorkflowInstanceSummary {
  id: string;
  workflowId: string;
  status: string;
  triggerType: string;
  nodes: WorkflowNodeInstanceSummary[];
  createdAt: string;
  updatedAt: string;
}

export interface WorkflowAdvanceResult {
  instance: WorkflowInstanceSummary;
  queuedNodes: string[];
  completed: boolean;
}

export interface WorkflowShardSummary {
  id: string;
  workflowInstanceId: string;
  workflowNodeInstanceId: string;
  nodeKey: string;
  shardIndex: number;
  status: string;
  input: unknown;
  output: unknown | null;
  createdAt: string;
  updatedAt: string;
}

export interface MaterializeWorkflowNodeResult {
  instance: WorkflowInstanceSummary;
  node: WorkflowNodeInstanceSummary;
  shards: WorkflowShardSummary[];
}

export interface RecoverWorkflowNodeResult {
  instance: WorkflowInstanceSummary;
  queuedNodes: string[];
}

export interface DispatchQueueSummary {
  id: string;
  jobInstanceId: string | null;
  workflowNodeInstanceId: string | null;
  priority: number;
  runAfter: string;
  status: string;
  attempt: number;
  workerSelector: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface QueueOverview {
  pending: number;
  running: number;
  done: number;
  failed: number;
  items: DispatchQueueSummary[];
}

export interface WorkerSummary {
  workerId: string;
  logicalInstanceId: string;
  clientInstanceId: string | null;
  app: string;
  namespace: string;
  cluster: string;
  region: string;
  capabilities: string[];
  structuredCapabilities?: WorkerCapabilitiesSummary;
  workerPool?: string | null;
  master?: WorkerMasterSummary;
  generation: number;
  status: string;
  statusReason: string | null;
  replacedByWorkerId: string | null;
  lastSequence: number;
}

export interface WorkerMasterSummary {
  domain: string;
  isMaster: boolean;
  masterWorkerId: string | null;
  term: number;
  fencingToken: string | null;
}

export interface WorkerCapabilitiesSummary {
  tags: string[];
  normalProcessors?: WorkerProcessorSummary[];
  scriptRunners: WorkerScriptRunnerSummary[];
  pluginProcessors: WorkerPluginProcessorSummary[];
}

export interface WorkerProcessorSummary {
  name: string;
  description: string;
}

export interface WorkerScriptRunnerSummary {
  language: string;
  sandboxBackend: string;
}

export interface WorkerPluginProcessorSummary {
  type: string;
  processorNames: string[];
  processors?: WorkerProcessorSummary[];
}

export interface WorkerSessionHistorySummary {
  workerId: string;
  logicalInstanceId: string;
  generation: number;
  status: string;
  statusReason: string | null;
  statusEvidence: string | null;
  leaseExpiresAt: string;
  lastHeartbeatAt: string;
  lastSequence: number;
  replacedByWorkerId: string | null;
}

export interface WorkerSessionEventSummary {
  id: string;
  workerId: string;
  logicalInstanceId: string;
  eventType: string;
  reason: string | null;
  detailJson: string | null;
  createdAt: string;
}

export interface WorkerLifecycleHistoryResponse {
  sessions: WorkerSessionHistorySummary[];
  events: WorkerSessionEventSummary[];
}

export interface WorkerListResponse {
  online: number;
  items: WorkerSummary[];
}

export interface InstanceEventSummary {
  id: string;
  instanceId: string;
  instance_type: string;
  eventType: string;
  message: string;
  payload: string | null;
  createdAt: string;
}

export interface WorkflowReplayResponse {
  instance: WorkflowInstanceSummary;
  workflow: WorkflowSummary;
  events: InstanceEventSummary[];
  graph: JobTopologyResponse;
}

function normalizeWorkflowEdgeCondition(condition: unknown): WorkflowEdgeSpec['condition'] {
  if (condition === null || condition === undefined) {
    return condition as null | undefined;
  }
  if (typeof condition !== 'string') {
    return condition as WorkflowEdgeSpec['condition'];
  }
  const normalized = condition.trim().toLowerCase();
  if (normalized === 'success' || normalized === 'succeeded') {
    return 'on_success';
  }
  if (normalized === 'failure' || normalized === 'failed') {
    return 'on_failure';
  }
  return condition as WorkflowEdgeSpec['condition'];
}

export function normalizeWorkflowDefinition(definition: WorkflowDefinition): WorkflowDefinition {
  return {
    ...definition,
    edges: definition.edges.map((edge) => ({
      ...edge,
      condition: normalizeWorkflowEdgeCondition(edge.condition),
    })),
  };
}

function normalizeWorkflowPayload(payload: { name: string; definition: WorkflowDefinition }): { name: string; definition: WorkflowDefinition } {
  return { ...payload, definition: normalizeWorkflowDefinition(payload.definition) };
}

export async function listWorkflows(): Promise<WorkflowSummary[]> {
  return request<WorkflowSummary[]>('/api/v1/workflows');
}

export async function getWorkflow(id: string): Promise<WorkflowSummary> {
  return request<WorkflowSummary>(`/api/v1/workflows/${encodeURIComponent(id)}`);
}

export async function createWorkflow(payload: { name: string; definition: WorkflowDefinition }): Promise<WorkflowSummary> {
  return request<WorkflowSummary>('/api/v1/workflows', { method: 'POST', body: JSON.stringify(normalizeWorkflowPayload(payload)) });
}

export async function updateWorkflow(id: string, payload: { name: string; definition: WorkflowDefinition }): Promise<WorkflowSummary> {
  return request<WorkflowSummary>(`/api/v1/workflows/${encodeURIComponent(id)}`, { method: 'PATCH', body: JSON.stringify(normalizeWorkflowPayload(payload)) });
}

export async function validateWorkflow(id: string): Promise<WorkflowValidationResult> {
  return request<WorkflowValidationResult>(`/api/v1/workflows/${encodeURIComponent(id)}/validate`, { method: 'POST', body: JSON.stringify({}) });
}

export async function dryRunWorkflow(definition: WorkflowDefinition): Promise<WorkflowDryRunResponse> {
  return request<WorkflowDryRunResponse>('/api/v1/workflows/dry-run', { method: 'POST', body: JSON.stringify(normalizeWorkflowDefinition(definition)) });
}

export async function runWorkflow(id: string): Promise<WorkflowInstanceSummary> {
  return request<WorkflowInstanceSummary>(`/api/v1/workflows/${encodeURIComponent(id)}/run`, { method: 'POST', body: JSON.stringify({ triggerType: 'api' }) });
}

export async function getWorkflowInstance(instanceId: string): Promise<WorkflowInstanceSummary> {
  return request<WorkflowInstanceSummary>(`/api/v1/workflow-instances/${encodeURIComponent(instanceId)}`);
}

export async function getWorkflowReplay(instanceId: string): Promise<WorkflowReplayResponse> {
  return request<WorkflowReplayResponse>(`/api/v1/workflow-instances/${encodeURIComponent(instanceId)}/replay`);
}

export async function advanceWorkflowInstance(instanceId: string, payload: { nodeKey: string; status: string; message?: string }): Promise<WorkflowAdvanceResult> {
  return request<WorkflowAdvanceResult>(`/api/v1/workflow-instances/${encodeURIComponent(instanceId)}/advance`, { method: 'POST', body: JSON.stringify(payload) });
}

export async function materializeNextWorkflowNode(): Promise<MaterializeWorkflowNodeResult> {
  return request<MaterializeWorkflowNodeResult>('/api/v1/workflow-instances/materialize-next', { method: 'POST', body: JSON.stringify({}) });
}

export async function recoverWorkflowNode(instanceId: string, payload: { nodeKey: string; action: 'retry' | 'skip' | 'fail'; message?: string }): Promise<RecoverWorkflowNodeResult> {
  return request<RecoverWorkflowNodeResult>(`/api/v1/workflow-instances/${encodeURIComponent(instanceId)}/recover`, { method: 'POST', body: JSON.stringify(payload) });
}

export async function listWorkflowShards(instanceId: string): Promise<WorkflowShardSummary[]> {
  return request<WorkflowShardSummary[]>(`/api/v1/workflow-instances/${encodeURIComponent(instanceId)}/shards`);
}

export async function listWorkers(): Promise<WorkerListResponse> {
  return request<WorkerListResponse>('/api/v1/workers');
}

export async function getWorkerLifecycleHistory(): Promise<WorkerLifecycleHistoryResponse> {
  return request<WorkerLifecycleHistoryResponse>('/api/v1/workers/history');
}

export async function getDispatchQueue(): Promise<QueueOverview> {
  return request<QueueOverview>('/api/v1/dispatch-queue');
}

export function workflowEventStreamUrl(instanceId: string): string {
  return `${API_BASE}/api/v1/events/instances/${encodeURIComponent(instanceId)}/stream`;
}
