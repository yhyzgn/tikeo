import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const clientSource = readFileSync(new URL('../../api/notifications.ts', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../NotificationCenterPage.tsx', import.meta.url), 'utf8');
const channelDrawerSource = readFileSync(new URL('../notifications/ChannelDrawer.tsx', import.meta.url), 'utf8');
const providerSchemaSource = readFileSync(new URL('../notifications/providerSchema.ts', import.meta.url), 'utf8');
const templateDrawerSource = readFileSync(new URL('../notifications/TemplateDrawer.tsx', import.meta.url), 'utf8');
const templateCatalogSource = readFileSync(new URL('../notifications/templateCatalog.ts', import.meta.url), 'utf8');

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

  test('does not overclaim vault secret resolution for notification channels', () => {
    expect(channelDrawerSource).toContain('当前运行时解析 env: 前缀或环境变量名');
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
    expect(channelDrawerSource).toContain('模板变量');
    expect(channelDrawerSource).toContain('官方文档');
    expect(channelDrawerSource).not.toContain('渠道配置 JSON');
    expect(channelDrawerSource).not.toContain('密钥引用 JSON');
  });

  test('links channel scope, tenant resources, and secret reference choices', () => {
    for (const token of ['listNamespaces', 'listAppScopes', 'listWorkerPools', 'listSecrets']) {
      expect(channelDrawerSource).toContain(token);
    }
    expect(channelDrawerSource).toContain('filteredApps');
    expect(channelDrawerSource).toContain('filteredWorkerPools');
    expect(channelDrawerSource).toContain('filteredSecrets');
    expect(channelDrawerSource).toContain("nextScopeType === 'global'");
    expect(channelDrawerSource).toContain('clearScopeDependents');
  });

  test('keeps channel examples as normal channel rows instead of a separate use-case data tab', () => {
    expect(pageSource).toContain('通知渠道');
    expect(pageSource + channelDrawerSource).not.toContain('用例数据');
    expect(pageSource + channelDrawerSource).not.toContain('通知配置用例');
    expect(pageSource + channelDrawerSource).not.toContain('套用为新渠道');
    expect(pageSource + channelDrawerSource).not.toContain('channelExampleRows');
    expect(pageSource + channelDrawerSource).not.toContain('selectedChannelExample');
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
    for (const token of ['examples', 'channelExampleCount', 'applyExample', '套用示例', '示例：']) {
      expect(providerSchemaSource + channelDrawerSource).toContain(token);
    }
    for (const token of ['env:TIKEO_NOTIFICATION_WEBHOOK_URL', 'env:SLACK_WEBHOOK_URL', 'env:DINGTALK_WEBHOOK_URL', 'env:FEISHU_WEBHOOK_URL', 'env:WECOM_WEBHOOK_URL', 'env:PAGERDUTY_ROUTING_KEY', 'env:TIKEO_SMTP_URL']) {
      expect(providerSchemaSource).toContain(token);
    }
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
    expect(channelDrawerSource).toContain('按当前 scope 过滤 Secret 引用');
    expect(channelDrawerSource).toContain('保持现有渠道配置');
    expect(channelDrawerSource).toContain('保持现有密钥引用');
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
    expect(channelDrawerSource).toContain('开启替换密钥引用后才能修改高级密钥引用对象。');
    expect(channelDrawerSource).toContain('保持现有渠道配置');
    expect(channelDrawerSource).toContain('保持现有密钥引用');
  });

  test('lets operators send one test notification from the channel edit drawer and inspect detailed results', () => {
    for (const token of [
      'testNotificationChannel',
      '/test-send',
      '发一条试试',
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
      'disabled={!testSendSupported || testingChannel}',
      '该渠道类型不支持测试发送',
      'selectedMessageType?.id',
    ]) {
      expect(channelDrawerSource).toContain(token);
    }
  });

  test('normalizes generated example textarea values into JSON strings before applying them', () => {
    for (const token of [
      'exampleFieldValue',
      "field.type === 'textarea'",
      'JSON.stringify(value, null, 2)',
    ]) {
      expect(channelDrawerSource).toContain(token);
    }
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

});
