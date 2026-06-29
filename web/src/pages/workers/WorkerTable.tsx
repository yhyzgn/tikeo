import { ApartmentOutlined, CrownOutlined, NodeIndexOutlined, SearchOutlined } from '@ant-design/icons';
import { Badge, Card, Collapse, Empty, Input, List, Select, Space, Tag, Tooltip, Typography } from 'antd';
import { useMemo, useState } from 'react';

import type { WorkerListResponse, WorkerSummary } from '../../api/client';
import { filterWorkers, groupWorkersByNamespaceApp, uniqueSorted } from './workerPageModel';
import { useI18n } from '../../i18n/I18nContext';

export function visibleCapabilityTags(worker: WorkerSummary) {
  return uniqueSorted(worker.structuredCapabilities?.tags ?? []);
}

export function visibleNormalProcessors(worker: WorkerSummary) {
  return (worker.structuredCapabilities?.normalProcessors?.map((processor) => processor.name) ?? []);
}

export function capabilityFilterValues(worker: WorkerSummary) {
  return [
    ...visibleCapabilityTags(worker),
    ...visibleNormalProcessors(worker).map((name) => `Normal:${name}`),
    ...(worker.structuredCapabilities?.scriptRunners.map((runner) => `Script:${runner.language}`) ?? []),
    ...(worker.structuredCapabilities?.pluginProcessors.flatMap((plugin) =>
      (plugin.processors?.map((processor) => `Plugin:${plugin.type}:${processor.name}`) ?? plugin.processorNames.map((name) => `Plugin:${plugin.type}:${name}`))
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

function WorkerNode({ worker, isEnglish }: { worker: WorkerSummary; isEnglish: boolean }) {
  const tags = visibleCapabilityTags(worker);
  const normalProcessors = visibleNormalProcessors(worker);
  const scriptRunners = worker.structuredCapabilities?.scriptRunners ?? [];
  const pluginProcessors = worker.structuredCapabilities?.pluginProcessors ?? [];
  const isMaster = worker.master?.isMaster === true;
  const workerPool = worker.workerPool?.trim();

  return (
    <List.Item className={`worker-node ${isMaster ? 'worker-node--master' : 'worker-node--follower'}`}>
      <div className="worker-node__main">
        <div className="worker-node__identity">
          <Space size={8} wrap>
            {isMaster ? <Tag className="worker-role-tag worker-role-tag--master" icon={<CrownOutlined />}>{isEnglish ? 'Master' : '主节点'}</Tag> : <Tag color="blue">{isEnglish ? 'Follower' : '从节点'}</Tag>}
            <Badge status={statusColor(worker)} text={<Typography.Text strong copyable data-runtime-text>{worker.workerId}</Typography.Text>} />
          </Space>
          <Typography.Text type="secondary">
            {isEnglish ? 'Logical instance' : '逻辑实例'}: <span data-runtime-text>{worker.logicalInstanceId || '-'}</span> · {isEnglish ? 'Client instance' : '客户端实例'}: <span data-runtime-text>{worker.clientInstanceId || '-'}</span> · {isEnglish ? 'Sequence' : '序列'}: <span data-runtime-text>{worker.lastSequence}</span>
          </Typography.Text>
        </div>
        <div className="worker-node__election">
          <Tag color="geekblue">{isEnglish ? 'Generation' : '代际'} <span data-runtime-text>{worker.generation}</span></Tag>
          {worker.master?.domain ? <Tag color="blue">{isEnglish ? 'Election domain' : '选举域'} <span data-runtime-text>{worker.master.domain}</span></Tag> : <Tag>{isEnglish ? 'Election disabled' : '未启用选举'}</Tag>}
          {worker.master?.term ? <Tag color="purple">{isEnglish ? 'Term' : '任期'} <span data-runtime-text>{worker.master.term}</span></Tag> : null}
          {workerPool ? <Tag color="geekblue">{isEnglish ? 'Execution pool' : '执行池'} <span data-runtime-text>{workerPool}</span></Tag> : <Tag>{isEnglish ? 'Any pool' : '不限执行池'}</Tag>}
          {!isMaster && worker.master?.masterWorkerId ? <Tooltip title={worker.master.masterWorkerId}><Tag className="worker-role-tag worker-role-tag--master">{isEnglish ? 'Known master' : '主节点已知'}</Tag></Tooltip> : null}
        </div>
        <div className="worker-node__capabilities">
          {tags.map((item) => <Tag key={item} data-runtime-text>{item}</Tag>)}
          {normalProcessors.map((item) => <Tag key={`normal:${item}`} color="purple">{isEnglish ? 'Processor' : '处理器'}: <span data-runtime-text>{item}</span></Tag>)}
          {scriptRunners.map((runner) => <Tag key={`script:${runner.language}:${runner.sandboxBackend}`} color="green">{isEnglish ? 'Script' : '脚本'}: <span data-runtime-text>{runner.language} · {runner.sandboxBackend}</span></Tag>)}
          {pluginProcessors.flatMap((plugin) => plugin.processorNames.map((name) => <Tag key={`plugin:${plugin.type}:${name}`} color="blue">{isEnglish ? 'Plugin' : '插件'}: <span data-runtime-text>{plugin.type} · {name}</span></Tag>))}
          {tags.length + normalProcessors.length + scriptRunners.length + pluginProcessors.length === 0 ? <Typography.Text type="secondary">{isEnglish ? 'No structured capabilities' : '无结构化能力'}</Typography.Text> : null}
        </div>
      </div>
    </List.Item>
  );
}

export function WorkerTable({ workers, loading }: WorkerTableProps) {
  const { locale } = useI18n();
  const isEnglish = locale === 'en-US';
  const [query, setQuery] = useState('');
  const [namespace, setNamespace] = useState('');
  const [workerPool, setWorkerPool] = useState('');
  const [capability, setCapability] = useState('');

  const namespaces = useMemo(() => uniqueSorted(workers.items.map((worker) => worker.namespace)), [workers.items]);
  const workerPools = useMemo(() => uniqueSorted(workers.items.map((worker) => worker.workerPool ?? '')), [workers.items]);
  const capabilities = useMemo(
    () => uniqueSorted(workers.items.flatMap(capabilityFilterValues)),
    [workers.items],
  );
  const filteredWorkers = useMemo(
    () => filterWorkers(workers.items, { query, namespace, workerPool, capability }),
    [workers.items, query, namespace, workerPool, capability],
  );
  const scopeGroups = useMemo(() => groupWorkersByNamespaceApp(filteredWorkers), [filteredWorkers]);

  return (
    <Card
      className="worker-ops-card worker-topology-card"
      title={<Space orientation="vertical" size={0}><span>{isEnglish ? 'App cluster nodes' : '应用集群节点'}</span><Typography.Text type="secondary">{isEnglish ? 'Grouped by namespace/app, with optional execution-pool filtering.' : '先按命名空间 / 应用分组，可按执行池定位对应 Worker'}</Typography.Text></Space>}
      extra={<Tag color="blue">{filteredWorkers.length}/{workers.items.length}</Tag>}
      loading={loading}
    >
      <div className="worker-toolbar">
        <Input prefix={<SearchOutlined />} allowClear placeholder={isEnglish ? 'Search workers / apps / regions / capabilities / processors' : '搜索 Worker / 应用 / 区域 / 能力 / 处理器'} value={query} onChange={(event) => setQuery(event.target.value)} />
        <Select allowClear placeholder={isEnglish ? 'Namespace' : '命名空间'} value={namespace || undefined} onChange={(value) => setNamespace(value ?? '')} options={namespaces.map((value) => ({ label: value, value }))} />
        <Select allowClear placeholder={isEnglish ? 'Execution pool' : '执行池'} value={workerPool || undefined} onChange={(value) => setWorkerPool(value ?? '')} options={workerPools.map((value) => ({ label: value, value }))} />
        <Select allowClear placeholder={isEnglish ? 'Capability' : '能力'} value={capability || undefined} onChange={(value) => setCapability(value ?? '')} options={capabilities.map((value) => ({ label: value, value }))} />
      </div>
      {scopeGroups.length === 0 ? <Empty description={isEnglish ? 'No matching online Workers' : '没有匹配的在线 Worker'} /> : null}
      <Collapse
        key={scopeGroups.map((group) => group.scopeKey).join('|')}
        className="worker-scope-collapse"
        defaultActiveKey={scopeGroups.map((group) => group.scopeKey)}
        items={scopeGroups.map((group) => ({
          key: group.scopeKey,
          label: (
            <Space wrap className="worker-scope-title">
              <ApartmentOutlined />
              <Typography.Text strong data-runtime-text>{group.namespace}/{group.app}</Typography.Text>
              <Tag color="geekblue">{isEnglish ? `${group.clusters.length} clusters` : `${group.clusters.length} 个集群`}</Tag>
              <Tag color="blue">{isEnglish ? `${group.workers.length} nodes` : `${group.workers.length} 个节点`}</Tag>
            </Space>
          ),
          children: (
            <div className="worker-cluster-tree">
              {group.clusters.map((cluster) => (
                <section className="worker-cluster-node" key={`${group.scopeKey}:${cluster.cluster}:${cluster.region}`}>
                  <div className="worker-cluster-node__header">
                    <Space wrap>
                      <NodeIndexOutlined />
                      <Typography.Text strong data-runtime-text>{cluster.cluster}</Typography.Text>
                      <Tag data-runtime-text>{cluster.region}</Tag>
                      <Tag className={cluster.master ? 'worker-role-tag worker-role-tag--master' : undefined} color={cluster.master ? undefined : 'red'}>{cluster.master ? <>{isEnglish ? 'Master' : '主节点'} <span data-runtime-text>{cluster.master.workerId}</span></> : (isEnglish ? 'No master found' : '未发现主节点')}</Tag>
                      <Tag color="blue">{isEnglish ? `Followers ${cluster.followers.length}` : `从节点 ${cluster.followers.length}`}</Tag>
                    </Space>
                  </div>
                  <List
                    className="worker-node-list"
                    dataSource={cluster.workers}
                    renderItem={(worker) => <WorkerNode worker={worker} isEnglish={isEnglish} />}
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
