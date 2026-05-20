import { ConfigProvider, theme } from 'antd';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';

import { logout, setAuthToken } from './api/client';
import { AppShell } from './components/AppShell';
import { AuthGuard, RequirePermission } from './components/AuthGuard';
import { Dashboard } from './pages/Dashboard';
import { InstancesPage } from './pages/InstancesPage';
import { JobsPage } from './pages/JobsPage';
import { WorkflowsPage } from './pages/WorkflowsPage';
import { LoginPage } from './pages/LoginPage';
import { AuditLogsPage } from './pages/AuditLogsPage';
import { ScriptsPage } from './pages/ScriptsPage';
import { UsersPage } from './pages/UsersPage';
import { WorkersPage } from './pages/WorkersPage';
import { Result, Button } from 'antd';

function AppLayout() {
  const navigate = useNavigate();

  const handleLogout = () => {
    void logout().catch(() => undefined);
    setAuthToken(null);
    navigate('/login', { replace: true });
  };

  return (
    <AppShell onLogout={handleLogout}>
      <Routes>
        <Route path="/dashboard" element={<Dashboard />} />
        <Route path="/jobs" element={<JobsPage />} />
        <Route path="/instances" element={<InstancesPage />} />
        <Route path="/workflows" element={<RequirePermission resource="workflows" action="read"><WorkflowsPage /></RequirePermission>} />
        <Route path="/workers" element={<RequirePermission resource="workers" action="read"><WorkersPage /></RequirePermission>} />
        <Route path="/users" element={<RequirePermission resource="users" action="read"><UsersPage /></RequirePermission>} />
        <Route path="/scripts" element={<RequirePermission resource="scripts" action="read"><ScriptsPage /></RequirePermission>} />
        <Route path="/audit" element={<RequirePermission resource="audit" action="read"><AuditLogsPage /></RequirePermission>} />
        <Route path="/forbidden" element={<ForbiddenPage />} />
      </Routes>
    </AppShell>
  );
}

function ForbiddenPage() {
  const navigate = useNavigate();
  return (
    <Result
      status="403"
      title="403"
      subTitle="当前账号没有访问该功能的权限"
      extra={<Button type="primary" onClick={() => navigate('/dashboard', { replace: true })}>返回总览</Button>}
    />
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
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route element={<AuthGuard />}>
          <Route element={<AppLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            <Route path="*" element={<Navigate to="/dashboard" replace />} />
          </Route>
        </Route>
      </Routes>
    </ConfigProvider>
  );
}
