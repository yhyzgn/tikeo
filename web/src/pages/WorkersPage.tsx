import { useEffect, useState } from 'react';
import { Button, Card, Col, List, Row, Space, Statistic, Tag, Typography, message } from 'antd';
import { getDispatchQueue, listWorkers, type QueueOverview, type WorkerListResponse } from '../api/client';

export function WorkersPage() {
  const [workers, setWorkers] = useState<WorkerListResponse>({ online: 0, items: [] });
  const [queue, setQueue] = useState<QueueOverview>({ pending: 0, running: 0, done: 0, failed: 0, items: [] });
  const [loading, setLoading] = useState(false);

  const refresh = async () => {
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
  };

  useEffect(() => { void refresh(); }, []);

  return (
    <Space direction="vertical" size={18} style={{ width: '100%' }}>
      <div className="hero-panel">
        <div className="hero-panel__content">
          <Tag className="soft-tag" color="blue">Phase 2 · Worker Mesh</Tag>
          <Typography.Title level={1}>Worker 集群</Typography.Title>
          <Typography.Paragraph className="hero-panel__desc">
            展示通过反向隧道注册的在线 Worker，以及 workflow/job dispatch queue 的实时积压状态。
          </Typography.Paragraph>
        </div>
        <div className="hero-panel__summary"><strong>{workers.online}</strong><span>online</span></div>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={12} md={6}><Card><Statistic title="Pending" value={queue.pending} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title="Running" value={queue.running} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title="Done" value={queue.done} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title="Failed" value={queue.failed} /></Card></Col>
      </Row>

      <Row gutter={[18, 18]}>
        <Col xs={24} lg={12}>
          <Card title="在线 Worker" extra={<Button loading={loading} onClick={refresh}>刷新</Button>}>
            <List
              dataSource={workers.items}
              locale={{ emptyText: '暂无在线 Worker' }}
              renderItem={(worker) => (
                <List.Item>
                  <List.Item.Meta
                    title={<Space><span>{worker.worker_id}</span><Tag color="blue">{worker.namespace}/{worker.app}</Tag></Space>}
                    description={<Space wrap><span>{worker.cluster} · {worker.region}</span>{worker.capabilities.map((capability) => <Tag key={capability}>{capability}</Tag>)}<span>seq={worker.last_sequence}</span></Space>}
                  />
                </List.Item>
              )}
            />
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="Dispatch Queue">
            <List
              dataSource={queue.items}
              locale={{ emptyText: '暂无队列项' }}
              renderItem={(item) => (
                <List.Item>
                  <List.Item.Meta
                    title={<Space><span>{item.id}</span><Tag color={item.status === 'pending' ? 'gold' : item.status === 'running' ? 'processing' : 'default'}>{item.status}</Tag></Space>}
                    description={<span>job={item.job_instance_id ?? '-'} · workflow_node={item.workflow_node_instance_id ?? '-'} · attempt={item.attempt}</span>}
                  />
                </List.Item>
              )}
            />
          </Card>
        </Col>
      </Row>
    </Space>
  );
}
