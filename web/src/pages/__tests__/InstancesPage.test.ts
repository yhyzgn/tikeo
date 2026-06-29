import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = [
  readFileSync(new URL('../InstancesPage.tsx', import.meta.url), 'utf8'),
  readFileSync(new URL('../../components/logs/WorkerLogTerminal.tsx', import.meta.url), 'utf8'),
  readFileSync(new URL('../../components/logs/AnsiLogLine.tsx', import.meta.url), 'utf8'),
  readFileSync(new URL('../../components/logs/terminalLogs.css', import.meta.url), 'utf8'),
  readFileSync(new URL('../../components/logs/logTime.ts', import.meta.url), 'utf8'),
  readFileSync(new URL('../../styles.css', import.meta.url), 'utf8'),
].join('\n');

describe('instance log drawer executor visibility', () => {
  test('shows executor details for single instances and broadcast child attempts separately', () => {
    expect(source).toContain("selectedInstance?.executionMode === 'single' ? '执行器' : '广播子执行'");
    expect(source).toContain("selectedInstance?.executionMode === 'single' ? [{");
    expect(source).toContain('workerId: selectedInstance.workerId ?? selectedInstance.latestLog?.workerId');
    expect(source).toContain('status: selectedInstance.status');
    expect(source).toContain('updatedAt: selectedInstance.updatedAt');
    expect(source).toContain('dataSource={selectedInstance?.executionMode === \'single\' ?');
    expect(source).toContain("'暂无执行器信息' : '暂无广播子执行'");
  });

  test('loads attempts and logs together and keeps worker/status columns visible', () => {
    expect(source).toContain('listInstanceAttempts(instance.id)');
    expect(source).toContain('listInstanceLogs(instance.id)');
    expect(source).toContain("{ title: 'Worker', dataIndex: 'workerId'");
    expect(source).toContain("{ title: 'Status', dataIndex: 'status'");
    expect(source).toContain("title: 'Updated At'");
    expect(source).toContain("dataIndex: 'updatedAt'");
    expect(source).toContain('const workerLogGroups = groupLogsByWorker(logs);');
    expect(source).toContain('className="instance-log-terminal"');
    expect(source).toContain('查看日志');
  });
});


describe('instance list worker visibility and grouped logs', () => {

  test('centers status bubbles with inline-flex alignment', () => {
    expect(source).toContain('justify-content: center');
    expect(source).toContain('vertical-align: middle');
    expect(source).toContain('text-align: center');
  });

  test('centers all table headers and body cells globally', () => {
    expect(source).toContain('.ant-table-thead > tr > th');
    expect(source).toContain('.ant-table-tbody > tr > td');
    expect(source).toContain('text-align: center !important;');
    expect(source).toContain('.ant-table-column-sorters');
    expect(source).toContain('justify-content: center;');
  });

  test('renders instance ids as clickable copy targets', () => {
    expect(source).toContain('copyInstanceId(instance.id)');
    expect(source).toContain('className="instance-copy-id"');
    expect(source).toContain('title="点击复制实例 ID"');
    expect(source).toContain("navigator.clipboard.writeText(instanceId)");
    expect(source).toContain("message.success(t('实例 ID 已复制'))");
    expect(source).toContain('.instance-copy-id.ant-typography');
    expect(source).toContain('cursor: pointer;');
  });
  test('copies the original execution node id while displaying the abbreviated label', () => {
    expect(source).toContain('copyWorkerId(workerId)');
    expect(source).toContain('onCopyWorkerId(workerId)');
    expect(source).toContain('formatWorkerDisplayId(workerId)');
    expect(source).toContain('title="点击复制执行节点"');
    expect(source).toContain("message.success(t('执行节点已复制'))");
  });

  test('shows the assigned worker in the instance table', () => {
    expect(source).toContain("title: '执行节点', key: 'executionNodes'");
    expect(source).toContain('displayExecutionNodes(instance, attemptsByInstance.get(instance.id),');
    expect(source).toContain("instance.workerId ?? instance.latestLog?.workerId ?? '暂无 worker'");
  });

  test('widens the log drawer and groups execution logs by worker', () => {
    expect(source).toContain('width="60vw"');
    expect(source).toContain('const workerLogGroups = groupLogsByWorker(logs);');
    expect(source).toContain('groups.map((group) =>');
    expect(source).toContain("title={`${t('Worker')} ${group.workerId}`}");
    expect(source).toContain('group.logs.map((log) =>');
    expect(source).toContain('role="log"');
    expect(source).toContain('renderAnsiLogLine');
    expect(source).toContain('useI18n');
    expect(source).toContain("aria-label={`${t('Worker')} ${group.workerId} ${t('执行日志')}`}");
    expect(source).toContain('instance-log-terminal');
  });

  test('uses theme-aware terminal log classes and ansi rendering', () => {
    expect(source).toContain('className="instance-log-drawer"');
    expect(source).toContain('className="instance-log-terminal__line"');
    expect(source).toContain('renderAnsiLogLine(message)');
  });



  test('uses a narrower drawer and balanced instance execution-node columns', () => {
    expect(source).toContain('width="60vw"');
    expect(source).toContain('attemptsByInstance');
    expect(source).toContain('displayExecutionNodes(instance, attemptsByInstance.get(instance.id),');
    expect(source).toContain("title: 'Instance'");
    expect(source).toContain('width: 220');
    expect(source).toContain("title: '执行节点', key: 'executionNodes', width: 340");
    expect(source).toContain("{ title: 'Status', dataIndex: 'status', width: 110");
    expect(source).toContain("title: 'Updated At'");
    expect(source).toContain('width: 360');
    expect(source).toContain("title: 'Actions'");
    expect(source).toContain('width: 140');
    expect(source).toContain('scroll={{ x: 1_440 }}');
    expect(source).toContain('className="instance-log-attempt-time"');
    expect(source).toContain('white-space: nowrap;');
  });


  test('marks runtime log and result messages as i18n raw data', () => {
    expect(source).toContain('role="log" data-runtime-text');
    expect(source).toContain('className="instance-log-terminal__message" data-runtime-text');
    expect(source).toContain('className="instance-result-panel__message-body" data-runtime-text');
    expect(source).toContain('style={{ maxWidth: 188 }} data-runtime-text');
  });

  test('shows log timestamps and binds terminal highlight colors to theme tokens', () => {
    expect(source).toContain('formatLogTimestamp(log.createdAt)');
    expect(source).toContain("String(sequence).padStart(3, '0')");
    expect(source).toContain('formatLogSequence(log.sequence)');
    expect(source).toContain('formatIsoOffset');
    expect(source).toContain('grid-template-columns: var(--instance-log-seq-width, 6ch) max-content max-content minmax(0, 1fr)');
    expect(source).toContain('text-align: right');
    expect(source).toContain('className="instance-log-terminal__time"');
    expect(source).toContain('dateTime={log.createdAt}');
    expect(source).toContain('--terminal-highlight-bg');
    expect(source).toContain('--terminal-time');
    expect(source).toContain('html[data-theme="dark"]');
    expect(source).toContain('--terminal-bright-blue:');
    expect(source).toContain('--terminal-bg: #f8fafc;');
    expect(source).toContain('--terminal-bg: #18181b;');
  });
});


