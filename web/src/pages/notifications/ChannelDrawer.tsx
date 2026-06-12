import { Alert, AutoComplete, Button, Card, Col, Descriptions, Drawer, Form, Input, Row, Select, Space, Switch, Tag, Tooltip, Typography, message } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  listAppScopes,
  listNamespaces,
  listSecrets,
  listWorkerPools,
  type AppScopeSummary,
  type NamespaceSummary,
  type SecretSummary,
  type WorkerPoolSummary,
} from '../../api/client';
import {
  createNotificationChannel,
  testNotificationChannel,
  updateNotificationChannel,
  type CreateNotificationChannelRequest,
  type NotificationChannelSummary,
  type NotificationChannelTypeSummary,
  type TestNotificationChannelResult,
  type UpdateNotificationChannelRequest,
} from '../../api/notifications';
import { PermissionGate } from '../../components/Permission';
import { useI18n } from '../../i18n';
import { assertNoRedactedMarker, blankToNull, compactObject, formatJson, parseJsonObject, parseMaybeJson } from './jsonUtils';
import { findMessageType, providerSchemaFor, type ProviderFieldSchema, type ProviderSchema } from './providerSchema';

const CHANNEL_SCOPE_OPTIONS = ['global', 'namespace', 'app', 'worker_pool'];

interface ChannelDrawerProps {
  open: boolean;
  channelTypes: NotificationChannelTypeSummary[];
  editingChannel: NotificationChannelSummary | null;
  onClose: () => void;
  onSaved: () => Promise<void>;
}

interface ChannelFormValues {
  scopeType: string;
  namespace?: string;
  app?: string;
  workerPool?: string;
  name: string;
  provider: string;
  enabled: boolean;
  messageType: string;
  config?: Record<string, unknown>;
  secretRefs?: Record<string, unknown>;
  template?: Record<string, unknown>;
  useInlineTemplate?: boolean;
  advancedConfigJsonText?: string;
  advancedSecretRefsJsonText?: string;
  safetyPolicyJsonText?: string;
  replaceConfig?: boolean;
  replaceSecretRefs?: boolean;
}

function valueAsString(value: unknown): string | undefined {
  if (typeof value === 'string') return value;
  if (value === undefined || value === null) return undefined;
  return JSON.stringify(value, null, 2);
}

function readObject(raw: string | null | undefined): Record<string, unknown> {
  try {
    const parsed = JSON.parse(raw ?? '{}') as unknown;
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed as Record<string, unknown> : {};
  } catch {
    return {};
  }
}

function renderFieldInput(field: ProviderFieldSchema, disabled = false) {
  if (field.type === 'textarea') {
    return <Input.TextArea rows={field.rows ?? 4} spellCheck={false} placeholder={field.placeholder} disabled={disabled} />;
  }
  if (field.type === 'boolean') {
    return <Switch disabled={disabled} />;
  }
  if (field.type === 'tags' || field.type === 'emailList') {
    return <Select mode="tags" tokenSeparators={[',', ' ']} placeholder={field.placeholder} options={field.options} disabled={disabled} />;
  }
  if (field.type === 'select') {
    return <Select placeholder={field.placeholder} options={field.options} disabled={disabled} />;
  }
  return <Input placeholder={field.placeholder} disabled={disabled} />;
}

function keepExistingPlaceholder(field: ProviderFieldSchema, replacing: boolean | undefined): string | undefined {
  if (replacing) return field.placeholder;
  return field.secret ? '保持现有私密配置' : '保持现有渠道配置';
}

function fieldRequired(
  field: ProviderFieldSchema,
  editing: boolean,
  replacing: boolean | undefined,
): boolean {
  return Boolean(field.required && (!editing || replacing));
}

