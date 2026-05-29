import { DeleteOutlined, EditOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { Button, Card, DatePicker, Form, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useState } from 'react';

import { createCalendar, deleteCalendar, listAppScopes, listCalendars, listNamespaces, type AppScopeSummary, type CalendarSummary, type NamespaceSummary } from '../api/client';

interface CalendarFormValues {
  namespace: string;
  app: string;
  name: string;
  timezone: string;
  excludedDates?: any[];
  holidays?: any[];
  maintenanceWindows?: Array<{ range: [any, any] }>;
  freezeWindows?: Array<{ range: [any, any] }>;
}

export function CalendarsPage() {
  const [items, setItems] = useState<CalendarSummary[]>([]);
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<CalendarSummary | null>(null);
  const [form] = Form.useForm<CalendarFormValues>();

  const reload = async () => {
    setLoading(true);
    try {
      const [calendars, namespaceItems, appItems] = await Promise.all([listCalendars(), listNamespaces(), listAppScopes()]);
      setItems(calendars);
      setNamespaces(namespaceItems);
      setApps(appItems);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void reload(); }, []);

  const handleSubmit = async () => {
    const values = await form.validateFields();
    
    const excludedDates = (values.excludedDates || []).map((d: any) => d.format('YYYY-MM-DD'));
    const holidays = (values.holidays || []).map((d: any) => d.format('YYYY-MM-DD'));
    
    const maintenanceWindows = (values.maintenanceWindows || []).map((item) => {
      if (item && item.range && item.range[0] && item.range[1]) {
        return {
          start: item.range[0].toISOString(),
          end: item.range[1].toISOString(),
        };
      }
      return null;
    }).filter(Boolean);

    const freezeWindows = (values.freezeWindows || []).map((item) => {
      if (item && item.range && item.range[0] && item.range[1]) {
        return {
          start: item.range[0].toISOString(),
          end: item.range[1].toISOString(),
        };
      }
      return null;
    }).filter(Boolean);

    await createCalendar({
      namespace: values.namespace,
      app: values.app,
      name: values.name,
      timezone: values.timezone || 'UTC',
      excludedDates,
      holidays,
      maintenanceWindows: maintenanceWindows as any[],
      freezeWindows: freezeWindows as any[],
    });

    setOpen(false);
    setEditingItem(null);
    form.resetFields();
    message.success('Calendar 已保存');
    await reload();
  };

  const handleEdit = (item: CalendarSummary) => {
    setEditingItem(item);
    form.setFieldsValue({
      namespace: item.namespace,
      app: item.app,
      name: item.name,
      timezone: item.timezone,
      excludedDates: item.excludedDates.map((d) => dayjs(d)),
      holidays: item.holidays.map((d) => dayjs(d)),
      maintenanceWindows: item.maintenanceWindows.map((w) => ({
        range: [dayjs(w.start), dayjs(w.end)],
      })),
      freezeWindows: item.freezeWindows.map((w) => ({
        range: [dayjs(w.start), dayjs(w.end)],
      })),
    });
    setOpen(true);
  };

  const handleDelete = async (id: string) => {
    await deleteCalendar(id);
    message.success('Calendar 已删除');
    await reload();
  };

  return (
    <Space direction="vertical" size={20} style={{ width: '100%' }}>
      <div>
        <Typography.Title level={2}>调度日历</Typography.Title>
        <Typography.Text type="secondary">集中维护 namespace/app 作用域下的节假日、维护窗口和冻结窗口；任务可通过 Calendar 引用绑定。</Typography.Text>
      </div>
      <Card extra={<Space><Button icon={<ReloadOutlined />} onClick={() => void reload()}>刷新</Button><Button type="primary" icon={<PlusOutlined />} onClick={() => { setEditingItem(null); form.resetFields(); form.setFieldsValue({ namespace: 'default', app: 'default', timezone: 'Asia/Shanghai' }); setOpen(true); }}>新建 Calendar</Button></Space>}>
        <Table<CalendarSummary>
          rowKey="id"
          loading={loading}
          dataSource={items}
          columns={[
            { title: '名称', dataIndex: 'name' },
            { title: '范围', render: (_, item) => `${item.namespace}/${item.app}` },
            { title: '时区', dataIndex: 'timezone' },
            { title: '排除日期', render: (_, item) => <Space wrap>{[...item.excludedDates, ...item.holidays].map((date) => <Tag key={date}>{date}</Tag>)}</Space> },
            { title: '维护/冻结窗口', render: (_, item) => `${item.maintenanceWindows.length}/${item.freezeWindows.length}` },
            { title: '操作', width: 180, render: (_, item) => (
              <Space>
                <Button size="small" icon={<EditOutlined />} onClick={() => handleEdit(item)}>编辑</Button>
                <Button danger size="small" icon={<DeleteOutlined />} onClick={() => void handleDelete(item.id)}>删除</Button>
              </Space>
            ) },
          ]}
        />
      </Card>
      <Modal title={editingItem ? "更新 Calendar" : "新建 Calendar"} open={open} width={760} onOk={() => void handleSubmit()} onCancel={() => setOpen(false)} okText="保存">
        <Form form={form} layout="vertical">
          <Form.Item name="namespace" label="Namespace" rules={[{ required: true }]}><Select disabled={!!editingItem} showSearch options={namespaces.map((item) => ({ value: item.name, label: item.name }))} /></Form.Item>
          <Form.Item name="app" label="App" rules={[{ required: true }]}><Select disabled={!!editingItem} showSearch options={apps.map((item) => ({ value: item.name, label: `${item.namespace}/${item.name}` }))} /></Form.Item>
          <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input disabled={!!editingItem} placeholder="cn-maintenance" /></Form.Item>
          <Form.Item name="timezone" label="时区"><Input placeholder="Asia/Shanghai" /></Form.Item>
          <Form.Item name="excludedDates" label="排除日期"><DatePicker multiple style={{ width: '100%' }} placeholder="选择排除日期" /></Form.Item>
          <Form.Item name="holidays" label="节假日"><DatePicker multiple style={{ width: '100%' }} placeholder="选择节假日" /></Form.Item>
          
          <Typography.Paragraph strong style={{ marginTop: 16 }}>维护窗口</Typography.Paragraph>
          <Form.List name="maintenanceWindows">
            {(fields, { add, remove }) => (
              <>
                {fields.map(({ key, name, ...restField }) => (
                  <Space key={key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                    <Form.Item
                      {...restField}
                      name={[name, 'range']}
                      rules={[{ required: true, message: '请选择时间范围' }]}
                    >
                      <DatePicker.RangePicker showTime />
                    </Form.Item>
                    <Button danger onClick={() => remove(name)} icon={<DeleteOutlined />}>删除</Button>
                  </Space>
                ))}
                <Form.Item>
                  <Button type="dashed" onClick={() => add()} block icon={<PlusOutlined />}>添加维护窗口</Button>
                </Form.Item>
              </>
            )}
          </Form.List>

          <Typography.Paragraph strong style={{ marginTop: 16 }}>冻结窗口</Typography.Paragraph>
          <Form.List name="freezeWindows">
            {(fields, { add, remove }) => (
              <>
                {fields.map(({ key, name, ...restField }) => (
                  <Space key={key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                    <Form.Item
                      {...restField}
                      name={[name, 'range']}
                      rules={[{ required: true, message: '请选择时间范围' }]}
                    >
                      <DatePicker.RangePicker showTime />
                    </Form.Item>
                    <Button danger onClick={() => remove(name)} icon={<DeleteOutlined />}>删除</Button>
                  </Space>
                ))}
                <Form.Item>
                  <Button type="dashed" onClick={() => add()} block icon={<PlusOutlined />}>添加冻结窗口</Button>
                </Form.Item>
              </>
            )}
          </Form.List>
        </Form>
      </Modal>
    </Space>
  );
}
