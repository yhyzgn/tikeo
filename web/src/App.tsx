import { ConfigProvider, theme } from 'antd';
import antdEnUS from 'antd/locale/en_US';
import antdZhCN from 'antd/locale/zh_CN';
import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';

import { getAuthToken, getBootstrapStatus, logout, setAuthErrorHandler, setAuthToken, type BootstrapStatusResponse } from './api/client';
import { AppShell } from './components/AppShell';
import { AuthGuard, RequirePermission } from './components/AuthGuard';
import { ForbiddenPage } from './components/ForbiddenPage';
import { KeepAliveOutlet } from './components/KeepAliveOutlet';
import { RouteFallback } from './components/RouteFallback';
import { ROUTE_META } from './routes';
import { useI18n } from './i18n';
import { DEFAULT_INFO_COLOR, DEFAULT_PRIMARY_COLOR, PRIMARY_COLOR_STORAGE_KEY, THEME_MODE_STORAGE_KEY, ThemeSettingsContext, normalizeHexColor, normalizeThemeMode, resolveThemeMode, type ThemeMode, type ThemePreference } from './theme';

const Dashboard = lazy(() => import('./pages/Dashboard').then((module) => ({ default: module.Dashboard })));
const InstancesPage = lazy(() => import('./pages/InstancesPage').then((module) => ({ default: module.InstancesPage })));
const JobsPage = lazy(() => import('./pages/JobsPage').then((module) => ({ default: module.JobsPage })));
const JobTopologyPage = lazy(() => import('./pages/JobTopologyPage').then((module) => ({ default: module.JobTopologyPage })));
const WorkflowEditorPage = lazy(() => import('./pages/WorkflowsPage').then((module) => ({ default: module.WorkflowEditorPage })));
const WorkflowsPage = lazy(() => import('./pages/WorkflowsPage').then((module) => ({ default: module.WorkflowsPage })));
const LoginPage = lazy(() => import('./pages/LoginPage').then((module) => ({ default: module.LoginPage })));
const SuperAdminSetupPage = lazy(() => import('./pages/SuperAdminSetupPage').then((module) => ({ default: module.SuperAdminSetupPage })));
const AlertDeliveryPage = lazy(() => import('./pages/AlertDeliveryPage').then((module) => ({ default: module.AlertDeliveryPage })));
const NotificationCenterPage = lazy(() => import('./pages/NotificationCenterPage').then((module) => ({ default: module.NotificationCenterPage })));
const AuditLogsPage = lazy(() => import('./pages/AuditLogsPage').then((module) => ({ default: module.AuditLogsPage })));
const ScriptsPage = lazy(() => import('./pages/ScriptsPage').then((module) => ({ default: module.ScriptsPage })));
const ScriptEditorPage = lazy(() => import('./pages/ScriptsPage').then((module) => ({ default: module.ScriptEditorPage })));
const UsersPage = lazy(() => import('./pages/UsersPage').then((module) => ({ default: module.UsersPage })));
const RolesPage = lazy(() => import('./pages/RolesPage').then((module) => ({ default: module.RolesPage })));
const ScopesPage = lazy(() => import('./pages/ScopesPage').then((module) => ({ default: module.ScopesPage })));
const CalendarsPage = lazy(() => import('./pages/CalendarsPage').then((module) => ({ default: module.CalendarsPage })));
const PluginsPage = lazy(() => import('./pages/PluginsPage').then((module) => ({ default: module.PluginsPage })));
const ApiKeysPage = lazy(() => import('./pages/ApiKeysPage').then((module) => ({ default: module.ApiKeysPage })));
const GitOpsPage = lazy(() => import('./pages/GitOpsPage').then((module) => ({ default: module.GitOpsPage })));
const WorkersPage = lazy(() => import('./pages/WorkersPage').then((module) => ({ default: module.WorkersPage })));
const DispatchQueuePage = lazy(() => import('./pages/DispatchQueuePage').then((module) => ({ default: module.DispatchQueuePage })));

