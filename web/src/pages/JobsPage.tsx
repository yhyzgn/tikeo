import { Button, Card, Drawer, Form, Input, Popconfirm, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { createJob, deleteJob, listJobs, listScripts, triggerJob, updateJob, type CreateJobRequest, type JobSummary, type ScriptSummary, type UpdateJobRequest } from '../api/client';
import { PermissionGate, useCan } from '../components/Permission';
import { useUrlQueryState } from '../hooks/useUrlQueryState';

export function JobsPage() {
  const canWriteJobs = useCan('jobs', 'write');
  const canExecuteInstances = useCan('instances', 'execute');
  const { query, setQuery } = useUrlQueryState({ page: 1, page_size: 8, keyword: '', scheduleType: '' });
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [scripts, setScripts] = useState<ScriptSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<CreateJobRequest & { executorType?: 'sdk' | 'script'; scriptId?: string }>();
  const [editForm] = Form.useForm<UpdateJobRequest & { executorType?: 'sdk' | 'script'; scriptId?: string }>();
  const [createDrawerOpen, setCreateDrawerOpen] = useState(false);
  const [editingJob, setEditingJob] = useState<JobSummary | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [page, scriptPage] = await Promise.all([listJobs(), listScripts()]);
      setJobs(page.items);
      setScripts(scriptPage.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const scriptOptions = scripts.map((script) => ({ label: `${script.name} · ${script.language} · ${script.status}`, value: script.id }));
  const scriptProcessor = (scriptId?: string) => scriptId ? `script:${scriptId}` : undefined;
  const parseProcessor = (processorName?: string | null) => processorName?.startsWith('script:')
    ? { executorType: 'script' as const, scriptId: processorName.slice('script:'.length), processorName }
    : { executorType: 'sdk' as const, scriptId: undefined, processorName: processorName ?? undefined };


  const openCreateDrawer = () => {
    form.resetFields();
    form.setFieldsValue({ namespace: 'default', app: 'default', scheduleType: 'api', enabled: true, executorType: 'sdk' });
    setCreateDrawerOpen(true);
  };

  const openEditDrawer = (job: JobSummary) => {
    setEditingJob(job);
    const processor = parseProcessor(job.processorName);
    editForm.setFieldsValue({
      name: job.name,
      scheduleType: job.scheduleType,
      scheduleExpr: job.scheduleExpr,
      processorName: processor.executorType === 'sdk' ? processor.processorName : undefined,
      executorType: processor.executorType,
      scriptId: processor.scriptId,
      enabled: job.enabled,
    });
  };

  const handleEditSubmit = async (values: UpdateJobRequest & { executorType?: 'sdk' | 'script'; scriptId?: string }) => {
    if (!editingJob) return;
    if (!canWriteJobs) { message.error('当前账号无权限编辑任务'); return; }
    try {
      const processorName = values.executorType === 'script' ? scriptProcessor(values.scriptId) : values.processorName;
      const updated = await updateJob(editingJob.id, { ...values, processorName });
      setJobs((current) => current.map((item) => item.id === updated.id ? updated : item));
      setEditingJob(null);
      editForm.resetFields();
      message.success(`已更新 ${updated.name}`);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新任务失败');
    }
  };

  const handleDelete = async (job: JobSummary) => {
    if (!canWriteJobs) { message.error('当前账号无权限删除任务'); return; }
    try {
      await deleteJob(job.id);
      message.success(`已删除 ${job.name}`);
      await load();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除任务失败');
    }
  };


  const filteredJobs = useMemo(() => jobs.filter((job) => {
    const keyword = String(query.keyword ?? '').trim().toLowerCase();
    const scheduleType = String(query.scheduleType ?? '').trim();
    const matchesKeyword = keyword === '' || [job.name, job.namespace, job.app, job.processorName ?? '', job.id].some((value) => value.toLowerCase().includes(keyword));
    const matchesSchedule = scheduleType === '' || job.scheduleType === scheduleType;
    return matchesKeyword && matchesSchedule;
  }), [jobs, query.keyword, query.scheduleType]);

  const columns: ColumnsType<JobSummary> = [
    { title: 'Name', dataIndex: 'name' },
    { title: 'Namespace / App', render: (_, job) => <Space direction="vertical" size={0}><strong>{job.namespace}</strong><Typography.Text type="secondary" style={{ fontSize: 12 }}>{job.app}</Typography.Text></Space> },
    { title: 'Schedule', dataIndex: 'scheduleType', render: (value: string) => <Tag color="blue" className="soft-tag">{value}</Tag> },
    { title: 'Processor', dataIndex: 'processorName', render: (value: string | null, job) => <Typography.Text code>{value || job.id}</Typography.Text> },
    {
      title: 'Enabled',
      dataIndex: 'enabled',
      render: (value: boolean, job) => (
        <Switch
          size="small"
          checked={value}
          disabled={!canWriteJobs}
          onChange={async (enabled) => {
            try {
              const updated = await updateJob(job.id, { enabled });
              setJobs((current) => current.map((item) => item.id === job.id ? updated : item));
              message.success(enabled ? `已启用 ${job.name}` : `已禁用 ${job.name}`);
            } catch (err) {
              message.error(err instanceof Error ? err.message : '更新任务状态失败');
            }
          }}
        />
      ),
    },
    {
      title: 'Actions',
      width: 260,
      align: 'right',
      render: (_, job) => (
        <Space size={4} className="table-action-strip">
          {canExecuteInstances ? (
            <Button
              size="small"
              type="link"
              onClick={async () => {
                try {
                  await triggerJob(job.id, { triggerType: 'api', executionMode: 'single' });
                  message.success(`已触发 ${job.name}`);
                  await load();
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '触发失败');
                }
              }}
            >
              单机执行
            </Button>
          ) : null}
          {canExecuteInstances ? (
            <Button
              size="small"
              type="link"
              onClick={async () => {
                try {
                  await triggerJob(job.id, { triggerType: 'api', executionMode: 'broadcast' });
                  message.success(`已广播触发 ${job.name}`);
                  await load();
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '触发失败');
                }
              }}
            >
              广播
            </Button>
          ) : null}
          <PermissionGate resource="jobs" action="write">
            <Button size="small" type="link" onClick={() => openEditDrawer(job)}>编辑</Button>
          </PermissionGate>
          <PermissionGate resource="jobs" action="write">
            <Popconfirm title="删除任务" description="删除后该任务将无法再触发，历史实例保留用于审计。" onConfirm={() => void handleDelete(job)}>
              <Button size="small" type="link" danger>删除</Button>
            </Popconfirm>
          </PermissionGate>
        </Space>
      ),
    },
  ];

  return (
    <div className="page-stack">
      <Drawer
        title="创建任务"
        open={createDrawerOpen}
        onClose={() => { setCreateDrawerOpen(false); form.resetFields(); }}
        width={520}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">配置任务所属 namespace/app、调度类型和 Worker processor 绑定；创建后在列表统一启停和触发。</Typography.Paragraph>
        <Form
          form={form}
          layout="vertical"
          initialValues={{ namespace: 'default', app: 'default', scheduleType: 'api', enabled: true }}
          onFinish={async (values) => {
            if (!canWriteJobs) { message.error('当前账号无权限创建任务'); return; }
            try {
              const processorName = values.executorType === 'script' ? scriptProcessor(values.scriptId) : values.processorName;
              await createJob({ ...values, processorName });
              message.success('任务已创建');
              form.resetFields();
              setCreateDrawerOpen(false);
              await load();
            } catch (err) {
              message.error(err instanceof Error ? err.message : '创建失败');
            }
          }}
        >
          <Form.Item name="namespace" label="Namespace" rules={[{ required: true }]}><Input placeholder="default" /></Form.Item>
          <Form.Item name="app" label="App" rules={[{ required: true }]}><Input placeholder="default" /></Form.Item>
          <Form.Item name="name" label="任务名称" rules={[{ required: true }]}><Input placeholder="demo.echo" /></Form.Item>
          <Form.Item name="executorType" label="执行器类型"><Select options={[{ value: 'sdk', label: 'SDK Processor' }, { value: 'script', label: 'Script' }]} /></Form.Item>
          <Form.Item noStyle shouldUpdate={(prev, next) => prev.executorType !== next.executorType}>
            {({ getFieldValue }) => getFieldValue('executorType') === 'script' ? (
              <Form.Item name="scriptId" label="脚本" rules={[{ required: true, message: '请选择脚本' }]}><Select showSearch options={scriptOptions} optionFilterProp="label" placeholder="选择已创建脚本" /></Form.Item>
            ) : (
              <Form.Item name="processorName" label="SDK Processor"><Input placeholder="demo.echo" /></Form.Item>
            )}
          </Form.Item>
          <Form.Item name="scheduleType" label="调度类型"><Select options={[{ value: 'api' }, { value: 'cron' }, { value: 'fixed_rate' }]} /></Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked"><Switch /></Form.Item>
          <PermissionGate resource="jobs" action="write"><Button type="primary" htmlType="submit" block>创建任务</Button></PermissionGate>
        </Form>
      </Drawer>

      <Drawer
        title={editingJob ? `编辑任务 - ${editingJob.name}` : '编辑任务'}
        open={editingJob !== null}
        onClose={() => { setEditingJob(null); editForm.resetFields(); }}
        width={520}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">编辑任务基础信息、调度配置、Processor 绑定和启用状态；namespace/app 暂不支持变更，避免历史实例归属漂移。</Typography.Paragraph>
        <Form form={editForm} layout="vertical" onFinish={(values) => void handleEditSubmit(values)}>
          <Form.Item label="Namespace / App"><Typography.Text code>{editingJob ? `${editingJob.namespace}/${editingJob.app}` : '-'}</Typography.Text></Form.Item>
          <Form.Item name="name" label="任务名称" rules={[{ required: true }]}><Input /></Form.Item>
          <Form.Item name="executorType" label="执行器类型"><Select options={[{ value: 'sdk', label: 'SDK Processor' }, { value: 'script', label: 'Script' }]} /></Form.Item>
          <Form.Item noStyle shouldUpdate={(prev, next) => prev.executorType !== next.executorType}>
            {({ getFieldValue }) => getFieldValue('executorType') === 'script' ? (
              <Form.Item name="scriptId" label="脚本" rules={[{ required: true, message: '请选择脚本' }]}><Select showSearch options={scriptOptions} optionFilterProp="label" placeholder="选择已创建脚本" /></Form.Item>
            ) : (
              <Form.Item name="processorName" label="SDK Processor"><Input placeholder="demo.echo" /></Form.Item>
            )}
          </Form.Item>
          <Form.Item name="scheduleType" label="调度类型"><Select options={[{ value: 'api' }, { value: 'cron' }, { value: 'fixed_rate' }]} /></Form.Item>
          <Form.Item name="scheduleExpr" label="调度表达式"><Input placeholder="cron 或 fixed_rate 表达式，可留空" /></Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked"><Switch /></Form.Item>
          <PermissionGate resource="jobs" action="write"><Button type="primary" htmlType="submit" block>保存任务</Button></PermissionGate>
        </Form>
      </Drawer>

      <Card
        className="clean-card"
        title="任务列表"
        extra={<Space wrap className="card-toolbar"><PermissionGate resource="jobs" action="write"><Button type="primary" onClick={openCreateDrawer}>新建任务</Button></PermissionGate><Input allowClear placeholder="搜索任务/Namespace/App" value={String(query.keyword ?? '')} onChange={(event) => setQuery({ keyword: event.target.value, page: 1 })} style={{ width: 220 }} /><Select allowClear placeholder="调度类型" value={query.scheduleType || undefined} onChange={(value) => setQuery({ scheduleType: value ?? '', page: 1 })} style={{ width: 130 }} options={[{ value: 'api' }, { value: 'cron' }, { value: 'fixed_rate' }]} /><Button onClick={load}>刷新</Button></Space>}
      >
        <Table rowKey="id" loading={loading} columns={columns} dataSource={filteredJobs} pagination={{ pageSize: Number(query.page_size) || 8, current: Number(query.page) || 1, onChange: (page, pageSize) => setQuery({ page, page_size: pageSize }) }} size="middle" />
      </Card>
    </div>
  );
}
