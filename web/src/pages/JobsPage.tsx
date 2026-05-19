import { Button, Card, Form, Input, Select, Space, Switch, Table, Tag, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';

import { createJob, triggerJob, type CreateJobRequest, type JobSummary } from '../api/client';

export interface JobsPageProps {
  jobs: JobSummary[];
  loading: boolean;
  onRefresh: () => Promise<void>;
  onTriggered: () => Promise<void>;
}

export function JobsPage({ jobs, loading, onRefresh, onTriggered }: JobsPageProps) {
  const [form] = Form.useForm<CreateJobRequest>();

  const columns: ColumnsType<JobSummary> = [
    { title: 'Name', dataIndex: 'name' },
    { title: 'Namespace', dataIndex: 'namespace' },
    { title: 'App', dataIndex: 'app' },
    { title: 'Schedule', dataIndex: 'schedule_type', render: (value: string) => <Tag>{value}</Tag> },
    { title: 'Enabled', dataIndex: 'enabled', render: (value: boolean) => value ? 'Yes' : 'No' },
    {
      title: 'Actions',
      render: (_, job) => (
        <Button
          type="link"
          onClick={async () => {
            await triggerJob(job.id, { trigger_type: 'api' });
            message.success(`已触发 ${job.name}`);
            await onTriggered();
          }}
        >
          Trigger
        </Button>
      ),
    },
  ];

  return (
    <div className="page-stack">
      <Card title="Create Job">
        <Form
          form={form}
          layout="inline"
          initialValues={{ namespace: 'default', app: 'default', schedule_type: 'api', enabled: true }}
          onFinish={async (values) => {
            await createJob(values);
            message.success('Job created');
            form.resetFields(['name', 'schedule_expr']);
            await onRefresh();
          }}
        >
          <Form.Item name="namespace" rules={[{ required: true }]}><Input placeholder="namespace" /></Form.Item>
          <Form.Item name="app" rules={[{ required: true }]}><Input placeholder="app" /></Form.Item>
          <Form.Item name="name" rules={[{ required: true }]}><Input placeholder="job name" /></Form.Item>
          <Form.Item name="schedule_type">
            <Select style={{ width: 130 }} options={[{ value: 'api' }, { value: 'cron' }, { value: 'fixed_rate' }]} />
          </Form.Item>
          <Form.Item name="enabled" valuePropName="checked"><Switch checkedChildren="on" unCheckedChildren="off" /></Form.Item>
          <Form.Item><Button type="primary" htmlType="submit">Create</Button></Form.Item>
        </Form>
      </Card>
      <Card
        title="Jobs"
        extra={<Space><Button onClick={onRefresh}>Refresh</Button></Space>}
      >
        <Table rowKey="id" loading={loading} columns={columns} dataSource={jobs} pagination={{ pageSize: 8 }} />
      </Card>
    </div>
  );
}
