import { Alert, AutoComplete, Button, Card, Col, Descriptions, Drawer, Form, Input, Row, Select, Space, Switch, Tag, Typography, message } from 'antd';
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
import { channelExampleCount, findMessageType, providerSchemaFor, type ProviderFieldSchema, type ProviderMessageTypeExample, type ProviderSchema } from './providerSchema';

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
  return field.secret ? '保持现有密钥引用' : '保持现有渠道配置';
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
      throw new Error(`${fieldLabel} 包含脱敏占位符；编辑时请启用替换并填写完整新值，或保持现有渠道配置/保持现有密钥引用。`);
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

function scopeHelp(scopeType: string | undefined): string {
  if (scopeType === 'global') return '全局渠道不绑定 Namespace/App/Worker Pool；所有匹配策略可引用。';
  if (scopeType === 'namespace') return '先选择 Namespace；渠道只服务该命名空间下的策略。';
  if (scopeType === 'app') return '先选择 Namespace，再从该 Namespace 下联动选择 App。';
  if (scopeType === 'worker_pool') return '先选择 Namespace 和 App，再联动选择 Worker Pool。';
  return '选择作用域后，Namespace、App、Worker Pool 与 Secret 候选会自动联动过滤。';
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

function exampleTemplateFieldValue(field: ProviderFieldSchema, template: Record<string, unknown>): unknown {
  const value = template[field.key] ?? field.defaultValue;
  if (field.type === 'textarea' && value !== undefined && typeof value !== 'string') {
    return JSON.stringify(value, null, 2);
  }
  return value;
}

function exampleFieldValue(field: ProviderFieldSchema, example: ProviderMessageTypeExample): unknown {
  return exampleTemplateFieldValue(field, example.template ?? {});
}

function applyDefaults(form: ReturnType<typeof Form.useForm<ChannelFormValues>>[0], schema: ProviderSchema, config: Record<string, unknown> = {}, secretRefs: Record<string, unknown> = {}, template: Record<string, unknown> = {}) {
  const messageType = valueAsString(config.messageType) ?? valueAsString(template.messageType) ?? schema.defaultMessageType;
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
  const [selectedExampleName, setSelectedExampleName] = useState<string>();
  const [testingChannel, setTestingChannel] = useState(false);
  const [testResult, setTestResult] = useState<TestNotificationChannelResult | null>(null);

  const currentType = channelTypes.find((item) => item.type === provider) ?? (provider ? undefined : channelTypes[0]);
  const schema = useMemo(() => providerSchemaFor(currentType, provider), [currentType, provider]);
  const selectedMessageType = useMemo(() => findMessageType(schema, messageType), [messageType, schema]);
  const selectedExamples = selectedMessageType?.examples ?? [];
  const selectedExample = selectedExamples.find((item) => item.name === selectedExampleName) ?? selectedExamples[0];

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
    applyDefaults(form, schema, form.getFieldValue('config'), form.getFieldValue('secretRefs'), form.getFieldValue('template'));
  }, [form, open, provider, schema]);

  useEffect(() => {
    if (!open || !messageType) return;
    const existingTemplate = form.getFieldValue('template') ?? {};
    const selected = findMessageType(schema, messageType);
    form.setFieldsValue({
      template: Object.fromEntries(selected.templateFields.map((field) => [field.key, existingTemplate[field.key] ?? field.defaultValue])),
    });
    setSelectedExampleName(selected.examples?.[0]?.name);
  }, [form, messageType, open, schema]);

  const close = () => {
    form.resetFields();
    setTestResult(null);
    setSelectedExampleName(undefined);
    onClose();
  };

  const applyExample = (example: ProviderMessageTypeExample | undefined) => {
    if (!example) return;
    const nextMessageType = valueAsString(example.template?.messageType) ?? valueAsString(example.config?.messageType) ?? selectedMessageType?.id ?? schema.defaultMessageType;
    const nextSchema = findMessageType(schema, nextMessageType);
    const currentConfig = form.getFieldValue('config') ?? {};
    const currentSecretRefs = form.getFieldValue('secretRefs') ?? {};
    const nextTemplate = Object.fromEntries(nextSchema.templateFields.map((field) => [field.key, exampleFieldValue(field, example)]));
    form.setFieldsValue({
      messageType: nextMessageType,
      config: { ...currentConfig, ...example.config },
      secretRefs: { ...currentSecretRefs, ...example.secretRefs },
      template: nextTemplate,
      useInlineTemplate: true,
      replaceConfig: true,
      replaceSecretRefs: true,
    });
    setSelectedExampleName(example.name);
    message.success(t('示例已套用；发送前请确认 env: 密钥引用已经在部署环境中配置。'));
  };

  const sendTestNotification = async () => {
    if (!editingChannel) return;
    setTestingChannel(true);
    setTestResult(null);
    try {
      const sample = selectedExample?.sample ?? {};
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
        throw new Error(t('切换提供方时必须同时替换渠道配置和密钥引用，避免旧 provider 配置误用。'));
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
      const advancedSecretRefs = parseJsonObject(values.advancedSecretRefsJsonText, t('高级密钥引用对象'), {}) ?? {};
      assertNoRedactedMarker(values.advancedConfigJsonText, t('高级配置 JSON'));
      assertNoRedactedMarker(values.advancedSecretRefsJsonText, t('高级密钥引用对象'));
      if (!editingChannel || values.replaceConfig) assertNoRedactedValue({ ...advancedConfig, ...config }, t('渠道配置'));
      if (!editingChannel || values.replaceSecretRefs) assertNoRedactedValue({ ...advancedSecretRefs, ...secretRefs }, t('密钥引用'));
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
    <Drawer title={editingChannel ? t('编辑通知渠道') : t('新建渠道')} open={open} onClose={close} width={980} destroyOnClose>
      <Alert type="info" showIcon style={{ marginBottom: 16 }} message={t('Schema 驱动渠道配置')} description={t('内置提供方按官方机器人/Webhook 结构生成字段；每条渠道配置都维护自己的 secretRefs，密钥只保存引用，当前运行时解析 env: 前缀或环境变量名，不要填写真实密钥值，也不要把多个生产渠道共用成一个全局引用。')} />
      <Form form={form} layout="vertical" onFinish={(values) => void submit(values)}>
        {editingChannel ? (
          <Alert
            type="warning"
            showIcon
            style={{ marginBottom: 16 }}
            message={t('编辑保护：默认保持现有渠道配置和保持现有密钥引用')}
            description={t('渠道详情只返回脱敏配置且不返回 secretRefsJson；如果只改名称、作用域或启用状态，请不要打开替换开关。需要更换 URL、routing key、签名密钥或 SMTP 密码时，再打开对应开关并填写完整新值。')}
          />
        ) : null}
        <Row gutter={16}>
          <Col xs={24} md={12}><Form.Item name="name" label={t('名称')} rules={[{ required: true, message: t('请输入名称') }]}><Input placeholder="billing-ops-webhook" /></Form.Item></Col>
          <Col xs={24} md={12}><Form.Item name="provider" label={t('提供方')} rules={[{ required: true }]}><Select showSearch options={providerOptions} /></Form.Item></Col>
          <Col xs={24} md={8}><Form.Item name="scopeType" label={t('作用域类型')} rules={[{ required: true }]}><Select options={CHANNEL_SCOPE_OPTIONS.map((value) => ({ value, label: value }))} onChange={clearScopeDependents} /></Form.Item></Col>
          <Col xs={24} md={8}><Form.Item name="messageType" label={t('消息类型')} rules={[{ required: true }]}><Select disabled={configControlsDisabled} options={schema.messageTypes.map((item) => ({ value: item.id, label: `${item.label} · ${item.id}` }))} /></Form.Item></Col>
          <Col xs={24} md={8}><Form.Item name="enabled" label={t('启用')} valuePropName="checked"><Switch /></Form.Item></Col>
          {scopeType !== 'global' ? <Col xs={24} md={8}><Form.Item name="namespace" label={t('Namespace')} rules={[{ required: scopeType === 'namespace' || scopeType === 'app' || scopeType === 'worker_pool', message: t('请选择命名空间') }]}><Select showSearch options={namespaceOptions} onChange={() => form.setFieldsValue({ app: undefined, workerPool: undefined, secretRefs: undefined })} /></Form.Item></Col> : null}
          {scopeType === 'app' || scopeType === 'worker_pool' ? <Col xs={24} md={8}><Form.Item name="app" label={t('App')} rules={[{ required: true, message: t('请选择应用') }]}><Select showSearch disabled={appSelectDisabled} placeholder={appSelectDisabled ? t('先选择 Namespace') : undefined} options={appOptions} onChange={() => form.setFieldsValue({ workerPool: undefined, secretRefs: undefined })} /></Form.Item></Col> : null}
          {scopeType === 'worker_pool' ? <Col xs={24} md={8}><Form.Item name="workerPool" label={t('Worker Pool')} rules={[{ required: true, message: t('请选择 Worker Pool') }]}><Select showSearch disabled={workerPoolSelectDisabled} placeholder={workerPoolSelectDisabled ? t('先选择 Namespace 和 App') : undefined} options={workerPoolOptions} /></Form.Item></Col> : null}
        </Row>
        <Alert type="success" showIcon style={{ marginBottom: 16 }} message={t('联动配置提示')} description={t(scopeHelp(scopeType))} />

        <Descriptions size="small" bordered column={1} style={{ marginBottom: 16 }} items={[
          { key: 'provider', label: t('提供方结构'), children: <Space wrap><Tag>{schema.category}</Tag><Typography.Text>{schema.description}</Typography.Text></Space> },
          { key: 'message', label: t('消息类型说明'), children: selectedMessageType?.description ?? '-' },
          { key: 'examples', label: t('示例数量'), children: `${channelExampleCount(schema)} ${t('条')}` },
          { key: 'docs', label: t('官方文档'), children: <Space wrap>{schema.docs.map((doc) => <Typography.Link key={doc.url} href={doc.url} target="_blank" rel="noreferrer">{doc.label}</Typography.Link>)}</Space> },
          { key: 'vars', label: t('模板变量'), children: <Space wrap>{schema.templateVariables.map((variable) => <Tag key={variable}>{variable}</Tag>)}</Space> },
        ]} />

        <Card
          size="small"
          title={t('示例配置')}
          style={{ marginBottom: 16 }}
          extra={<Button disabled={configControlsDisabled || !selectedExample} onClick={() => applyExample(selectedExample)}>{t('套用示例')}</Button>}
        >
          <Space direction="vertical" style={{ width: '100%' }}>
            <Typography.Paragraph type="secondary">{t('每种渠道的每种消息类型都提供 1-2 条安全示例；示例只写入 channel-scoped env: 密钥引用，不包含真实 token。生产中请为每条渠道替换成自己的 env 名称。')}</Typography.Paragraph>
            <Select
              value={selectedExample?.name}
              onChange={setSelectedExampleName}
              options={selectedExamples.map((item) => ({ value: item.name, label: `${t('示例：')}${item.name}` }))}
              style={{ width: '100%' }}
            />
            <Input.TextArea rows={6} readOnly value={formatJson(JSON.stringify(selectedExample ?? {}))} />
          </Space>
        </Card>

        <Typography.Title level={5}>{t('渠道配置')}</Typography.Title>
        {editingChannel ? <Form.Item name="replaceConfig" label={t('替换渠道配置')} valuePropName="checked"><Switch /></Form.Item> : null}
        <Row gutter={16}>
          {schema.configFields.map((field) => (
            <Col xs={24} md={field.type === 'textarea' ? 24 : 12} key={field.key}>
              <Form.Item name={['config', field.key]} label={t(field.label)} valuePropName={field.type === 'boolean' ? 'checked' : 'value'} rules={[{ required: fieldRequired(field, Boolean(editingChannel), replaceConfig), message: t('请填写必填配置') }]} extra={field.help ? t(field.help) : undefined}>
                {renderFieldInput({ ...field, placeholder: keepExistingPlaceholder(field, !editingChannel || replaceConfig) }, configControlsDisabled)}
              </Form.Item>
            </Col>
          ))}
        </Row>

        <Typography.Title level={5}>{t('密钥引用')}</Typography.Title>
        {editingChannel ? <Form.Item name="replaceSecretRefs" label={t('替换密钥引用')} valuePropName="checked"><Switch /></Form.Item> : null}
        <Row gutter={16}>
          {schema.secretFields.map((field) => (
            <Col xs={24} md={12} key={field.key}>
              <Form.Item name={['secretRefs', field.key]} label={t(field.label)} rules={[{ required: fieldRequired(field, Boolean(editingChannel), replaceSecretRefs), message: t('请填写密钥引用') }]} extra={field.help ? t(field.help) : t('按当前 scope 过滤 Secret 引用；可选择 env 类型 Secret 引用，或手工填写这条渠道自己的 env:NAME。')}>
                <AutoComplete allowClear disabled={secretControlsDisabled} options={scopedSecretOptions} placeholder={keepExistingPlaceholder(field, !editingChannel || replaceSecretRefs)} filterOption={(input, option) => String(option?.label ?? option?.value ?? '').toLowerCase().includes(input.toLowerCase())} />
              </Form.Item>
            </Col>
          ))}
        </Row>

        <Typography.Title level={5}>{t('消息模板')}</Typography.Title>
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
        <Card size="small" title={t('模板预览')} style={{ marginBottom: 16 }}>
          <Typography.Paragraph type="secondary">{t('预览展示渠道级 inline 模板结构；默认不会写入 config.template，避免覆盖策略引用的 enabled 存储模板。')}</Typography.Paragraph>
          <Input.TextArea rows={8} readOnly value={previewValue(renderedTemplatePreview)} />
        </Card>

        {editingChannel ? (
          <Card
            size="small"
            title={t('发一条试试')}
            style={{ marginBottom: 16 }}
            extra={<PermissionGate resource="notifications" action="test"><Button disabled={!testSendSupported || testingChannel} loading={testingChannel} onClick={() => void sendTestNotification()}>{t('发一条试试')}</Button></PermissionGate>}
          >
            <Typography.Paragraph type="secondary">{t('测试会使用后端已保存的渠道配置真实发送一条测试通知，并记录 message/attempt；返回结果只展示脱敏目标和脱敏后的渲染 payload。未保存的表单变更请先保存后再测试。')}</Typography.Paragraph>
            {!testSendSupported ? <Alert type="warning" showIcon style={{ marginBottom: 12 }} message={t('该渠道类型不支持测试发送')} /> : null}
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

        <Typography.Title level={5}>{t('高级选项')}</Typography.Title>
        <Row gutter={16}>
          <Col xs={24} md={12}><Form.Item name="advancedConfigJsonText" label={t('高级配置 JSON')} extra={t(configControlsDisabled ? '开启替换渠道配置后才能修改高级配置 JSON。' : '仅用于保留 provider 特殊字段；表单字段会覆盖同名键。')}><Input.TextArea rows={4} spellCheck={false} disabled={configControlsDisabled} onBlur={(event) => { const value = parseMaybeJson(event.target.value); if (value && typeof value === 'object') form.setFieldValue('advancedConfigJsonText', JSON.stringify(value, null, 2)); }} /></Form.Item></Col>
          <Col xs={24} md={12}><Form.Item name="advancedSecretRefsJsonText" label={t('高级密钥引用对象')} extra={t(secretControlsDisabled ? '开启替换密钥引用后才能修改高级密钥引用对象。' : '仅填写密钥引用，不填写真实密钥值。')}><Input.TextArea rows={4} spellCheck={false} disabled={secretControlsDisabled} placeholder="{}" /></Form.Item></Col>
          <Col xs={24}><Form.Item name="safetyPolicyJsonText" label={t('安全策略 JSON')}><Input.TextArea rows={4} spellCheck={false} placeholder="{}" /></Form.Item></Col>
        </Row>
        <Space>
          <PermissionGate resource="notifications" action="manage"><Button type="primary" htmlType="submit" loading={saving}>{editingChannel ? t('保存渠道') : t('创建渠道')}</Button></PermissionGate>
          <Button onClick={close}>{t('取消')}</Button>
        </Space>
      </Form>
    </Drawer>
  );
}
