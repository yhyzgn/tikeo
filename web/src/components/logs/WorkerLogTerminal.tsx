import { Card, Space, Tag } from 'antd';

import type { JobInstanceLogSummary } from '../../api/client';
import { renderAnsiLogLine } from './AnsiLogLine';
import { formatLogTimestamp } from './logTime';

type WorkerLogGroup = {
  workerId: string;
  logs: JobInstanceLogSummary[];
};

export const groupLogsByWorker = (logs: JobInstanceLogSummary[]): WorkerLogGroup[] => {
  const groups = new Map<string, JobInstanceLogSummary[]>();
  for (const log of logs) {
    const workerId = log.workerId || '未知 worker';
    const workerLogs = groups.get(workerId) ?? [];
    workerLogs.push(log);
    groups.set(workerId, workerLogs);
  }
  return [...groups.entries()].map(([workerId, workerLogs]) => ({ workerId, logs: workerLogs }));
};


const renderLogMessage = (log: JobInstanceLogSummary) => {
  const message = log.governanceEvent === 'script_execution_governance'
    ? (log.governanceMessage ?? log.message)
    : log.message;
  return renderAnsiLogLine(message);
};

type WorkerLogTerminalProps = {
  groups: WorkerLogGroup[];
};

export function WorkerLogTerminal({ groups }: WorkerLogTerminalProps) {
  return (
    <Space direction="vertical" size={14} style={{ width: '100%' }}>
      {groups.map((group) => (
        <Card
          key={group.workerId}
          size="small"
          className="instance-log-terminal-card"
          title={`Worker ${group.workerId}`}
          extra={<Tag color="blue">{group.logs.length} 条日志</Tag>}
        >
          <div className="instance-log-terminal" role="log" aria-label={`Worker ${group.workerId} execution logs`}>
            {group.logs.map((log) => (
              <div key={log.id} className="instance-log-terminal__line">
                <span className="instance-log-terminal__seq">#{log.sequence}</span>
                <time className="instance-log-terminal__time" dateTime={log.createdAt}>{formatLogTimestamp(log.createdAt)}</time>
                <span className={`instance-log-terminal__level instance-log-terminal__level--${log.level}`}>{log.level}</span>
                <span className="instance-log-terminal__message">{renderLogMessage(log)}</span>
              </div>
            ))}
          </div>
        </Card>
      ))}
    </Space>
  );
}
