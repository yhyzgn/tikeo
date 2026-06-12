import { Alert, Button, Card, Drawer, Empty, Form, Input, InputNumber, Popconfirm, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { JobSummary } from '../../api/client';
import { createJobNotificationBinding, deleteJobNotificationBinding, listJobNotificationBindings, listNotificationChannels, listNotificationTemplates, previewJobNotificationBinding, updateJobNotificationBinding, validateJobNotificationBinding, type JobNotificationBindingSummary, type NotificationChannelSummary, type NotificationTemplateSummary, type SaveJobNotificationBindingRequest } from '../../api/notifications';
import { useI18n } from '../../i18n/I18nContext';

type Props = {
  job: JobSummary | null;
  open: boolean;
  onClose: () => void;
};

type FormValues = SaveJobNotificationBindingRequest & { bindingId?: string };

const triggerOptions = [
  { value: 'failure', label: '失败' },
  { value: 'success', label: '成功' },
  { value: 'always', label: '总是' },
  { value: 'cancelled', label: '取消' },
  { value: 'retry_scheduled', label: '重试中' },
  { value: 'retry_exhausted', label: '重试耗尽' },
  { value: 'advanced', label: '高级' },
];

const advancedEvents = [
  'job_instance.succeeded',
  'job_instance.failed',
  'job_instance.partial_failed',
  'job_instance.cancelled',
  'job_instance.retry_scheduled',
  'job_instance.retry_exhausted',
  'job_instance.no_eligible_worker',
  'job_instance.script_governance_failure',
];

const triggerColor = (trigger: string) => trigger === 'success' ? 'green' : trigger === 'always' ? 'blue' : trigger.includes('retry') ? 'orange' : 'red';

export function JobNotificationConfigDrawer({ job, open, onClose }: Props) {
  const { t } = useI18n();
  const [form] = Form.useForm<FormValues>();
  const trigger = Form.useWatch('trigger', form);
  const selectedChannelIds = Form.useWatch('channelIds', form) ?? [];
  const [bindings, setBindings] = useState<JobNotificationBindingSummary[]>([]);
  const [channels, setChannels] = useState<NotificationChannelSummary[]>([]);
  const [templates, setTemplates] = useState<NotificationTemplateSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [preview, setPreview] = useState<Record<string, unknown> | null>(null);

  const load = useCallback(async () => {
    if (!job || !open) return;
    setLoading(true);
    try {
      const [nextBindings, nextChannels, nextTemplates] = await Promise.all([
        listJobNotificationBindings(job.id),
        listNotificationChannels({ enabled: true }),
        listNotificationTemplates({ enabled: true }),
      ]);
      setBindings(nextBindings);
      setChannels(nextChannels);
      setTemplates(nextTemplates);
    } catch (error) {
      message.error(error instanceof Error ? error.message : t('加载通知配置失败'));
    } finally {
      setLoading(false);
    }
  }, [job, open, t]);

  useEffect(() => { void load(); }, [load]);

  const selectedProviders = useMemo(() => channels.filter((item) => selectedChannelIds.includes(item.id)).map((item) => item.provider), [channels, selectedChannelIds]);
  const templateOptions = templates
    .filter((item) => selectedProviders.length === 0 || selectedProviders.includes(item.provider))
    .map((item) => ({ value: item.id, label: `${item.name} · ${item.provider}/${item.messageType}` }));

  const startCreate = () => {
    setPreview(null);
    form.setFieldsValue({ name: `${job?.name ?? 'Job'} 失败通知`, trigger: 'failure', channelIds: [], enabled: true, severity: 'critical', dedupeSeconds: 300, includeLogLink: true, includeLogExcerpt: false, logExcerptLines: 80 });
  };

  const edit = (binding: JobNotificationBindingSummary) => {
    setPreview(null);
    form.setFieldsValue({ bindingId: binding.id, name: binding.name, trigger: binding.trigger, eventTypes: binding.eventTypes, channelIds: binding.channelIds, templateRef: binding.templateRef, enabled: binding.enabled, severity: binding.severity, dedupeSeconds: binding.dedupeSeconds, includeLogLink: binding.includeLogLink, includeLogExcerpt: binding.includeLogExcerpt, logExcerptLines: binding.logExcerptLines });
  };

  const save = async () => {
    if (!job) return;
    const values = await form.validateFields();
    setSaving(true);
    try {
      if (values.bindingId) {
        await updateJobNotificationBinding(job.id, values.bindingId, values);
      } else {
        await createJobNotificationBinding(job.id, values);
      }
      message.success(t('通知配置已保存'));
      form.resetFields();
      setPreview(null);
      await load();
    } catch (error) {
      message.error(error instanceof Error ? error.message : t('保存通知配置失败'));
    } finally {
      setSaving(false);
    }
  };

  const runPreview = async () => {
    if (!job) return;
    const values = await form.validateFields();
    const result = await previewJobNotificationBinding(job.id, values);
    setPreview(result as unknown as Record<string, unknown>);
  };

  const runValidate = async () => {
    if (!job) return;
    const values = await form.validateFields();
    const result = await validateJobNotificationBinding(job.id, values);
    if (result.valid) message.success(t('通知规则校验通过'));
    else message.warning(result.issues.join('; '));
  };

  return (
    <Drawer title={job ? `${t('任务通知配置')} · ${job.name}` : t('任务通知配置')} open={open} onClose={onClose} width={1040} destroyOnClose>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Alert type="info" showIcon message={t('渠道和凭证仍由 Notification Center 统一管理；这里仅声明该任务在什么状态下通知哪些渠道。')} />
        <Card size="small" title={t('已有通知规则')} extra={<Button type="primary" onClick={startCreate}>{t('新建规则')}</Button>}>
          <Table<JobNotificationBindingSummary>
            rowKey="id"
            loading={loading}
            dataSource={bindings}
            pagination={false}
            locale={{ emptyText: <Empty description={t('当前任务还没有专属通知规则')} /> }}
            columns={[
              { title: t('名称'), dataIndex: 'name' },
              { title: t('触发'), dataIndex: 'trigger', render: (value) => <Tag color={triggerColor(String(value))}>{String(value)}</Tag> },
              { title: t('事件'), dataIndex: 'eventTypes', render: (items: string[]) => <Space wrap>{items.map((item) => <Tag key={item}>{item}</Tag>)}</Space> },
              { title: t('渠道数'), render: (_, item) => item.channelIds.length },
              { title: t('模板'), dataIndex: 'templateRef', render: (value) => value ? <Typography.Text code>{value}</Typography.Text> : <Typography.Text type="secondary">{t('未选择')}</Typography.Text> },
              { title: t('状态'), dataIndex: 'enabled', render: (value) => value ? <Tag color="green">{t('启用')}</Tag> : <Tag>{t('停用')}</Tag> },
              { title: t('操作'), align: 'right', render: (_, item) => <Space><Button type="link" size="small" onClick={() => edit(item)}>{t('编辑')}</Button><Popconfirm title={t('删除通知规则')} onConfirm={async () => { if (job) { await deleteJobNotificationBinding(job.id, item.id); await load(); } }}><Button type="link" danger size="small">{t('删除')}</Button></Popconfirm></Space> },
            ]}
          />
        </Card>
        <Card size="small" title={t('规则编辑')}>
          <Form form={form} layout="vertical">
            <Form.Item name="bindingId" hidden><Input /></Form.Item>
            <Form.Item name="name" label={t('规则名称')} rules={[{ required: true }]}><Input /></Form.Item>
            <div className="form-grid two">
              <Form.Item name="trigger" label={t('触发条件')} rules={[{ required: true }]}><Select options={triggerOptions.map((item) => ({ ...item, label: t(item.label) }))} /></Form.Item>
              <Form.Item name="severity" label={t('严重级别')} rules={[{ required: true }]}><Select options={['info', 'warning', 'critical'].map((value) => ({ value, label: value }))} /></Form.Item>
            </div>
            {trigger === 'advanced' ? <Form.Item name="eventTypes" label={t('高级事件')} rules={[{ required: true }]}><Select mode="multiple" options={advancedEvents.map((value) => ({ value, label: value }))} /></Form.Item> : null}
            <Form.Item name="channelIds" label={t('通知渠道')} rules={[{ required: true }]}><Select mode="multiple" options={channels.map((item) => ({ value: item.id, label: `${item.name} · ${item.provider} · ${item.targetRedacted}` }))} /></Form.Item>
            <Form.Item name="templateRef" label={t('通知模板')}><Select allowClear options={templateOptions} /></Form.Item>
            <div className="form-grid three">
              <Form.Item name="dedupeSeconds" label={t('去重窗口')}><InputNumber min={0} max={86400} addonAfter="秒" style={{ width: '100%' }} /></Form.Item>
              <Form.Item name="includeLogLink" label={t('包含日志链接')} valuePropName="checked"><Switch /></Form.Item>
              <Form.Item name="includeLogExcerpt" label={t('包含日志摘要')} valuePropName="checked"><Switch /></Form.Item>
            </div>
            <Form.Item name="logExcerptLines" label={t('日志摘要行数')}><InputNumber min={1} max={500} style={{ width: '100%' }} /></Form.Item>
            <Space>
              <Button onClick={() => void runValidate()}>{t('校验')}</Button>
              <Button onClick={() => void runPreview()}>{t('预览')}</Button>
              <Button type="primary" loading={saving} onClick={() => void save()}>{t('保存')}</Button>
            </Space>
          </Form>
          {preview ? <pre className="json-preview">{JSON.stringify(preview, null, 2)}</pre> : null}
        </Card>
      </Space>
    </Drawer>
  );
}
