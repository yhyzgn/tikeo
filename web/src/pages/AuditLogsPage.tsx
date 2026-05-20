import { FilterOutlined, SafetyCertificateOutlined } from '@ant-design/icons';
import { Card, Select, Space, Table, Tag, Typography } from 'antd';
import { useCallback, useEffect, useState } from 'react';

import type { AuditLogSummary } from '../api/client';
import { listAuditLogs } from '../api/client';

const ACTION_COLORS: Record<string, string> = {
  create: 'green',
  update: 'blue',
  delete: 'red',
  login: 'cyan',
  logout: 'default',
  trigger: 'purple',
};

export function AuditLogsPage() {
  const [logs, setLogs] = useState<AuditLogSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionFilter, setActionFilter] = useState<string | undefined>(undefined);

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    try {
      const page = await listAuditLogs();
      setLogs(page.items ?? []);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchLogs();
  }, [fetchLogs]);

  const filtered = actionFilter ? logs.filter((l) => l.action === actionFilter) : logs;

  const columns = [
    { title: 'Time', dataIndex: 'created_at', key: 'time', width: 200, render: (v: string) => new Date(v).toLocaleString() },
    { title: 'Actor', dataIndex: 'actor', key: 'actor', width: 140 },
    {
      title: 'Action',
      dataIndex: 'action',
      key: 'action',
      width: 100,
      render: (v: string) => <Tag color={ACTION_COLORS[v] ?? 'default'}>{v}</Tag>,
    },
    { title: 'Resource', key: 'resource', width: 200, render: (_: unknown, r: AuditLogSummary) => <span>{r.resource_type}/{r.resource_id}</span> },
    { title: 'Detail', dataIndex: 'detail', key: 'detail', ellipsis: true },
    { title: 'IP', dataIndex: 'ip_address', key: 'ip', width: 140, render: (v: string | null) => v ?? '-' },
  ];

  const uniqueActions = [...new Set(logs.map((l) => l.action))];

  return (
    <div className="page-stack">
      <section className="hero-panel">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color="volcano" className="soft-tag"><SafetyCertificateOutlined /> Audit</Tag>
            <Typography.Title level={1}>审计日志</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            平台写操作审计追踪记录，支持按操作类型筛选。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary">
          <strong>{logs.length}</strong>
          <span>entries</span>
        </div>
      </section>

      <Card>
        <Space style={{ marginBottom: 16 }}>
          <FilterOutlined />
          <Select
            allowClear
            placeholder="Filter action"
            style={{ width: 160 }}
            value={actionFilter}
            onChange={setActionFilter}
            options={uniqueActions.map((a) => ({ label: a, value: a }))}
          />
        </Space>
        <Table
          rowKey="id"
          dataSource={filtered}
          columns={columns}
          loading={loading}
          pagination={{ pageSize: 20, showSizeChanger: false }}
          size="small"
        />
      </Card>
    </div>
  );
}
