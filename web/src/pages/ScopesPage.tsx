import { Button, Card, Col, Form, Input, Row, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  createAppScope,
  createNamespace,
  createWorkerPool,
  deleteAppScope,
  deleteNamespace,
  deleteWorkerPool,
  listAppScopes,
  listNamespaces,
  listWorkerPools,
  type AppScopeSummary,
  type CreateAppScopeRequest,
  type CreateNamespaceRequest,
  type CreateWorkerPoolRequest,
  type NamespaceSummary,
  type WorkerPoolSummary,
} from '../api/client';
import { GuardedButton, PermissionGate, useCan } from '../components/Permission';

export function ScopesPage() {
  const canManageScopes = useCan('tenants', 'manage');
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [workerPools, setWorkerPools] = useState<WorkerPoolSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [namespaceForm] = Form.useForm<CreateNamespaceRequest>();
  const [appForm] = Form.useForm<CreateAppScopeRequest>();
  const [poolForm] = Form.useForm<CreateWorkerPoolRequest>();

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [namespaceData, appData, poolData] = await Promise.all([listNamespaces(), listAppScopes(), listWorkerPools()]);
      setNamespaces(namespaceData);
      setApps(appData);
      setWorkerPools(poolData);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '加载租户范围失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const namespaceOptions = useMemo(() => namespaces.map((item) => ({ value: item.name, label: item.name })), [namespaces]);
  const appOptions = useMemo(() => apps.map((item) => ({ value: item.name, label: `${item.namespace}/${item.name}` })), [apps]);

  const handleNamespaceCreate = async (values: CreateNamespaceRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理租户范围'); return; }
    await createNamespace(values);
    namespaceForm.resetFields();
    message.success('命名空间已创建');
    await refresh();
  };

  const handleAppCreate = async (values: CreateAppScopeRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理租户范围'); return; }
    await createAppScope(values);
    appForm.resetFields(['name']);
    message.success('应用已创建');
    await refresh();
  };

  const handleWorkerPoolCreate = async (values: CreateWorkerPoolRequest) => {
    if (!canManageScopes) { message.error('当前账号无权限管理租户范围'); return; }
    await createWorkerPool(values);
    poolForm.resetFields(['name']);
    message.success('Worker Pool 已创建');
    await refresh();
  };


  const handleNamespaceDelete = async (id: string) => {
    await deleteNamespace(id);
    message.success('命名空间已删除');
    await refresh();
  };

  const handleAppDelete = async (id: string) => {
    await deleteAppScope(id);
    message.success('应用已删除');
    await refresh();
  };

  const handleWorkerPoolDelete = async (id: string) => {
    await deleteWorkerPool(id);
    message.success('Worker Pool 已删除');
    await refresh();
  };

  const namespaceColumns: ColumnsType<NamespaceSummary> = [
    { title: '命名空间', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '创建时间', dataIndex: 'created_at' },
    { title: '更新时间', dataIndex: 'updated_at' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除命名空间" confirmDescription="仅空命名空间可删除；含应用、Worker Pool 或任务时后端会拒绝。" onConfirm={() => void handleNamespaceDelete(record.id)}>删除</GuardedButton> },
  ];

  const appColumns: ColumnsType<AppScopeSummary> = [
    { title: '命名空间', dataIndex: 'namespace', render: (value: string) => <Tag color="blue">{value}</Tag> },
    { title: '应用', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '更新时间', dataIndex: 'updated_at' },
    { title: '操作', width: 120, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除应用" confirmDescription="仅空应用可删除；含 Worker Pool 或任务时后端会拒绝。" onConfirm={() => void handleAppDelete(record.id)}>删除</GuardedButton> },
  ];

  const poolColumns: ColumnsType<WorkerPoolSummary> = [
    { title: '命名空间', dataIndex: 'namespace', render: (value: string) => <Tag color="blue">{value}</Tag> },
    { title: '应用', dataIndex: 'app', render: (value: string) => <Tag color="purple">{value}</Tag> },
    { title: 'Worker Pool', dataIndex: 'name', render: (name: string) => <strong>{name}</strong> },
    { title: '更新时间', dataIndex: 'updated_at' },
    { title: '操作', width: 140, render: (_, record) => <GuardedButton resource="tenants" action="manage" type="link" size="small" danger confirmTitle="删除 Worker Pool" confirmDescription="删除后不会影响在线 Worker，会移除该持久化元数据。" onConfirm={() => void handleWorkerPoolDelete(record.id)}>删除</GuardedButton> },
  ];

  return (
    <div className="page-stack scope-management-page">
      <Card className="hero-panel scope-management-hero">
        <Space direction="vertical" size={4}>
          <Typography.Title level={2}>租户范围</Typography.Title>
          <Typography.Paragraph type="secondary">
            管理 namespace、app 和 Worker Pool 元数据；这些范围会被 API Token scope binding、Worker 可见性和后续 OIDC 映射复用。
          </Typography.Paragraph>
        </Space>
        <Button onClick={() => void refresh()} loading={loading}>刷新</Button>
      </Card>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card className="clean-card scope-form-card" title="创建命名空间">
            <Form form={namespaceForm} layout="vertical" onFinish={(values) => void handleNamespaceCreate(values)}>
              <Form.Item name="name" label="命名空间" rules={[{ required: true, message: '请输入命名空间' }]}><Input placeholder="default / payments" /></Form.Item>
              <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建命名空间</Button></PermissionGate>
            </Form>
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card className="clean-card scope-form-card" title="创建应用">
            <Form form={appForm} layout="vertical" onFinish={(values) => void handleAppCreate(values)}>
              <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
              <Form.Item name="name" label="应用" rules={[{ required: true, message: '请输入应用名' }]}><Input placeholder="billing / settlement" /></Form.Item>
              <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建应用</Button></PermissionGate>
            </Form>
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card className="clean-card scope-form-card" title="创建 Worker Pool">
            <Form form={poolForm} layout="vertical" onFinish={(values) => void handleWorkerPoolCreate(values)}>
              <Form.Item name="namespace" label="命名空间" rules={[{ required: true, message: '请选择命名空间' }]}><Select options={namespaceOptions} placeholder="选择 namespace" /></Form.Item>
              <Form.Item name="app" label="应用" rules={[{ required: true, message: '请选择应用' }]}><Select options={appOptions} placeholder="选择 app" /></Form.Item>
              <Form.Item name="name" label="Worker Pool" rules={[{ required: true, message: '请输入 Worker Pool' }]}><Input placeholder="critical / batch" /></Form.Item>
              <PermissionGate resource="tenants" action="manage"><Button type="primary" htmlType="submit" block>创建 Worker Pool</Button></PermissionGate>
            </Form>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={8}><Card className="clean-card" title="命名空间"><Table rowKey="id" loading={loading} columns={namespaceColumns} dataSource={namespaces} pagination={{ pageSize: 6 }} size="small" /></Card></Col>
        <Col xs={24} xl={8}><Card className="clean-card" title="应用"><Table rowKey="id" loading={loading} columns={appColumns} dataSource={apps} pagination={{ pageSize: 6 }} size="small" /></Card></Col>
        <Col xs={24} xl={8}><Card className="clean-card" title="Worker Pool"><Table rowKey="id" loading={loading} columns={poolColumns} dataSource={workerPools} pagination={{ pageSize: 6 }} size="small" /></Card></Col>
      </Row>
    </div>
  );
}
