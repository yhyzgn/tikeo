import {
  ApiOutlined,
  AlertOutlined,
  BarsOutlined,
  AuditOutlined,
  BranchesOutlined,
  CodeOutlined,
  KeyOutlined,
  CloudSyncOutlined,
  AppstoreAddOutlined,
  DashboardOutlined,
  DeploymentUnitOutlined,
  SafetyCertificateOutlined,
  PartitionOutlined,
  ScheduleOutlined,
  ThunderboltOutlined,
  UserOutlined,
} from '@ant-design/icons';
import type { ReactNode } from 'react';

export interface RoutePermission {
  resource: string;
  action: string;
}

export interface AppRouteMeta {
  path: string;
  label: string;
  menuKey: string;
  icon?: ReactNode;
  permission?: RoutePermission;
  menu?: boolean;
  disabled?: boolean;
  group?: 'main' | 'governance' | 'coming-soon';
}

export const ROUTE_META = {
  dashboard: { path: '/dashboard', menuKey: '/dashboard', label: '总览', icon: <DashboardOutlined />, menu: true, group: 'main' },
  jobs: { path: '/jobs', menuKey: '/jobs', label: '任务', icon: <ThunderboltOutlined />, menu: true, group: 'main' },
  jobTopology: { path: '/jobs/topology', menuKey: '/jobs', label: '任务拓扑', permission: { resource: 'jobs', action: 'read' }, menu: false },
  instances: { path: '/instances', menuKey: '/instances', label: '实例', icon: <DeploymentUnitOutlined />, menu: true, group: 'main' },
  workflows: { path: '/workflows', menuKey: '/workflows', label: '工作流', icon: <BranchesOutlined />, permission: { resource: 'workflows', action: 'read' }, menu: true, group: 'main' },
  workers: { path: '/workers', menuKey: '/workers', label: 'Worker 集群', icon: <ApiOutlined />, permission: { resource: 'workers', action: 'read' }, menu: true, group: 'main' },
  dispatchQueue: { path: '/workers/dispatch-queue', menuKey: '/workers/dispatch-queue', label: '调度队列', icon: <BarsOutlined />, permission: { resource: 'workers', action: 'read' }, menu: true, group: 'main' },
  users: { path: '/users', menuKey: '/users', label: '用户管理', icon: <UserOutlined />, permission: { resource: 'users', action: 'read' }, menu: true, group: 'governance' },
  scopes: { path: '/scopes', menuKey: '/scopes', label: '租户范围', icon: <PartitionOutlined />, permission: { resource: 'tenants', action: 'read' }, menu: true, group: 'governance' },
  calendars: { path: '/calendars', menuKey: '/calendars', label: '调度日历', icon: <ScheduleOutlined />, permission: { resource: 'tenants', action: 'read' }, menu: true, group: 'governance' },
  scripts: { path: '/scripts', menuKey: '/scripts', label: '脚本管理', icon: <CodeOutlined />, permission: { resource: 'scripts', action: 'read' }, menu: true, group: 'governance' },
  plugins: { path: '/plugins', menuKey: '/plugins', label: '插件系统', icon: <AppstoreAddOutlined />, permission: { resource: 'tenants', action: 'read' }, menu: true, group: 'governance' },
  apiKeys: { path: '/api-keys', menuKey: '/api-keys', label: 'API-Key', icon: <KeyOutlined />, permission: { resource: 'tenants', action: 'manage' }, menu: true, group: 'governance' },
  gitops: { path: '/gitops', menuKey: '/gitops', label: 'GitOps/IaC', icon: <CloudSyncOutlined />, permission: { resource: 'tenants', action: 'read' }, menu: true, group: 'governance' },
  scriptEdit: { path: '/scripts/:id/edit', menuKey: '/scripts', label: '编辑脚本', permission: { resource: 'scripts', action: 'manage' }, menu: false },
  alerts: { path: '/alerts', menuKey: '/alerts', label: '告警投递', icon: <AlertOutlined />, permission: { resource: 'audit', action: 'read' }, menu: true, group: 'governance' },
  audit: { path: '/audit', menuKey: '/audit', label: '审计日志', icon: <AuditOutlined />, permission: { resource: 'audit', action: 'read' }, menu: true, group: 'governance' },
  workflowNew: { path: '/workflows/new', menuKey: '/workflows', label: '新增工作流', permission: { resource: 'workflows', action: 'manage' }, menu: false },
  workflowEdit: { path: '/workflows/:id/edit', menuKey: '/workflows', label: '编辑工作流', permission: { resource: 'workflows', action: 'manage' }, menu: false },
  securityNext: { path: 'security-next', menuKey: 'security-next', label: '安全策略', icon: <SafetyCertificateOutlined />, disabled: true, menu: true, group: 'coming-soon' },
} satisfies Record<string, AppRouteMeta>;

export const MENU_ROUTE_META: AppRouteMeta[] = Object.values(ROUTE_META).filter((route) => route.menu);
