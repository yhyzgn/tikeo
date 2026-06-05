import { BulbOutlined, LogoutOutlined, MoonOutlined } from '@ant-design/icons';
import { Avatar, Badge, Button, ColorPicker, Layout, Menu, Select, Space, Tooltip, Typography } from 'antd';
import type { MenuProps } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { hasPermission, usePrincipal } from './AuthGuard';
import { MENU_GROUPS, MENU_ROUTE_META } from '../routes';
import { DEFAULT_PRIMARY_COLOR, useThemeSettings } from '../theme';
import { LOCALE_OPTIONS, useI18n } from '../i18n';
import { TikeeLogo } from './TikeeLogo';

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
  const { primaryColor, mode, resolvedMode, setPrimaryColor, resetPrimaryColor, setMode } = useThemeSettings();
  const { locale, setLocale, t } = useI18n();

  const visibleRoutes = MENU_ROUTE_META.filter((route) => !route.permission || hasPermission(principal, route.permission.resource, route.permission.action));
  const selectedRoute = [...visibleRoutes]
    .filter((route) => location.pathname === route.path || location.pathname.startsWith(`${route.path}/`))
    .sort((left, right) => right.path.length - left.path.length)[0];
  const selectedKey = selectedRoute?.menuKey ?? '/' + location.pathname.split('/').filter(Boolean)[0];
  const overviewItems: MenuProps['items'] = visibleRoutes
    .filter((route) => route.group === 'overview')
    .map((route) => ({ key: route.menuKey, icon: route.icon, label: t(route.label), disabled: route.disabled }));
  const groupedItems: MenuProps['items'] = MENU_GROUPS.map((group) => {
    const children = visibleRoutes
      .filter((route) => route.group === group.key)
      .map((route) => ({ key: route.menuKey, icon: route.icon, label: t(route.label), disabled: route.disabled }));
    if (children.length === 0) return null;
    return { key: `group:${group.key}`, icon: group.icon, label: t(group.label), children };
  }).filter((item): item is NonNullable<typeof item> => item !== null);
  const menuItems: MenuProps['items'] = [...overviewItems, ...groupedItems];
  const activeGroupKey = selectedRoute?.group && selectedRoute.group !== 'overview' ? `group:${selectedRoute.group}` : undefined;
  const routeOpenKeys = useMemo(() => (activeGroupKey ? [activeGroupKey] : []), [activeGroupKey]);
  const [openKeys, setOpenKeys] = useState<string[]>(routeOpenKeys);

  useEffect(() => {
    setOpenKeys((current) => {
      const nextKeys = routeOpenKeys.filter((key) => !current.includes(key));
      return nextKeys.length > 0 ? [...current, ...nextKeys] : current;
    });
  }, [routeOpenKeys]);

  return (
    <Layout className="app-shell">
      <Sider width={304} className="app-shell__sider">
        <div className="app-shell__brand">
          <TikeeLogo size={64} />
          <div>
            <div className="app-shell__brand-title">tikee</div>
            <div className="app-shell__brand-subtitle">Task Platform</div>
          </div>
        </div>
        <Menu
          className="app-shell__menu"
          mode="inline"
          selectedKeys={[selectedKey]}
          openKeys={openKeys}
          onOpenChange={(keys) => setOpenKeys(keys.map(String))}
          onClick={(event) => {
            if (!event.key.startsWith('group:')) navigate(event.key);
          }}
          items={menuItems}
        />
      </Sider>
      <Layout className="app-shell__main">
        <Header className="app-shell__header">
          <div>
            <Typography.Title level={3} className="app-shell__title">
              {t('分布式任务调度平台')}
            </Typography.Title>
            <Typography.Text className="app-shell__subtitle">{t('轻量、容器友好、Worker 主动隧道连接')}</Typography.Text>
          </div>
          <Space className="app-shell__user" size={14}>
            <Tooltip title={mode === 'system' ? `${t('跟随系统')}：${t('当前')}${resolvedMode === 'dark' ? t('暗色') : t('亮色')}` : t('选择明暗主题')}>
              <Select
                aria-label={t('选择明暗主题')}
                value={mode}
                onChange={setMode}
                style={{ width: 116 }}
                options={[
                  { value: 'system', label: t('跟随系统') },
                  { value: 'light', label: <Space size={6}><BulbOutlined />{t('亮色')}</Space> },
                  { value: 'dark', label: <Space size={6}><MoonOutlined />{t('暗色')}</Space> },
                ]}
              />
            </Tooltip>
            <Tooltip title={t('设置全局主色调')}>
              <ColorPicker
                value={primaryColor}
                presets={[{ label: t('站点主色'), colors: [DEFAULT_PRIMARY_COLOR, '#4f46e5', '#7c3aed', '#0891b2', '#059669', '#dc2626'] }]}
                onChange={(color) => setPrimaryColor(color.toHexString())}
                panelRender={(_, { components: { Picker, Presets } }) => (
                  <div className="theme-color-picker-panel">
                    <Picker />
                    <Presets />
                    <Button size="small" onClick={resetPrimaryColor}>{t('恢复默认主色')}</Button>
                  </div>
                )}
              />
            </Tooltip>

            <Select
              aria-label={t('选择语言')}
              value={locale}
              onChange={setLocale}
              style={{ width: 112 }}
              options={LOCALE_OPTIONS}
            />
            <Badge status="processing" text={isAdmin ? "Admin" : "Dev"} />
            <Avatar className="app-shell__avatar">{username.slice(0, 1).toUpperCase()}</Avatar>
            <Typography.Text className="app-shell__username">{username}</Typography.Text>
            <Button icon={<LogoutOutlined />} onClick={onLogout}>
              {t('退出')}
            </Button>
          </Space>
        </Header>
        <Content className="app-shell__content">{children}</Content>
      </Layout>
    </Layout>
  );
}