describe('instance execution result view', () => {
  test('shows concrete execution result and refreshes instance details with logs', () => {
    expect(source).toContain('getInstance(instance.id)');
    expect(source).toContain('new EventSource(instanceLogStreamUrl(selectedInstance.id))');
    expect(source).toContain("source.addEventListener('instance.log'");
    expect(source).toContain('执行结果');
    expect(source).toContain('buildExecutionResultNodes(instance, attempts, logs)');
    expect(source).toContain('节点执行结果');
    expect(source).toContain('单节点结果');
    expect(source).toContain('广播节点结果');
    expect(source).toContain('instance-result-nodes__list');
    expect(source).toContain('instance-result-nodes__meta-row');
    expect(source).toContain('.instance-result-nodes__node-head .ant-typography code');
    expect(source).toContain('border-radius: 6px;');
    expect(source).toContain('overflow: visible;');
    expect(source).toContain('max-width: none;');
    expect(source).toContain('instance-result-nodes__message');
    expect(source).toContain('node.result?.message');
    expect(source).not.toContain('renderBroadcastResults(instance, attempts, logs)');
    expect(source).not.toContain('暂无执行结果');
  });
});


describe('instance list realtime refresh', () => {
  test('subscribes to instance list SSE and only uses REST as an unhealthy-stream fallback', () => {
    expect(source).toContain('instanceListStreamUrl');
    expect(source).toContain('new EventSource(instanceListStreamUrl({ pageSize, pageToken }))');
    expect(source).toContain("source.addEventListener('instances.snapshot'");
    expect(source).toContain('streamHealthyRef.current = true;');
    expect(source).toContain('setInstances(snapshot.instances);');
    expect(source).toContain('if (snapshot.attempts)');
    expect(source).toContain('INSTANCE_LIST_STREAM_WATCHDOG_MS');
    expect(source).toContain('INSTANCE_LIST_FALLBACK_INTERVAL_MS');
    expect(source).toContain('if (!streamHealthyRef.current)');
    expect(source).toContain('window.clearTimeout(watchdogTimer);');
    expect(source).toContain('window.clearInterval(fallbackTimer);');
  });

  test('does not eagerly fetch every instance attempts page during list loading', () => {
    expect(source).toContain('listInstanceAttempts(instance.id)');
    expect(source).toContain('const [logPage, attemptPage, freshInstance] = await Promise.all([');
    expect(source).not.toContain('sortedInstances.map(async (instance)');
    expect(source).not.toContain('attemptPairs');
  });

  test('loads only the current server-side instance page', () => {
    expect(source).toContain('listInstances({ pageSize, pageToken })');
    expect(source).toContain('const pageToken = useMemo(() => (tablePage > 1 ? String((tablePage - 1) * pageSize) : null)');
    expect(source).toContain('setTotalInstances(snapshot.totalCount ?? snapshot.instances.length);');
    expect(source).toContain('total: hasInstanceFilters(filters) ? filteredInstances.length : totalInstances');
    expect(source).toContain('dataSource={filteredInstances}');
    expect(source).not.toContain('jobPage.items.map((job) => listJobInstances(job.id))');
  });
});


describe('instance list filters', () => {
  test('supports URL-driven semantic filters and filtered table data', () => {
    expect(source).toContain('useSearchParams');
    expect(source).toContain('filtersFromSearchParams(searchParams)');
    expect(source).toContain('instanceMatchesFilters(instance, jobName, filters)');
    expect(source).toContain('semanticFilterLabel(filters)');
    expect(source).toContain('instance-filter-panel');
    expect(source).toContain('状态');
    expect(source).toContain('任务');
    expect(source).toContain('触发方式');
    expect(source).toContain('执行模式');
    expect(source).toContain('Worker');
    expect(source).toContain('实例 / 日志关键字');
    expect(source).toContain('dataSource={filteredInstances}');
    expect(source).toContain('没有匹配当前过滤条件的实例');
  });
});
