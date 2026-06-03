import { SearchOutlined } from '@ant-design/icons';
import { Card, Input, Select, Space, Table, Tag, Typography } from 'antd';
import { useMemo, useState } from 'react';

import type { WorkerListResponse, WorkerSummary } from '../../api/client';
import { persistentPagination, usePersistentTablePageSize } from '../../utils/pagination';
import { filterWorkers, uniqueSorted } from './workerPageModel';

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

export function WorkerTable({ workers, loading }: WorkerTableProps) {
  const [query, setQuery] = useState('');
  const [namespace, setNamespace] = useState('');
  const [capability, setCapability] = useState('');
  const [pageSize, setPageSize] = usePersistentTablePageSize();

  const namespaces = useMemo(() => uniqueSorted(workers.items.map((worker) => worker.namespace)), [workers.items]);
  const capabilities = useMemo(
    () => uniqueSorted(workers.items.flatMap(capabilityFilterValues)),
    [workers.items],
  );
  const filteredWorkers = useMemo(
    () => filterWorkers(workers.items, { query, namespace, capability }),
    [workers.items, query, namespace, capability],
  );

  return (
    <Card
      className="worker-ops-card"
      title={<Space direction="vertical" size={0}><span>在线 Worker</span><Typography.Text type="secondary">按 namespace / capability 快速定位执行容量</Typography.Text></Space>}
      extra={<Tag color="blue">{filteredWorkers.length}/{workers.items.length}</Tag>}
    >
      <div className="worker-toolbar">
        <Input prefix={<SearchOutlined />} allowClear placeholder="搜索 worker / app / region / capability / processor" value={query} onChange={(event) => setQuery(event.target.value)} />
        <Select allowClear placeholder="Namespace" value={namespace || undefined} onChange={(value) => setNamespace(value ?? '')} options={namespaces.map((value) => ({ label: value, value }))} />
        <Select allowClear placeholder="Capability" value={capability || undefined} onChange={(value) => setCapability(value ?? '')} options={capabilities.map((value) => ({ label: value, value }))} />
      </div>
      <Table<WorkerSummary>
        rowKey="workerId"
        size="middle"
        loading={loading}
        dataSource={filteredWorkers}
        pagination={persistentPagination(pageSize, setPageSize)}
        columns={[
          {
            title: 'Worker',
            dataIndex: 'workerId',
            ellipsis: true,
            render: (value: string, worker) => (
              <Space direction="vertical" size={2}>
                <Typography.Text strong copyable>{value}</Typography.Text>
                <Typography.Text type="secondary">seq={worker.lastSequence}</Typography.Text>
                {worker.master?.isMaster ? <Tag color="gold">Master</Tag> : <Tag>Follower</Tag>}
              </Space>
            ),
          },
          {
            title: 'Scope',
            width: 210,
            render: (_, worker) => <Tag color="geekblue">{worker.namespace}/{worker.app}</Tag>,
          },
          {
            title: 'Placement',
            width: 190,
            render: (_, worker) => (
              <Space direction="vertical" size={2}>
                <Typography.Text>{worker.cluster} · {worker.region}</Typography.Text>
                <Typography.Text type="secondary">{worker.master?.domain ?? '未启用选举'}</Typography.Text>
              </Space>
            ),
          },
          {
            title: 'Capabilities',
            dataIndex: 'capabilities',
            render: (_, worker) => {
              const visible = visibleCapabilityTags(worker);
              return visible.length > 0
                ? <Space size={[4, 4]} wrap>{visible.map((item) => <Tag key={item}>{item}</Tag>)}</Space>
                : <Typography.Text type="secondary">-</Typography.Text>;
            },
          },
          {
            title: 'SDK Processors',
            dataIndex: 'capabilities',
            render: (_, worker) => {
              const processors = visibleSdkProcessors(worker);
              return processors.length > 0
                ? <Space size={[4, 4]} wrap>{processors.map((item) => <Tag key={item} color="purple">{item}</Tag>)}</Space>
                : <Typography.Text type="secondary">-</Typography.Text>;
            },
          },
          {
            title: 'Plugin Processors',
            render: (_, worker) => {
              const plugins = worker.structuredCapabilities?.pluginProcessors ?? [];
              return plugins.length > 0
                ? <Space size={[4, 4]} wrap>{plugins.flatMap((plugin) => plugin.processorNames.map((name) => <Tag key={`${plugin.type}:${name}`} color="blue">{plugin.type} · {name}</Tag>))}</Space>
                : <Typography.Text type="secondary">-</Typography.Text>;
            },
          },
          {
            title: 'Script Runners',
            render: (_, worker) => {
              const runners = worker.structuredCapabilities?.scriptRunners ?? [];
              return runners.length > 0
                ? <Space size={[4, 4]} wrap>{runners.map((runner) => <Tag key={`${runner.language}:${runner.sandboxBackend}`} color="green">{runner.language} · {runner.sandboxBackend}</Tag>)}</Space>
                : <Typography.Text type="secondary">-</Typography.Text>;
            },
          },
        ]}
        locale={{ emptyText: '没有匹配的在线 Worker' }}
      />
    </Card>
  );
}
