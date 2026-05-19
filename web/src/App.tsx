import { Alert, ConfigProvider, theme } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { listJobInstances, listJobs, type JobInstanceSummary, type JobSummary } from './api/client';
import { AppShell } from './components/AppShell';
import { Dashboard } from './pages/Dashboard';
import { InstancesPage } from './pages/InstancesPage';
import { JobsPage } from './pages/JobsPage';

export function App() {
  const [activePage, setActivePage] = useState('dashboard');
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [instances, setInstances] = useState<JobInstanceSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
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
    return <Dashboard jobs={jobs} instances={instances} />;
  }, [activePage, instances, jobs, loading, refresh]);

  return (
    <ConfigProvider theme={{ algorithm: theme.defaultAlgorithm }}>
      <AppShell activeKey={activePage} onNavigate={setActivePage}>
        {error ? <Alert type="error" showIcon message="API 调用失败" description={error} /> : null}
        {page}
      </AppShell>
    </ConfigProvider>
  );
}
