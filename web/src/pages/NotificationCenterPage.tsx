import { Alert, Button, Card, Col, Drawer, Form, Input, InputNumber, Popconfirm, Row, Select, Space, Statistic, Switch, Table, Tabs, Tag, Typography, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import {
  createNotificationPolicy,
  deleteNotificationChannel,
  deleteNotificationPolicy,
  deleteNotificationTemplate,
  getNotificationDeliveryQueueStatus,
  listNotificationChannelTypes,
  listNotificationChannels,
  listNotificationMessages,
  listNotificationPolicies,
  listNotificationTemplates,
  retryDueNotificationDeliveryAttempts,
  updateNotificationPolicy,
  validateNotificationPolicy,
  type CreateNotificationPolicyRequest,
  type NotificationChannelSummary,
  type NotificationChannelTypeSummary,
  type NotificationDeliveryAttemptSummary,
  type NotificationDeliveryQueueStatus,
  type NotificationMessageSummary,
  type NotificationPolicySummary,
  type NotificationTemplateSummary,
  type UpdateNotificationPolicyRequest,
} from '../api/notifications';
import { blankToNull, formatJson, parseJsonObject } from './notifications/jsonUtils';
import { ChannelDrawer } from './notifications/ChannelDrawer';
import { TemplateDrawer } from './notifications/TemplateDrawer';
import { notificationTemplateOptions, selectedPolicyProviders } from './notifications/templateCatalog';
import { PermissionGate } from '../components/Permission';
import { useRouteActive } from '../hooks/useRouteActivation';
import { useI18n } from '../i18n';
import { ROUTE_META } from '../routes';

const stateColor: Record<string, string> = {
  delivered: 'green',
  retry_pending: 'gold',
  dead_letter: 'red',
  retry_consumed: 'blue',
  pending: 'gold',
  failed: 'red',
};

const POLICY_OWNER_OPTIONS = ['global', 'namespace', 'app', 'job', 'workflow', 'workflow_node', 'alert_rule', 'worker_pool'];
const EVENT_FAMILY_OPTIONS = ['job_instance', 'workflow', 'alert', 'worker', 'script_governance'];
const SEVERITY_OPTIONS = ['info', 'warning', 'critical'];

interface PolicyFormValues {
  ownerType: string;
  ownerId?: string;
  name: string;
  eventFamily: string;
  eventFilterJsonText: string;
  channelIds: string[];
  templateRef?: string;
  severity: string;
  enabled: boolean;
  dedupeSeconds: number;
  throttleJsonText?: string;
  quietHoursJsonText?: string;
  escalationJsonText?: string;
}

function extractChannelIds(raw: string): string[] {
  try {
    const value = JSON.parse(raw) as unknown;
    if (Array.isArray(value)) {
      return value.flatMap((item) => {
        if (typeof item === 'string') return [item];
        if (item && typeof item === 'object') {
          const record = item as Record<string, unknown>;
          const id = record.channelId ?? record.channel_id ?? record.id;
          return typeof id === 'string' ? [id] : [];
        }
        return [];
      });
    }
    if (typeof value === 'string') return [value];
    if (value && typeof value === 'object') {
      const record = value as Record<string, unknown>;
      const id = record.channelId ?? record.channel_id ?? record.id;
      return typeof id === 'string' ? [id] : [];
    }
  } catch {
    return [];
  }
  return [];
}

export function NotificationCenterPage() {
  const active = useRouteActive(ROUTE_META.notifications.path);
  const { t } = useI18n();
  const [policyForm] = Form.useForm<PolicyFormValues>();
  const [channelTypes, setChannelTypes] = useState<NotificationChannelTypeSummary[]>([]);
  const [channels, setChannels] = useState<NotificationChannelSummary[]>([]);
  const [policies, setPolicies] = useState<NotificationPolicySummary[]>([]);
  const [templates, setTemplates] = useState<NotificationTemplateSummary[]>([]);
  const [messages, setMessages] = useState<NotificationMessageSummary[]>([]);
  const [status, setStatus] = useState<NotificationDeliveryQueueStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retrying, setRetrying] = useState(false);
  const [savingPolicy, setSavingPolicy] = useState(false);
  const [validatingPolicyId, setValidatingPolicyId] = useState<string | null>(null);
  const [channelDrawerOpen, setChannelDrawerOpen] = useState(false);
  const [policyDrawerOpen, setPolicyDrawerOpen] = useState(false);
  const [templateDrawerOpen, setTemplateDrawerOpen] = useState(false);
  const [editingChannel, setEditingChannel] = useState<NotificationChannelSummary | null>(null);
  const [editingPolicy, setEditingPolicy] = useState<NotificationPolicySummary | null>(null);
  const [editingTemplate, setEditingTemplate] = useState<NotificationTemplateSummary | null>(null);

  const refresh = useMemo(() => async () => {
    const [typesData, channelsData, policiesData, templatesData, messagesData, statusData] = await Promise.all([
      listNotificationChannelTypes(),
      listNotificationChannels(),
      listNotificationPolicies(),
      listNotificationTemplates(),
      listNotificationMessages(),
      getNotificationDeliveryQueueStatus(),
    ]);
    setChannelTypes(typesData);
    setChannels(channelsData);
    setPolicies(policiesData);
    setTemplates(templatesData);
    setMessages(messagesData.slice(0, 20));
    setStatus(statusData);
    setError(null);
  }, []);

  useEffect(() => {
    let mounted = true;
    if (!active) return undefined;
    void refresh().catch((cause: unknown) => {
      if (!mounted) return;
      setError(cause instanceof Error ? cause.message : String(cause));
    });
    return () => {
      mounted = false;
    };
  }, [active, refresh]);

  const runRetry = async () => {
    setRetrying(true);
    try {
      const result = await retryDueNotificationDeliveryAttempts({ limit: 50, maxAttempts: 3, backoffSeconds: 300 });
      message.success(`${t('通知中心重试扫描完成')}：${t('扫描')} ${result.scanned}，${t('已投递')} ${result.delivered}`);
      await refresh();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
    setRetrying(false);
    }
  };

  const openCreateChannel = () => {
    setEditingChannel(null);
    setChannelDrawerOpen(true);
  };

  const openEditChannel = (channel: NotificationChannelSummary) => {
    setEditingChannel(channel);
    setChannelDrawerOpen(true);
  };

  const closeChannelDrawer = () => {
    setChannelDrawerOpen(false);
    setEditingChannel(null);
  };

  const handleDeleteChannel = async (channel: NotificationChannelSummary) => {
    try {
      await deleteNotificationChannel(channel.id);
      message.success(t('通知渠道已删除'));
      await refresh();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const openCreateTemplate = () => {
    setEditingTemplate(null);
    setTemplateDrawerOpen(true);
  };

  const openEditTemplate = (template: NotificationTemplateSummary) => {
    setEditingTemplate(template);
    setTemplateDrawerOpen(true);
  };

  const closeTemplateDrawer = () => {
    setTemplateDrawerOpen(false);
    setEditingTemplate(null);
  };

  const handleDeleteTemplate = async (template: NotificationTemplateSummary) => {
    try {
      await deleteNotificationTemplate(template.id);
      message.success(t('通知模板已删除'));
      await refresh();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const openCreatePolicy = () => {
    setEditingPolicy(null);
    policyForm.resetFields();
    policyForm.setFieldsValue({
      ownerType: 'global',
      eventFamily: 'job_instance',
      eventFilterJsonText: '{\n  "eventTypes": ["job_instance.failed"],\n  "statuses": ["failed"]\n}',
      channelIds: [],
      severity: 'critical',
      enabled: true,
      dedupeSeconds: 300,
      throttleJsonText: '',
      quietHoursJsonText: '',
      escalationJsonText: '',
    });
    setPolicyDrawerOpen(true);
  };

  const openEditPolicy = (policy: NotificationPolicySummary) => {
    setEditingPolicy(policy);
    policyForm.resetFields();
    policyForm.setFieldsValue({
      ownerType: policy.ownerType,
      ownerId: policy.ownerId ?? undefined,
      name: policy.name,
      eventFamily: policy.eventFamily,
      eventFilterJsonText: formatJson(policy.eventFilterJson),
      channelIds: extractChannelIds(policy.channelRefsJson),
      templateRef: policy.templateRef ?? undefined,
      severity: policy.severity,
      enabled: policy.enabled,
      dedupeSeconds: policy.dedupeSeconds,
      throttleJsonText: formatJson(policy.throttleJson, ''),
      quietHoursJsonText: formatJson(policy.quietHoursJson, ''),
      escalationJsonText: formatJson(policy.escalationJson, ''),
    });
    setPolicyDrawerOpen(true);
  };

  const closePolicyDrawer = () => {
    setPolicyDrawerOpen(false);
    setEditingPolicy(null);
    policyForm.resetFields();
  };

  const policyPayload = (values: PolicyFormValues): CreateNotificationPolicyRequest => ({
    ownerType: values.ownerType,
    ownerId: blankToNull(values.ownerId),
    name: values.name,
    eventFamily: values.eventFamily,
    eventFilter: parseJsonObject(values.eventFilterJsonText, t('事件过滤 JSON'), {}) ?? {},
    channelRefs: values.channelIds.map((channelId) => ({ channelId })),
    templateRef: blankToNull(values.templateRef),
    severity: values.severity,
    enabled: values.enabled,
    dedupeSeconds: values.dedupeSeconds,
  });

  const advancedPolicyPatch = (values: PolicyFormValues): UpdateNotificationPolicyRequest => ({
    throttle: parseJsonObject(values.throttleJsonText, t('限流 JSON'), null),
    quietHours: parseJsonObject(values.quietHoursJsonText, t('静默时段 JSON'), null),
    escalation: parseJsonObject(values.escalationJsonText, t('升级 JSON'), null),
  });

  const handlePolicySubmit = async (values: PolicyFormValues) => {
    setSavingPolicy(true);
    try {
      if (editingPolicy) {
        await updateNotificationPolicy(editingPolicy.id, {
          ...policyPayload(values),
          ...advancedPolicyPatch(values),
        });
        message.success(t('通知策略已更新'));
      } else {
        const created = await createNotificationPolicy(policyPayload(values));
        const advanced = advancedPolicyPatch(values);
        if (advanced.throttle || advanced.quietHours || advanced.escalation) {
          await updateNotificationPolicy(created.id, advanced);
        }
        message.success(t('通知策略已创建'));
      }
      closePolicyDrawer();
      await refresh();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSavingPolicy(false);
    }
  };

  const handleDeletePolicy = async (policy: NotificationPolicySummary) => {
    try {
      await deleteNotificationPolicy(policy.id);
      message.success(t('通知策略已删除'));
      await refresh();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const handleValidatePolicy = async (policy: NotificationPolicySummary) => {
    setValidatingPolicyId(policy.id);
    try {
      const result = await validateNotificationPolicy(policy.id);
      if (result.valid) {
        message.success(`${t('校验通过')}：${result.channelCount} ${t('个渠道')}`);
      } else {
        message.error(`${t('校验失败')}：${result.issues.join('; ')}`);
      }
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setValidatingPolicyId(null);
    }
  };

  const selectedPolicyChannelIds = Form.useWatch('channelIds', policyForm);
  const channelOptions = channels.map((channel) => ({
    value: channel.id,
    label: `${channel.name} · ${channel.provider} · ${channel.targetRedacted}`,
    disabled: !channel.enabled,
  }));
  const policyProviderFilter = selectedPolicyProviders(channels, selectedPolicyChannelIds);
  const templateRefOptions = notificationTemplateOptions(templates, policyProviderFilter);

  return (
    <Space direction="vertical" size={20} style={{ width: '100%' }}>
      <div>
        <Typography.Title level={2}>{t('通知中心')}</Typography.Title>
        <Typography.Text type="secondary">
          {t('管理可复用出站渠道、策略、消息与通用投递队列；提供方目标已脱敏，密钥引用不会在响应中展示。')}
        </Typography.Text>
      </div>
      {error ? <Alert type="error" showIcon message={t('通知中心加载失败')} description={error} /> : null}
      <Row gutter={[16, 16]}>
        <Col xs={12} md={6}><Card><Statistic title={t('渠道')} value={channels.length} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title={t('策略')} value={policies.length} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title={t('模板')} value={templates.length} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title={t('待重试')} value={status?.retryPending ?? 0} valueStyle={{ color: '#d48806' }} /></Card></Col>
        <Col xs={12} md={6}><Card><Statistic title={t('死信队列')} value={status?.deadLetter ?? 0} valueStyle={{ color: '#cf1322' }} /></Card></Col>
      </Row>
      <Tabs
        items={[
          {
            key: 'channels',
            label: t('渠道'),
            children: (
              <Card
                title={t('通知渠道')}
                extra={<Space wrap><Tag>{channelTypes.length} {t('种提供方类型')}</Tag><PermissionGate resource="notifications" action="manage"><Button type="primary" onClick={openCreateChannel}>{t('新建渠道')}</Button></PermissionGate></Space>}
              >
                <Table<NotificationChannelSummary>
                  rowKey="id"
                  dataSource={channels}
                  pagination={{ pageSize: 8 }}
                  columns={[
                    { title: t('名称'), dataIndex: 'name' },
                    { title: t('提供方'), dataIndex: 'provider', render: (value: string) => <Tag>{value}</Tag> },
                    { title: t('作用域'), render: (_, row) => `${row.scopeType}${row.namespace ? `/${row.namespace}` : ''}${row.app ? `/${row.app}` : ''}` },
                    { title: t('目标'), dataIndex: 'targetRedacted' },
                    { title: t('密钥'), dataIndex: 'secretConfigured', render: (value: boolean) => <Tag color={value ? 'green' : 'default'}>{value ? t('已配置') : t('未配置')}</Tag> },
                    { title: t('启用'), dataIndex: 'enabled', render: (value: boolean) => <Tag color={value ? 'green' : 'red'}>{value ? t('是') : t('否')}</Tag> },
                    {
                      title: t('操作'),
                      render: (_, row) => (
                        <Space>
                          <PermissionGate resource="notifications" action="manage"><Button size="small" onClick={() => openEditChannel(row)}>{t('编辑')}</Button></PermissionGate>
                          <PermissionGate resource="notifications" action="manage">
                            <Popconfirm title={t('删除通知渠道')} description={t('被策略引用的渠道会被后端拒绝删除。')} onConfirm={() => void handleDeleteChannel(row)}>
                              <Button size="small" danger>{t('删除')}</Button>
                            </Popconfirm>
                          </PermissionGate>
                        </Space>
                      ),
                    },
                  ]}
                />
              </Card>
            ),
          },
          {
            key: 'templates',
            label: t('模板'),
            children: (
              <Card title={t('通知模板')} extra={<PermissionGate resource="notifications" action="manage"><Button type="primary" onClick={openCreateTemplate}>{t('新建模板')}</Button></PermissionGate>}>
                <Table<NotificationTemplateSummary>
                  rowKey="id"
                  dataSource={templates}
                  pagination={{ pageSize: 8 }}
                  columns={[
                    { title: t('模板 Key'), dataIndex: 'templateKey' },
                    { title: t('名称'), dataIndex: 'name' },
                    { title: t('提供方'), dataIndex: 'provider', render: (value: string) => <Tag>{value}</Tag> },
                    { title: t('消息类型'), dataIndex: 'messageType' },
                    { title: t('启用'), dataIndex: 'enabled', render: (value: boolean) => <Tag color={value ? 'green' : 'red'}>{value ? t('是') : t('否')}</Tag> },
                    { title: t('创建时间'), dataIndex: 'createdAt' },
                    {
                      title: t('操作'),
                      render: (_, row) => (
                        <Space>
                          <PermissionGate resource="notifications" action="manage"><Button size="small" onClick={() => openEditTemplate(row)}>{t('预览')}</Button></PermissionGate>
                          <PermissionGate resource="notifications" action="manage"><Button size="small" onClick={() => openEditTemplate(row)}>{t('编辑')}</Button></PermissionGate>
                          <PermissionGate resource="notifications" action="manage">
                            <Popconfirm title={t('删除通知模板')} description={t('删除后引用该模板的策略可能需要改用其他模板引用。')} onConfirm={() => void handleDeleteTemplate(row)}>
                              <Button size="small" danger>{t('删除')}</Button>
                            </Popconfirm>
                          </PermissionGate>
                        </Space>
                      ),
                    },
                  ]}
                />
              </Card>
            ),
          },
          {
            key: 'policies',
            label: t('策略'),
            children: (
              <Card title={t('通知策略')} extra={<PermissionGate resource="notifications" action="manage"><Button type="primary" onClick={openCreatePolicy}>{t('新建策略')}</Button></PermissionGate>}>
                <Table<NotificationPolicySummary>
                  rowKey="id"
                  dataSource={policies}
                  pagination={{ pageSize: 8 }}
                  columns={[
                    { title: t('名称'), dataIndex: 'name' },
                    { title: t('所有者'), render: (_, row) => `${row.ownerType}:${row.ownerId ?? '*'}` },
                    { title: t('事件族'), dataIndex: 'eventFamily' },
                    { title: t('级别'), dataIndex: 'severity', render: (value: string) => <Tag>{value}</Tag> },
                    { title: t('去重窗口'), dataIndex: 'dedupeSeconds' },
                    { title: t('启用'), dataIndex: 'enabled', render: (value: boolean) => <Tag color={value ? 'green' : 'red'}>{value ? t('是') : t('否')}</Tag> },
                    {
                      title: t('操作'),
                      render: (_, row) => (
                        <Space>
                          <Button size="small" loading={validatingPolicyId === row.id} onClick={() => void handleValidatePolicy(row)}>{t('校验')}</Button>
                          <PermissionGate resource="notifications" action="manage"><Button size="small" onClick={() => openEditPolicy(row)}>{t('编辑')}</Button></PermissionGate>
                          <PermissionGate resource="notifications" action="manage">
                            <Popconfirm title={t('删除通知策略')} description={t('删除后不会再为匹配事件生成新消息。')} onConfirm={() => void handleDeletePolicy(row)}>
                              <Button size="small" danger>{t('删除')}</Button>
                            </Popconfirm>
                          </PermissionGate>
                        </Space>
                      ),
                    },
                  ]}
                />
              </Card>
            ),
          },
          {
            key: 'delivery',
            label: t('投递'),
            children: (
              <Space direction="vertical" size={16} style={{ width: '100%' }}>
                <Card title={t('投递队列')} extra={<PermissionGate resource="notifications" action="test"><Button loading={retrying} onClick={runRetry}>{t('重试到期投递')}</Button></PermissionGate>}>
                  <Row gutter={[16, 16]}>
                    <Col xs={12} md={4}><Statistic title={t('总数')} value={status?.totalAttempts ?? 0} /></Col>
                    <Col xs={12} md={4}><Statistic title={t('已投递')} value={status?.delivered ?? 0} valueStyle={{ color: '#389e0d' }} /></Col>
                    <Col xs={12} md={4}><Statistic title={t('待重试')} value={status?.retryPending ?? 0} valueStyle={{ color: '#d48806' }} /></Col>
                    <Col xs={12} md={4}><Statistic title={t('重试已消费')} value={status?.retryConsumed ?? 0} /></Col>
                    <Col xs={12} md={4}><Statistic title={t('死信队列')} value={status?.deadLetter ?? 0} valueStyle={{ color: '#cf1322' }} /></Col>
                    <Col xs={12} md={4}><Statistic title={t('失败')} value={status?.failed ?? 0} valueStyle={{ color: '#cf1322' }} /></Col>
                  </Row>
                </Card>
                <Card title={t('最近死信队列')}>
                  <Table<NotificationDeliveryAttemptSummary>
                    rowKey="id"
                    dataSource={status?.recentDeadLetters ?? []}
                    pagination={false}
                    columns={[
                      { title: t('提供方'), dataIndex: 'provider' },
                      { title: t('目标'), dataIndex: 'targetRedacted' },
                      { title: t('尝试次数'), dataIndex: 'attempt', width: 96 },
                      { title: t('状态'), dataIndex: 'retryState', render: (value: string) => <Tag color={stateColor[value] ?? 'default'}>{value}</Tag> },
                      { title: t('错误'), dataIndex: 'error', ellipsis: true },
                      { title: t('创建时间'), dataIndex: 'createdAt' },
                    ]}
                  />
                </Card>
              </Space>
            ),
          },
          {
            key: 'messages',
            label: t('消息'),
            children: (
              <Card title={t('最近消息')}>
                <Table<NotificationMessageSummary>
                  rowKey="id"
                  dataSource={messages}
                  pagination={{ pageSize: 8 }}
                  columns={[
                    { title: t('事件'), dataIndex: 'eventType' },
                    { title: t('资源'), render: (_, row) => `${row.resourceType}:${row.resourceId}` },
                    { title: t('主题'), dataIndex: 'subject', ellipsis: true },
                    { title: t('状态'), dataIndex: 'status', render: (value: string) => <Tag color={stateColor[value] ?? 'default'}>{value}</Tag> },
                    { title: t('创建时间'), dataIndex: 'createdAt' },
                  ]}
                />
              </Card>
            ),
          },
        ]}
      />
      <ChannelDrawer
        open={channelDrawerOpen}
        channelTypes={channelTypes}
        editingChannel={editingChannel}
        onClose={closeChannelDrawer}
        onSaved={refresh}
      />
      <TemplateDrawer
        open={templateDrawerOpen}
        channelTypes={channelTypes}
        editingTemplate={editingTemplate}
        onClose={closeTemplateDrawer}
        onSaved={refresh}
      />

      <Drawer title={editingPolicy ? t('编辑通知策略') : t('新建策略')} open={policyDrawerOpen} onClose={closePolicyDrawer} width={920} destroyOnClose>
        <Form form={policyForm} layout="vertical" onFinish={(values) => void handlePolicySubmit(values)}>
          <Row gutter={16}>
            <Col xs={24} md={12}><Form.Item name="name" label={t('名称')} rules={[{ required: true, message: t('请输入名称') }]}><Input placeholder="billing failed jobs" /></Form.Item></Col>
            <Col xs={24} md={12}><Form.Item name="enabled" label={t('启用')} valuePropName="checked"><Switch /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="ownerType" label={t('所有者类型')} rules={[{ required: true }]}><Select options={POLICY_OWNER_OPTIONS.map((value) => ({ value, label: value }))} /></Form.Item></Col>
            <Col xs={24} md={16}><Form.Item name="ownerId" label={t('所有者 ID')}><Input placeholder="prod/billing 或 job id" /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="eventFamily" label={t('事件族')} rules={[{ required: true }]}><Select options={EVENT_FAMILY_OPTIONS.map((value) => ({ value, label: value }))} /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="severity" label={t('级别')} rules={[{ required: true }]}><Select options={SEVERITY_OPTIONS.map((value) => ({ value, label: value }))} /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="dedupeSeconds" label={t('去重窗口')} rules={[{ required: true }]}><InputNumber min={0} max={86400} style={{ width: '100%' }} /></Form.Item></Col>
          </Row>
          <Form.Item name="channelIds" label={t('投递渠道')} rules={[{ required: true, message: t('请选择至少一个渠道') }]}>
            <Select mode="multiple" showSearch options={channelOptions} />
          </Form.Item>
          <Form.Item name="eventFilterJsonText" label={t('事件过滤 JSON')} rules={[{ required: true, message: t('请输入事件过滤 JSON') }]}>
            <Input.TextArea rows={7} spellCheck={false} />
          </Form.Item>
          <Form.Item name="templateRef" label={t('模板引用')} extra={t('只能选择已启用且与所选渠道提供方匹配的存储模板。')}>
            <Select allowClear showSearch options={templateRefOptions} placeholder="optional-template-ref" filterOption={(input, option) => String(option?.label ?? option?.value ?? '').toLowerCase().includes(input.toLowerCase())} />
          </Form.Item>
          <Row gutter={16}>
            <Col xs={24} md={8}><Form.Item name="throttleJsonText" label={t('限流 JSON')}><Input.TextArea rows={4} spellCheck={false} placeholder="{}" /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="quietHoursJsonText" label={t('静默时段 JSON')}><Input.TextArea rows={4} spellCheck={false} placeholder="{}" /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="escalationJsonText" label={t('升级 JSON')}><Input.TextArea rows={4} spellCheck={false} placeholder="{}" /></Form.Item></Col>
          </Row>
          <Space>
            <PermissionGate resource="notifications" action="manage"><Button type="primary" htmlType="submit" loading={savingPolicy}>{editingPolicy ? t('保存策略') : t('创建策略')}</Button></PermissionGate>
            <Button onClick={closePolicyDrawer}>{t('取消')}</Button>
          </Space>
        </Form>
      </Drawer>
    </Space>
  );
}
