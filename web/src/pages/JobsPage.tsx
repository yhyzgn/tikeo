import { Button, Card, Dropdown, Form, Input, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useState } from 'react';

import { createJob, listJobs, triggerJob, type CreateJobRequest, type JobSummary } from '../api/client';

export function JobsPage() {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<CreateJobRequest>();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const page = await listJobs();
      setJobs(page.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const columns: ColumnsType<JobSummary> = [
    { title: 'Name', dataIndex: 'name' },
    { title: 'Namespace / App', render: (_, job) => <Space direction="vertical" size={0}><strong>{job.namespace}</strong><Typography.Text type="secondary" style={{ fontSize: 12 }}>{job.app}</Typography.Text></Space> },
    { title: 'Schedule', dataIndex: 'schedule_type', render: (value: string) => <Tag color="blue" className="soft-tag">{value}</Tag> },
    { title: 'Enabled', dataIndex: 'enabled', render: (value: boolean) => <Switch size="small" checked={value} disabled /> },
    {
      title: 'Actions',
      width: 120,
      render: (_, job) => (
        <Dropdown.Button
          type="primary"
          menu={{
            items: [
              { key: 'single', label: '单机执行' },
              { key: 'broadcast', label: '广播执行' },
            ],
            onClick: async ({ key }) => {
              try {
                await triggerJob(job.id, { trigger_type: 'api', execution_mode: key === 'broadcast' ? 'broadcast' : 'single' });
                message.success(key === 'broadcast' ? `已广播触发 ${job.name}` : `已触发 ${job.name}`);
                await load();
              } catch (err) {
                message.error(err instanceof Error ? err.message : '触发失败');
              }
            },
          }}
          onClick={async () => {
            try {
              await triggerJob(job.id, { trigger_type: 'api', execution_mode: 'single' });
              message.success(`已触发 ${job.name}`);
              await load();
            } catch (err) {
              message.error(err instanceof Error ? err.message : '触发失败');
            }
          }}
        >
          触发
        </Dropdown.Button>
      ),
    },
  ];

  return (
    <div className="page-stack">
      <Card className="clean-card" title="创建任务" extra={<Typography.Text type="secondary">Worker 需匹配 Namespace 与 App</Typography.Text>}>
        <Form
          form={form}
          layout="inline"
          initialValues={{ namespace: 'default', app: 'default', schedule_type: 'api', enabled: true }}
          onFinish={async (values) => {
            try {
              await createJob(values);
              message.success('任务已创建');
              form.resetFields(['name', 'schedule_expr']);
              await load();
            } catch (err) {
              message.error(err instanceof Error ? err.message : '创建失败');
            }
          }}
        >
          <Form.Item name="namespace" rules={[{ required: true }]}><Input placeholder="namespace" style={{ width: 120 }} /></Form.Item>
          <Form.Item name="app" rules={[{ required: true }]}><Input placeholder="app" style={{ width: 120 }} /></Form.Item>
          <Form.Item name="name" rules={[{ required: true }]}><Input placeholder="job name" style={{ width: 160 }} /></Form.Item>
          <Form.Item name="schedule_type">
            <Select style={{ width: 110 }} options={[{ value: 'api' }, { value: 'cron' }, { value: 'fixed_rate' }]} />
          </Form.Item>
          <Form.Item><Button type="primary" htmlType="submit">Create</Button></Form.Item>
        </Form>
      </Card>
      <Card
        className="clean-card"
        title="任务列表"
        extra={<Space><Button onClick={load}>刷新</Button></Space>}
      >
        <Table rowKey="id" loading={loading} columns={columns} dataSource={jobs} pagination={{ pageSize: 8 }} size="middle" />
      </Card>
    </div>
  );
}
