import { ApartmentOutlined, CrownOutlined, NodeIndexOutlined, SearchOutlined } from '@ant-design/icons';
import { Badge, Card, Collapse, Empty, Input, List, Select, Space, Tag, Tooltip, Typography } from 'antd';
import { useMemo, useState } from 'react';

import type { WorkerListResponse, WorkerSummary } from '../../api/client';
import { filterWorkers, groupWorkersByNamespaceApp, uniqueSorted } from './workerPageModel';

export function visibleCapabilityTags(worker: WorkerSummary) {
  return uniqueSorted(worker.structuredCapabilities?.tags ?? []);
}

export function visibleSdkProcessors(worker: WorkerSummary) {
  return worker.structuredCapabilities?.sdkProcessors ?? [];
}

export function capabilityFilterValues(worker: WorkerSummary) {
  return [
    ...visibleCapabilityTags(worker),
    ...visibleSdkProcessors(worker).map((name) => `SDK:${name}`),
    ...(worker.structuredCapabilities?.scriptRunners.map((runner) => `Script:${runner.language}`) ?? []),
    ...(worker.structuredCapabilities?.pluginProcessors.flatMap((plugin) =>
      plugin.processorNames.map((name) => `Plugin:${plugin.type}:${name}`)
    ) ?? []),
  ];
}

interface WorkerTableProps {
  workers: WorkerListResponse;
  loading: boolean;
}

function statusColor(worker: WorkerSummary) {
  if (worker.status === 'online') return 'success';
  if (worker.status === 'degraded') return 'warning';
  if (worker.status === 'offline') return 'error';
  return 'default';
}

function WorkerNode({ worker }: { worker: WorkerSummary }) {
  const tags = visibleCapabilityTags(worker);
  const sdkProcessors = visibleSdkProcessors(worker);
  const scriptRunners = worker.structuredCapabilities?.scriptRunners ?? [];
  const pluginProcessors = worker.structuredCapabilities?.pluginProcessors ?? [];
  const isMaster = worker.master?.isMaster === true;

  return (
    <List.Item className={`worker-node ${isMaster ? 'worker-node--master' : 'worker-node--follower'}`}>
      <div className="worker-node__main">
        <div className="worker-node__identity">
          <Space size={8} wrap>
            {isMaster ? <Tag icon={<CrownOutlined />} color="gold">主节点</Tag> : <Tag>从节点</Tag>}
            <Badge status={statusColor(worker)} text={<Typography.Text strong copyable>{worker.workerId}</Typography.Text>} />
          </Space>
          <Typography.Text type="secondary">
            逻辑实例：{worker.logicalInstanceId || '-'} · 客户端实例：{worker.clientInstanceId || '-'} · 序列：{worker.lastSequence}
          </Typography.Text>
        </div>
        <div className="worker-node__election">
          <Tag color="geekblue">代际 {worker.generation}</Tag>
          {worker.master?.domain ? <Tag color="blue">选举域 {worker.master.domain}</Tag> : <Tag>未启用选举</Tag>}
          {worker.master?.term ? <Tag color="purple">任期 {worker.master.term}</Tag> : null}
          {!isMaster && worker.master?.masterWorkerId ? <Tooltip title={worker.master.masterWorkerId}><Tag color="gold">主节点已知</Tag></Tooltip> : null}
        </div>
        <div className="worker-node__capabilities">
          {tags.map((item) => <Tag key={item}>{item}</Tag>)}
          {sdkProcessors.map((item) => <Tag key={`sdk:${item}`} color="purple">处理器：{item}</Tag>)}
          {scriptRunners.map((runner) => <Tag key={`script:${runner.language}:${runner.sandboxBackend}`} color="green">脚本：{runner.language} · {runner.sandboxBackend}</Tag>)}
          {pluginProcessors.flatMap((plugin) => plugin.processorNames.map((name) => <Tag key={`plugin:${plugin.type}:${name}`} color="blue">插件：{plugin.type} · {name}</Tag>))}
          {tags.length + sdkProcessors.length + scriptRunners.length + pluginProcessors.length === 0 ? <Typography.Text type="secondary">无结构化能力</Typography.Text> : null}
        </div>
      </div>
    </List.Item>
  );
}

export function WorkerTable({ workers, loading }: WorkerTableProps) {
  const [query, setQuery] = useState('');
  const [namespace, setNamespace] = useState('');
  const [capability, setCapability] = useState('');

  const namespaces = useMemo(() => uniqueSorted(workers.items.map((worker) => worker.namespace)), [workers.items]);
  const capabilities = useMemo(
    () => uniqueSorted(workers.items.flatMap(capabilityFilterValues)),
    [workers.items],
  );
  const filteredWorkers = useMemo(
    () => filterWorkers(workers.items, { query, namespace, capability }),
    [workers.items, query, namespace, capability],
  );
  const scopeGroups = useMemo(() => groupWorkersByNamespaceApp(filteredWorkers), [filteredWorkers]);

  return (
    <Card
      className="worker-ops-card worker-topology-card"
      title={<Space direction="vertical" size={0}><span>应用集群节点</span><Typography.Text type="secondary">先按命名空间 / 应用分组，再展开查看集群、主节点和从节点</Typography.Text></Space>}
      extra={<Tag color="blue">{filteredWorkers.length}/{workers.items.length}</Tag>}
      loading={loading}
    >
      <div className="worker-toolbar">
        <Input prefix={<SearchOutlined />} allowClear placeholder="搜索 Worker / 应用 / 区域 / 能力 / 处理器" value={query} onChange={(event) => setQuery(event.target.value)} />
        <Select allowClear placeholder="命名空间" value={namespace || undefined} onChange={(value) => setNamespace(value ?? '')} options={namespaces.map((value) => ({ label: value, value }))} />
        <Select allowClear placeholder="能力" value={capability || undefined} onChange={(value) => setCapability(value ?? '')} options={capabilities.map((value) => ({ label: value, value }))} />
      </div>
      {scopeGroups.length === 0 ? <Empty description="没有匹配的在线 Worker" /> : null}
      <Collapse
        key={scopeGroups.map((group) => group.scopeKey).join('|')}
        className="worker-scope-collapse"
        defaultActiveKey={scopeGroups.map((group) => group.scopeKey)}
        items={scopeGroups.map((group) => ({
          key: group.scopeKey,
          label: (
            <Space wrap className="worker-scope-title">
              <ApartmentOutlined />
              <Typography.Text strong>{group.namespace}/{group.app}</Typography.Text>
              <Tag color="geekblue">{group.clusters.length} 个集群</Tag>
              <Tag color="blue">{group.workers.length} 个节点</Tag>
            </Space>
          ),
          children: (
            <div className="worker-cluster-tree">
              {group.clusters.map((cluster) => (
                <section className="worker-cluster-node" key={`${group.scopeKey}:${cluster.cluster}:${cluster.region}`}>
                  <div className="worker-cluster-node__header">
                    <Space wrap>
                      <NodeIndexOutlined />
                      <Typography.Text strong>{cluster.cluster}</Typography.Text>
                      <Tag>{cluster.region}</Tag>
                      <Tag color={cluster.master ? 'gold' : 'orange'}>{cluster.master ? `主节点 ${cluster.master.workerId}` : '未发现主节点'}</Tag>
                      <Tag color="blue">从节点 {cluster.followers.length}</Tag>
                    </Space>
                  </div>
                  <List
                    className="worker-node-list"
                    dataSource={cluster.workers}
                    renderItem={(worker) => <WorkerNode worker={worker} />}
                  />
                </section>
              ))}
            </div>
          ),
        }))}
      />
    </Card>
  );
}
