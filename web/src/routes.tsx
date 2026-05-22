import {
  ApiOutlined,
  AuditOutlined,
  BranchesOutlined,
  CodeOutlined,
  DashboardOutlined,
  DeploymentUnitOutlined,
  SafetyCertificateOutlined,
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
  instances: { path: '/instances', menuKey: '/instances', label: '实例', icon: <DeploymentUnitOutlined />, menu: true, group: 'main' },
  workflows: { path: '/workflows', menuKey: '/workflows', label: '工作流', icon: <BranchesOutlined />, permission: { resource: 'workflows', action: 'read' }, menu: true, group: 'main' },
  workers: { path: '/workers', menuKey: '/workers', label: 'Worker 集群', icon: <ApiOutlined />, permission: { resource: 'workers', action: 'read' }, menu: true, group: 'main' },
  users: { path: '/users', menuKey: '/users', label: '用户管理', icon: <UserOutlined />, permission: { resource: 'users', action: 'read' }, menu: true, group: 'governance' },
  scripts: { path: '/scripts', menuKey: '/scripts', label: '脚本管理', icon: <CodeOutlined />, permission: { resource: 'scripts', action: 'read' }, menu: true, group: 'governance' },
  scriptEdit: { path: '/scripts/:id/edit', menuKey: '/scripts', label: '编辑脚本', permission: { resource: 'scripts', action: 'manage' }, menu: false },
  audit: { path: '/audit', menuKey: '/audit', label: '审计日志', icon: <AuditOutlined />, permission: { resource: 'audit', action: 'read' }, menu: true, group: 'governance' },
  workflowNew: { path: '/workflows/new', menuKey: '/workflows', label: '新增工作流', permission: { resource: 'workflows', action: 'manage' }, menu: false },
  workflowEdit: { path: '/workflows/:id/edit', menuKey: '/workflows', label: '编辑工作流', permission: { resource: 'workflows', action: 'manage' }, menu: false },
  securityNext: { path: 'security-next', menuKey: 'security-next', label: '安全策略', icon: <SafetyCertificateOutlined />, disabled: true, menu: true, group: 'coming-soon' },
} satisfies Record<string, AppRouteMeta>;

export const MENU_ROUTE_META: AppRouteMeta[] = Object.values(ROUTE_META).filter((route) => route.menu);
