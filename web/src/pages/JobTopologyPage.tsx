import { Alert, Button, Card, Descriptions, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { getJobImpact, getJobTopology, listJobs, type JobImpactResponse, type JobSummary, type JobTopologyEdge, type JobTopologyResponse, type JobTopologyUnresolvedRef } from '../api/client';
import { ROUTE_META } from '../routes';
import { ImpactJobTags, TopologyCanvas } from './jobs/TopologyCanvas';

export function JobTopologyPage() {
  const navigate = useNavigate();
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [topology, setTopology] = useState<JobTopologyResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [jobImpact, setJobImpact] = useState<JobImpactResponse | null>(null);
  const [impactLoading, setImpactLoading] = useState(false);

  const jobNames = useMemo(() => new Map(jobs.map((job) => [job.id, job.name])), [jobs]);
  const jobLabel = (jobId: string) => jobNames.get(jobId) ?? jobId;

  const loadJobImpact = useCallback(async (jobId: string) => {
    setSelectedJobId(jobId);
    setImpactLoading(true);
    try {
      setJobImpact(await getJobImpact(jobId));
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载影响分析失败');
      setJobImpact(null);
    } finally {
      setImpactLoading(false);
    }
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [jobPage, graph] = await Promise.all([listJobs(), getJobTopology()]);
      setJobs(jobPage.items);
      setTopology(graph);
      const firstJob = graph.nodes.find((node) => node.type === 'job')?.id ?? null;
      if (firstJob) void loadJobImpact(firstJob);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载任务拓扑失败');
      setTopology(null);
    } finally {
      setLoading(false);
    }
  }, [loadJobImpact]);

  useEffect(() => { void load(); }, [load]);

  return (
    <div className="page-stack">
      <Space orientation="vertical" size={18} style={{ width: '100%' }}>
        <div className="hero-panel workflow-hero workflow-editor-hero">
          <div className="hero-panel__content">
            <Button className="workflow-back-button" onClick={() => navigate(ROUTE_META.jobs.path)}>← 返回任务列表</Button>
            <Tag className="soft-tag" color="blue">Phase 4 · Job Topology</Tag>
            <Typography.Title level={1}>任务拓扑</Typography.Title>
            <Typography.Paragraph className="hero-panel__desc">
              二级页面承载完整图形画布、跨工作流影响分析与依赖明细。
            </Typography.Paragraph>
          </div>
          <div className="hero-panel__summary"><strong>{topology?.nodes.length ?? 0}</strong><span>nodes</span></div>
        </div>

        <Card className="clean-card" title="拓扑控制" extra={<Button onClick={() => void load()} loading={loading}>刷新拓扑</Button>} />

        {topology?.unresolved.length ? <Alert type="warning" showIcon message="无法解析的引用" description="部分工作流节点指向不存在或当前账号不可见的 Job，需要修复后拓扑才完整。" /> : null}

        <TopologyCanvas topology={topology} loading={loading} selectedJobId={selectedJobId} onSelectJob={(jobId) => void loadJobImpact(jobId)} />

        <Card size="small" title="跨工作流影响分析" loading={impactLoading}>
          {jobImpact ? (
            <Space orientation="vertical" size={12} style={{ width: '100%' }}>
              <Alert type={jobImpact.riskSummary.riskLevel === 'high' ? 'error' : jobImpact.riskSummary.riskLevel === 'medium' ? 'warning' : 'success'} showIcon message={<span data-runtime-text>{`${jobImpact.targetJob.name} · ${jobImpact.riskSummary.riskLevel}`}</span>} description={<span data-runtime-text>{jobImpact.riskSummary.reasons.join('；')}</span>} />
              <Descriptions size="small" column={2}>
                <Descriptions.Item label="引用工作流">{jobImpact.referencingWorkflows.map((workflow) => <Tag key={workflow.id} data-runtime-text>{workflow.name}</Tag>)}</Descriptions.Item>
                <Descriptions.Item label="未解析引用">{jobImpact.riskSummary.unresolvedCount}</Descriptions.Item>
                <Descriptions.Item label="upstreamJobs"><ImpactJobTags jobs={jobImpact.upstreamJobs} empty="无上游任务" /></Descriptions.Item>
                <Descriptions.Item label="downstreamJobs"><ImpactJobTags jobs={jobImpact.downstreamJobs} empty="无下游任务" /></Descriptions.Item>
              </Descriptions>
            </Space>
          ) : <Typography.Text type="secondary">点击画布中的 Job 节点查看影响范围。</Typography.Text>}
        </Card>

        <TopologyTables topology={topology} loading={loading} jobLabel={jobLabel} />
      </Space>
    </div>
  );
}

function TopologyTables({ topology, loading, jobLabel }: { topology: JobTopologyResponse | null; loading: boolean; jobLabel: (jobId: string) => string }) {
  const dependencyColumns: ColumnsType<JobTopologyEdge> = [
    { title: 'From', dataIndex: 'from', render: (value: string) => <Typography.Text code>{jobLabel(value)}</Typography.Text> },
    { title: 'To', dataIndex: 'to', render: (value: string) => <Typography.Text code>{jobLabel(value)}</Typography.Text> },
    { title: 'Workflow', dataIndex: 'workflowName', render: (value: string | null) => <span data-runtime-text>{value ?? '-'}</span> },
    { title: 'Condition', dataIndex: 'condition', render: (value: string | null) => <Tag>{value ?? 'always'}</Tag> },
  ];
  const refColumns: ColumnsType<JobTopologyEdge> = [
    { title: 'Workflow', dataIndex: 'workflowName', render: (value: string | null) => <span data-runtime-text>{value ?? '-'}</span> },
    { title: 'Job', dataIndex: 'to', render: (value: string) => <Typography.Text code>{jobLabel(value)}</Typography.Text> },
    { title: 'Node', dataIndex: 'label', render: (value: string | null) => <span data-runtime-text>{value ?? '-'}</span> },
  ];
  const unresolvedColumns: ColumnsType<JobTopologyUnresolvedRef> = [
    { title: 'Workflow', dataIndex: 'workflowName', render: (value: string | null) => <span data-runtime-text>{value ?? '-'}</span> },
    { title: 'Node', dataIndex: 'nodeKey', render: (value: string) => <span data-runtime-text>{value}</span> },
    { title: 'Missing Job', dataIndex: 'missingJobId', render: (value: string) => <Typography.Text code>{value}</Typography.Text> },
    { title: 'Reason', dataIndex: 'reason', render: (value: string) => <span data-runtime-text>{value}</span> },
  ];
  return (
    <Card size="small" title="拓扑明细">
      <Typography.Title level={5}>任务依赖</Typography.Title>
      <Table<JobTopologyEdge> rowKey="id" loading={loading} dataSource={(topology?.edges ?? []).filter((edge) => edge.type === 'workflow_job_dependency')} pagination={false} size="small" columns={dependencyColumns} />
      <Typography.Title level={5} style={{ marginTop: 24 }}>工作流引用</Typography.Title>
      <Table<JobTopologyEdge> rowKey="id" loading={loading} dataSource={(topology?.edges ?? []).filter((edge) => edge.type === 'workflow_job_ref')} pagination={{ pageSize: 6 }} size="small" columns={refColumns} />
      <Typography.Title level={5} style={{ marginTop: 24 }}>无法解析的引用</Typography.Title>
      <Table<JobTopologyUnresolvedRef> rowKey={(item) => `${item.workflowId}:${item.nodeKey}:${item.missingJobId}`} loading={loading} dataSource={topology?.unresolved ?? []} pagination={false} size="small" columns={unresolvedColumns} />
    </Card>
  );
}
