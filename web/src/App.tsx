import { ConfigProvider, theme } from 'antd';
import { lazy, Suspense } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';

import { getAuthToken, logout, setAuthErrorHandler, setAuthToken } from './api/client';
import { AppShell } from './components/AppShell';
import { AuthGuard, RequirePermission } from './components/AuthGuard';
import { ForbiddenPage } from './components/ForbiddenPage';
import { RouteFallback } from './components/RouteFallback';
import { ROUTE_META } from './routes';

const Dashboard = lazy(() => import('./pages/Dashboard').then((module) => ({ default: module.Dashboard })));
const InstancesPage = lazy(() => import('./pages/InstancesPage').then((module) => ({ default: module.InstancesPage })));
const JobsPage = lazy(() => import('./pages/JobsPage').then((module) => ({ default: module.JobsPage })));
const WorkflowEditorPage = lazy(() => import('./pages/WorkflowsPage').then((module) => ({ default: module.WorkflowEditorPage })));
const WorkflowsPage = lazy(() => import('./pages/WorkflowsPage').then((module) => ({ default: module.WorkflowsPage })));
const LoginPage = lazy(() => import('./pages/LoginPage').then((module) => ({ default: module.LoginPage })));
const AuditLogsPage = lazy(() => import('./pages/AuditLogsPage').then((module) => ({ default: module.AuditLogsPage })));
const ScriptsPage = lazy(() => import('./pages/ScriptsPage').then((module) => ({ default: module.ScriptsPage })));
const ScriptEditorPage = lazy(() => import('./pages/ScriptsPage').then((module) => ({ default: module.ScriptEditorPage })));
const UsersPage = lazy(() => import('./pages/UsersPage').then((module) => ({ default: module.UsersPage })));
const ScopesPage = lazy(() => import('./pages/ScopesPage').then((module) => ({ default: module.ScopesPage })));
const WorkersPage = lazy(() => import('./pages/WorkersPage').then((module) => ({ default: module.WorkersPage })));

function GuardedRoute({ route, children }: { route: { permission?: { resource: string; action: string } }; children: React.ReactNode }) {
  if (!route.permission) return <>{children}</>;
  return <RequirePermission resource={route.permission.resource} action={route.permission.action}>{children}</RequirePermission>;
}

function LoginRoute() {
  if (getAuthToken() !== null) {
    return <Navigate to={ROUTE_META.dashboard.path} replace />;
  }
  return <LoginPage />;
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
          <Route path={ROUTE_META.dashboard.path} element={<Dashboard />} />
          <Route path={ROUTE_META.jobs.path} element={<JobsPage />} />
          <Route path={ROUTE_META.instances.path} element={<InstancesPage />} />
          <Route path={ROUTE_META.workflows.path} element={<GuardedRoute route={ROUTE_META.workflows}><WorkflowsPage /></GuardedRoute>} />
          <Route path={ROUTE_META.workflowNew.path} element={<GuardedRoute route={ROUTE_META.workflowNew}><WorkflowEditorPage /></GuardedRoute>} />
          <Route path={ROUTE_META.workflowEdit.path} element={<GuardedRoute route={ROUTE_META.workflowEdit}><WorkflowEditorPage /></GuardedRoute>} />
          <Route path={ROUTE_META.workers.path} element={<GuardedRoute route={ROUTE_META.workers}><WorkersPage /></GuardedRoute>} />
          <Route path={ROUTE_META.users.path} element={<GuardedRoute route={ROUTE_META.users}><UsersPage /></GuardedRoute>} />
          <Route path={ROUTE_META.scopes.path} element={<GuardedRoute route={ROUTE_META.scopes}><ScopesPage /></GuardedRoute>} />
          <Route path={ROUTE_META.scripts.path} element={<GuardedRoute route={ROUTE_META.scripts}><ScriptsPage /></GuardedRoute>} />
          <Route path={ROUTE_META.scriptEdit.path} element={<GuardedRoute route={ROUTE_META.scriptEdit}><ScriptEditorPage /></GuardedRoute>} />
          <Route path={ROUTE_META.audit.path} element={<GuardedRoute route={ROUTE_META.audit}><AuditLogsPage /></GuardedRoute>} />
          <Route path="/forbidden" element={<ForbiddenPage />} />
        </Routes>
      </Suspense>
    </AppShell>
  );
}

export function App() {
  return (
    <ConfigProvider
      theme={{
        algorithm: theme.defaultAlgorithm,
        token: {
          colorPrimary: '#2563eb',
          colorInfo: '#0ea5e9',
          colorBgBase: '#f6f8fc',
          colorTextBase: '#172033',
          borderRadius: 12,
          fontFamily: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        },
      }}
    >
      <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route path="/" element={<Navigate to={ROUTE_META.dashboard.path} replace />} />
          <Route path="/login" element={<LoginRoute />} />
          <Route element={<AuthGuard />}>
            <Route element={<AppLayout />}>
              <Route index element={<Navigate to={ROUTE_META.dashboard.path} replace />} />
              <Route path="*" element={<Navigate to={ROUTE_META.dashboard.path} replace />} />
            </Route>
          </Route>
        </Routes>
      </Suspense>
    </ConfigProvider>
  );
}
