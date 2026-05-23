import { useCallback, useEffect, useState } from 'react';
import { Col, Row, message } from 'antd';

import { getDispatchQueue, listWorkers, type QueueOverview, type WorkerListResponse } from '../api/client';
import { DispatchQueuePanel } from './workers/DispatchQueuePanel';
import { WorkerClusterOverview, WorkerQueueStats } from './workers/WorkerClusterOverview';
import { WorkerTable } from './workers/WorkerTable';

export function WorkersPage() {
  const [workers, setWorkers] = useState<WorkerListResponse>({ online: 0, items: [] });
  const [queue, setQueue] = useState<QueueOverview>({ pending: 0, running: 0, done: 0, failed: 0, items: [] });
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [workerData, queueData] = await Promise.all([listWorkers(), getDispatchQueue()]);
      setWorkers(workerData);
      setQueue(queueData);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '加载 Worker 状态失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  return (
    <div className="page-stack worker-cluster-page">
      <WorkerClusterOverview workers={workers} queue={queue} loading={loading} onRefresh={refresh} />
      <WorkerQueueStats queue={queue} />
      <Row gutter={[18, 18]} align="stretch">
        <Col xs={24} xl={15}><WorkerTable workers={workers} loading={loading} /></Col>
        <Col xs={24} xl={9}><DispatchQueuePanel queue={queue} loading={loading} /></Col>
      </Row>
    </div>
  );
}
