import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

import { providerSchemaFor } from '../notifications/providerSchema';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const clientSource = readFileSync(new URL('../../api/notifications.ts', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../NotificationCenterPage.tsx', import.meta.url), 'utf8');
const channelDrawerSource = readFileSync(new URL('../notifications/ChannelDrawer.tsx', import.meta.url), 'utf8');
const providerSchemaSource = readFileSync(new URL('../notifications/providerSchema.ts', import.meta.url), 'utf8');
const templateDrawerSource = readFileSync(new URL('../notifications/TemplateDrawer.tsx', import.meta.url), 'utf8');
const templateCatalogSource = readFileSync(new URL('../notifications/templateCatalog.ts', import.meta.url), 'utf8');
const publicConsoleSource = readFileSync(new URL('../PublicInstanceConsolePage.tsx', import.meta.url), 'utf8');
const messageDetailDrawerSource = readFileSync(new URL('../notifications/NotificationMessageDetailDrawer.tsx', import.meta.url), 'utf8');
const stylesSource = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('notification center console page', () => {
  test('wires Notification Center as a first-class observability menu route', () => {
    expect(routesSource).toContain('notifications:');
    expect(routesSource).toContain('/notifications');
    expect(routesSource).toContain('通知中心');
    expect(routesSource).toContain("resource: 'notifications'");
    expect(appSource).toContain('NotificationCenterPage');
    expect(appSource).toContain('ROUTE_META.notifications.path');
  });

  test('uses generic notification center endpoints instead of legacy alert delivery only', () => {
    expect(clientSource).toContain('/api/v1/notification-channel-types');
    expect(clientSource).toContain('/api/v1/notification-channels');
    expect(clientSource).toContain('/api/v1/notification-policies');
    expect(clientSource).toContain('/api/v1/notification-messages');
    expect(clientSource).toContain('/api/v1/notification-delivery-attempts:queue-status');
    expect(clientSource).toContain('/api/v1/notification-delivery-attempts:retry-due');
    expect(pageSource).toContain('listNotificationChannels');
    expect(pageSource).toContain('listNotificationPolicies');
    expect(pageSource).toContain('getNotificationDeliveryQueueStatus');
    expect(pageSource).toContain('提供方目标已脱敏');
    expect(pageSource).toContain('通知中心');
  });

  test('exposes channel and policy configuration operations instead of read-only inspection', () => {
    for (const token of [
      'deleteNotificationChannel',
      'createNotificationPolicy',
      'updateNotificationPolicy',
      'deleteNotificationPolicy',
      'validateNotificationPolicy',
    ]) {
      expect(clientSource).toContain(token);
      if (token === 'createNotificationChannel' || token === 'updateNotificationChannel') {
        expect(channelDrawerSource).toContain(token);
      } else {
        expect(pageSource).toContain(token);
      }
    }
    expect(channelDrawerSource).toContain('createNotificationChannel');
    expect(channelDrawerSource).toContain('updateNotificationChannel');
    expect(pageSource).toContain('channelDrawerOpen');
    expect(pageSource).toContain('policyDrawerOpen');
    expect(pageSource).toContain('新建渠道');
    expect(pageSource).toContain('新建策略');
    expect(pageSource).toContain('校验');
    expect(pageSource).toContain('删除');
  });

  test('frames channel secretRefs as direct private credentials with optional env compatibility', () => {
    expect(channelDrawerSource).toContain('保存后立即生效');
    expect(channelDrawerSource).toContain('可直接填写真实值');
    expect(channelDrawerSource).toContain('env:NAME');
    expect(channelDrawerSource).not.toContain('当前运行时解析 env: 前缀或环境变量名');
    expect(channelDrawerSource).not.toContain('真实值放在部署环境变量或 Secret 中');
    expect(pageSource + channelDrawerSource).not.toContain('env 或 vault');
    expect(pageSource + channelDrawerSource).not.toContain('vault 路径');
  });

  test('uses a schema-driven channel drawer instead of raw JSON-only editing', () => {
    expect(pageSource).toContain('ChannelDrawer');
    expect(channelDrawerSource).toContain('ProviderSchema');
    expect(channelDrawerSource).toContain('messageType');
    expect(channelDrawerSource).toContain('schema.configFields');
    expect(channelDrawerSource).toContain('schema.secretFields');
    expect(channelDrawerSource).toContain('schema.messageTypes');
    expect(channelDrawerSource + readFileSync(new URL('../notifications/TemplateVariableCatalog.tsx', import.meta.url), 'utf8')).toContain('模板变量');
    expect(channelDrawerSource).toContain('官方文档');
    expect(channelDrawerSource).not.toContain('渠道配置 JSON');
    expect(channelDrawerSource).not.toContain('密钥引用 JSON');
  });


  test('keeps the left scope path synchronized with right-side cascade progress', () => {
    expect(channelDrawerSource).toContain('channelScopeSteps');
    expect(channelDrawerSource).toContain('channel-scope-step--current');
    expect(channelDrawerSource).toContain('channel-scope-step--done');
    expect(channelDrawerSource).toContain('channel-scope-step--pending');
    expect(channelDrawerSource).toContain('channel-scope-step--skipped');
    expect(channelDrawerSource).toContain('待选择');
    expect(channelDrawerSource).toContain('当前');
    expect(channelDrawerSource).toContain('已完成');
    expect(channelDrawerSource).toContain('不适用');
  });

  test('links channel scope, scope resources, and secret reference choices', () => {
    for (const token of ['listNamespaces', 'listAppScopes', 'listWorkerPools', 'listSecrets']) {
      expect(channelDrawerSource).toContain(token);
    }
    expect(channelDrawerSource).toContain('filteredApps');
    expect(channelDrawerSource).toContain('filteredWorkerPools');
    expect(channelDrawerSource).toContain('filteredSecrets');
    expect(channelDrawerSource).toContain("nextScopeType === 'global'");
    expect(channelDrawerSource).toContain('clearScopeDependents');
  });

  test('keeps channel examples as normal channel rows and removes drawer example-apply UI', () => {
    expect(pageSource).toContain('通知渠道');
    expect(providerSchemaSource).toContain('examples');
    for (const token of [
      '用例数据',
      '通知配置用例',
      '套用为新渠道',
      '示例配置',
      '套用示例',
      '示例数量',
      'channelExampleRows',
      'selectedChannelExample',
      'selectedExampleName',
      'applyExample',
      'channelExampleCount',
      'exampleFieldValue',
    ]) {
      expect(pageSource + channelDrawerSource).not.toContain(token);
    }
  });

  test('has built-in provider schema fallbacks for rich message types and templates', () => {
    for (const token of ['slack', 'dingtalk', 'feishu', 'wechat_work', 'pagerduty', 'email', 'webhook']) {
      expect(providerSchemaSource).toContain(token);
    }
    for (const token of ['blockKit', 'actionCard', 'feedCard', 'interactive', 'share_chat', 'markdown', 'news', 'file', 'voice', 'html', 'trigger', 'resolve']) {
      expect(providerSchemaSource).toContain(token);
    }
    for (const token of ['{{subject}}', '{{body}}', '{{eventType}}', '{{resourceId}}', '{{severity}}']) {
      expect(providerSchemaSource).toContain(token);
    }
    expect(providerSchemaSource).toContain('examples');
    expect(providerSchemaSource).not.toContain('generatedExample');
    expect(providerSchemaSource).not.toContain(`direct-channel-${'token'}`);
    expect(providerSchemaSource).not.toContain('directWebhookUrl');
    expect(channelDrawerSource).not.toContain('套用示例');
    for (const provider of ['webhook', 'slack', 'dingtalk', 'feishu', 'wechat_work', 'pagerduty', 'email']) {
      const schema = providerSchemaFor(null, provider);
      for (const messageType of schema.messageTypes) {
        expect(messageType.examples).toBeUndefined();
      }
    }
    for (const token of ['env:TIKEO_NOTIFICATION_WEBHOOK_URL', 'env:SLACK_WEBHOOK_URL', 'env:DINGTALK_WEBHOOK_URL', 'env:FEISHU_WEBHOOK_URL', 'env:WECOM_WEBHOOK_URL', 'env:PAGERDUTY_ROUTING_KEY', 'env:TIKEO_SMTP_URL']) {
      expect(providerSchemaSource).not.toContain(token);
    }
  });



  test('keeps job notification advanced event values canonical while displaying localized labels', () => {
    const drawerSource = readFileSync(new URL('../notifications/JobNotificationConfigDrawer.tsx', import.meta.url), 'utf8');
    expect(drawerSource).toContain('JOB_NOTIFICATION_EVENT_OPTIONS');
    expect(drawerSource).toContain("value: 'job_instance.running'");
    expect(drawerSource).toContain("label: '运行中'");
    expect(drawerSource).toContain('normalizeJobNotificationEventTypes');
    expect(drawerSource).toContain("'job_instance.运行中': 'job_instance.running'");
    expect(drawerSource).toContain('payload = normalizeJobNotificationFormValues(values)');
    expect(drawerSource).not.toContain('options={advancedEvents.map((value) => ({ value, label: value }))}');
  });

  test('documents notification template variables with a localized mapping catalog instead of raw mixed tags', () => {
    const variableCatalogSource = readFileSync(new URL('../notifications/TemplateVariableCatalog.tsx', import.meta.url), 'utf8');
    expect(channelDrawerSource).toContain('TemplateVariableCatalog');
    expect(templateDrawerSource).toContain('TemplateVariableCatalog');
    expect(variableCatalogSource).toContain('QuestionCircleOutlined');
    expect(variableCatalogSource).toContain('Modal');
    expect(variableCatalogSource).toContain('变量映射表');
    expect(variableCatalogSource).toContain('搜索变量');
    expect(variableCatalogSource).toContain('标准字段');
    expect(variableCatalogSource).toContain('任务上下文');
    expect(variableCatalogSource).toContain('template-variable-catalog__grid');
    expect(variableCatalogSource).toContain('template-variable-card__placeholder');
    expect(variableCatalogSource).toContain('width="min(1120px, calc(100vw - 32px))"');
    expect(variableCatalogSource).toContain('zIndex={1400}');
    expect(variableCatalogSource).toContain('const previewRows = rows');
    expect(variableCatalogSource).not.toContain('slice(0, 6)');
    expect(variableCatalogSource).not.toContain('template-variable-catalog__chip--more');
    expect(stylesSource).toContain('template-variable-catalog__preview');
    expect(stylesSource).toContain('overflow: auto;');
    expect(stylesSource).toContain('template-variable-card__placeholder');
    expect(stylesSource).toContain('grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));');
    expect(stylesSource).toContain('white-space: nowrap;');
    expect(stylesSource).toContain('white-space: normal;');
    expect(stylesSource).not.toContain('.template-variable-catalog__toolbar {\n  position: sticky;');
    expect(variableCatalogSource).toContain('变量由消息标准字段与事件 payload 顶层字段共同提供');
    expect(variableCatalogSource).not.toContain('<Table');
    expect(providerSchemaSource).toContain('{{jobId}}');
    expect(providerSchemaSource).toContain('{{instanceId}}');
    expect(providerSchemaSource).toContain('{{operatorName}}');
    expect(providerSchemaSource).toContain('{{logsUrl}}');
  });

  test('covers official built-in provider variants and linked drawer affordances', () => {
    for (const token of [
      'attachments',
      'markdown_v2',
      'template_card',
      'atUserIds',
      'mentionedList',
      'mentionedMobileList',
      'customDetails',
      'clientUrl',
      'threadTs',
      'routingKey',
    ]) {
      expect(providerSchemaSource).toContain(token);
    }
    for (const token of ['scopedSecretOptions', 'appSelectDisabled', 'workerPoolSelectDisabled', 'templatePreview', 'replaceConfig', 'replaceSecretRefs']) {
      expect(channelDrawerSource).toContain(token);
    }
    expect(channelDrawerSource).toContain('先选择 Namespace');
    expect(channelDrawerSource).toContain('机器人地址 / 私密凭据');
    expect(channelDrawerSource).toContain('机器人/Webhook 地址');
    expect(channelDrawerSource).toContain('保存后立即生效');
    expect(channelDrawerSource).toContain('可直接填写真实值');
    expect(channelDrawerSource).toContain('保持现有渠道配置');
    expect(channelDrawerSource).toContain('保持现有私密配置');

    const metadataSchema = providerSchemaFor({
      type: 'feishu',
      label: 'Feishu/Lark Bot',
      category: 'office_bot',
      description: 'metadata fixture',
      targetKind: 'webhook',
      pluginProvided: false,
      supportsTestSend: true,
      requiredConfigKeys: [],
      requiredTargetKeys: ['url'],
      secretConfigKeys: ['url', 'signingKey'],
      template: {
        messageTypes: [{ id: 'interactive', label: 'Interactive', description: 'card', templateFields: [] }],
        secretFields: [
          { key: 'url', label: 'Webhook URL secret ref', type: 'string', required: true, secret: true },
          { key: 'signingKey', label: 'Signing secret ref', type: 'string', secret: true },
        ],
      },
    }, 'feishu');
    const urlSecretField = metadataSchema.secretFields.find((item) => item.key === 'url');
    expect(urlSecretField?.label).toBe('机器人/Webhook 地址');
    expect(urlSecretField?.placeholder).toBe('env:CHANNEL_WEBHOOK_URL');
    expect(urlSecretField?.help).toContain('可直接填写真实值');
  });

  test('organizes the channel drawer into summary, linked scope, replacement, and advanced sections', () => {
    expect(channelDrawerSource).toContain('配置摘要');
    expect(channelDrawerSource).toContain('作用域路径');
    expect(channelDrawerSource).toContain('身份与作用域');
    expect(channelDrawerSource).toContain('提供方与消息形态');
    expect(channelDrawerSource).toContain('投递目标与私密凭据');
    expect(channelDrawerSource).toContain('渠道参数');
    expect(channelDrawerSource).toContain('消息覆盖策略');
    expect(channelDrawerSource).toContain('扩展 JSON 与安全策略');
    expect(channelDrawerSource).toContain('channel-drawer-map');
    expect(channelDrawerSource).toContain('channel-domain-panel');
    expect(channelDrawerSource).not.toContain('01 · 基础配置');
    expect(channelDrawerSource).not.toContain('02 · 连接层');
    expect(channelDrawerSource).not.toContain('03 · 消息层');
    expect(channelDrawerSource).not.toContain('04 · 治理层');
    expect(channelDrawerSource).toContain('channel-advanced-collapse');
    expect(channelDrawerSource).toContain('按领域选择是否覆盖已保存连接信息');
    expect(channelDrawerSource).toContain('name="replaceSecretRefs"');
    expect(channelDrawerSource).toContain('name="replaceConfig"');
    expect(channelDrawerSource.indexOf('投递目标与私密凭据')).toBeLessThan(channelDrawerSource.lastIndexOf('schema.secretFields.map'));
    expect(channelDrawerSource.indexOf('渠道参数')).toBeLessThan(channelDrawerSource.lastIndexOf('schema.configFields.map'));
    expect(channelDrawerSource.indexOf('扩展 JSON 与安全策略')).toBeLessThan(channelDrawerSource.lastIndexOf('advancedConfigJsonText'));
  });

  test('allows metadata-only channel edits without re-entering preserved secrets', () => {
    expect(channelDrawerSource).toContain('fieldRequired');
    expect(channelDrawerSource).toContain("field.required && (!editing || replacing)");
    expect(channelDrawerSource).toContain('replaceConfig');
    expect(channelDrawerSource).toContain('replaceSecretRefs');
    expect(channelDrawerSource).toContain('configControlsDisabled');
    expect(channelDrawerSource).toContain('secretControlsDisabled');
    expect(channelDrawerSource).toContain('开启替换渠道配置后才能修改消息类型和 inline 模板字段。');
    expect(channelDrawerSource).toContain('开启替换渠道配置后才能修改高级配置 JSON。');
    expect(channelDrawerSource).toContain('开启替换私密配置后才能修改高级私密配置 JSON。');
    expect(channelDrawerSource).toContain('保持现有渠道配置');
    expect(channelDrawerSource).toContain('保持现有私密配置');
  });

  test('lets operators send one test notification from the channel edit drawer and inspect detailed results', () => {
    for (const token of [
      'testNotificationChannel',
      '/test-send',
      '测试',
      'testingChannel',
      'testResult',
      '测试结果',
      'delivered',
      'provider',
      'targetRedacted',
      'statusCode',
      'retryState',
      'messageId',
      'attemptId',
      'renderedPayload',
      'channel-test-result',
      'channel-test-result__grid',
      '投递状态',
      '请求摘要',
      '渲染 Payload',
      'error',
    ]) {
      expect(clientSource + channelDrawerSource).toContain(token);
    }
    expect(channelDrawerSource).toContain('返回结果只展示脱敏目标和脱敏后的渲染 payload');
  });

  test('guards channel test send with provider support and keeps example selection safe', () => {
    for (const token of [
      'testSendSupported',
      'currentType?.supportsTestSend === true',
      'testDisabledReason',
      '该渠道类型不支持测试发送',
      'selectedMessageType?.examples?.[0]?.sample',
    ]) {
      expect(channelDrawerSource).toContain(token);
    }
  });

  test('keeps generated examples out of the channel drawer apply path', () => {
    for (const token of ['selectedExampleName', 'applyExample', 'exampleFieldValue', '套用示例', '示例配置']) {
      expect(channelDrawerSource).not.toContain(token);
    }
    expect(channelDrawerSource).toContain('selectedMessageType?.examples?.[0]?.sample');
  });

  test('wires first-class notification template endpoints and page tab', () => {
    for (const token of [
      '/api/v1/notification-templates',
      'listNotificationTemplates',
      'createNotificationTemplate',
      'updateNotificationTemplate',
      'deleteNotificationTemplate',
      'renderNotificationTemplate',
      '/render',
    ]) {
      expect(clientSource).toContain(token);
    }
    expect(pageSource).toContain('TemplateDrawer');
    expect(pageSource).toContain('templates');
    expect(pageSource).toContain('listNotificationTemplates');
    expect(pageSource).toContain('templateKey');
    expect(pageSource).toContain('messageType');
    expect(pageSource).toContain('createdAt');
    expect(pageSource).toContain('新建模板');
    expect(pageSource).toContain('预览');
  });

  test('uses schema-driven template drawer with render preview and no secret fields', () => {
    for (const token of [
      'providerSchemaFor',
      'schema.messageTypes',
      'selectedMessageType?.templateFields',
      'createNotificationTemplate',
      'updateNotificationTemplate',
      'renderNotificationTemplate',
      '渲染预览',
    ]) {
      expect(templateDrawerSource).toContain(token);
    }
    expect(templateDrawerSource).not.toContain('secretRefsJson');
    expect(templateDrawerSource).not.toContain('schema.secretFields');
  });

  test('offers policy template options from enabled stored templates only', () => {
    expect(templateCatalogSource).toContain('notificationTemplateOptions');
    expect(templateCatalogSource).toContain('selectedPolicyProviders');
    expect(templateCatalogSource).toContain('!template.enabled');
    expect(templateCatalogSource).not.toContain('builtInTemplateRefs');
    expect(pageSource).toContain('templateRefOptions');
    expect(pageSource).toContain('只能选择已启用且与所选渠道提供方匹配的存储模板');
    expect(pageSource).toContain('Select allowClear showSearch options={templateRefOptions}');
    expect(pageSource).not.toContain('AutoComplete');
    expect(pageSource).not.toContain('手工输入外部系统已同步');
    expect(pageSource).not.toContain("name=\"templateRef\" label={t('模板引用')}><Input");
  });



  test('keeps notification trace runtime payloads and log messages out of DOM auto-localization', () => {
    const detailDrawerSource = readFileSync(new URL('../notifications/NotificationMessageDetailDrawer.tsx', import.meta.url), 'utf8');
    expect(detailDrawerSource).toContain('className="json-preview" data-runtime-text');
    expect(detailDrawerSource).toContain('children: <span data-runtime-text>{trace.message.subject}</span>');
    expect(detailDrawerSource).toContain('children: <span data-runtime-text>{trace.message.body}</span>');
    expect(detailDrawerSource).toContain('render: (value: string) => <span data-runtime-text>{value}</span>');
    expect(publicConsoleSource).toContain('className="public-console-page__log" data-runtime-text');
    expect(publicConsoleSource).toContain('<Typography.Paragraph data-runtime-text>{trace.message.body}</Typography.Paragraph>');
  });

  test('lets operators open notification message trace with delivery attempts and job logs', () => {
    expect(clientSource).toContain('getNotificationMessageTrace');
    expect(clientSource).toContain('/api/v1/notification-messages/${encodeURIComponent(messageId)}/trace');
    expect(pageSource).toContain('NotificationMessageDetailDrawer');
    expect(pageSource).toContain('setDetailMessage');
    expect(pageSource).toContain('详情');
    for (const token of ['执行日志透传', 'Delivery attempts', 'trace?.attempts', 'trace?.instance']) {
      expect(messageDetailDrawerSource).toContain(token);
    }
  });

});
