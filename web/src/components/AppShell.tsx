import { LogoutOutlined } from '@ant-design/icons';
import { Avatar, Badge, Button, ColorPicker, Layout, Menu, Space, Tooltip, Typography } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import type { ReactNode } from 'react';
import { hasPermission, usePrincipal } from './AuthGuard';
import { MENU_ROUTE_META } from '../routes';
import { DEFAULT_PRIMARY_COLOR, useThemeSettings } from '../theme';

const { Header, Sider, Content } = Layout;

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
  const { primaryColor, setPrimaryColor, resetPrimaryColor } = useThemeSettings();

  const selectedKey = '/' + location.pathname.split('/').filter(Boolean)[0];
  const visibleRoutes = MENU_ROUTE_META.filter((route) => !route.permission || hasPermission(principal, route.permission.resource, route.permission.action));
  const menuItems = [
    ...visibleRoutes
      .filter((route) => route.group === 'main')
      .map((route) => ({ key: route.menuKey, icon: route.icon, label: route.label, disabled: route.disabled })),
    ...visibleRoutes.some((route) => route.group === 'governance') ? [{ type: 'divider' as const }] : [],
    ...visibleRoutes
      .filter((route) => route.group === 'governance')
      .map((route) => ({ key: route.menuKey, icon: route.icon, label: route.label, disabled: route.disabled })),
    { type: 'divider' as const },
    ...visibleRoutes
      .filter((route) => route.group === 'coming-soon')
      .map((route) => ({ key: route.menuKey, icon: route.icon, label: route.label, disabled: route.disabled })),
  ];

  return (
    <Layout className="app-shell">
      <Sider breakpoint="lg" collapsedWidth="0" width={264} className="app-shell__sider">
        <div className="app-shell__brand">
          <div className="app-shell__brand-mark">S</div>
          <div>
            <div className="app-shell__brand-title">tikee</div>
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
            <Tooltip title="设置全局主色调">
              <ColorPicker
                value={primaryColor}
                presets={[{ label: '站点主色', colors: [DEFAULT_PRIMARY_COLOR, '#4f46e5', '#7c3aed', '#0891b2', '#059669', '#dc2626'] }]}
                onChange={(color) => setPrimaryColor(color.toHexString())}
                panelRender={(_, { components: { Picker, Presets } }) => (
                  <div className="theme-color-picker-panel">
                    <Picker />
                    <Presets />
                    <Button size="small" onClick={resetPrimaryColor}>恢复默认主色</Button>
                  </div>
                )}
              />
            </Tooltip>
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
