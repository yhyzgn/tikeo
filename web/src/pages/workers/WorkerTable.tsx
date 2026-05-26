import { SearchOutlined } from '@ant-design/icons';
import { Card, Input, Select, Space, Table, Tag, Typography } from 'antd';
import { useMemo, useState } from 'react';

import type { WorkerListResponse, WorkerSummary } from '../../api/client';
import { filterWorkers, uniqueSorted } from './workerPageModel';

interface WorkerTableProps {
  workers: WorkerListResponse;
  loading: boolean;
}

export function WorkerTable({ workers, loading }: WorkerTableProps) {
  const [query, setQuery] = useState('');
  const [namespace, setNamespace] = useState('');
  const [capability, setCapability] = useState('');

  const namespaces = useMemo(() => uniqueSorted(workers.items.map((worker) => worker.namespace)), [workers.items]);
  const capabilities = useMemo(() => uniqueSorted(workers.items.flatMap((worker) => worker.capabilities)), [workers.items]);
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
        <Input prefix={<SearchOutlined />} allowClear placeholder="搜索 worker / app / region / capability" value={query} onChange={(event) => setQuery(event.target.value)} />
        <Select allowClear placeholder="Namespace" value={namespace || undefined} onChange={(value) => setNamespace(value ?? '')} options={namespaces.map((value) => ({ label: value, value }))} />
        <Select allowClear placeholder="Capability" value={capability || undefined} onChange={(value) => setCapability(value ?? '')} options={capabilities.map((value) => ({ label: value, value }))} />
      </div>
      <Table<WorkerSummary>
        rowKey="workerId"
        size="middle"
        loading={loading}
        dataSource={filteredWorkers}
        pagination={{ pageSize: 8 }}
        columns={[
          {
            title: 'Worker',
            dataIndex: 'workerId',
            ellipsis: true,
            render: (value: string, worker) => (
              <Space direction="vertical" size={2}>
                <Typography.Text strong copyable>{value}</Typography.Text>
                <Typography.Text type="secondary">seq={worker.lastSequence}</Typography.Text>
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
            render: (_, worker) => <Typography.Text>{worker.cluster} · {worker.region}</Typography.Text>,
          },
          {
            title: 'Capabilities',
            dataIndex: 'capabilities',
            render: (items: string[]) => <Space size={[4, 4]} wrap>{items.map((item) => <Tag key={item}>{item}</Tag>)}</Space>,
          },
        ]}
        locale={{ emptyText: '没有匹配的在线 Worker' }}
      />
    </Card>
  );
}