const KEEP_ALIVE_ROUTES = [
  { path: ROUTE_META.dashboard.path, element: <Dashboard /> },
  { path: ROUTE_META.jobs.path, element: <JobsPage /> },
  { path: ROUTE_META.instances.path, element: <InstancesPage /> },
  { path: ROUTE_META.workflows.path, element: <GuardedRoute route={ROUTE_META.workflows}><WorkflowsPage /></GuardedRoute> },
  { path: ROUTE_META.workers.path, element: <GuardedRoute route={ROUTE_META.workers}><WorkersPage /></GuardedRoute> },
  { path: ROUTE_META.dispatchQueue.path, element: <GuardedRoute route={ROUTE_META.dispatchQueue}><DispatchQueuePage /></GuardedRoute> },
  { path: ROUTE_META.users.path, element: <GuardedRoute route={ROUTE_META.users}><UsersPage /></GuardedRoute> },
  { path: ROUTE_META.roles.path, element: <GuardedRoute route={ROUTE_META.roles}><RolesPage /></GuardedRoute> },
  { path: ROUTE_META.scopes.path, element: <GuardedRoute route={ROUTE_META.scopes}><ScopesPage /></GuardedRoute> },
  { path: ROUTE_META.calendars.path, element: <GuardedRoute route={ROUTE_META.calendars}><CalendarsPage /></GuardedRoute> },
  { path: ROUTE_META.plugins.path, element: <GuardedRoute route={ROUTE_META.plugins}><PluginsPage /></GuardedRoute> },
  { path: ROUTE_META.apiKeys.path, element: <GuardedRoute route={ROUTE_META.apiKeys}><ApiKeysPage /></GuardedRoute> },
  { path: ROUTE_META.gitops.path, element: <GuardedRoute route={ROUTE_META.gitops}><GitOpsPage /></GuardedRoute> },
  { path: ROUTE_META.scripts.path, element: <GuardedRoute route={ROUTE_META.scripts}><ScriptsPage /></GuardedRoute> },
  { path: ROUTE_META.notifications.path, element: <GuardedRoute route={ROUTE_META.notifications}><NotificationCenterPage /></GuardedRoute> },
  { path: ROUTE_META.alerts.path, element: <GuardedRoute route={ROUTE_META.alerts}><AlertDeliveryPage /></GuardedRoute> },
  { path: ROUTE_META.audit.path, element: <GuardedRoute route={ROUTE_META.audit}><AuditLogsPage /></GuardedRoute> },
];

function GuardedRoute({ route, children }: { route: { permission?: { resource: string; action: string } }; children: React.ReactNode }) {
  if (!route.permission) return <>{children}</>;
  return <RequirePermission resource={route.permission.resource} action={route.permission.action}>{children}</RequirePermission>;
}

function LoginRoute({ bootstrap }: { bootstrap: BootstrapStatusResponse }) {
  if (bootstrap.registrationOpen) {
    return <Navigate to="/setup" replace />;
  }
  if (getAuthToken() !== null) {
    return <Navigate to={ROUTE_META.dashboard.path} replace />;
  }
  return <LoginPage />;
}

function SetupRoute({ bootstrap, onRegistered }: { bootstrap: BootstrapStatusResponse; onRegistered: () => void }) {
  if (!bootstrap.registrationOpen) {
    return <Navigate to={getAuthToken() ? ROUTE_META.dashboard.path : '/login'} replace />;
  }
  return <SuperAdminSetupPage onRegistered={onRegistered} />;
}

