import { Button, Card, Checkbox, Drawer, Form, Input, Space, Switch, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useState } from 'react';

import {
  createRole,
  deleteRole,
  listMenuPermissionCatalog,
  listPermissionCatalog,
  listRoles,
  listUiActionPermissionCatalog,
  updateRole,
  type CreateRoleRequest,
  type MenuPermissionCatalogItem,
  type PermissionCatalogItem,
  type RoleSummary,
  type UiActionPermissionCatalogItem,
} from '../api/client';
import { GuardedButton, PermissionGate, useCanUiAction } from '../components/Permission';
import { useRouteActive } from '../hooks/useRouteActivation';
import { useI18n } from '../i18n';
import { ROUTE_META } from '../routes';
import { persistentPagination, usePersistentTablePageSize } from '../utils/pagination';

interface RoleFormValues extends CreateRoleRequest {}

export function RolesPage() {
  const { t } = useI18n();
  const canCreate = useCanUiAction('roles.create', { resource: 'roles', action: 'manage' });
  const [roles, setRoles] = useState<RoleSummary[]>([]);
  const [permissions, setPermissions] = useState<PermissionCatalogItem[]>([]);
  const [menus, setMenus] = useState<MenuPermissionCatalogItem[]>([]);
  const [uiActions, setUiActions] = useState<UiActionPermissionCatalogItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<RoleSummary | null>(null);
  const [form] = Form.useForm<RoleFormValues>();
  const [pageSize, setPageSize] = usePersistentTablePageSize();
  const active = useRouteActive(ROUTE_META.roles.path);

  const permissionIdByKey = useMemo(() => {
    const map = new Map<string, string>();
    permissions.forEach((permission) => map.set(`${permission.resource}:${permission.action}`, permission.id));
    return map;
  }, [permissions]);

  const fetchAll = async () => {
    setLoading(true);
    try {
      const [roleData, permissionData, menuData, actionData] = await Promise.all([
        listRoles(),
        listPermissionCatalog(),
        listMenuPermissionCatalog(),
        listUiActionPermissionCatalog(),
      ]);
      setRoles(roleData);
      setPermissions(permissionData);
      setMenus(menuData);
      setUiActions(actionData);
    } catch (err) {
      message.error(err instanceof Error ? err.message : t('加载角色数据失败'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (active) void fetchAll();
  }, [active]);

  const openCreate = () => {
    setEditingRole(null);
    form.resetFields();
    form.setFieldsValue({ enabled: true, permissionIds: [], menuKeys: [], uiActionKeys: [] });
    setDrawerOpen(true);
  };

  const openEdit = (role: RoleSummary) => {
    setEditingRole(role);
    form.setFieldsValue({
      name: role.name,
      displayName: role.displayName,
      description: role.description,
      enabled: role.enabled,
      permissionIds: role.permissions
        .map((permission) => permissionIdByKey.get(`${permission.resource}:${permission.action}`))
        .filter((id): id is string => Boolean(id)),
      menuKeys: role.menuKeys,
      uiActionKeys: role.uiActionKeys,
    });
    setDrawerOpen(true);
  };

  const closeDrawer = () => {
    setDrawerOpen(false);
    setEditingRole(null);
    form.resetFields();
  };

  const handleFinish = async (values: RoleFormValues) => {
    try {
      if (editingRole) {
        await updateRole(editingRole.id, {
          displayName: values.displayName,
          description: values.description,
          enabled: values.enabled,
          permissionIds: values.permissionIds ?? [],
          menuKeys: values.menuKeys ?? [],
          uiActionKeys: values.uiActionKeys ?? [],
        });
        message.success(t('角色已更新'));
      } else {
        await createRole({
          name: values.name,
          displayName: values.displayName,
          description: values.description,
          enabled: values.enabled,
          permissionIds: values.permissionIds ?? [],
          menuKeys: values.menuKeys ?? [],
          uiActionKeys: values.uiActionKeys ?? [],
        });
        message.success(t('角色已创建'));
      }
      closeDrawer();
      await fetchAll();
    } catch (err) {
      message.error(err instanceof Error ? err.message : t('保存角色失败'));
    }
  };

  const handleDelete = async (role: RoleSummary) => {
    try {
      await deleteRole(role.id);
      message.success(t('角色已删除'));
      await fetchAll();
    } catch (err) {
      message.error(err instanceof Error ? err.message : t('删除角色失败'));
    }
  };

  const columns: ColumnsType<RoleSummary> = [
    {
      title: t('角色'),
      dataIndex: 'displayName',
      align: 'center',
      render: (_, role) => (
        <Space direction="vertical" size={2}>
          <Space>
            <strong>{role.displayName}</strong>
            {role.builtin ? <Tag color="purple">OWNER</Tag> : null}
            {!role.assignable ? <Tag color="cyan">{t('系统保留')}</Tag> : null}
            {!role.enabled ? <Tag color="default">{t('已禁用')}</Tag> : null}
          </Space>
          <Typography.Text type="secondary">{role.name}</Typography.Text>
        </Space>
      ),
    },
    { title: t('后端权限'), align: 'center', render: (_, role) => role.permissions.length },
    { title: t('菜单'), align: 'center', render: (_, role) => role.menuKeys.length },
    { title: t('操作元素'), align: 'center', render: (_, role) => role.uiActionKeys.length },
    { title: t('更新时间'), dataIndex: 'updatedAt', align: 'center', width: 220 },
    {
      title: t('操作'),
      align: 'center',
      width: 180,
      render: (_, role) => (
        <Space>
          <GuardedButton resource="roles" action="manage" uiActionKey="roles.edit" type="link" size="small" disabled={role.builtin} onClick={() => openEdit(role)}>
            {t('编辑')}
          </GuardedButton>
          <GuardedButton
            resource="roles"
            action="manage"
            uiActionKey="roles.delete"
            type="link"
            size="small"
            danger
            disabled={role.builtin}
            confirmTitle={t('确定要删除该角色吗？')}
            confirmDescription={t('删除角色前必须确保没有用户正在使用它。')}
            onConfirm={() => void handleDelete(role)}
          >
            {t('删除')}
          </GuardedButton>
        </Space>
      ),
    },
  ];

  return (
    <div className="page-stack">
      <Drawer title={editingRole ? t('编辑角色') : t('创建角色')} open={drawerOpen} onClose={closeDrawer} width={920} destroyOnClose>
        <Typography.Paragraph type="secondary">
          {t('角色权限由后端接口权限、菜单入口权限和界面操作元素权限共同组成；后端接口权限始终是最终安全边界。')}
        </Typography.Paragraph>
        <Form form={form} layout="vertical" initialValues={{ enabled: true, permissionIds: [], menuKeys: [], uiActionKeys: [] }} onFinish={handleFinish}>
          <Form.Item name="name" label={t('角色标识')} rules={[{ required: !editingRole, message: t('请输入角色标识') }]}>
            <Input disabled={Boolean(editingRole)} placeholder="tenant-admin" />
          </Form.Item>
          <Form.Item name="displayName" label={t('显示名称')} rules={[{ required: true, message: t('请输入显示名称') }]}>
            <Input placeholder={t('租户管理员')} />
          </Form.Item>
          <Form.Item name="description" label={t('描述')}>
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item name="enabled" label={t('启用')} valuePropName="checked">
            <Switch disabled={editingRole?.builtin} />
          </Form.Item>
          <MatrixSection title={t('后端接口权限')} name="permissionIds" options={permissions.map((permission) => ({ label: `${permission.resource}:${permission.action} · ${permission.description}`, value: permission.id }))} />
          <MatrixSection title={t('菜单权限')} name="menuKeys" options={menus.map((menu) => ({ label: `${menu.label} · ${menu.routePath}`, value: menu.key }))} />
          <MatrixSection title={t('界面操作元素权限')} name="uiActionKeys" options={uiActions.map((action) => ({ label: `${action.label} · ${action.key}${action.dangerous ? ` · ${t('危险操作')}` : ''}`, value: action.key }))} />
          <Space>
            <PermissionGate resource="roles" action="manage"><Button type="primary" htmlType="submit" disabled={editingRole?.builtin}>{editingRole ? t('保存角色') : t('创建角色')}</Button></PermissionGate>
            <Button onClick={closeDrawer}>{t('取消')}</Button>
          </Space>
        </Form>
      </Drawer>

      <Card
        className="clean-card"
        title={t('角色管理')}
        extra={<Space wrap className="card-toolbar"><PermissionGate resource="roles" action="manage">{canCreate ? <Button type="primary" onClick={openCreate}>{t('新建角色')}</Button> : null}</PermissionGate><Button onClick={fetchAll}>{t('刷新')}</Button></Space>}
      >
        <Table rowKey="id" loading={loading} columns={columns} dataSource={roles} pagination={persistentPagination(pageSize, setPageSize)} size="middle" />
      </Card>
    </div>
  );
}

function MatrixSection({ title, name, options }: { title: string; name: keyof RoleFormValues; options: { label: string; value: string }[] }) {
  return (
    <Card size="small" title={title} className="clean-card" style={{ marginBottom: 16 }}>
      <Form.Item name={name} noStyle>
        <Checkbox.Group options={options} className="role-matrix-checkboxes" />
      </Form.Item>
    </Card>
  );
}
