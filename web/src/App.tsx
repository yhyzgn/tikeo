import { Alert, ConfigProvider, theme } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  getAuthToken,
  listJobInstances,
  listJobs,
  logout,
  me,
  setAuthToken,
  type AuthSession,
  type JobInstanceSummary,
  type JobSummary,
  type MeResponse,
} from './api/client';
import { AppShell } from './components/AppShell';
import { Dashboard } from './pages/Dashboard';
import { InstancesPage } from './pages/InstancesPage';
import { JobsPage } from './pages/JobsPage';
import { LoginPage } from './pages/LoginPage';
import { ScriptsPage } from './pages/ScriptsPage';
import { UsersPage } from './pages/UsersPage';

export function App() {
  const [activePage, setActivePage] = useState('dashboard');
  const [principal, setPrincipal] = useState<MeResponse | null>(null);
  const [bootstrapping, setBootstrapping] = useState(() => getAuthToken() !== null);
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (principal === null) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const jobPage = await listJobs();
      setJobs(jobPage.items);
      const instancePages = await Promise.all(jobPage.items.map((job) => listJobInstances(job.id)));
      setInstances(instancePages.flatMap((page) => page.items));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '加载失败');
    } finally {
      setLoading(false);
    }
  }, [principal]);

  useEffect(() => {
    if (getAuthToken() === null) {
      setBootstrapping(false);
      return;
    }
    let cancelled = false;
    me()
      .then((current) => {
        if (!cancelled) {
          setPrincipal(current);
        }
      })
      .catch(() => {
        setAuthToken(null);
      })
      .finally(() => {
        if (!cancelled) {
          setBootstrapping(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const page = useMemo(() => {
    if (activePage === 'jobs') {
      return <JobsPage jobs={jobs} loading={loading} onRefresh={refresh} onTriggered={refresh} />;
    }
    if (activePage === 'instances') {
      return <InstancesPage jobs={jobs} instances={instances} />;
    }
    if (activePage === 'users') {
      return <UsersPage />;
    }
    if (activePage === 'scripts') {
      return <ScriptsPage />;
    }
    return <Dashboard jobs={jobs} instances={instances} />;
  }, [activePage, instances, jobs, loading, refresh]);

  const handleAuthenticated = (session: AuthSession) => {
    setPrincipal({ username: session.username, roles: session.roles });
  };

  const handleLogout = () => {
    void logout().catch(() => undefined);
    setAuthToken(null);
    setPrincipal(null);
    setJobs([]);
    setInstances([]);
    setActivePage('dashboard');
  };

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
      {bootstrapping ? (
        <div className="login-page">
          <Alert type="info" showIcon message="正在恢复会话" />
        </div>
      ) : principal === null ? (
        <LoginPage onAuthenticated={handleAuthenticated} />
      ) : (
        <AppShell activeKey={activePage} username={principal.username} roles={principal.roles} onNavigate={setActivePage} onLogout={handleLogout}>
          {error ? <Alert type="error" showIcon message="API 调用失败" description={error} /> : null}
          {page}
        </AppShell>
      )}
    </ConfigProvider>
  );
}
