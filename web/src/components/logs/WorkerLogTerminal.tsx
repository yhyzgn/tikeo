import { Card, Space, Tag } from 'antd';

import type { JobInstanceLogSummary } from '../../api/client';
import { renderAnsiLogLine } from './AnsiLogLine';
import { useI18n, type LocaleCode } from '../../i18n/I18nContext';
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


const formatLogSequence = (sequence: number) => `#${String(sequence).padStart(3, '0')}`;

const formatLogCount = (count: number, locale: LocaleCode) => locale === 'en-US' ? `${count} logs` : `${count} 条日志`;

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
  const { locale, t } = useI18n();
  return (
    <Space direction="vertical" size={14} style={{ width: '100%' }}>
      {groups.map((group) => (
        <Card
          key={group.workerId}
          size="small"
          className="instance-log-terminal-card"
          title={`${t('Worker')} ${group.workerId}`}
          extra={<Tag color="blue">{formatLogCount(group.logs.length, locale)}</Tag>}
        >
          <div className="instance-log-terminal" role="log" aria-label={`${t('Worker')} ${group.workerId} ${t('执行日志')}`}>
            {group.logs.map((log) => (
              <div key={log.id} className="instance-log-terminal__line">
                <span className="instance-log-terminal__seq">{formatLogSequence(log.sequence)}</span>
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
