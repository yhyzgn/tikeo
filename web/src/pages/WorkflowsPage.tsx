import { useEffect, useMemo, useState } from 'react';
import { Button, Card, Form, Input, List, Space, Tag, Typography, message } from 'antd';
import { createWorkflow, listWorkflows, runWorkflow, validateWorkflow, type WorkflowDefinition, type WorkflowSummary } from '../api/client';

const DEFAULT_DEFINITION = JSON.stringify({
  nodes: [{ key: 'start', name: 'Start', kind: 'job' }],
  edges: [],
}, null, 2);

export function WorkflowsPage() {
  const [items, setItems] = useState<WorkflowSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<{ name: string; definition: string }>();

  const fetchItems = async () => {
    setLoading(true);
    try { setItems(await listWorkflows()); } finally { setLoading(false); }
  };

  useEffect(() => { void fetchItems(); }, []);

  const latest = useMemo(() => items[0], [items]);

  const submit = async (values: { name: string; definition: string }) => {
    let definition: WorkflowDefinition;
    try { definition = JSON.parse(values.definition) as WorkflowDefinition; }
    catch { message.error('Workflow definition 必须是合法 JSON'); return; }
    await createWorkflow({ name: values.name, definition });
    message.success('Workflow 已创建');
    form.resetFields();
    form.setFieldValue('definition', DEFAULT_DEFINITION);
    await fetchItems();
  };

  const validate = async (id: string) => {
    const result = await validateWorkflow(id);
    if (result.valid) message.success('DAG 校验通过');
    else message.error(result.errors.join('; '));
  };

  const run = async (id: string) => {
    const instance = await runWorkflow(id);
    message.success(`Workflow instance queued: ${instance.id}`);
  };

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div>
        <Typography.Title level={2}>工作流</Typography.Title>
        <Typography.Text type="secondary">Phase2 DAG 工作流定义、校验和最小运行入口</Typography.Text>
      </div>
      <Card title="创建 Workflow JSON 定义">
        <Form form={form} layout="vertical" initialValues={{ definition: DEFAULT_DEFINITION }} onFinish={submit}>
          <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input placeholder="daily-pipeline" /></Form.Item>
          <Form.Item name="definition" label="定义 JSON" rules={[{ required: true }]}>
            <Input.TextArea rows={10} spellCheck={false} />
          </Form.Item>
          <Button type="primary" htmlType="submit">创建工作流</Button>
        </Form>
      </Card>
      <Card title="工作流列表" extra={<Button onClick={fetchItems}>刷新</Button>}>
        <List
          loading={loading}
          dataSource={items}
          locale={{ emptyText: '暂无工作流' }}
          renderItem={(item) => (
            <List.Item actions={[<Button key="validate" onClick={() => validate(item.id)}>校验</Button>, <Button key="run" type="primary" onClick={() => run(item.id)}>运行</Button>]}>
              <List.Item.Meta
                title={<Space><span>{item.name}</span><Tag color="blue">{item.status}</Tag></Space>}
                description={<span>{item.id} · nodes: {item.definition.nodes.length} · edges: {item.definition.edges.length}</span>}
              />
            </List.Item>
          )}
        />
      </Card>
      {latest ? <Typography.Text type="secondary">最新工作流：{latest.name}</Typography.Text> : null}
    </Space>
  );
}
