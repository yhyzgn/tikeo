import { Alert, Button, Card, Col, Drawer, Form, Input, Row, Select, Space, Switch, Tag, Typography, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import {
  createNotificationTemplate,
  renderNotificationTemplate,
  updateNotificationTemplate,
  type CreateNotificationTemplateRequest,
  type NotificationChannelTypeSummary,
  type NotificationTemplateSummary,
  type UpdateNotificationTemplateRequest,
} from '../../api/notifications';
import { PermissionGate } from '../../components/Permission';
import { useI18n } from '../../i18n';
import { compactObject, formatJson, parseJsonObject, parseMaybeJson } from './jsonUtils';
import { TemplateVariableCatalog } from './TemplateVariableCatalog';
import { findMessageType, providerSchemaFor, type ProviderFieldSchema } from './providerSchema';

interface TemplateDrawerProps {
  open: boolean;
  channelTypes: NotificationChannelTypeSummary[];
  editingTemplate: NotificationTemplateSummary | null;
  onClose: () => void;
  onSaved: () => Promise<void>;
}

interface TemplateFormValues {
  templateKey: string;
  name: string;
  description?: string;
  provider: string;
  messageType: string;
  enabled: boolean;
  body?: Record<string, unknown>;
  variablesJsonText?: string;
  sampleJsonText?: string;
}

function readObject(raw: string | null | undefined): Record<string, unknown> {
  try {
    const parsed = JSON.parse(raw ?? '{}') as unknown;
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed as Record<string, unknown> : {};
  } catch {
    return {};
  }
}

function renderFieldInput(field: ProviderFieldSchema) {
  if (field.type === 'textarea') return <Input.TextArea rows={field.rows ?? 4} spellCheck={false} placeholder={field.placeholder} />;
  if (field.type === 'boolean') return <Switch />;
  if (field.type === 'tags' || field.type === 'emailList') return <Select mode="tags" tokenSeparators={[',', ' ']} placeholder={field.placeholder} options={field.options} />;
  if (field.type === 'select') return <Select placeholder={field.placeholder} options={field.options} />;
  return <Input placeholder={field.placeholder} />;
}

function mergeFieldValues(fields: ProviderFieldSchema[], values: Record<string, unknown> | undefined): Record<string, unknown> {
  const source = values ?? {};
  return compactObject(Object.fromEntries(fields.map((field) => [field.key, source[field.key]])));
}

function defaultSample(): string {
  return JSON.stringify({
    subject: 'Job failed',
    body: 'billing-sync failed after 3 retries',
    eventType: 'job_instance.failed',
    resourceType: 'job_instance',
    resourceId: 'job-123',
    severity: 'critical',
  }, null, 2);
}

export function TemplateDrawer({ open, channelTypes, editingTemplate, onClose, onSaved }: TemplateDrawerProps) {
  const { t } = useI18n();
  const [form] = Form.useForm<TemplateFormValues>();
  const provider = Form.useWatch('provider', form);
  const messageType = Form.useWatch('messageType', form);
  const [saving, setSaving] = useState(false);
  const [rendering, setRendering] = useState(false);
  const [preview, setPreview] = useState<string>('');

  const currentType = channelTypes.find((item) => item.type === provider) ?? channelTypes[0];
  const schema = useMemo(() => providerSchemaFor(currentType, provider), [currentType, provider]);
  const selectedMessageType = useMemo(() => findMessageType(schema, messageType), [messageType, schema]);
  const providerOptions = channelTypes.map((item) => ({ value: item.type, label: `${item.label} · ${item.type}` }));

  useEffect(() => {
    if (!open) return;
    form.resetFields();
    setPreview('');
    if (editingTemplate) {
      form.setFieldsValue({
        templateKey: editingTemplate.templateKey,
        name: editingTemplate.name,
        description: editingTemplate.description ?? undefined,
        provider: editingTemplate.provider,
        messageType: editingTemplate.messageType,
        enabled: editingTemplate.enabled,
        body: readObject(editingTemplate.bodyJson),
        variablesJsonText: formatJson(editingTemplate.variablesJson, '{}'),
        sampleJsonText: defaultSample(),
      });
      return;
    }
    const defaultType = channelTypes[0]?.type ?? 'webhook';
    const defaultSchema = providerSchemaFor(channelTypes[0], defaultType);
    form.setFieldsValue({
      provider: defaultType,
      messageType: defaultSchema.defaultMessageType,
      enabled: true,
      variablesJsonText: '{}',
      sampleJsonText: defaultSample(),
    });
  }, [channelTypes, editingTemplate, form, open]);

  useEffect(() => {
    if (!open || !provider) return;
    const nextSchema = providerSchemaFor(channelTypes.find((item) => item.type === provider), provider);
    const currentMessageType = form.getFieldValue('messageType');
    if (!nextSchema.messageTypes.some((item) => item.id === currentMessageType)) {
      form.setFieldsValue({ messageType: nextSchema.defaultMessageType, body: {} });
    }
  }, [channelTypes, form, open, provider]);

  useEffect(() => {
    if (!open || !messageType) return;
    const existingBody = form.getFieldValue('body') ?? {};
    const selected = findMessageType(schema, messageType);
    form.setFieldsValue({
      body: Object.fromEntries(selected.templateFields.map((field) => [field.key, existingBody[field.key] ?? field.defaultValue])),
    });
  }, [form, messageType, open, schema]);

  const close = () => {
    form.resetFields();
    setPreview('');
    onClose();
  };

  const payloadFrom = (values: TemplateFormValues): CreateNotificationTemplateRequest => ({
    templateKey: values.templateKey,
    name: values.name,
    description: values.description?.trim() ? values.description.trim() : null,
    provider: values.provider,
    messageType: values.messageType,
    enabled: values.enabled,
    body: mergeFieldValues(selectedMessageType.templateFields, values.body),
    variables: parseMaybeJson(values.variablesJsonText) ?? {},
  });

  const submit = async (values: TemplateFormValues) => {
    setSaving(true);
    try {
      if (editingTemplate) {
        await updateNotificationTemplate(editingTemplate.id, payloadFrom(values) as UpdateNotificationTemplateRequest);
        message.success(t('通知模板已更新'));
      } else {
        await createNotificationTemplate(payloadFrom(values));
        message.success(t('通知模板已创建'));
      }
      close();
      await onSaved();
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  const renderPreview = async () => {
    setRendering(true);
    try {
      const values = form.getFieldsValue();
      const result = await renderNotificationTemplate(editingTemplate?.id ?? values.templateKey, {
        provider: values.provider,
        messageType: values.messageType,
        template: mergeFieldValues(selectedMessageType.templateFields, values.body),
        sample: parseJsonObject(values.sampleJsonText, t('预览样本 JSON'), {}) ?? {},
      });
      setPreview(JSON.stringify(result.rendered, null, 2));
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRendering(false);
    }
  };

  return (
    <Drawer title={editingTemplate ? t('编辑通知模板') : t('新建通知模板')} open={open} onClose={close} width={920} destroyOnClose>
      <Alert type="info" showIcon style={{ marginBottom: 16 }} message={t('Schema 驱动模板编辑')} description={t('模板字段由 provider 与 messageType 联动生成，只编辑消息体和变量；不会展示渠道密钥字段或密钥引用。')} />
      <Form form={form} layout="vertical" onFinish={(values) => void submit(values)}>
        <Row gutter={16}>
          <Col xs={24} md={12}><Form.Item name="templateKey" label={t('模板 Key')} rules={[{ required: true, message: t('请输入模板 Key') }]}><Input placeholder="billing-job-failed" /></Form.Item></Col>
          <Col xs={24} md={12}><Form.Item name="name" label={t('名称')} rules={[{ required: true, message: t('请输入名称') }]}><Input placeholder={t('计费任务失败')} /></Form.Item></Col>
          <Col xs={24} md={12}><Form.Item name="provider" label={t('提供方')} rules={[{ required: true }]}><Select showSearch options={providerOptions} /></Form.Item></Col>
          <Col xs={24} md={8}><Form.Item name="messageType" label={t('消息类型')} rules={[{ required: true }]}><Select options={schema.messageTypes.map((item) => ({ value: item.id, label: `${item.label} · ${item.id}` }))} /></Form.Item></Col>
          <Col xs={24} md={4}><Form.Item name="enabled" label={t('启用')} valuePropName="checked"><Switch /></Form.Item></Col>
          <Col xs={24}><Form.Item name="description" label={t('描述')}><Input.TextArea rows={2} /></Form.Item></Col>
        </Row>

        <Card size="small" style={{ marginBottom: 16 }}>
          <Space orientation="vertical" size={8}>
            <Space wrap><Tag>{schema.provider}</Tag><Tag>{selectedMessageType.id}</Tag><Typography.Text>{selectedMessageType.description}</Typography.Text></Space>
            <TemplateVariableCatalog variables={schema.templateVariables} compact t={t} />
          </Space>
        </Card>

        <Typography.Title level={5}>{t('模板字段')}</Typography.Title>
        <Row gutter={16}>
          {selectedMessageType?.templateFields.map((field) => (
            <Col xs={24} md={field.type === 'textarea' ? 24 : 12} key={field.key}>
              <Form.Item name={['body', field.key]} label={t(field.label)} valuePropName={field.type === 'boolean' ? 'checked' : 'value'} rules={[{ required: field.required, message: t('请填写模板字段') }]} extra={field.help ? t(field.help) : undefined}>
                {renderFieldInput(field)}
              </Form.Item>
            </Col>
          ))}
        </Row>
        <Form.Item name="variablesJsonText" label={t('变量 JSON')} extra={t('可选：保存模板变量说明或默认值，不填写真实密钥。')}>
          <Input.TextArea rows={4} spellCheck={false} placeholder="{}" />
        </Form.Item>
        <Form.Item name="sampleJsonText" label={t('预览样本 JSON')}>
          <Input.TextArea rows={5} spellCheck={false} />
        </Form.Item>
        <Card size="small" title={t('渲染预览')} style={{ marginBottom: 16 }} extra={<PermissionGate resource="notifications" action="manage"><Button loading={rendering} onClick={() => void renderPreview()}>{t('渲染预览')}</Button></PermissionGate>}>
          <Input.TextArea rows={8} readOnly value={preview} placeholder={t('点击渲染预览调用后端模板渲染接口')} />
        </Card>
        <Space>
          <PermissionGate resource="notifications" action="manage"><Button type="primary" htmlType="submit" loading={saving}>{editingTemplate ? t('保存模板') : t('创建模板')}</Button></PermissionGate>
          <Button onClick={close}>{t('取消')}</Button>
        </Space>
      </Form>
    </Drawer>
  );
}
