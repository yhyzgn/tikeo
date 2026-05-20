import {
  ApiOutlined,
  AuditOutlined,
  CodeOutlined,
  DashboardOutlined,
  DeploymentUnitOutlined,
  LogoutOutlined,
  SafetyCertificateOutlined,
  ThunderboltOutlined,
  BranchesOutlined,
  UserOutlined,
} from '@ant-design/icons';
import { Avatar, Badge, Button, Layout, Menu, Space, Typography } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import type { ReactNode } from 'react';
import { hasPermission, usePrincipal } from './AuthGuard';

const { Header, Sider, Content } = Layout;

const MENU_ITEMS = [
  { key: '/dashboard', icon: <DashboardOutlined />, label: '总览' },
  { key: '/jobs', icon: <ThunderboltOutlined />, label: '任务' },
  { key: '/instances', icon: <DeploymentUnitOutlined />, label: '实例' },
  { key: '/workflows', icon: <BranchesOutlined />, label: '工作流', resource: 'workflows', action: 'read' },
];

const PROTECTED_ITEMS = [
  { key: '/users', icon: <UserOutlined />, label: '用户管理', resource: 'users', action: 'read' },
  { key: '/scripts', icon: <CodeOutlined />, label: '脚本管理', resource: 'scripts', action: 'read' },
  { key: '/audit', icon: <AuditOutlined />, label: '审计日志', resource: 'audit', action: 'read' },
];

const COMING_SOON_ITEMS = [
  { key: 'workers-next', icon: <ApiOutlined />, label: 'Worker 集群', disabled: true },
  { key: 'security-next', icon: <SafetyCertificateOutlined />, label: '安全策略', disabled: true },
];

export interface AppShellProps {
  children: ReactNode;
  onLogout: () => void;
}

export function AppShell({ children, onLogout }: AppShellProps) {
  const principal = usePrincipal();
  const navigate = useNavigate();
  const location = useLocation();
  const username = principal?.username ?? '';
  const roles = principal?.roles ?? [];
  const isAdmin = roles.includes('admin');
  const protectedItems = PROTECTED_ITEMS.filter((item) => hasPermission(principal, item.resource, item.action));

  const selectedKey = '/' + location.pathname.split('/').filter(Boolean)[0];

  const menuItems = [
    ...MENU_ITEMS.filter((item) => !('resource' in item) || (typeof item.resource === 'string' && typeof item.action === 'string' && hasPermission(principal, item.resource, item.action))),
    ...(protectedItems.length > 0
      ? [{ type: 'divider' as const }, ...protectedItems]
      : []),
    { type: 'divider' as const },
    ...COMING_SOON_ITEMS,
  ];

  return (
    <Layout className="app-shell">
      <Sider breakpoint="lg" collapsedWidth="0" width={264} className="app-shell__sider">
        <div className="app-shell__brand">
          <div className="app-shell__brand-mark">S</div>
          <div>
            <div className="app-shell__brand-title">scheduler</div>
            <div className="app-shell__brand-subtitle">Task Platform</div>
          </div>
        </div>
        <Menu
          className="app-shell__menu"
          mode="inline"
          selectedKeys={[selectedKey]}
          onClick={(event) => navigate(event.key)}
          items={menuItems}
        />
      </Sider>
      <Layout className="app-shell__main">
        <Header className="app-shell__header">
          <div>
            <Typography.Title level={3} className="app-shell__title">
              分布式任务调度平台
            </Typography.Title>
            <Typography.Text className="app-shell__subtitle">轻量、容器友好、Worker 主动隧道连接</Typography.Text>
          </div>
          <Space className="app-shell__user" size={14}>
            <Badge status="processing" text={isAdmin ? "Admin" : "Dev"} />
            <Avatar className="app-shell__avatar">{username.slice(0, 1).toUpperCase()}</Avatar>
            <Typography.Text className="app-shell__username">{username}</Typography.Text>
            <Button icon={<LogoutOutlined />} onClick={onLogout}>
              退出
            </Button>
          </Space>
        </Header>
        <Content className="app-shell__content">{children}</Content>
      </Layout>
    </Layout>
  );
}
