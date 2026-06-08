import { useCallback, useEffect, useState } from 'react';
import { Button, Col, Row, message } from 'antd';

import { getWorkerLifecycleHistory, listWorkers, type WorkerLifecycleHistoryResponse, type WorkerListResponse } from '../api/client';
import { WorkerClusterOverview } from './workers/WorkerClusterOverview';
import { WorkerTable } from './workers/WorkerTable';
import { WorkerLifecycleHistory } from './workers/WorkerLifecycleHistory';
import { ROUTE_META } from '../routes';
import { useRouteActive } from '../hooks/useRouteActivation';
import { useNavigate } from 'react-router-dom';
import { useI18n } from '../i18n/I18nContext';

const WORKER_REFRESH_INTERVAL_MS = 3_000;

export function WorkersPage() {
  const [workers, setWorkers] = useState<WorkerListResponse>({ online: 0, items: [] });
  const [history, setHistory] = useState<WorkerLifecycleHistoryResponse>({ sessions: [], events: [] });
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const { locale } = useI18n();
  const isEnglish = locale === 'en-US';
  const active = useRouteActive(ROUTE_META.workers.path);

  const refresh = useCallback(async (options?: { silent?: boolean }) => {
    if (!options?.silent) {
      setLoading(true);
    }
    try {
      const workerData = await listWorkers();
      setWorkers(workerData);
      const historyData = await getWorkerLifecycleHistory();
      setHistory(historyData);
    } catch (error) {
      if (!options?.silent) {
        message.error(error instanceof Error ? error.message : (isEnglish ? 'Failed to load Worker status' : '加载 Worker 状态失败'));
      }
    } finally {
      if (!options?.silent) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => { if (active) void refresh(); }, [active, refresh]);
  useEffect(() => {
    if (!active) return undefined;
    const interval = window.setInterval(() => {
      void refresh({ silent: true });
    }, WORKER_REFRESH_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [active, refresh]);

  return (
    <div className="page-stack worker-cluster-page">
      <WorkerClusterOverview workers={workers} loading={loading} onRefresh={refresh} extraAction={<Button onClick={() => navigate(ROUTE_META.dispatchQueue.path)}>{isEnglish ? 'View dispatch queue' : '查看调度队列'}</Button>} />
      <Row gutter={[18, 18]} align="stretch">
        <Col xs={24}><WorkerTable workers={workers} loading={loading} /></Col>
        <Col xs={24}><WorkerLifecycleHistory history={history} loading={loading} /></Col>
      </Row>
    </div>
  );
}
