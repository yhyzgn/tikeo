import { DownloadOutlined, FilterOutlined, ReloadOutlined, SafetyCertificateOutlined } from '@ant-design/icons';
import { Button, Card, Form, Input, Select, Space, Table, Tag, Tooltip, Typography, message } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { AuditLogQuery, AuditLogSummary } from '../api/client';
import { exportAuditLogs, listAuditLogs } from '../api/client';

const ACTION_COLORS: Record<string, string> = {
  create: 'green',
  update: 'blue',
  delete: 'red',
  login: 'cyan',
  logout: 'default',
  trigger: 'purple',
  claim: 'geekblue',
  run: 'purple',
  recover: 'orange',
};

const PAGE_SIZE = 20;

export function AuditLogsPage() {
  const [form] = Form.useForm<AuditLogQuery>();
  const [logs, setLogs] = useState<AuditLogSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [query, setQuery] = useState<AuditLogQuery>({ page_size: PAGE_SIZE });

  const fetchLogs = useCallback(async (nextQuery: AuditLogQuery = query) => {
    setLoading(true);
    try {
      const page = await listAuditLogs({ ...nextQuery, page_size: nextQuery.page_size ?? PAGE_SIZE });
      setLogs(page.items ?? []);
      setTotal(page.total ?? page.items?.length ?? 0);
    } finally {
      setLoading(false);
    }
  }, [query]);

  useEffect(() => {
    void fetchLogs(query);
  }, [fetchLogs, query]);

  const actionOptions = useMemo(
    () => [...new Set(logs.map((log) => log.action))].map((action) => ({ label: action, value: action })),
    [logs],
  );

  const columns = [
    { title: 'Time', dataIndex: 'created_at', key: 'time', width: 200, render: (v: string) => new Date(v).toLocaleString() },
    { title: 'Actor', dataIndex: 'actor', key: 'actor', width: 140 },
    {
      title: 'Action',
      dataIndex: 'action',
      key: 'action',
      width: 120,
      render: (v: string) => <Tag color={ACTION_COLORS[v] ?? 'default'}>{v}</Tag>,
    },
    { title: 'Resource', key: 'resource', width: 240, render: (_: unknown, r: AuditLogSummary) => <span>{r.resource_type}/{r.resource_id}</span> },
    {
      title: 'Result',
      dataIndex: 'result',
      key: 'result',
      width: 100,
      render: (v: string, r: AuditLogSummary) => (
        <Tooltip title={r.failure_reason ?? undefined}>
          <Tag color={v === 'failed' ? 'red' : 'green'}>{v}</Tag>
        </Tooltip>
      ),
    },
    { title: 'Trace', dataIndex: 'trace_id', key: 'trace', width: 160, ellipsis: true, render: (v: string | null) => v ?? '-' },
    { title: 'Detail', dataIndex: 'detail', key: 'detail', ellipsis: true },
    {
      title: 'Snapshot',
      key: 'snapshot',
      width: 120,
      render: (_: unknown, r: AuditLogSummary) => (r.before || r.after ? <Tag color="geekblue">before/after</Tag> : '-'),
    },
    { title: 'IP', dataIndex: 'ip_address', key: 'ip', width: 150, render: (v: string | null) => v ?? '-' },
  ];

  const applyFilters = (values: AuditLogQuery) => {
    setQuery({ ...values, page_size: PAGE_SIZE, page_token: undefined });
  };

  const resetFilters = () => {
    form.resetFields();
    setQuery({ page_size: PAGE_SIZE });
  };

  const exportCurrent = async () => {
    const exported = await exportAuditLogs({ ...query, page_size: 500, page_token: undefined });
    const blob = new Blob([JSON.stringify(exported, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `scheduler-audit-${new Date().toISOString()}.json`;
    link.click();
    URL.revokeObjectURL(url);
    void message.success(`已导出 ${exported.exported} 条审计记录`);
  };

  return (
    <div className="page-stack">
      <section className="hero-panel">
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color="volcano" className="soft-tag"><SafetyCertificateOutlined /> Audit</Tag>
            <Typography.Title level={1}>审计日志</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            平台写操作审计追踪记录，支持服务端分页与 actor/action/resource 过滤。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary">
          <strong>{total}</strong>
          <span>matched</span>
        </div>
      </section>

      <Card>
        <Form form={form} layout="inline" onFinish={applyFilters} style={{ marginBottom: 16, rowGap: 12 }}>
          <Form.Item><FilterOutlined /></Form.Item>
          <Form.Item name="actor">
            <Input allowClear placeholder="Actor" style={{ width: 150 }} />
          </Form.Item>
          <Form.Item name="action">
            <Select
              allowClear
              showSearch
              placeholder="Action"
              style={{ width: 150 }}
              options={actionOptions}
            />
          </Form.Item>
          <Form.Item name="resource_type">
            <Input allowClear placeholder="Resource type" style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="resource_id">
            <Input allowClear placeholder="Resource id" style={{ width: 180 }} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">查询</Button>
              <Button onClick={resetFilters}>重置</Button>
              <Button icon={<ReloadOutlined />} onClick={() => void fetchLogs(query)} />
              <Button icon={<DownloadOutlined />} onClick={() => void exportCurrent()}>导出 JSON</Button>
            </Space>
          </Form.Item>
        </Form>
        <Table
          rowKey="id"
          dataSource={logs}
          columns={columns}
          loading={loading}
          pagination={{
            pageSize: PAGE_SIZE,
            total,
            showSizeChanger: false,
            onChange: (page) => setQuery((current) => ({ ...current, page_token: String((page - 1) * PAGE_SIZE) })),
          }}
          size="small"
        />
      </Card>
    </div>
  );
}
