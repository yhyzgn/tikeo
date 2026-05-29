import { Button, Card, Drawer, Form, Input, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useState } from 'react';

import {
  createUser,
  deleteUser,
  listUsers,
  updateUser,
  type CreateUserRequest,
  type UserSummary,
} from '../api/client';
import { GuardedButton, PermissionGate, useCan } from '../components/Permission';
import { persistentPagination, usePersistentTablePageSize } from '../utils/pagination';

export function UsersPage() {
  const canManageUsers = useCan('users', 'manage');
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<CreateUserRequest>();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [pageSize, setPageSize] = usePersistentTablePageSize();

  const fetchUsersList = async () => {
    setLoading(true);
    try {
      const data = await listUsers();
      setUsers(data);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '获取用户列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void fetchUsersList();
  }, []);

  const openCreateDrawer = () => {
    setEditingId(null);
    form.resetFields();
    form.setFieldsValue({ role: 'viewer' });
    setDrawerOpen(true);
  };

  const handleEdit = (user: UserSummary) => {
    setEditingId(user.id);
    form.setFieldsValue({
      username: user.username,
      role: user.role,
      password: undefined,
    });
    setDrawerOpen(true);
  };

  const closeDrawer = () => {
    setDrawerOpen(false);
    setEditingId(null);
    form.resetFields();
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteUser(id);
      message.success('用户已删除');
      await fetchUsersList();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除用户失败');
    }
  };

  const handleFinish = async (values: CreateUserRequest) => {
    try {
      if (editingId) {
        // Update user
        await updateUser(editingId, {
          password: values.password || undefined,
          role: values.role,
        });
        message.success('用户信息已更新');
      } else {
        // Create user
        if (!values.password) {
          message.error('创建新用户必须填写密码');
          return;
        }
        await createUser(values);
        message.success('新用户已创建');
      }
      closeDrawer();
      await fetchUsersList();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '保存失败');
    }
  };

  const columns: ColumnsType<UserSummary> = [
    { title: 'Username', dataIndex: 'username', render: (val: string) => <strong>{val}</strong> },
    {
      title: 'Role',
      dataIndex: 'role',
      render: (role: string) => {
        const color = role === 'admin' ? 'red' : role === 'operator' ? 'orange' : 'blue';
        return <Tag color={color}>{role.toUpperCase()}</Tag>;
      },
    },
    { title: 'Created At', dataIndex: 'createdAt' },
    {
      title: 'Actions',
      width: 160,
      render: (_, record) => (
        <Space size="middle">
          <GuardedButton resource="users" action="manage" type="link" size="small" onClick={() => handleEdit(record)}>
            编辑
          </GuardedButton>
          <GuardedButton
            resource="users"
            action="manage"
            type="link"
            size="small"
            danger
            confirmTitle="确定要删除该用户吗？"
            confirmDescription="删除用户会立即移除其登录与管理能力。"
            onConfirm={() => void handleDelete(record.id)}
          >
            删除
          </GuardedButton>
        </Space>
      ),
    },
  ];

  return (
    <div className="page-stack">
      <Drawer
        title={editingId ? '编辑用户' : '创建用户'}
        open={drawerOpen}
        onClose={closeDrawer}
        width={640}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">用户创建与角色调整会影响登录权限；编辑用户时留空密码表示不修改。</Typography.Paragraph>
        <Form
          form={form}
          layout="vertical"
          initialValues={{ role: 'viewer' }}
          onFinish={(values) => { if (!canManageUsers) { message.error('当前账号无权限管理用户'); return; } void handleFinish(values); }}
        >
          <Form.Item name="username" label="用户名" rules={[{ required: true, message: '请输入用户名' }]}>
            <Input placeholder="用户名" disabled={editingId !== null} />
          </Form.Item>
          <Form.Item name="password" label="密码" rules={editingId ? [] : [{ required: true, message: '请输入密码' }]}>
            <Input.Password placeholder={editingId ? '新密码（留空则不修改）' : '密码'} />
          </Form.Item>
          <Form.Item name="role" label="角色" rules={[{ required: true }]}>
            <Select options={[{ value: 'admin', label: 'ADMIN' }, { value: 'operator', label: 'OPERATOR' }, { value: 'viewer', label: 'VIEWER' }]} />
          </Form.Item>
          <Space>
            <PermissionGate resource="users" action="manage"><Button type="primary" htmlType="submit">{editingId ? '保存用户' : '创建用户'}</Button></PermissionGate>
            <Button onClick={closeDrawer}>取消</Button>
          </Space>
        </Form>
      </Drawer>

      <Card
        className="clean-card"
        title="用户列表"
        extra={<Space wrap className="card-toolbar"><PermissionGate resource="users" action="manage"><Button type="primary" onClick={openCreateDrawer}>新建用户</Button></PermissionGate><Button onClick={fetchUsersList}>刷新</Button></Space>}
      >
        <Table
          rowKey="id"
          loading={loading}
          columns={columns}
          dataSource={users}
          pagination={persistentPagination(pageSize, setPageSize)}
          size="middle"
        />
      </Card>
    </div>
  );
}