function previewValue(value: unknown): string {
  if (value === undefined || value === null || value === '') return '-';
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

function assertNoRedactedValue(value: unknown, fieldLabel: string) {
  if (typeof value === 'string') {
    if (value.includes('***redacted***') || /^https?:\/\/[^/]+\/\.\.\.$/.test(value) || value.endsWith(':secret-ref')) {
      throw new Error(`${fieldLabel} 包含脱敏占位符；编辑时请启用替换并填写完整新值，或保持现有渠道配置/保持现有私密配置。`);
    }
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item) => assertNoRedactedValue(item, fieldLabel));
    return;
  }
  if (value && typeof value === 'object') {
    Object.values(value).forEach((item) => assertNoRedactedValue(item, fieldLabel));
  }
}

function templatePreview(schema: ProviderSchema, messageType: string | undefined, values: Record<string, unknown> | undefined): string {
  const selected = findMessageType(schema, messageType);
  const merged = buildTemplate(messageType ?? schema.defaultMessageType, schema, values);
  const preview = selected.preview ?? merged;
  return JSON.stringify({ messageType: selected.id, ...preview, ...merged }, null, 2);
}

function channelTestDisabledReason(provider: string | undefined, messageType: string | undefined, editingChannel: NotificationChannelSummary | null, supportsTestSend: boolean): string | null {
  if (!supportsTestSend) return '该渠道类型不支持测试发送';
  if (editingChannel && !editingChannel.enabled) return '渠道未启用，不能发送测试通知。';
  if (editingChannel && !editingChannel.targetConfigured) return '渠道目标未配置，不能发送测试通知。';
  if (provider === 'feishu' && ['image', 'share_chat'].includes(messageType ?? '')) {
    return '飞书 image/share_chat 需要真实 image_key/share_chat_id，示例占位值不适合直接测试。';
  }
  if (provider === 'wechat_work' && ['image', 'file', 'voice'].includes(messageType ?? '')) {
    return '企业微信 image/file/voice 需要真实素材内容或 media_id，示例占位值不适合直接测试。';
  }
  return null;
}

function scopeHelp(scopeType: string | undefined): string {
  if (scopeType === 'global') return '全局渠道不绑定 Namespace/App/Worker Pool；所有匹配策略可引用。';
  if (scopeType === 'namespace') return '先选择 Namespace；渠道只服务该命名空间下的策略。';
  if (scopeType === 'app') return '先选择 Namespace，再从该 Namespace 下联动选择 App。';
  if (scopeType === 'worker_pool') return '先选择 Namespace 和 App，再联动选择 Worker Pool。';
  return '选择作用域后，Namespace、App、Worker Pool 与可选 Secret 候选会自动联动过滤。';
}

function mergeFieldValues(fields: ProviderFieldSchema[], values: Record<string, unknown> | undefined): Record<string, unknown> {
  const source = values ?? {};
  return compactObject(Object.fromEntries(fields.map((field) => [field.key, source[field.key]])));
}

function buildTemplate(messageType: string, schema: ProviderSchema, values: Record<string, unknown> | undefined): Record<string, unknown> {
  const selected = findMessageType(schema, messageType);
  const fieldValues = mergeFieldValues(selected.templateFields, values);
  return compactObject({ ...fieldValues, messageType });
}

function applyDefaults(form: ReturnType<typeof Form.useForm<ChannelFormValues>>[0], schema: ProviderSchema, config: Record<string, unknown> = {}, secretRefs: Record<string, unknown> = {}, template: Record<string, unknown> = {}) {
  const rawMessageType = valueAsString(config.messageType) ?? valueAsString(template.messageType) ?? schema.defaultMessageType;
  const messageType = schema.messageTypes.some((item) => item.id === rawMessageType) ? rawMessageType : schema.defaultMessageType;
  const selected = findMessageType(schema, messageType);
  form.setFieldsValue({
    messageType,
    config: Object.fromEntries(schema.configFields.map((field) => [field.key, config[field.key] ?? field.defaultValue])),
    secretRefs: Object.fromEntries(schema.secretFields.map((field) => [field.key, secretRefs[field.key] ?? field.defaultValue])),
    template: Object.fromEntries(selected.templateFields.map((field) => [field.key, template[field.key] ?? config[field.key] ?? field.defaultValue])),
  });
}

function secretOptionLabel(secret: SecretSummary): string {
  return `${secret.namespace}/${secret.app}/${secret.name}`;
}