function AppLayout() {
  const navigate = useNavigate();

  const handleLogout = () => {
    void logout().catch(() => undefined);
    setAuthToken(null);
    navigate('/login', { replace: true });
  };

  setAuthErrorHandler({
    onUnauthorized: () => {
      setAuthToken(null);
      navigate('/login', { replace: true });
    },
    onForbidden: (message) => {
      navigate('/forbidden', { replace: true, state: { message } });
    },
  });

  return (
    <AppShell onLogout={handleLogout}>
      <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route path="/" element={<Navigate to={ROUTE_META.dashboard.path} replace />} />
          <Route path={ROUTE_META.dashboard.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.jobs.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.jobTopology.path} element={<JobTopologyPage />} />
          <Route path={ROUTE_META.instances.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.workflows.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.workflowNew.path} element={<GuardedRoute route={ROUTE_META.workflowNew}><WorkflowEditorPage /></GuardedRoute>} />
          <Route path={ROUTE_META.workflowEdit.path} element={<GuardedRoute route={ROUTE_META.workflowEdit}><WorkflowEditorPage /></GuardedRoute>} />
          <Route path={ROUTE_META.workers.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.dispatchQueue.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.users.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.roles.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.scopes.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.calendars.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.plugins.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.apiKeys.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.gitops.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.scripts.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.scriptEdit.path} element={<GuardedRoute route={ROUTE_META.scriptEdit}><ScriptEditorPage /></GuardedRoute>} />
          <Route path={ROUTE_META.notifications.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.alerts.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path={ROUTE_META.audit.path} element={<KeepAliveOutlet routes={KEEP_ALIVE_ROUTES} />} />
          <Route path="/forbidden" element={<ForbiddenPage />} />
        </Routes>
      </Suspense>
    </AppShell>
  );
}

/**
 * Tikeo web application root.
 *
 * The component owns global Ant Design theming, browser/system dark-mode resolution, bootstrap
 * registration routing, authentication guards, and keep-alive route composition. Keep data refresh
 * logic inside individual pages so cached routes stay responsive without remounting the whole app.
 */
export function App() {
  const { locale } = useI18n();
  const [primaryColor, setPrimaryColorState] = useState(() => {
    if (typeof window === 'undefined') return DEFAULT_PRIMARY_COLOR;
    return normalizeHexColor(window.localStorage.getItem(PRIMARY_COLOR_STORAGE_KEY)) ?? DEFAULT_PRIMARY_COLOR;
  });
  const [mode, setModeState] = useState<ThemePreference>(() => {
    if (typeof window === 'undefined') return 'system';
    return normalizeThemeMode(window.localStorage.getItem(THEME_MODE_STORAGE_KEY));
  });
  const [systemPrefersDark, setSystemPrefersDark] = useState(() => {
    if (typeof window === 'undefined') return false;
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  });
  const [bootstrap, setBootstrap] = useState<BootstrapStatusResponse | null>(null);
  const [bootstrapError, setBootstrapError] = useState<string | null>(null);

  const setPrimaryColor = (color: string) => {
    const normalized = normalizeHexColor(color) ?? DEFAULT_PRIMARY_COLOR;
    setPrimaryColorState(normalized);
    window.localStorage.setItem(PRIMARY_COLOR_STORAGE_KEY, normalized);
  };

  const resetPrimaryColor = () => {
    setPrimaryColorState(DEFAULT_PRIMARY_COLOR);
    window.localStorage.removeItem(PRIMARY_COLOR_STORAGE_KEY);
  };

  const setMode = (nextMode: ThemePreference) => {
    const normalized = normalizeThemeMode(nextMode);
    setModeState(normalized);
    window.localStorage.setItem(THEME_MODE_STORAGE_KEY, normalized);
  };

  const toggleMode = () => setMode(mode === 'system' ? resolvedMode : mode === 'dark' ? 'light' : 'dark');

  const resolvedMode: ThemeMode = resolveThemeMode(mode, systemPrefersDark);

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (event: MediaQueryListEvent) => setSystemPrefersDark(event.matches);
    setSystemPrefersDark(media.matches);
    media.addEventListener('change', handleChange);
    return () => media.removeEventListener('change', handleChange);
  }, []);

  useEffect(() => {
    let cancelled = false;
    getBootstrapStatus()
      .then((status) => {
        if (!cancelled) setBootstrap(status);
      })
      .catch((cause) => {
        if (!cancelled) setBootstrapError(cause instanceof Error ? cause.message : '初始化状态检查失败');
      });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    document.documentElement.style.setProperty('--app-primary-color', primaryColor);
    document.documentElement.style.setProperty('--app-info-color', DEFAULT_INFO_COLOR);
    document.documentElement.dataset.theme = resolvedMode;
    document.documentElement.dataset.themePreference = mode;
  }, [primaryColor, mode, resolvedMode]);

  const themeSettings = useMemo(() => ({ primaryColor, mode, resolvedMode, setPrimaryColor, resetPrimaryColor, setMode, toggleMode }), [primaryColor, mode, resolvedMode]);
  const refreshBootstrap = () => {
    setBootstrap({ initialized: true, registrationOpen: false, bootstrapAdminUsername: null });
  };

  return (
    <ThemeSettingsContext.Provider value={themeSettings}>
      <ConfigProvider
        locale={locale === 'en-US' ? antdEnUS : antdZhCN}
        theme={{
          algorithm: resolvedMode === 'dark' ? theme.darkAlgorithm : theme.defaultAlgorithm,
          token: {
            colorPrimary: primaryColor,
            colorInfo: DEFAULT_INFO_COLOR,
            colorBgBase: resolvedMode === 'dark' ? '#191919' : '#f6f8fc',
            colorTextBase: resolvedMode === 'dark' ? '#e2e8f0' : '#172033',
            borderRadius: 12,
            controlHeight: 36,
            controlHeightSM: 28,
            controlHeightLG: 44,
            fontFamily: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
          },
        }}
      >
        <Suspense fallback={<RouteFallback />}>
          {bootstrapError ? <div className="route-fallback">{bootstrapError}</div> : null}
          {!bootstrap && !bootstrapError ? <RouteFallback /> : null}
          {bootstrap ? (
            <Routes>
              <Route path="/" element={<Navigate to={bootstrap.registrationOpen ? '/setup' : ROUTE_META.dashboard.path} replace />} />
              <Route path="/setup" element={<SetupRoute bootstrap={bootstrap} onRegistered={refreshBootstrap} />} />
              <Route path="/login" element={<LoginRoute bootstrap={bootstrap} />} />
              <Route element={<AuthGuard />}>
                <Route path="/*" element={<AppLayout />} />
              </Route>
            </Routes>
          ) : null}
        </Suspense>
      </ConfigProvider>
    </ThemeSettingsContext.Provider>
  );
}
