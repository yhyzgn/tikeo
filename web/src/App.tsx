import { Alert, ConfigProvider, theme } from 'antd';
import { useCallback } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';

import { logout, setAuthToken } from './api/client';
import { AppShell } from './components/AppShell';
import { AuthGuard, RequireAdmin } from './components/AuthGuard';
import { Dashboard } from './pages/Dashboard';
import { InstancesPage } from './pages/InstancesPage';
import { JobsPage } from './pages/JobsPage';
import { LoginPage } from './pages/LoginPage';
import { AuditLogsPage } from './pages/AuditLogsPage';
import { ScriptsPage } from './pages/ScriptsPage';
import { UsersPage } from './pages/UsersPage';

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
        <Route path="/users" element={<RequireAdmin><UsersPage /></RequireAdmin>} />
        <Route path="/scripts" element={<RequireAdmin><ScriptsPage /></RequireAdmin>} />
        <Route path="/audit" element={<RequireAdmin><AuditLogsPage /></RequireAdmin>} />
      </Routes>
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