function secretRuntimeRef(secret: SecretSummary): string | null {
  try {
    const parsed = JSON.parse(secret.valueRef) as unknown;
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      const record = parsed as Record<string, unknown>;
      if (record.kind === 'env' && typeof record.name === 'string' && record.name.trim()) {
        return `env:${record.name.trim()}`;
      }
    }
  } catch {
    return null;
  }
  return null;
}

export function ChannelDrawer({ open, channelTypes, editingChannel, onClose, onSaved }: ChannelDrawerProps) {
  const { t } = useI18n();
  const [form] = Form.useForm<ChannelFormValues>();
  const provider = Form.useWatch('provider', form);
  const scopeType = Form.useWatch('scopeType', form);
  const namespace = Form.useWatch('namespace', form);
  const app = Form.useWatch('app', form);
  const messageType = Form.useWatch('messageType', form);
  const replaceConfig = Form.useWatch('replaceConfig', form);
  const replaceSecretRefs = Form.useWatch('replaceSecretRefs', form);
  const [saving, setSaving] = useState(false);
  const [namespaces, setNamespaces] = useState<NamespaceSummary[]>([]);
  const [apps, setApps] = useState<AppScopeSummary[]>([]);
  const [workerPools, setWorkerPools] = useState<WorkerPoolSummary[]>([]);
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [testingChannel, setTestingChannel] = useState(false);
  const [testResult, setTestResult] = useState<TestNotificationChannelResult | null>(null);

  const currentType = channelTypes.find((item) => item.type === provider) ?? (provider ? undefined : channelTypes[0]);
  const schema = useMemo(() => providerSchemaFor(currentType, provider), [currentType, provider]);
  const selectedMessageType = useMemo(() => findMessageType(schema, messageType), [messageType, schema]);

  const loadScopeOptions = useCallback(async () => {
    const [namespaceData, appData, workerPoolData, secretData] = await Promise.all([
      listNamespaces(),
      listAppScopes(),
      listWorkerPools(),
      listSecrets(),
    ]);
    setNamespaces(namespaceData);
    setApps(appData);
    setWorkerPools(workerPoolData);
    setSecrets(secretData);
  }, []);

  useEffect(() => {
    if (!open) return;
    void loadScopeOptions().catch((cause) => message.error(cause instanceof Error ? cause.message : String(cause)));
  }, [loadScopeOptions, open]);

  const filteredApps = useMemo(() => apps.filter((item) => !namespace || item.namespace === namespace), [apps, namespace]);
  const filteredWorkerPools = useMemo(() => workerPools.filter((item) => (!namespace || item.namespace === namespace) && (!app || item.app === app)), [app, namespace, workerPools]);
  const filteredSecrets = useMemo(() => secrets.filter((item) => (!namespace || item.namespace === namespace) && (!app || item.app === app)), [app, namespace, secrets]);
  const scopedSecretOptions = filteredSecrets.flatMap((item) => {
    const value = secretRuntimeRef(item);
    return value ? [{ value, label: `${secretOptionLabel(item)} · ${value}` }] : [];
  });

  const providerOptions = channelTypes.map((item) => ({ value: item.type, label: `${item.label} · ${item.type}` }));
  const namespaceOptions = namespaces.map((item) => ({ value: item.name, label: item.name }));
  const appOptions = filteredApps.map((item) => ({ value: item.name, label: `${item.namespace}/${item.name}` }));
  const workerPoolOptions = filteredWorkerPools.map((item) => ({ value: item.name, label: `${item.namespace}/${item.app}/${item.name}` }));
  const appSelectDisabled = (scopeType === 'app' || scopeType === 'worker_pool') && !namespace;
  const workerPoolSelectDisabled = scopeType === 'worker_pool' && (!namespace || !app);
  const useInlineTemplate = Form.useWatch('useInlineTemplate', form);
  const currentTemplate = Form.useWatch('template', form);
  const renderedTemplatePreview = templatePreview(schema, messageType, currentTemplate);
  const configControlsDisabled = Boolean(editingChannel && !replaceConfig);
  const secretControlsDisabled = Boolean(editingChannel && !replaceSecretRefs);
  const testSendSupported = currentType?.supportsTestSend === true;
  const testDisabledReason = channelTestDisabledReason(provider, messageType, editingChannel, testSendSupported);

  const clearScopeDependents = (nextScopeType: string) => {
    if (nextScopeType === 'global') form.setFieldsValue({ namespace: undefined, app: undefined, workerPool: undefined, secretRefs: undefined });
    if (nextScopeType === 'namespace') form.setFieldsValue({ app: undefined, workerPool: undefined, secretRefs: undefined });
    if (nextScopeType === 'app') form.setFieldsValue({ workerPool: undefined, secretRefs: undefined });
    if (nextScopeType === 'worker_pool') form.setFieldsValue({ secretRefs: undefined });
  };

  useEffect(() => {
    if (!open) return;
    form.resetFields();
    if (editingChannel) {
      const config = readObject(editingChannel.configJson);
      const template = readObject(valueAsString(config.template));
      form.setFieldsValue({
        scopeType: editingChannel.scopeType,
        namespace: editingChannel.namespace ?? undefined,
        app: editingChannel.app ?? undefined,
        workerPool: editingChannel.workerPool ?? undefined,
        name: editingChannel.name,
        provider: editingChannel.provider,
        enabled: editingChannel.enabled,
          advancedConfigJsonText: formatJson(editingChannel.configJson),
          advancedSecretRefsJsonText: '',
          safetyPolicyJsonText: formatJson(editingChannel.safetyPolicyJson, ''),
          replaceConfig: false,
          replaceSecretRefs: false,
          useInlineTemplate: Boolean(config.template),
        });
      applyDefaults(form, providerSchemaFor(channelTypes.find((item) => item.type === editingChannel.provider), editingChannel.provider), config, {}, template);
    } else {
      const defaultType = channelTypes[0]?.type ?? 'webhook';
      const defaultSchema = providerSchemaFor(channelTypes[0], defaultType);
      form.setFieldsValue({
        scopeType: 'global',
        provider: defaultType,
        enabled: true,
        safetyPolicyJsonText: '',
        advancedConfigJsonText: '{}',
        advancedSecretRefsJsonText: '{}',
        replaceConfig: true,
        replaceSecretRefs: true,
        useInlineTemplate: false,
      });
      applyDefaults(form, defaultSchema);
    }
  }, [channelTypes, editingChannel, form, open]);

  useEffect(() => {
    if (!open || !provider) return;
    const currentConfig = form.getFieldValue('config') ?? {};
    const currentTemplate = form.getFieldValue('template') ?? {};
    const currentMessageType = form.getFieldValue('messageType');
    applyDefaults(
      form,
      schema,
      { ...currentConfig, messageType: currentMessageType },
      form.getFieldValue('secretRefs'),
      currentTemplate,
    );
  }, [form, open, provider, schema]);

  useEffect(() => {
    if (!open || !messageType) return;
    const existingTemplate = form.getFieldValue('template') ?? {};
    const selected = findMessageType(schema, messageType);
    form.setFieldsValue({
      template: Object.fromEntries(selected.templateFields.map((field) => [field.key, existingTemplate[field.key] ?? field.defaultValue])),
    });
  }, [form, messageType, open, schema]);

  const close = () => {
    form.resetFields();
    setTestResult(null);
    onClose();
  };

  const sendTestNotification = async () => {
    if (!editingChannel) return;
    setTestingChannel(true);
    setTestResult(null);
    try {
      const sample = selectedMessageType?.examples?.[0]?.sample ?? {};
      const result = await testNotificationChannel(editingChannel.id, {
        subject: valueAsString(sample.subject) ?? 'Tikeo notification channel test',
        body: valueAsString(sample.body) ?? 'This is a test notification sent from the channel edit drawer.',
        eventType: valueAsString(sample.eventType) ?? 'notification.channel_test',
        resourceType: valueAsString(sample.resourceType) ?? 'notification_channel',
        resourceId: valueAsString(sample.resourceId) ?? editingChannel.id,
        severity: valueAsString(sample.severity) ?? 'info',
        payload: typeof sample === 'object' && !Array.isArray(sample) ? sample : {},
      });
      setTestResult(result);
      if (result.delivered) {
        message.success(t('测试通知已被提供方接收'));
      } else {
        message.warning(t('测试通知未送达，请查看测试结果详情'));
      }
    } catch (cause) {
      const fallback: TestNotificationChannelResult = {
        channelId: editingChannel.id,
        messageId: '-',
        attemptId: '-',
        provider: editingChannel.provider,
        targetRedacted: editingChannel.targetRedacted,
        delivered: false,
        statusCode: null,
        retryState: 'request_failed',
        error: cause instanceof Error ? cause.message : String(cause),
        renderedPayload: null,
        createdAt: new Date().toISOString(),
      };
      setTestResult(fallback);
      message.error(fallback.error ?? t('测试通知发送失败'));
    } finally {
      setTestingChannel(false);
    }
  };

  const submit = async (values: ChannelFormValues) => {
    setSaving(true);
    try {
      if (editingChannel && values.provider !== editingChannel.provider && (!values.replaceConfig || !values.replaceSecretRefs)) {
        throw new Error(t('切换提供方时必须同时替换渠道配置和私密配置，避免旧 provider 配置误用。'));
      }
      const fieldConfig = mergeFieldValues(schema.configFields, values.config);
      const secretRefs = mergeFieldValues(schema.secretFields, values.secretRefs);
      const config = compactObject({
        ...fieldConfig,
        messageType: values.messageType,
        ...(values.useInlineTemplate ? { template: buildTemplate(values.messageType, schema, values.template) } : {}),
      });
      const advancedConfig = parseJsonObject(values.advancedConfigJsonText, t('高级配置 JSON'), {}) ?? {};
      if (!values.useInlineTemplate) {
        delete advancedConfig.template;
      }
      const advancedSecretRefs = parseJsonObject(values.advancedSecretRefsJsonText, t('高级私密配置 JSON'), {}) ?? {};
      assertNoRedactedMarker(values.advancedConfigJsonText, t('高级配置 JSON'));
      assertNoRedactedMarker(values.advancedSecretRefsJsonText, t('高级私密配置 JSON'));
      if (!editingChannel || values.replaceConfig) assertNoRedactedValue({ ...advancedConfig, ...config }, t('渠道配置'));
      if (!editingChannel || values.replaceSecretRefs) assertNoRedactedValue({ ...advancedSecretRefs, ...secretRefs }, t('私密配置'));
      const payloadBase = {
        scopeType: values.scopeType,
        namespace: blankToNull(values.namespace),
        app: blankToNull(values.app),
        workerPool: blankToNull(values.workerPool),
        name: values.name,
        provider: values.provider,
        enabled: values.enabled,
      };
      const safetyPolicy = parseJsonObject(values.safetyPolicyJsonText, t('安全策略 JSON'), null);
      if (editingChannel) {
        const payload: UpdateNotificationChannelRequest = {
          ...payloadBase,
          safetyPolicy,
        };
        if (values.replaceConfig) payload.config = { ...advancedConfig, ...config };
        if (values.replaceSecretRefs) payload.secretRefs = { ...advancedSecretRefs, ...secretRefs };
        await updateNotificationChannel(editingChannel.id, payload);
        message.success(t('通知渠道已更新'));
      } else {
        const payload: CreateNotificationChannelRequest = {
          ...payloadBase,
          config: { ...advancedConfig, ...config },
          secretRefs: { ...advancedSecretRefs, ...secretRefs },
          safetyPolicy,
        };
        await createNotificationChannel(payload);
        message.success(t('通知渠道已创建'));
      }
      close();
      await onSaved();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Drawer title={editingChannel ? t('编辑通知渠道') : t('新建通知渠道')} open={open} onClose={close} width={1040} destroyOnClose>
      <Form form={form} layout="vertical" onFinish={(values) => void submit(values)}>
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Card
            size="small"
            title={t('1. 基本信息')}
            extra={<Form.Item name="enabled" label={t('启用')} valuePropName="checked" style={{ marginBottom: 0 }}><Switch /></Form.Item>}
          >
            <Row gutter={16}>
              <Col xs={24} md={12}><Form.Item name="name" label={t('渠道名称')} rules={[{ required: true, message: t('请输入名称') }]}><Input placeholder="feishu-interactive-card-prod" /></Form.Item></Col>
              <Col xs={24} md={12}><Form.Item name="provider" label={t('提供方')} rules={[{ required: true }]}><Select showSearch options={providerOptions} /></Form.Item></Col>
              <Col xs={24} md={8}><Form.Item name="scopeType" label={t('作用域类型')} rules={[{ required: true }]}><Select options={CHANNEL_SCOPE_OPTIONS.map((value) => ({ value, label: value }))} onChange={clearScopeDependents} /></Form.Item></Col>
              {scopeType !== 'global' ? <Col xs={24} md={8}><Form.Item name="namespace" label={t('Namespace')} rules={[{ required: scopeType === 'namespace' || scopeType === 'app' || scopeType === 'worker_pool', message: t('请选择命名空间') }]}><Select showSearch options={namespaceOptions} onChange={() => form.setFieldsValue({ app: undefined, workerPool: undefined, secretRefs: undefined })} /></Form.Item></Col> : null}
              {scopeType === 'app' || scopeType === 'worker_pool' ? <Col xs={24} md={8}><Form.Item name="app" label={t('App')} rules={[{ required: true, message: t('请选择应用') }]}><Select showSearch disabled={appSelectDisabled} placeholder={appSelectDisabled ? t('先选择 Namespace') : undefined} options={appOptions} onChange={() => form.setFieldsValue({ workerPool: undefined, secretRefs: undefined })} /></Form.Item></Col> : null}
              {scopeType === 'worker_pool' ? <Col xs={24} md={8}><Form.Item name="workerPool" label={t('Worker Pool')} rules={[{ required: true, message: t('请选择 Worker Pool') }]}><Select showSearch disabled={workerPoolSelectDisabled} placeholder={workerPoolSelectDisabled ? t('先选择 Namespace 和 App') : undefined} options={workerPoolOptions} /></Form.Item></Col> : null}
            </Row>
            <Alert type="success" showIcon message={t('作用域联动')} description={t(scopeHelp(scopeType))} />
          </Card>

          <Card size="small" title={t('2. 渠道类型与消息格式')}>
            <Row gutter={16}>
              <Col xs={24} md={10}><Form.Item name="messageType" label={t('消息类型')} rules={[{ required: true }]} extra={editingChannel ? t('编辑时切换消息类型会自动开启替换渠道配置，保存后立即生效。') : undefined}><Select options={schema.messageTypes.map((item) => ({ value: item.id, label: `${item.label} · ${item.id}` }))} onChange={() => editingChannel ? form.setFieldValue('replaceConfig', true) : undefined} /></Form.Item></Col>
              <Col xs={24} md={14}>
                <Descriptions size="small" bordered column={1} items={[
                  { key: 'provider', label: t('提供方结构'), children: <Space wrap><Tag>{schema.category}</Tag><Typography.Text>{schema.description}</Typography.Text></Space> },
                  { key: 'message', label: t('消息类型说明'), children: selectedMessageType?.description ?? '-' },
                ]} />
              </Col>
            </Row>
            <Space direction="vertical" style={{ width: '100%' }}>
              <Space wrap>{schema.messageTypes.map((item) => <Tag key={item.id} color={item.id === messageType ? 'blue' : 'default'}>{item.label} · {item.id}</Tag>)}</Space>
              <Space wrap><Typography.Text type="secondary">{t('官方文档')}</Typography.Text>{schema.docs.map((doc) => <Typography.Link key={doc.url} href={doc.url} target="_blank" rel="noreferrer">{doc.label}</Typography.Link>)}</Space>
            </Space>
          </Card>

          <Card size="small" title={t('3. 机器人地址 / 私密凭据')}>
            <Alert
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
              message={t('每一条渠道单独保存自己的真实连接信息')}
              description={t('机器人/Webhook 地址、Signing secret、routing key、SMTP URL/password、appId/appSecret 等都在这里直接填写；保存后立即生效，响应不会回显 secretRefsJson。也兼容 env:NAME，但不是必须。')}
            />
            {editingChannel ? <Form.Item name="replaceSecretRefs" label={t('替换私密配置')} valuePropName="checked" extra={t('不开启时会保持服务端已有私密值；开启后必须填写完整新值。')}><Switch /></Form.Item> : null}
            <Row gutter={16}>
              {schema.secretFields.map((field) => (
                <Col xs={24} md={12} key={field.key}>
                  <Form.Item name={['secretRefs', field.key]} label={t(field.label)} rules={[{ required: fieldRequired(field, Boolean(editingChannel), replaceSecretRefs), message: t('请填写私密配置') }]} extra={field.help ? t(field.help) : t('可直接填写本渠道真实值；也可从当前 scope 的 Secret 候选选择 env:NAME。')}>
                    <AutoComplete allowClear disabled={secretControlsDisabled} options={scopedSecretOptions} placeholder={keepExistingPlaceholder(field, !editingChannel || replaceSecretRefs)} filterOption={(input, option) => String(option?.label ?? option?.value ?? '').toLowerCase().includes(input.toLowerCase())} />
                  </Form.Item>
                </Col>
              ))}
            </Row>
          </Card>

          <Card size="small" title={t('4. 渠道参数')}>
            {editingChannel ? <Form.Item name="replaceConfig" label={t('替换渠道配置')} valuePropName="checked" extra={t('不开启时只保存基本信息/启用状态/作用域，不覆盖已保存的渠道 configJson。')}><Switch /></Form.Item> : null}
            {schema.configFields.length > 0 ? (
              <Row gutter={16}>
                {schema.configFields.map((field) => (
                  <Col xs={24} md={field.type === 'textarea' ? 24 : 12} key={field.key}>
                    <Form.Item name={['config', field.key]} label={t(field.label)} valuePropName={field.type === 'boolean' ? 'checked' : 'value'} rules={[{ required: fieldRequired(field, Boolean(editingChannel), replaceConfig), message: t('请填写必填配置') }]} extra={field.help ? t(field.help) : undefined}>
                      {renderFieldInput({ ...field, placeholder: keepExistingPlaceholder(field, !editingChannel || replaceConfig) }, configControlsDisabled)}
                    </Form.Item>
                  </Col>
                ))}
              </Row>
            ) : <Typography.Text type="secondary">{t('当前提供方没有额外渠道参数。')}</Typography.Text>}
          </Card>

          <Card size="small" title={t('5. 消息内容模板')}>
            <Form.Item name="useInlineTemplate" label={t('渠道级 inline 模板覆盖')} valuePropName="checked" extra={t(configControlsDisabled ? '开启替换渠道配置后才能修改消息类型和 inline 模板字段。' : '默认关闭：策略引用的已启用存储模板会在运行时优先生效；只有需要此渠道固定覆盖策略模板时才开启。')}>
              <Switch disabled={configControlsDisabled} />
            </Form.Item>
            <Row gutter={16}>
              {selectedMessageType?.templateFields.map((field) => (
                <Col xs={24} md={field.type === 'textarea' ? 24 : 12} key={field.key}>
                  <Form.Item name={['template', field.key]} label={t(field.label)} valuePropName={field.type === 'boolean' ? 'checked' : 'value'} rules={[{ required: fieldRequired(field, Boolean(editingChannel), replaceConfig) && Boolean(useInlineTemplate), message: t('请填写模板字段') }]}> 
                    {renderFieldInput(field, configControlsDisabled)}
                  </Form.Item>
                </Col>
              ))}
            </Row>
            <Row gutter={16}>
              <Col xs={24} md={12}>
                <Card size="small" title={t('模板预览')}>
                  <Input.TextArea rows={10} readOnly value={previewValue(renderedTemplatePreview)} />
                </Card>
              </Col>
              <Col xs={24} md={12}>
                <Card size="small" title={t('可用模板变量')}>
                  <Space wrap>{schema.templateVariables.map((variable) => <Tag key={variable}>{variable}</Tag>)}</Space>
                </Card>
              </Col>
            </Row>
          </Card>

          {editingChannel ? (
            <Card
              size="small"
              title={t('6. 测试')}
              extra={<PermissionGate resource="notifications" action="test"><Tooltip title={testDisabledReason ? t(testDisabledReason) : t('测试')}><Button type="primary" size="small" disabled={Boolean(testDisabledReason) || testingChannel} loading={testingChannel} onClick={() => void sendTestNotification()}>{t('测试')}</Button></Tooltip></PermissionGate>}
            >
              <Typography.Paragraph type="secondary">{t('测试使用服务端已保存的渠道配置真实发送一条测试通知，并记录 message/attempt；返回结果只展示脱敏目标和脱敏后的渲染 payload，并包含状态码、错误、message/attempt 等详细字段。未保存的表单变更请先保存后再测试。')}</Typography.Paragraph>
              {testDisabledReason ? <Alert type="warning" showIcon style={{ marginBottom: 12 }} message={t(testDisabledReason)} /> : null}
              {testResult ? (
                <Space direction="vertical" style={{ width: '100%' }}>
                  <Descriptions size="small" bordered column={1} title={t('测试结果')} items={[
                    { key: 'delivered', label: t('delivered'), children: String(testResult.delivered) },
                    { key: 'provider', label: t('provider'), children: testResult.provider },
                    { key: 'targetRedacted', label: t('targetRedacted'), children: testResult.targetRedacted },
                    { key: 'statusCode', label: t('statusCode'), children: testResult.statusCode ?? '-' },
                    { key: 'retryState', label: t('retryState'), children: testResult.retryState },
                    { key: 'messageId', label: t('messageId'), children: testResult.messageId },
                    { key: 'attemptId', label: t('attemptId'), children: testResult.attemptId },
                    { key: 'createdAt', label: t('createdAt'), children: testResult.createdAt },
                    { key: 'error', label: t('error'), children: testResult.error ?? '-' },
                  ]} />
                  <Input.TextArea rows={8} readOnly value={previewValue({ renderedPayload: testResult.renderedPayload })} />
                </Space>
              ) : null}
            </Card>
          ) : null}

          <Card size="small" title={t('高级 JSON / 安全策略')}>
            <Row gutter={16}>
              <Col xs={24} md={12}><Form.Item name="advancedConfigJsonText" label={t('高级配置 JSON')} extra={t(configControlsDisabled ? '开启替换渠道配置后才能修改高级配置 JSON。' : '仅用于保留 provider 特殊字段；表单字段会覆盖同名键。')}><Input.TextArea rows={4} spellCheck={false} disabled={configControlsDisabled} onBlur={(event) => { const value = parseMaybeJson(event.target.value); if (value && typeof value === 'object') form.setFieldValue('advancedConfigJsonText', JSON.stringify(value, null, 2)); }} /></Form.Item></Col>
              <Col xs={24} md={12}><Form.Item name="advancedSecretRefsJsonText" label={t('高级私密配置 JSON')} extra={t(secretControlsDisabled ? '开启替换私密配置后才能修改高级私密配置 JSON。' : '填写本渠道私密配置；可直接填写真实值，也可填写 env:NAME 兼容引用。响应不会回显 secretRefsJson。')}><Input.TextArea rows={4} spellCheck={false} disabled={secretControlsDisabled} placeholder="{}" /></Form.Item></Col>
              <Col xs={24}><Form.Item name="safetyPolicyJsonText" label={t('安全策略 JSON')}><Input.TextArea rows={4} spellCheck={false} placeholder="{}" /></Form.Item></Col>
            </Row>
          </Card>

          <Card size="small">
            <Space>
              <PermissionGate resource="notifications" action="manage"><Button type="primary" htmlType="submit" loading={saving}>{editingChannel ? t('保存渠道') : t('创建渠道')}</Button></PermissionGate>
              <Button onClick={close}>{t('取消')}</Button>
            </Space>
          </Card>
        </Space>
      </Form>
    </Drawer>
  );
}
