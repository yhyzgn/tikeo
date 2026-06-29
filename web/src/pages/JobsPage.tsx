import { Alert, Button, Card, DatePicker, Drawer, Form, Input, InputNumber, Popconfirm, Select, Space, Switch, Table, Tag, Timeline, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';

import { createJob, deleteJob, getJobSchedulingAdvice, listAppScopes, listCalendars, listJobs, listJobVersions, listNamespaces, listPlugins, listScripts, listWorkerPools, listWorkers, rollbackJob, triggerJob, updateJob, workerStreamUrl, type AppScopeSummary, type BroadcastSelectorRequest, type CalendarSummary, type CreateJobRequest, type JobCanaryPolicy, type JobSchedulingAdvice, type JobSummary, type JobRetryPolicy, type NamespaceSummary, type PluginSummary, type WorkerPoolSummary, type JobVersionSummary, type ScriptSummary, type UpdateJobRequest, type WorkerListResponse, type WorkerSummary } from '../api/client';
import { PermissionGate, useCan } from '../components/Permission';
import { ROUTE_META } from '../routes';
import { useRouteActive } from '../hooks/useRouteActivation';
import { useUrlQueryState } from '../hooks/useUrlQueryState';
import { TABLE_PAGE_SIZE_OPTIONS, usePersistentTablePageSize } from '../utils/pagination';
import { JobNotificationConfigDrawer } from './notifications/JobNotificationConfigDrawer';
import { durationExpr, parseFixedRate } from './jobs/jobSchedule';

const DEFAULT_RETRY_POLICY: JobRetryPolicy = {
  enabled: true,
  maxAttempts: 3,
  initialDelaySeconds: 5,
  backoffMultiplier: 2,
  maxDelaySeconds: 60,
};

const DEFAULT_CANARY_POLICY: JobCanaryPolicy = {
  metricsGateEnabled: false,
  minimumSamples: 5,
  evaluationWindow: 20,
  maxFailureRate: 0.5,
  autoRollback: true,
};

const canaryPolicyValue = (policy?: JobCanaryPolicy | null): JobCanaryPolicy => ({
  ...DEFAULT_CANARY_POLICY,
  ...(policy ?? {}),
});

const retryPolicyValue = (policy?: JobRetryPolicy | null): JobRetryPolicy => ({
  ...DEFAULT_RETRY_POLICY,
  ...(policy ?? {}),
});

type JobFormValues = Omit<CreateJobRequest & UpdateJobRequest, 'scheduleStartAt' | 'scheduleEndAt'> & {
  executorKind?: 'sdk' | 'script' | 'plugin';
  fixedRateValue?: number;
  fixedRateUnit?: string;
  fixedRateJitterValue?: number;
  fixedRateJitterUnit?: string;
  scheduleCalendarRef?: string | null;
  scheduleStartAt?: unknown;
  scheduleEndAt?: unknown;
};

type WorkerStreamSnapshot = {
  workers: WorkerListResponse;
};

export function JobsPage() {
  const navigate = useNavigate();
  const canWriteJobs = useCan('jobs', 'write');
  const canExecuteInstances = useCan('instances', 'execute');
  const active = useRouteActive(ROUTE_META.jobs.path);
  const [pageSize, setPageSize] = usePersistentTablePageSize();
  const queryDefaults = useMemo(() => ({ page: 1, page_size: pageSize, keyword: '', scheduleType: '' }), [pageSize]);
  const { query, setQuery } = useUrlQueryState(queryDefaults);
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [scripts, setScripts] = useState<ScriptSummary[]>([]);
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [calendars, setCalendars] = useState<CalendarSummary[]>([]);
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [workerPools, setWorkerPools] = useState<WorkerPoolSummary[]>([]);
  const [workers, setWorkers] = useState<WorkerSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<JobFormValues>();
  const [editForm] = Form.useForm<JobFormValues>();
  const [broadcastForm] = Form.useForm<{ tags?: string[]; region?: string; cluster?: string; labelsText?: string }>();
  const createNamespace = Form.useWatch('namespace', form);
  const createApp = Form.useWatch('app', form);
  const createWorkerPool = Form.useWatch('workerPool', form);
  const editNamespace = Form.useWatch('namespace', editForm);
  const editApp = Form.useWatch('app', editForm);
  const editWorkerPool = Form.useWatch('workerPool', editForm);
  const [createDrawerOpen, setCreateDrawerOpen] = useState(false);
  const [editingJob, setEditingJob] = useState<JobSummary | null>(null);
  const [versionJob, setVersionJob] = useState<JobSummary | null>(null);
  const [jobVersions, setJobVersions] = useState<JobVersionSummary[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);
  const [adviceJob, setAdviceJob] = useState<JobSummary | null>(null);
  const [notificationJob, setNotificationJob] = useState<JobSummary | null>(null);
  const [schedulingAdvice, setSchedulingAdvice] = useState<JobSchedulingAdvice | null>(null);
  const [adviceLoading, setAdviceLoading] = useState(false);
  const [broadcastJob, setBroadcastJob] = useState<JobSummary | null>(null);

  const scheduleTypeOptions = [
    { value: 'api', label: 'API 手动触发' },
    { value: 'cron', label: 'Cron 定时' },
    { value: 'fixed_rate', label: '固定频率' },
    { value: 'fixed_delay', label: '固定延迟' },
    { value: 'once', label: '一次性未来任务' },
    { value: 'daily_time_interval', label: 'Daily Time Interval' },
  ];
  const scheduleFilterOptions = scheduleTypeOptions.map((option) => ({ value: option.value, label: option.label }));

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [page, scriptPage, workerPage, pluginItems, calendarItems, namespaceItems, appItems, workerPoolItems] = await Promise.all([
        listJobs(),
        listScripts(),
        listWorkers().catch(() => ({ online: 0, items: [] })),
        listPlugins().catch(() => []),
        listCalendars().catch(() => []),
        listNamespaces(),
        listAppScopes(),
        listWorkerPools().catch(() => []),
      ]);
      setJobs(page.items);
      setScripts(scriptPage.items);
      setWorkers(workerPage.items);
      setPlugins(pluginItems);
      setCalendars(calendarItems);
      setNamespaces(namespaceItems);
      setApps(appItems);
      setWorkerPools(workerPoolItems);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { if (active) void load(); }, [active, load]);

  useEffect(() => {
    if (!active) return undefined;
    const source = new EventSource(workerStreamUrl());
    source.addEventListener('workers.snapshot', (event) => {
      try {
        const snapshot = JSON.parse((event as MessageEvent).data) as WorkerStreamSnapshot;
        setWorkers(snapshot.workers.items);
      } catch {
        // Ignore malformed stream frames; initial load/manual refresh keep processor choices usable.
      }
    });
    return () => source.close();
  }, [active]);

  const namespaceOptions = useMemo(() => namespaces
    .map((namespace) => ({ value: namespace.name, label: namespace.name }))
    .sort((left, right) => left.label.localeCompare(right.label)), [namespaces]);
  const appOptionsForNamespace = useCallback((namespace?: string | null) => {
    const normalizedNamespace = namespace?.trim();
    return apps
      .filter((app) => !normalizedNamespace || app.namespace === normalizedNamespace)
      .map((app) => ({ value: app.name, label: app.name }))
      .sort((left, right) => left.label.localeCompare(right.label));
  }, [apps]);
  const canaryJobOptionsForScope = useCallback((namespace?: string | null, app?: string | null, excludeJobId?: string | null) => {
    const normalizedNamespace = namespace?.trim();
    const normalizedApp = app?.trim();
    return jobs
      .filter((job) => job.id !== excludeJobId)
      .filter((job) => !normalizedNamespace || job.namespace === normalizedNamespace)
      .filter((job) => !normalizedApp || job.app === normalizedApp)
      .map((job) => ({ value: job.id, label: `${job.namespace}/${job.app}${job.workerPool ? `/${job.workerPool}` : ''} · ${job.name} · ${job.id}` }))
      .sort((left, right) => left.label.localeCompare(right.label));
  }, [jobs]);
  const applyNamespaceSelection = (targetForm: typeof form | typeof editForm, namespace?: string | null) => {
    const options = appOptionsForNamespace(namespace);
    const currentApp = String(targetForm.getFieldValue('app') ?? '').trim();
    if (currentApp && options.some((option) => option.value === currentApp)) {
      targetForm.setFieldsValue({ workerPool: undefined, canaryJobId: undefined, processorName: undefined, processorType: undefined });
      return;
    }
    targetForm.setFieldsValue({ app: undefined, workerPool: undefined, canaryJobId: undefined, processorName: undefined, processorType: undefined });
  };
  const applyAppSelection = (targetForm: typeof form | typeof editForm) => {
    targetForm.setFieldsValue({ workerPool: undefined, canaryJobId: undefined, processorName: undefined, processorType: undefined });
  };

  const applyWorkerPoolSelection = (targetForm: typeof form | typeof editForm) => {
    targetForm.setFieldsValue({ processorName: undefined, processorType: undefined });
  };

  const workerPoolOptionsForScope = (namespace?: string | null, app?: string | null) => {
    const normalizedNamespace = namespace?.trim();
    const normalizedApp = app?.trim();
    if (!normalizedNamespace || !normalizedApp) return [];
    return workerPools
      .filter((pool) => pool.namespace === normalizedNamespace && pool.app === normalizedApp)
      .map((pool) => ({
        value: pool.name,
        label: `${pool.name} · 并发 ${pool.maxConcurrency > 0 ? pool.maxConcurrency : '不限'} / 队列 ${pool.maxQueueDepth > 0 ? pool.maxQueueDepth : '不限'}`,
      }))
      .sort((left, right) => left.value.localeCompare(right.value));
  };

  const workersForScope = (namespace?: string | null, app?: string | null, workerPool?: string | null) => {
    const normalizedNamespace = namespace?.trim();
    const normalizedApp = app?.trim();
    const normalizedWorkerPool = workerPool?.trim();
    if (!normalizedNamespace || !normalizedApp) return [];
    return workers
      .filter((worker) => worker.namespace === normalizedNamespace && worker.app === normalizedApp)
      .filter((worker) => !normalizedWorkerPool || worker.workerPool === normalizedWorkerPool);
  };
  const processorOptionLabel = (name: string, description?: string | null) => (
    <Space orientation="vertical" size={0}>
      <Typography.Text data-runtime-text>{name}</Typography.Text>
      {description ? <Typography.Text type="secondary" style={{ fontSize: 12 }} data-runtime-text>{description}</Typography.Text> : null}
    </Space>
  );
  const processorSearchText = (name: string, description?: string | null) => `${name} ${description ?? ''}`.toLowerCase();
  const normalProcessorsForScope = (namespace?: string | null, app?: string | null, workerPool?: string | null) => {
    const byName = new Map<string, { name: string; description: string }>();
    for (const worker of workersForScope(namespace, app, workerPool)) {
      const structured = worker.structuredCapabilities;
      for (const processor of structured?.normalProcessors ?? []) {
        if (!processor.name.trim()) continue;
        const existing = byName.get(processor.name);
        if (!existing || (!existing.description && processor.description)) byName.set(processor.name, { name: processor.name, description: processor.description ?? '' });
      }
    }
    return [...byName.values()].sort((left, right) => left.name.localeCompare(right.name));
  };
  const pluginMetaByType = (processorType: string) => {
    for (const plugin of plugins.filter((item) => item.enabled)) {
      const processor = plugin.processorTypes.find((item) => item.type === processorType);
      if (processor) return { plugin, processor };
    }
    return null;
  };
  const pluginProcessorOptionsForScope = (namespace?: string | null, app?: string | null, workerPool?: string | null) => {
    const types = new Set<string>();
    for (const worker of workersForScope(namespace, app, workerPool)) {
      for (const plugin of worker.structuredCapabilities?.pluginProcessors ?? []) {
        if (plugin.type.trim()) types.add(plugin.type.trim());
      }
    }
    return [...types].sort().map((type) => {
      const meta = pluginMetaByType(type);
      const label = meta ? `${meta.plugin.name} · ${meta.processor.label} · ${type}` : type;
      return { value: type, label, searchText: label.toLowerCase() };
    });
  };
  const pluginProcessorNameOptions = (processorType?: string | null, namespace?: string | null, app?: string | null, workerPool?: string | null) => {
    const type = processorType?.trim();
    if (!type) return [];
    const byName = new Map<string, { name: string; description: string }>();
    for (const worker of workersForScope(namespace, app, workerPool)) {
      for (const plugin of worker.structuredCapabilities?.pluginProcessors ?? []) {
        if (plugin.type !== type) continue;
        for (const processor of plugin.processors ?? []) {
          if (!processor.name.trim()) continue;
          const existing = byName.get(processor.name);
          if (!existing || (!existing.description && processor.description)) byName.set(processor.name, { name: processor.name, description: processor.description ?? '' });
        }
        for (const name of plugin.processorNames) {
          const clean = name.trim();
          if (clean && !byName.has(clean)) byName.set(clean, { name: clean, description: '' });
        }
      }
    }
    return [...byName.values()].sort((left, right) => left.name.localeCompare(right.name)).map((processor) => ({
      value: processor.name,
      label: processorOptionLabel(processor.name, processor.description),
      searchText: processorSearchText(processor.name, processor.description),
    }));
  };
  const applyPluginProcessorSelection = (
    targetForm: typeof form | typeof editForm,
    processorType?: string | null,
    namespace?: string | null,
    app?: string | null,
  ) => {
    const options = pluginProcessorNameOptions(processorType, namespace, app, targetForm.getFieldValue('workerPool'));
    targetForm.setFieldsValue({ processorName: options[0]?.value ?? undefined });
  };

  const sdkProcessorOptions = (currentValue?: string | null, namespace?: string | null, app?: string | null, workerPool?: string | null) => {
    const options = normalProcessorsForScope(namespace, app, workerPool).map((processor) => ({
      value: processor.name,
      label: processorOptionLabel(processor.name, processor.description),
      searchText: processorSearchText(processor.name, processor.description),
    }));
    const current = currentValue?.trim();
    if (current && !options.some((option) => option.value === current)) {
      options.push({
        value: current,
        label: <Typography.Text type="secondary" data-runtime-text>{current} · 当前值未在所选 App 的在线 Worker 中注册</Typography.Text>,
        searchText: current.toLowerCase(),
        disabled: true,
      } as (typeof options)[number] & { disabled: boolean });
    }
    return options;
  };
  const selectSearchFilter = (input: string, option?: { searchText?: unknown; label?: unknown }) => String(option?.searchText ?? option?.label ?? '').toLowerCase().includes(input.toLowerCase());
  const calendarOptions = calendars.map((calendar) => ({ value: calendar.name, label: `${calendar.namespace}/${calendar.app} · ${calendar.name}` }));
  const calendarRefValue = (name?: string | null) => name ? { calendarRef: name } : null;
  const parseCalendarRef = (value?: unknown) => {
    if (!value || typeof value !== 'object') return undefined;
    const ref = (value as { calendarRef?: unknown }).calendarRef;
    return typeof ref === 'string' ? ref : undefined;
  };
  const datePickerValue = (value?: string | null) => value ? dayjs(value) : undefined;
  const isoDateValue = (value?: unknown) => {
    if (value === undefined) return undefined;
    if (value === null || value === '') return null;
    if (typeof value === 'string') return value;
    if (dayjs.isDayjs(value)) return value.toISOString();
    return undefined;
  };
  const scriptOptions = scripts
    .filter((script) => script.status === 'approved')
    .map((script) => ({ value: script.id, label: `${script.name} · ${script.language} · ${script.id}` }));
  const normalizeSchedule = <T extends { scheduleType?: string; scheduleExpr?: string | null; fixedRateValue?: number; fixedRateUnit?: string; fixedRateJitterValue?: number; fixedRateJitterUnit?: string; scheduleCalendarRef?: string | null }>(values: T) => {
    if (values.scheduleType === 'api') return { ...values, scheduleExpr: null };
    if (values.scheduleType === 'fixed_rate') {
      const interval = durationExpr(values.fixedRateValue, values.fixedRateUnit);
      const jitter = durationExpr(values.fixedRateJitterValue, values.fixedRateJitterUnit);
      return { ...values, scheduleExpr: jitter ? `${interval};jitter=${jitter}` : interval };
    }
    if (values.scheduleType === 'fixed_delay') return { ...values, scheduleExpr: durationExpr(values.fixedRateValue, values.fixedRateUnit) };
    return { ...values, scheduleCalendar: calendarRefValue(values.scheduleCalendarRef) };
  };
  const openCreateDrawer = () => {
    form.resetFields();
    form.setFieldsValue({ scheduleType: 'api', enabled: true, fixedRateUnit: 's', fixedRateJitterUnit: 's', executorKind: 'sdk', canaryPercent: 0, canaryPolicy: DEFAULT_CANARY_POLICY, misfirePolicy: 'fire_once', retryPolicy: DEFAULT_RETRY_POLICY });
    setCreateDrawerOpen(true);
  };

  const openEditDrawer = (job: JobSummary) => {
    setEditingJob(job);
  };

  useEffect(() => {
    if (!editingJob) return;
    const fixedRate = parseFixedRate(editingJob.scheduleExpr);
    editForm.resetFields();
    editForm.setFieldsValue({
      namespace: editingJob.namespace,
      app: editingJob.app,
      workerPool: editingJob.workerPool ?? undefined,
      name: editingJob.name,
      scheduleType: editingJob.scheduleType,
      scheduleExpr: ['cron', 'once', 'daily_time_interval'].includes(editingJob.scheduleType) ? editingJob.scheduleExpr : undefined,
      ...fixedRate,
      misfirePolicy: editingJob.misfirePolicy ?? 'fire_once',
      scheduleStartAt: datePickerValue(editingJob.scheduleStartAt),
      scheduleEndAt: datePickerValue(editingJob.scheduleEndAt),
      scheduleCalendarRef: parseCalendarRef(editingJob.scheduleCalendar),
      executorKind: editingJob.scriptId ? 'script' : editingJob.processorType ? 'plugin' : 'sdk',
      processorName: editingJob.processorName ?? undefined,
      processorType: editingJob.processorType ?? undefined,
      scriptId: editingJob.scriptId ?? undefined,
      canaryJobId: editingJob.canaryJobId ?? undefined,
      canaryPercent: editingJob.canaryPercent ?? 0,
      canaryPolicy: canaryPolicyValue(editingJob.canaryPolicy),
      enabled: editingJob.enabled,
      retryPolicy: retryPolicyValue(editingJob.retryPolicy),
    });
  }, [editForm, editingJob]);

  const parseBroadcastLabels = (labelsText?: string): Record<string, string> => {
    const labels: Record<string, string> = {};
    for (const item of String(labelsText ?? '').split(',').map((part) => part.trim()).filter(Boolean)) {
      const [key, ...rest] = item.split('=');
      const value = rest.join('=').trim();
      if (key?.trim() && value) labels[key.trim()] = value;
    }
    return labels;
  };

  const openBroadcastDrawer = (job: JobSummary) => {
    setBroadcastJob(job);
    broadcastForm.resetFields();
  };

  const handleBroadcastSubmit = async (values: { tags?: string[]; region?: string; cluster?: string; labelsText?: string }) => {
    if (!broadcastJob) return;
    const selector: BroadcastSelectorRequest = {
      tags: values.tags?.map((tag) => tag.trim()).filter(Boolean),
      region: values.region?.trim() || undefined,
      cluster: values.cluster?.trim() || undefined,
      labels: parseBroadcastLabels(values.labelsText),
    };
    if (selector.labels && Object.keys(selector.labels).length === 0) delete selector.labels;
    const instance = await triggerJob(broadcastJob.id, { triggerType: 'api', executionMode: 'broadcast', broadcastSelector: selector });
    message.success(instance.canaryRouting?.routed ? `已广播触发 ${broadcastJob.name}，命中灰度 ${instance.canaryRouting.routedJobId}` : `已广播触发 ${broadcastJob.name}`);
    setBroadcastJob(null);
    await load();
  };

  const validatePluginExecutor = (processorType?: string | null, processorName?: string | null, namespace?: string | null, app?: string | null, workerPool?: string | null) => {
    const selected = pluginProcessorOptionsForScope(namespace, app, workerPool).find((option) => option.value === processorType);
    if (!selected) throw new Error('请选择当前 App 在线 Worker 已注册的插件处理器类型');
    const candidates = pluginProcessorNameOptions(processorType, namespace, app, workerPool).map((option) => option.value);
    if (candidates.length === 0) throw new Error('当前 App 在线 Worker 未注册该插件执行器候选');
    if (!processorName?.trim()) throw new Error('请选择当前 App 在线 Worker 注册的任务处理器名');
    if (!candidates.includes(processorName.trim())) {
      throw new Error('任务处理器名必须来自当前 App 在线 Worker 的注册能力');
    }
  };

  const normalizeExecutor = <T extends { executorKind?: 'sdk' | 'script' | 'plugin'; processorName?: string | null; scriptId?: string | null; processorType?: string | null; namespace?: string | null; app?: string | null; workerPool?: string | null }>(values: T) => {
    const { executorKind: _ignoredExecutorKind, ...rest } = values;
    void _ignoredExecutorKind;
    if (values.executorKind === 'script') return { ...rest, processorName: null, processorType: null };
    if (values.executorKind === 'plugin') {
      validatePluginExecutor(values.processorType, values.processorName, values.namespace, values.app, values.workerPool);
      return { ...rest, scriptId: null };
    }
    return { ...rest, scriptId: null, processorType: null };
  };

  const handleEditSubmit = async (values: JobFormValues) => {
    if (!editingJob) return;
    if (!canWriteJobs) { message.error('当前账号无权限编辑任务'); return; }
    try {
      const { fixedRateValue: _ignoredFixedRateValue, fixedRateUnit: _ignoredFixedRateUnit, fixedRateJitterValue: _ignoredJitterValue, fixedRateJitterUnit: _ignoredJitterUnit, scheduleCalendarRef: _ignoredCalendarRef, ...scheduled } = normalizeSchedule(values);
      void _ignoredFixedRateValue;
      void _ignoredFixedRateUnit;
      void _ignoredJitterValue;
      void _ignoredJitterUnit;
      void _ignoredCalendarRef;
      const payload = normalizeExecutor({
        ...scheduled,
        scheduleStartAt: isoDateValue(scheduled.scheduleStartAt),
        scheduleEndAt: isoDateValue(scheduled.scheduleEndAt),
      });
      const updated = await updateJob(editingJob.id, payload);
      setJobs((current) => current.map((item) => item.id === updated.id ? updated : item));
      setEditingJob(null);
      editForm.resetFields();
      message.success(`已更新 ${updated.name}`);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新任务失败');
    }
  };


  const openVersionDrawer = async (job: JobSummary) => {
    setVersionJob(job);
    setVersionsLoading(true);
    try {
      const page = await listJobVersions(job.id);
      setJobVersions(page.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载版本历史失败');
      setJobVersions([]);
    } finally {
      setVersionsLoading(false);
    }
  };

  const handleRollback = async (version: JobVersionSummary) => {
    if (!versionJob) return;
    if (!canWriteJobs) { message.error('当前账号无权限回滚任务'); return; }
    try {
      const updated = await rollbackJob(versionJob.id, version.version_number);
      setJobs((current) => current.map((item) => item.id === updated.id ? updated : item));
      setVersionJob(updated);
      await openVersionDrawer(updated);
      message.success(`已回滚到版本 v${version.version_number}`);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '回滚任务失败');
    }
  };

  const handleDelete = async (job: JobSummary) => {
    if (!canWriteJobs) { message.error('当前账号无权限删除任务'); return; }
    try {
      await deleteJob(job.id);
      message.success(`已删除 ${job.name}`);
      await load();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除任务失败');
    }
  };

  const openAdviceDrawer = async (job: JobSummary) => {
    setAdviceJob(job);
    setAdviceLoading(true);
    try {
      setSchedulingAdvice(await getJobSchedulingAdvice(job.id));
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载调度建议失败');
      setSchedulingAdvice(null);
    } finally {
      setAdviceLoading(false);
    }
  };


  const filteredJobs = useMemo(() => jobs.filter((job) => {
    const keyword = String(query.keyword ?? '').trim().toLowerCase();
    const scheduleType = String(query.scheduleType ?? '').trim();
    const matchesKeyword = keyword === '' || [job.name, job.namespace, job.app, job.workerPool ?? '', job.processorName ?? '', job.scriptId ?? '', job.id].some((value) => value.toLowerCase().includes(keyword));
    const matchesSchedule = scheduleType === '' || job.scheduleType === scheduleType;
    return matchesKeyword && matchesSchedule;
  }), [jobs, query.keyword, query.scheduleType]);

  const renderAdviceStat = (label: string, value: ReactNode, helper?: ReactNode) => (
    <Card size="small" className="scheduling-advice-stat-card">
      <Typography.Text className="scheduling-advice-stat-label">{label}</Typography.Text>
      <div className="scheduling-advice-stat-value">{value}</div>
      {helper ? <Typography.Text className="scheduling-advice-stat-helper">{helper}</Typography.Text> : null}
    </Card>
  );

  const renderRetryPolicyFields = () => (
    <section className="job-retry-policy-section" aria-label="失败重试策略">
      <div className="job-retry-policy-header">
        <div className="job-retry-policy-copy">
          <Typography.Text strong className="job-retry-policy-heading">失败重试</Typography.Text>
          <Typography.Text type="secondary" className="job-retry-policy-desc">默认启用指数退避：首次失败 5 秒后重试，总共最多 3 次；取消、脚本治理拒绝等非运行期失败不会盲目重试。</Typography.Text>
        </div>
        <Form.Item name={["retryPolicy", "enabled"]} valuePropName="checked" className="job-retry-policy-toggle">
          <Switch checkedChildren="启用" unCheckedChildren="关闭" />
        </Form.Item>
      </div>
      <div className="job-retry-policy-grid">
        <Form.Item name={["retryPolicy", "maxAttempts"]} label="总尝试次数" rules={[{ required: true }]}>
          <InputNumber min={1} max={10} precision={0} style={{ width: '100%' }} />
        </Form.Item>
        <Form.Item name={["retryPolicy", "initialDelaySeconds"]} label="首次延迟" rules={[{ required: true }]}>
          <InputNumber min={0} max={86400} precision={0} addonAfter="秒" style={{ width: '100%' }} />
        </Form.Item>
        <Form.Item name={["retryPolicy", "backoffMultiplier"]} label="退避倍数" rules={[{ required: true }]}>
          <InputNumber min={1} max={10} precision={0} style={{ width: '100%' }} />
        </Form.Item>
        <Form.Item name={["retryPolicy", "maxDelaySeconds"]} label="最大延迟" rules={[{ required: true }]}>
          <InputNumber min={0} max={86400} precision={0} addonAfter="秒" style={{ width: '100%' }} />
        </Form.Item>
      </div>
    </section>
  );

  const renderCanaryPolicyFields = (targetOptions: Array<{ label: string; value: string }>) => (
    <section className="job-retry-policy-section" aria-label="灰度指标门禁">
      <div className="job-retry-policy-header">
        <div className="job-retry-policy-copy">
          <Typography.Text strong className="job-retry-policy-heading">灰度发布与自动回滚</Typography.Text>
          <Typography.Text type="secondary" className="job-retry-policy-desc">显式触发时按比例路由到 canary 任务；开启指标门禁后，Server 会基于 canary 目标最近真实终态实例计算失败率，超阈值自动把灰度比例降为 0%。</Typography.Text>
        </div>
        <Form.Item name={["canaryPolicy", "metricsGateEnabled"]} valuePropName="checked" className="job-retry-policy-toggle">
          <Switch checkedChildren="门禁" unCheckedChildren="关闭" />
        </Form.Item>
      </div>
      <Form.Item name="canaryJobId" label="灰度目标任务" extra="可选：显式触发当前任务时，按灰度比例路由到目标任务。"><Select allowClear showSearch optionFilterProp="label" placeholder="选择同 App 下的 canary 任务" options={targetOptions} /></Form.Item>
      <div className="job-retry-policy-grid">
        <Form.Item name="canaryPercent" label="灰度比例"><InputNumber min={0} max={100} precision={0} addonAfter="%" style={{ width: '100%' }} /></Form.Item>
        <Form.Item name={["canaryPolicy", "minimumSamples"]} label="最小样本数"><InputNumber min={1} max={1000} precision={0} style={{ width: '100%' }} /></Form.Item>
        <Form.Item name={["canaryPolicy", "evaluationWindow"]} label="评估窗口"><InputNumber min={1} max={10000} precision={0} addonAfter="条" style={{ width: '100%' }} /></Form.Item>
        <Form.Item name={["canaryPolicy", "maxFailureRate"]} label="失败率阈值"><InputNumber min={0} max={1} step={0.05} precision={2} addonAfter="0~1" style={{ width: '100%' }} /></Form.Item>
      </div>
      <Form.Item name={["canaryPolicy", "autoRollback"]} label="超阈值自动回滚" valuePropName="checked" extra="关闭后只在触发响应中返回门禁状态，不会自动把 canaryPercent 归零。"><Switch checkedChildren="自动" unCheckedChildren="只告警" /></Form.Item>
    </section>
  );

  const columns: ColumnsType<JobSummary> = [
    { title: 'Name', dataIndex: 'name' },
    { title: '作用域', render: (_, job) => <Space orientation="vertical" size={0}><strong>{job.namespace}</strong><Typography.Text type="secondary" style={{ fontSize: 12 }}>{job.app}</Typography.Text>{job.workerPool ? <Tag color="geekblue">执行池 {job.workerPool}</Tag> : <Typography.Text type="secondary" style={{ fontSize: 12 }}>不限执行池</Typography.Text>}</Space> },
    { title: 'Schedule', dataIndex: 'scheduleType', render: (value: string, job) => <Space><Tag color="blue" className="soft-tag">{value}</Tag><Tag>v{job.versionNumber}</Tag>{job.canaryPercent > 0 ? <Tag color="orange">canary {job.canaryPercent}%</Tag> : null}{job.retryPolicy?.enabled && job.retryPolicy.maxAttempts > 1 ? <Tag color="volcano">retry {job.retryPolicy.maxAttempts}x</Tag> : <Tag>no retry</Tag>}</Space> },
    { title: '执行器', render: (_, job) => job.scriptId ? <Tag color="purple">脚本 · {job.scriptId}</Tag> : job.processorType ? <Tag color="geekblue">插件 · {job.processorType} · {job.processorName || job.name}</Tag> : <Typography.Text code>{job.processorName || job.name}</Typography.Text> },
    {
      title: 'Enabled',
      dataIndex: 'enabled',
      render: (value: boolean, job) => (
        <Switch
          size="small"
          checked={value}
          disabled={!canWriteJobs}
          onChange={async (enabled) => {
            try {
              const updated = await updateJob(job.id, { enabled });
              setJobs((current) => current.map((item) => item.id === job.id ? updated : item));
              message.success(enabled ? `已启用 ${job.name}` : `已禁用 ${job.name}`);
            } catch (err) {
              message.error(err instanceof Error ? err.message : '更新任务状态失败');
            }
          }}
        />
      ),
    },
    {
      title: 'Actions',
      width: 260,
      align: 'right',
      render: (_, job) => (
        <Space size={4} className="table-action-strip">
          {canExecuteInstances ? (
            <Button
              size="small"
              type="link"
              onClick={async () => {
                try {
                  const instance = await triggerJob(job.id, { triggerType: 'api', executionMode: 'single' });
                  message.success(instance.canaryRouting?.routed ? `已触发 ${job.name}，命中灰度 ${instance.canaryRouting.routedJobId}` : `已触发 ${job.name}`);
                  await load();
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '触发失败');
                }
              }}
            >
              单机执行
            </Button>
          ) : null}
          {canExecuteInstances ? (
            <Button size="small" type="link" onClick={() => openBroadcastDrawer(job)}>
              广播
            </Button>
          ) : null}
          <Button size="small" type="link" onClick={() => void openAdviceDrawer(job)}>调度建议</Button>
          <Button size="small" type="link" onClick={() => void openVersionDrawer(job)}>版本</Button>
          <Button size="small" type="link" onClick={() => setNotificationJob(job)}>通知配置</Button>
          <PermissionGate resource="jobs" action="write">
            <Button size="small" type="link" onClick={() => openEditDrawer(job)}>编辑</Button>
          </PermissionGate>
          <PermissionGate resource="jobs" action="write">
            <Popconfirm title="删除任务" description="删除后该任务将无法再触发，历史实例保留用于审计。" onConfirm={() => void handleDelete(job)}>
              <Button size="small" type="link" danger>删除</Button>
            </Popconfirm>
          </PermissionGate>
        </Space>
      ),
    },
  ];

  return (
    <div className="page-stack">
      <JobNotificationConfigDrawer job={notificationJob} open={Boolean(notificationJob)} onClose={() => setNotificationJob(null)} />
      <Drawer
        title="创建任务"
        open={createDrawerOpen}
        onClose={() => { setCreateDrawerOpen(false); form.resetFields(); }}
        width={900}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">配置任务所属 namespace/app、可选执行池、调度类型和 Worker processor 绑定；创建后在列表统一启停和触发。</Typography.Paragraph>
        <Form
          form={form}
          layout="vertical"
          initialValues={{ scheduleType: 'api', enabled: true, canaryPercent: 0, canaryPolicy: DEFAULT_CANARY_POLICY, misfirePolicy: 'fire_once', retryPolicy: DEFAULT_RETRY_POLICY }}
          onFinish={async (values) => {
            if (!canWriteJobs) { message.error('当前账号无权限创建任务'); return; }
            try {
              const { fixedRateValue: _ignoredFixedRateValue, fixedRateUnit: _ignoredFixedRateUnit, fixedRateJitterValue: _ignoredJitterValue, fixedRateJitterUnit: _ignoredJitterUnit, scheduleCalendarRef: _ignoredCalendarRef, ...scheduled } = normalizeSchedule(values);
              void _ignoredFixedRateValue;
              void _ignoredFixedRateUnit;
              void _ignoredJitterValue;
              void _ignoredJitterUnit;
              void _ignoredCalendarRef;
              const payload = normalizeExecutor({
                ...scheduled,
                scheduleStartAt: isoDateValue(scheduled.scheduleStartAt),
                scheduleEndAt: isoDateValue(scheduled.scheduleEndAt),
              });
              await createJob(payload);
              message.success('任务已创建');
              form.resetFields();
              setCreateDrawerOpen(false);
              await load();
            } catch (err) {
              message.error(err instanceof Error ? err.message : '创建失败');
            }
          }}
        >
          <Form.Item name="namespace" label="Namespace" rules={[{ required: true }]}>
            <Select
              allowClear
              showSearch
              optionFilterProp="label"
              options={namespaceOptions}
              placeholder="选择作用域管理中的 Namespace"
              onChange={(value) => applyNamespaceSelection(form, value)}
            />
          </Form.Item>
          <Form.Item name="app" label="App" rules={[{ required: true }]}>
            <Select
              allowClear
              showSearch
              optionFilterProp="label"
              options={appOptionsForNamespace(createNamespace)}
              placeholder="选择作用域管理中的 App"
              disabled={!createNamespace}
              onChange={() => applyAppSelection(form)}
            />
          </Form.Item>
          <Form.Item name="workerPool" label="执行池（可选）" extra="执行池是 App 下的可选执行资源分组；选择后只匹配标记了相同 worker_pool / worker-pool 标签的 Worker，并应用该执行池的队列与并发配额。">
            <Select
              allowClear
              showSearch
              optionFilterProp="label"
              options={workerPoolOptionsForScope(createNamespace, createApp)}
              placeholder={createNamespace && createApp ? '选择执行池，留空表示不限执行池' : '先选择 Namespace 和 App'}
              disabled={!createNamespace || !createApp}
              onChange={() => applyWorkerPoolSelection(form)}
            />
          </Form.Item>
          <Form.Item name="name" label="任务名称" rules={[{ required: true }]}><Input placeholder="demo.echo" /></Form.Item>
          <Form.Item name="executorKind" label="执行方式" rules={[{ required: true }]}><Select options={[{ value: 'sdk', label: '处理器' }, { value: 'plugin', label: '插件处理器' }, { value: 'script', label: '脚本（沙箱自动执行）' }]} /></Form.Item>
          <Form.Item noStyle shouldUpdate={(prev, next) => prev.executorKind !== next.executorKind}>
            {({ getFieldValue }) => getFieldValue('executorKind') === 'script' ? (
              <Form.Item name="scriptId" label="具体脚本" extra="选择已审批脚本即可；Server 会按脚本语言匹配 Worker 注册的结构化 scriptRunners。" rules={[{ required: true, message: '请选择具体脚本' }]}><Select showSearch optionFilterProp="label" placeholder="选择已审批脚本" options={scriptOptions} /></Form.Item>
            ) : getFieldValue('executorKind') === 'plugin' ? (
              <><Form.Item name="processorType" label="插件处理器类型" rules={[{ required: true, message: '请选择插件处理器类型' }]}><Select showSearch filterOption={selectSearchFilter} placeholder="选择插件处理器类型" options={pluginProcessorOptionsForScope(createNamespace, createApp, createWorkerPool)} onChange={(value) => applyPluginProcessorSelection(form, value, createNamespace, createApp)} /></Form.Item><Form.Item noStyle shouldUpdate={(prev, next) => prev.processorType !== next.processorType || prev.processorName !== next.processorName || prev.workerPool !== next.workerPool}>{({ getFieldValue }) => <Form.Item name="processorName" label="任务处理器名" extra="只能选择当前 Namespace/App/执行池 在线 Worker 注册的插件执行器；描述来自 Worker 注册能力。" rules={[{ required: true, message: '请选择任务处理器名候选' }]}><Select placeholder="自动选择任务处理器名" options={pluginProcessorNameOptions(getFieldValue('processorType'), createNamespace, createApp, createWorkerPool)} filterOption={selectSearchFilter} /></Form.Item>}</Form.Item></>
            ) : (
              <Form.Item name="processorName" label="处理器" extra="只能选择当前 Namespace/App/执行池 在线 Worker 注册的普通处理器。Java/Spring Worker 通过 @TikeoProcessor 注册，可附带 description。" rules={[{ required: true, message: '请选择当前 App 已注册的普通处理器' }]}><Select allowClear showSearch placeholder="选择当前 App 的处理器" options={sdkProcessorOptions(form.getFieldValue('processorName'), createNamespace, createApp, createWorkerPool)} filterOption={selectSearchFilter} /></Form.Item>
            )}
          </Form.Item>
          <Form.Item name="scheduleType" label="调度类型"><Select options={scheduleTypeOptions} /></Form.Item>
          <Form.Item noStyle shouldUpdate={(prev, next) => prev.scheduleType !== next.scheduleType}>
            {({ getFieldValue }) => {
              const scheduleType = getFieldValue('scheduleType');
              if (scheduleType === 'cron') return <Form.Item name="scheduleExpr" label="Cron 表达式" extra="支持 ;tz=IANA 时区 和 ;exclude=YYYY-MM-DD,...，例如 0 30 9 * * * *;tz=Asia/Shanghai;exclude=2026-10-01。" rules={[{ required: true, message: '请输入 Cron 表达式' }]}><Input placeholder="0/30 * * * * * *;tz=Asia/Shanghai" /></Form.Item>;
              if (scheduleType === 'fixed_rate' || scheduleType === 'fixed_delay') return <><Space.Compact block><Form.Item name="fixedRateValue" label={scheduleType === 'fixed_delay' ? '固定延迟' : '固定频率'} rules={[{ required: true, message: '请输入间隔' }]} style={{ flex: 1 }}><InputNumber min={1} precision={0} style={{ width: '100%' }} placeholder="30" /></Form.Item><Form.Item name="fixedRateUnit" label="单位" rules={[{ required: true }]}><Select style={{ width: 120 }} options={[{ value: 's', label: '秒' }, { value: 'm', label: '分钟' }, { value: 'h', label: '小时' }, { value: 'd', label: '天' }]} /></Form.Item></Space.Compact>{scheduleType === 'fixed_rate' ? <Space.Compact block><Form.Item name="fixedRateJitterValue" label="Jitter 抖动" extra="可选：用于分散同频任务触发，防止惊群。" style={{ flex: 1 }}><InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="5" /></Form.Item><Form.Item name="fixedRateJitterUnit" label="单位"><Select style={{ width: 120 }} options={[{ value: 's', label: '秒' }, { value: 'm', label: '分钟' }, { value: 'h', label: '小时' }]} /></Form.Item></Space.Compact> : null}</>;
              if (scheduleType === 'once') return <Form.Item name="scheduleExpr" label="触发时间" rules={[{ required: true, message: '请输入 RFC3339 时间' }]}><Input placeholder="2026-05-29T20:00:00+08:00" /></Form.Item>;
              if (scheduleType === 'daily_time_interval') return <Form.Item name="scheduleExpr" label="Daily Time Interval" extra="格式：HH:MM-HH:MM/间隔@时区，例如 09:00-18:00/30m@Asia/Shanghai。" rules={[{ required: true, message: '请输入 Daily Time Interval 表达式' }]}><Input placeholder="09:00-18:00/30m@Asia/Shanghai" /></Form.Item>;
              return <Typography.Paragraph type="secondary">API 手动触发任务不会配置调度表达式，可通过 UI、SDK 或 HTTP API 显式触发。</Typography.Paragraph>;
            }}
          </Form.Item>
          <Form.Item name="misfirePolicy" label="Misfire 策略"><Select options={[{ value: 'fire_once', label: '补触发一次' }, { value: 'do_nothing', label: '跳过错过触发' }, { value: 'catch_up_limited', label: '有限追赶' }, { value: 'reschedule', label: '重排到当前' }, { value: 'latest_only', label: '仅保留最近一次' }]} /></Form.Item>
          {renderRetryPolicyFields()}
          <Space.Compact block>
            <Form.Item name="scheduleStartAt" label="生命周期开始" style={{ flex: 1 }}><DatePicker showTime style={{ width: '100%' }} placeholder="选择开始时间" /></Form.Item>
            <Form.Item name="scheduleEndAt" label="生命周期结束" style={{ flex: 1 }}><DatePicker showTime style={{ width: '100%' }} placeholder="选择结束时间" /></Form.Item>
          </Space.Compact>
          <Form.Item name="scheduleCalendarRef" label="调度日历" extra="可选：引用集中式 Calendar，自动排除节假日/维护窗口/冻结窗口。"><Select allowClear showSearch optionFilterProp="label" placeholder="选择 Calendar" options={calendarOptions} /></Form.Item>
          {renderCanaryPolicyFields(canaryJobOptionsForScope(createNamespace, createApp))}
          <Form.Item name="enabled" label="启用" valuePropName="checked"><Switch /></Form.Item>
          <PermissionGate resource="jobs" action="write"><Button type="primary" htmlType="submit" block>创建任务</Button></PermissionGate>
        </Form>
      </Drawer>

      <Drawer
        title={editingJob ? `编辑任务 - ${editingJob.name}` : '编辑任务'}
        open={editingJob !== null}
        onClose={() => { setEditingJob(null); editForm.resetFields(); }}
        width={900}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">编辑任务基础信息、所属 namespace/app/执行池、调度配置、Processor 绑定和启用状态；迁移后新的触发与 Worker 匹配会按目标作用域生效，历史实例仍保留原执行记录。</Typography.Paragraph>
        <Form form={editForm} layout="vertical" onFinish={(values) => void handleEditSubmit(values)}>
          <Space.Compact block>
            <Form.Item name="namespace" label="Namespace" rules={[{ required: true }]} style={{ flex: 1 }}>
              <Select
                allowClear
                showSearch
                optionFilterProp="label"
                options={namespaceOptions}
                placeholder="选择作用域管理中的 Namespace"
                onChange={(value) => applyNamespaceSelection(editForm, value)}
              />
            </Form.Item>
            <Form.Item name="app" label="App" rules={[{ required: true }]} style={{ flex: 1 }}>
              <Select
                allowClear
                showSearch
                optionFilterProp="label"
                options={appOptionsForNamespace(editNamespace)}
                placeholder="选择作用域管理中的 App"
                disabled={!editNamespace}
                onChange={() => applyAppSelection(editForm)}
              />
            </Form.Item>
          </Space.Compact>
          <Form.Item name="workerPool" label="执行池（可选）" extra="执行池是 App 下的可选执行资源分组；选择后新的触发只匹配同池 Worker，并应用该执行池配额。">
            <Select
              allowClear
              showSearch
              optionFilterProp="label"
              options={workerPoolOptionsForScope(editNamespace, editApp)}
              placeholder={editNamespace && editApp ? '选择执行池，留空表示不限执行池' : '先选择 Namespace 和 App'}
              disabled={!editNamespace || !editApp}
              onChange={() => applyWorkerPoolSelection(editForm)}
            />
          </Form.Item>
          <Form.Item name="name" label="任务名称" rules={[{ required: true }]}><Input /></Form.Item>
          <Form.Item name="executorKind" label="执行方式" rules={[{ required: true }]}><Select options={[{ value: 'sdk', label: '处理器' }, { value: 'plugin', label: '插件处理器' }, { value: 'script', label: '脚本（沙箱自动执行）' }]} /></Form.Item>
          <Form.Item noStyle shouldUpdate={(prev, next) => prev.executorKind !== next.executorKind}>
            {({ getFieldValue }) => getFieldValue('executorKind') === 'script' ? (
              <Form.Item name="scriptId" label="具体脚本" extra="选择已审批脚本即可；Server 会按脚本语言匹配 Worker 注册的结构化 scriptRunners。" rules={[{ required: true, message: '请选择具体脚本' }]}><Select showSearch optionFilterProp="label" placeholder="选择已审批脚本" options={scriptOptions} /></Form.Item>
            ) : getFieldValue('executorKind') === 'plugin' ? (
              <><Form.Item name="processorType" label="插件处理器类型" rules={[{ required: true, message: '请选择插件处理器类型' }]}><Select showSearch filterOption={selectSearchFilter} placeholder="选择插件处理器类型" options={pluginProcessorOptionsForScope(editNamespace, editApp, editWorkerPool)} onChange={(value) => applyPluginProcessorSelection(editForm, value, editNamespace, editApp)} /></Form.Item><Form.Item noStyle shouldUpdate={(prev, next) => prev.processorType !== next.processorType || prev.processorName !== next.processorName || prev.workerPool !== next.workerPool}>{({ getFieldValue }) => <Form.Item name="processorName" label="任务处理器名" extra="只能选择当前 Namespace/App/执行池 在线 Worker 注册的插件执行器；描述来自 Worker 注册能力。" rules={[{ required: true, message: '请选择任务处理器名候选' }]}><Select placeholder="自动选择任务处理器名" options={pluginProcessorNameOptions(getFieldValue('processorType'), editNamespace, editApp, editWorkerPool)} filterOption={selectSearchFilter} /></Form.Item>}</Form.Item></>
            ) : (
              <Form.Item name="processorName" label="处理器" extra="只能选择当前 Namespace/App/执行池 在线 Worker 注册的普通处理器。Java/Spring Worker 通过 @TikeoProcessor 注册，可附带 description。" rules={[{ required: true, message: '请选择当前 App 已注册的普通处理器' }]}><Select allowClear showSearch placeholder="选择当前 App 的处理器" options={sdkProcessorOptions(editForm.getFieldValue('processorName'), editNamespace, editApp, editWorkerPool)} filterOption={selectSearchFilter} /></Form.Item>
            )}
          </Form.Item>
          <Form.Item name="scheduleType" label="调度类型"><Select options={scheduleTypeOptions} /></Form.Item>
          <Form.Item noStyle shouldUpdate={(prev, next) => prev.scheduleType !== next.scheduleType}>
            {({ getFieldValue }) => {
              const scheduleType = getFieldValue('scheduleType');
              if (scheduleType === 'cron') return <Form.Item name="scheduleExpr" label="Cron 表达式" extra="支持 ;tz=IANA 时区 和 ;exclude=YYYY-MM-DD,...，例如 0 30 9 * * * *;tz=Asia/Shanghai;exclude=2026-10-01。" rules={[{ required: true, message: '请输入 Cron 表达式' }]}><Input placeholder="0/30 * * * * * *;tz=Asia/Shanghai" /></Form.Item>;
              if (scheduleType === 'fixed_rate' || scheduleType === 'fixed_delay') return <><Space.Compact block><Form.Item name="fixedRateValue" label={scheduleType === 'fixed_delay' ? '固定延迟' : '固定频率'} rules={[{ required: true, message: '请输入间隔' }]} style={{ flex: 1 }}><InputNumber min={1} precision={0} style={{ width: '100%' }} placeholder="30" /></Form.Item><Form.Item name="fixedRateUnit" label="单位" rules={[{ required: true }]}><Select style={{ width: 120 }} options={[{ value: 's', label: '秒' }, { value: 'm', label: '分钟' }, { value: 'h', label: '小时' }, { value: 'd', label: '天' }]} /></Form.Item></Space.Compact>{scheduleType === 'fixed_rate' ? <Space.Compact block><Form.Item name="fixedRateJitterValue" label="Jitter 抖动" extra="可选：用于分散同频任务触发，防止惊群。" style={{ flex: 1 }}><InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="5" /></Form.Item><Form.Item name="fixedRateJitterUnit" label="单位"><Select style={{ width: 120 }} options={[{ value: 's', label: '秒' }, { value: 'm', label: '分钟' }, { value: 'h', label: '小时' }]} /></Form.Item></Space.Compact> : null}</>;
              if (scheduleType === 'once') return <Form.Item name="scheduleExpr" label="触发时间" rules={[{ required: true, message: '请输入 RFC3339 时间' }]}><Input placeholder="2026-05-29T20:00:00+08:00" /></Form.Item>;
              if (scheduleType === 'daily_time_interval') return <Form.Item name="scheduleExpr" label="Daily Time Interval" extra="格式：HH:MM-HH:MM/间隔@时区，例如 09:00-18:00/30m@Asia/Shanghai。" rules={[{ required: true, message: '请输入 Daily Time Interval 表达式' }]}><Input placeholder="09:00-18:00/30m@Asia/Shanghai" /></Form.Item>;
              return <Typography.Paragraph type="secondary">API 手动触发任务不会配置调度表达式，可通过 UI、SDK 或 HTTP API 显式触发。</Typography.Paragraph>;
            }}
          </Form.Item>
          {renderCanaryPolicyFields(canaryJobOptionsForScope(editNamespace, editApp, editingJob?.id))}
          <Form.Item name="misfirePolicy" label="Misfire 策略"><Select options={[{ value: 'fire_once', label: '补触发一次' }, { value: 'do_nothing', label: '跳过错过触发' }, { value: 'catch_up_limited', label: '有限追赶' }, { value: 'reschedule', label: '重排到当前' }, { value: 'latest_only', label: '仅保留最近一次' }]} /></Form.Item>
          {renderRetryPolicyFields()}
          <Space.Compact block>
            <Form.Item name="scheduleStartAt" label="生命周期开始" style={{ flex: 1 }}><DatePicker showTime style={{ width: '100%' }} placeholder="选择开始时间" /></Form.Item>
            <Form.Item name="scheduleEndAt" label="生命周期结束" style={{ flex: 1 }}><DatePicker showTime style={{ width: '100%' }} placeholder="选择结束时间" /></Form.Item>
          </Space.Compact>
          <Form.Item name="scheduleCalendarRef" label="调度日历" extra="可选：引用集中式 Calendar，自动排除节假日/维护窗口/冻结窗口。"><Select allowClear showSearch optionFilterProp="label" placeholder="选择 Calendar" options={calendarOptions} /></Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked"><Switch /></Form.Item>
          <PermissionGate resource="jobs" action="write"><Button type="primary" htmlType="submit" block>保存任务</Button></PermissionGate>
        </Form>
      </Drawer>



      <Drawer
        title={broadcastJob ? `广播触发 - ${broadcastJob.name}` : '广播触发'}
        open={broadcastJob !== null}
        onClose={() => { setBroadcastJob(null); broadcastForm.resetFields(); }}
        width={900}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">可选填写 Worker 筛选条件；不填写时广播到当前 Namespace/App 下全部在线可调度 Worker。</Typography.Paragraph>
        <Form form={broadcastForm} layout="vertical" onFinish={(values) => void handleBroadcastSubmit(values)}>
          <Form.Item name="tags" label="Worker Tags" extra="匹配 Worker structuredCapabilities.tags，可输入多个。"><Select mode="tags" tokenSeparators={[',']} placeholder="java,blue" /></Form.Item>
          <Form.Item name="region" label="Region"><Input allowClear placeholder="cn / us-east-1" /></Form.Item>
          <Form.Item name="cluster" label="Cluster / Version"><Input allowClear placeholder="prod / v2" /></Form.Item>
          <Form.Item name="labelsText" label="Labels" extra="逗号分隔 key=value，例如 tier=gold,runtime=java。"><Input allowClear placeholder="tier=gold,runtime=java" /></Form.Item>
          <Button type="primary" htmlType="submit" block>按条件广播触发</Button>
        </Form>
      </Drawer>

      <Drawer
        title={versionJob ? `版本历史 - ${versionJob.name}` : '版本历史'}
        open={versionJob !== null}
        onClose={() => { setVersionJob(null); setJobVersions([]); }}
        width={680}
      >
        <Typography.Paragraph type="secondary">任务版本是每次创建、编辑和回滚后的不可变快照；回滚会生成新的最新版本，不会覆盖历史。</Typography.Paragraph>
        <Timeline
          pending={versionsLoading ? '加载版本历史...' : undefined}
          items={jobVersions.map((version) => ({
            color: version.version_number === versionJob?.versionNumber ? 'green' : 'blue',
            children: (
              <Space orientation="vertical" size={4} style={{ width: '100%' }}>
                <Space wrap>
                  <Tag color={version.version_number === versionJob?.versionNumber ? 'green' : 'default'}>v{version.version_number}</Tag>
                  <Typography.Text strong>{version.name}</Typography.Text>
                  <Tag data-runtime-text>{version.change_reason}</Tag>
                  {version.rolled_back_from_version ? <Tag color="orange">from v{version.rolled_back_from_version}</Tag> : null}
                </Space>
                <Typography.Text type="secondary"><span data-runtime-text>{version.schedule_type}{version.schedule_expr ? ` · ${version.schedule_expr}` : ''}</span> · {version.enabled ? '启用' : '禁用'} · <span data-runtime-text>{version.created_by}</span> · {version.created_at}</Typography.Text>
                <Typography.Text code>{version.script_id ? <>脚本 <span data-runtime-text>{version.script_id}</span></> : <span data-runtime-text>{version.processor_name ?? 'default processor'}</span>}</Typography.Text>
                <PermissionGate resource="jobs" action="write">
                  <Popconfirm title="回滚任务版本" description={`将任务恢复到 v${version.version_number}，并生成新的最新版本。`} onConfirm={() => void handleRollback(version)} disabled={version.version_number === versionJob?.versionNumber}>
                    <Button size="small" disabled={version.version_number === versionJob?.versionNumber}>回滚到此版本</Button>
                  </Popconfirm>
                </PermissionGate>
              </Space>
            ),
          }))}
        />
      </Drawer>



      <Drawer
        title={adviceJob ? `调度建议 - ${adviceJob.name}` : '调度建议'}
        open={adviceJob !== null}
        onClose={() => { setAdviceJob(null); setSchedulingAdvice(null); }}
        width={840}
      >
        <Typography.Paragraph className="scheduling-advice-intro" type="secondary">基于当前 Job 绑定、在线 Worker 能力和最近实例状态给出触发前建议；只读展示，不改变调度行为。</Typography.Paragraph>
        {adviceLoading ? <Typography.Text type="secondary">加载调度建议...</Typography.Text> : null}
        {schedulingAdvice ? (
          <Space orientation="vertical" size={16} className="scheduling-advice-panel">
            <Alert
              className="scheduling-advice-status"
              type={schedulingAdvice.severity === 'error' ? 'error' : schedulingAdvice.severity === 'warning' ? 'warning' : 'success'}
              showIcon
              message={schedulingAdvice.ready ? '当前可调度' : '当前不可调度'}
              description={<span data-runtime-text>{schedulingAdvice.reason}</span>}
            />

            <div className="scheduling-advice-grid">
              {renderAdviceStat('Required capability', <Typography.Text code data-runtime-text>{schedulingAdvice.requiredCapability ?? 'none'}</Typography.Text>, 'Worker 调度匹配所需能力')}
              {renderAdviceStat('Eligible workers', schedulingAdvice.eligibleWorkers.length ? (
                <Space size={[4, 4]} wrap>
                  {schedulingAdvice.eligibleWorkers.map((worker) => <Tag key={worker} data-runtime-text>{worker}</Tag>)}
                </Space>
              ) : <Tag color="red">0</Tag>, '当前在线且满足能力约束')}
              {renderAdviceStat('Recent window', `${schedulingAdvice.recentInstances} instances`, `${schedulingAdvice.recentFailures} failures in window`)}
              {renderAdviceStat('Estimated duration', `${schedulingAdvice.prediction.estimatedDurationSeconds}s`, '基于完整历史耗时估算')}
              {renderAdviceStat('recommendedConcurrency', schedulingAdvice.prediction.recommendedConcurrency, '推荐触发并发上限')}
              {renderAdviceStat('Worker capacity', (
                <Space size={4} wrap>
                  <Tag color="blue">{schedulingAdvice.prediction.workerCapacity.eligibleWorkerCount} workers</Tag>
                  <Tag>{schedulingAdvice.prediction.workerCapacity.advertisedCpuCores} CPU</Tag>
                  <Tag>{schedulingAdvice.prediction.workerCapacity.advertisedMemoryMb}MiB</Tag>
                </Space>
              ), 'Worker 广告资源汇总')}
            </div>

            <Card size="small" title="历史耗时" className="scheduling-advice-detail-card">
              <div className="scheduling-advice-metric-row">
                <span>avg <strong>{schedulingAdvice.history.averageDurationSeconds}s</strong></span>
                <span>p50 <strong>{schedulingAdvice.history.p50DurationSeconds}s</strong></span>
                <span>p95 <strong>{schedulingAdvice.history.p95DurationSeconds}s</strong></span>
                <span>max <strong>{schedulingAdvice.history.maxDurationSeconds}s</strong></span>
              </div>
              <div className="scheduling-advice-metric-row scheduling-advice-metric-row--muted">
                <span>已检查 {schedulingAdvice.history.inspectedInstances}</span>
                <span>完成 {schedulingAdvice.history.completedInstances}</span>
                <span>失败 {schedulingAdvice.history.failedInstances}</span>
              </div>
            </Card>

            <Card size="small" title="资源预测" className="scheduling-advice-detail-card">
              <Typography.Text strong>预测依据</Typography.Text>
              <div className="scheduling-advice-reasons">
                {schedulingAdvice.prediction.reasons.map((reason) => <Typography.Text key={reason} type="secondary" data-runtime-text>{reason}</Typography.Text>)}
              </div>
            </Card>
          </Space>
        ) : null}
      </Drawer>


      <Card
        className="clean-card"
        title="任务列表"
        extra={<Space wrap className="card-toolbar"><Button onClick={() => navigate(ROUTE_META.jobTopology.path)}>任务拓扑</Button><PermissionGate resource="jobs" action="write"><Button type="primary" onClick={openCreateDrawer}>新建任务</Button></PermissionGate><Input allowClear placeholder="搜索任务/Namespace/App" value={String(query.keyword ?? '')} onChange={(event) => setQuery({ keyword: event.target.value, page: 1 })} style={{ width: 220 }} /><Select allowClear placeholder="调度类型" value={query.scheduleType || undefined} onChange={(value) => setQuery({ scheduleType: value ?? '', page: 1 })} style={{ width: 130 }} options={scheduleFilterOptions} /><Button onClick={load}>刷新</Button></Space>}
      >
        <Table rowKey="id" loading={loading} columns={columns} dataSource={filteredJobs} pagination={{ pageSize: Number(query.page_size) || pageSize, current: Number(query.page) || 1, showSizeChanger: true, pageSizeOptions: TABLE_PAGE_SIZE_OPTIONS.map(String), onChange: (page, nextPageSize) => { setPageSize(nextPageSize); setQuery({ page, page_size: nextPageSize }); } }} size="middle" />
      </Card>
    </div>
  );
}
