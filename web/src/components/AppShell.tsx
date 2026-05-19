import {
  ApiOutlined,
  AuditOutlined,
  DashboardOutlined,
  DeploymentUnitOutlined,
  LogoutOutlined,
  SafetyCertificateOutlined,
  ThunderboltOutlined,
  UserOutlined,
} from '@ant-design/icons';
import { Avatar, Badge, Button, Layout, Menu, Space, Typography } from 'antd';
import type { ReactNode } from 'react';

const { Header, Sider, Content } = Layout;

export interface AppShellProps {
  children: ReactNode;
  activeKey: string;
  username: string;
  roles?: string[];
  onNavigate: (key: string) => void;
  onLogout: () => void;
}

export function AppShell({ children, activeKey, username, roles = [], onNavigate, onLogout }: AppShellProps) {
  const isAdmin = roles.includes('admin');

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
          selectedKeys={[activeKey]}
          onClick={(event) => onNavigate(event.key)}
          items={[
            { key: 'dashboard', icon: <DashboardOutlined />, label: '总览' },
            { key: 'jobs', icon: <ThunderboltOutlined />, label: '任务' },
            { key: 'instances', icon: <DeploymentUnitOutlined />, label: '实例' },
            ...(isAdmin ? [
              { type: 'divider' as const },
              { key: 'users', icon: <UserOutlined />, label: '用户管理' }
            ] : []),
            { type: 'divider' },
            { key: 'workers-next', icon: <ApiOutlined />, label: 'Worker 集群', disabled: true },
            { key: 'security-next', icon: <SafetyCertificateOutlined />, label: '安全策略', disabled: true },
            { key: 'audit-next', icon: <AuditOutlined />, label: '审计日志', disabled: true },
          ]}
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
