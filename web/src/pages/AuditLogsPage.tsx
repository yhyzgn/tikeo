import { DownloadOutlined, FilterOutlined, ReloadOutlined, SafetyCertificateOutlined } from '@ant-design/icons';
import { Button, Card, Form, Input, Select, Space, Table, Tag, Tooltip, Typography, message } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { AuditLogQuery, AuditLogSummary } from '../api/client';
import { exportAuditLogs, listAuditLogs } from '../api/client';
import { useRouteActive } from '../hooks/useRouteActivation';
import { useUrlQueryState } from '../hooks/useUrlQueryState';
import { ROUTE_META } from '../routes';
import { DEFAULT_TABLE_PAGE_SIZE, TABLE_PAGE_SIZE_OPTIONS, persistTablePageSize, usePersistentTablePageSize } from '../utils/pagination';

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

const AUDIT_QUERY_DEFAULTS = {
  page_size: DEFAULT_TABLE_PAGE_SIZE,
  page_token: '',
  actor: '',
  action: '',
  resource_type: '',
  resource_id: '',
  failure_reason: '',
};

export function AuditLogsPage() {
  const [form] = Form.useForm<AuditLogQuery>();
  const [logs, setLogs] = useState<AuditLogSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [pageSize, setPageSize] = usePersistentTablePageSize();
  const { query: urlQuery, setQuery: setUrlQuery, resetQuery } = useUrlQueryState(AUDIT_QUERY_DEFAULTS);
  const active = useRouteActive(ROUTE_META.audit.path);
  const query = useMemo<AuditLogQuery>(() => ({
    page_size: Number(urlQuery.page_size) || pageSize,
    page_token: urlQuery.page_token || undefined,
    actor: urlQuery.actor || undefined,
    action: urlQuery.action || undefined,
    resource_type: urlQuery.resource_type || undefined,
    resource_id: urlQuery.resource_id || undefined,
    failure_reason: urlQuery.failure_reason || undefined,
  }), [pageSize, urlQuery]);

  const effectivePageSize = query.page_size ?? pageSize;

  const fetchLogs = useCallback(async (nextQuery: AuditLogQuery = query) => {
    setLoading(true);
    try {
      const page = await listAuditLogs({ ...nextQuery, page_size: nextQuery.page_size ?? pageSize });
      setLogs(page.items ?? []);
      setTotal(page.total ?? page.items?.length ?? 0);
    } finally {
      setLoading(false);
    }
  }, [query]);

  useEffect(() => {
    if (active) void fetchLogs(query);
  }, [active, fetchLogs, query]);

  const actionOptions = useMemo(
    () => [...new Set(logs.map((log) => log.action))].map((action) => ({ label: action, value: action })),
    [logs],
  );

  const renderCompactText = (value: string | null | undefined, className?: string) => {
    if (!value) return <Typography.Text type="secondary">-</Typography.Text>;
    return (
      <Tooltip title={value}>
        <Typography.Text className={className} ellipsis>
          {value}
        </Typography.Text>
      </Tooltip>
    );
  };

  const columns = [
    {
      title: 'Time',
      dataIndex: 'createdAt',
      key: 'time',
      width: 170,
      render: (v: string) => {
        const date = new Date(v);
        return (
          <Space direction="vertical" size={0}>
            <Typography.Text strong>{date.toLocaleDateString()}</Typography.Text>
            <Typography.Text type="secondary" className="audit-log-subtext">{date.toLocaleTimeString()}</Typography.Text>
          </Space>
        );
      },
    },
    {
      title: 'Actor',
      dataIndex: 'actor',
      key: 'actor',
      width: 150,
      render: (v: string) => renderCompactText(v),
    },
    {
      title: 'Action',
      dataIndex: 'action',
      key: 'action',
      width: 110,
      render: (v: string) => <Tag color={ACTION_COLORS[v] ?? 'default'}>{v}</Tag>,
    },
    {
      title: 'Resource',
      key: 'resource',
      width: 260,
      render: (_: unknown, r: AuditLogSummary) => (
        <Space direction="vertical" size={2} className="audit-log-resource">
          <Tag color="blue">{r.resource_type}</Tag>
          {renderCompactText(r.resource_id, 'audit-log-mono')}
        </Space>
      ),
    },
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
    {
      title: 'Trace',
      dataIndex: 'trace_id',
      key: 'trace',
      width: 150,
      render: (v: string | null) => renderCompactText(v, 'audit-log-mono'),
    },
    {
      title: 'Detail',
      dataIndex: 'detail',
      key: 'detail',
      width: 280,
      render: (v: string | null) => renderCompactText(v, 'audit-log-detail'),
    },
    {
      title: 'Snapshot',
      key: 'snapshot',
      width: 140,
      render: (_: unknown, r: AuditLogSummary) => (r.before || r.after ? (
        <Space size={4} wrap>
          {r.before ? <Tag color="geekblue">before</Tag> : null}
          {r.after ? <Tag color="purple">after</Tag> : null}
        </Space>
      ) : <Typography.Text type="secondary">-</Typography.Text>),
    },
    {
      title: 'IP',
      dataIndex: 'ip_address',
      key: 'ip',
      width: 140,
      render: (v: string | null) => renderCompactText(v, 'audit-log-mono'),
    },
  ];

  useEffect(() => {
    form.setFieldsValue(query);
  }, [form, query]);

  const applyFilters = (values: AuditLogQuery) => {
    setUrlQuery({ ...values, page_size: pageSize, page_token: '' });
  };

  const resetFilters = () => {
    form.resetFields();
    resetQuery();
  };

  const exportCurrent = async () => {
    const exported = await exportAuditLogs({ ...query, page_size: 500, page_token: undefined });
    const blob = new Blob([JSON.stringify(exported, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `tikee-audit-${new Date().toISOString()}.json`;
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
            平台写操作与脚本治理审计追踪记录，支持服务端分页、actor/action/resource 与失败原因过滤。
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
          <Form.Item name="failure_reason">
            <Input allowClear placeholder="Failure reason" style={{ width: 190 }} />
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
          scroll={{ x: 1500 }}
          pagination={{
            pageSize: effectivePageSize,
            total,
            showSizeChanger: true,
            pageSizeOptions: TABLE_PAGE_SIZE_OPTIONS.map(String),
            current: query.page_token ? Math.floor(Number(query.page_token) / effectivePageSize) + 1 : 1,
            onChange: (page, nextPageSize) => {
              setPageSize(nextPageSize);
              persistTablePageSize(nextPageSize);
              setUrlQuery({
                page_size: nextPageSize,
                page_token: page > 1 ? String((page - 1) * nextPageSize) : '',
              });
            },
          }}
          size="small"
        />
      </Card>
    </div>
  );
}
