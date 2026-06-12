import type { NotificationChannelTypeSummary } from '../../api/notifications';

export type ProviderFieldType = 'string' | 'textarea' | 'boolean' | 'select' | 'tags' | 'url' | 'emailList';

export interface ProviderFieldOption {
  value: string;
  label: string;
}

export interface ProviderFieldSchema {
  key: string;
  label: string;
  type: ProviderFieldType;
  required?: boolean;
  secret?: boolean;
  placeholder?: string;
  help?: string;
  defaultValue?: unknown;
  options?: ProviderFieldOption[];
  rows?: number;
}

export interface ProviderMessageTypeExample {
  name: string;
  description?: string;
  config?: Record<string, unknown>;
  secretRefs?: Record<string, unknown>;
  template?: Record<string, unknown>;
  sample?: Record<string, unknown>;
}

export interface ProviderMessageTypeSchema {
  id: string;
  label: string;
  description: string;
  templateFields: ProviderFieldSchema[];
  preview?: Record<string, unknown>;
  examples?: ProviderMessageTypeExample[];
}

export interface ProviderDocLink {
  label: string;
  url: string;
}

export interface ProviderSchema {
  provider: string;
  label: string;
  category: string;
  description: string;
  configFields: ProviderFieldSchema[];
  secretFields: ProviderFieldSchema[];
  messageTypes: ProviderMessageTypeSchema[];
  templateVariables: string[];
  docs: ProviderDocLink[];
  defaultMessageType: string;
}

export const DEFAULT_TEMPLATE_VARIABLES = [
  '{{subject}}',
  '{{body}}',
  '{{eventType}}',
  '{{resourceType}}',
  '{{resourceId}}',
  '{{severity}}',
  '{{messageId}}',
  '{{policyId}}',
  '{{dedupeKey}}',
  '{{triggeredAt}}',
  '{{createdAt}}',
];

const webhookUrlField: ProviderFieldSchema = {
  key: 'url',
  label: '机器人/Webhook 地址',
  type: 'string',
  required: true,
  secret: true,
  placeholder: 'https://hooks.example.com/tikeo',
  help: '可直接填写真实值（本渠道 Webhook URL），保存后立即生效且响应不会回显；也支持 env:NAME 兼容引用。',
};

const authorizationField: ProviderFieldSchema = {
  key: 'authorization',
  label: 'Authorization header',
  type: 'string',
  secret: true,
  placeholder: 'Bearer <token>',
  help: '可直接填写本渠道 Authorization 值；也支持 env:NAME 兼容引用。',
};

const fallbackSchemas: Record<string, ProviderSchema> = {
  webhook: {
    provider: 'webhook',
    label: 'Generic Webhook',
    category: 'webhook',
    description: 'Provider-neutral JSON webhook with a configurable body template.',
    configFields: [],
    secretFields: [webhookUrlField, authorizationField],
    messageTypes: [
      {
        id: 'json',
        label: 'JSON payload',
        description: 'Send a JSON body rendered from the configured template.',
        templateFields: [
          { key: 'body', label: 'JSON body template', type: 'textarea', required: true, rows: 10, placeholder: '{\n  "text": "{{subject}}",\n  "eventType": "{{eventType}}"\n}' },
        ],
      },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'Webhook contract', url: 'https://datatracker.ietf.org/doc/rfc9110/' }],
    defaultMessageType: 'json',
  },
  slack: {
    provider: 'slack',
    label: 'Slack Incoming Webhook',
    category: 'office_bot',
    description: 'Slack incoming webhook. Supports text, Block Kit, legacy attachments, and optional thread_ts replies.',
    configFields: [{ key: 'threadTs', label: 'Thread timestamp', type: 'string', placeholder: '1699999999.000100', help: 'Optional Slack thread_ts for replies in an existing thread.' }],
    secretFields: [webhookUrlField],
    messageTypes: [
      {
        id: 'text',
        label: 'Text',
        description: 'Simple Slack text message.',
        templateFields: [{ key: 'text', label: 'Text template', type: 'textarea', required: true, rows: 5, placeholder: '[tikeo/{{severity}}] {{subject}}\n{{body}}' }],
      },
      {
        id: 'blockKit',
        label: 'Block Kit',
        description: 'Slack payload with blocks plus fallback text.',
        templateFields: [
          { key: 'text', label: 'Fallback text', type: 'textarea', required: true, rows: 3, placeholder: '{{subject}}' },
          { key: 'blocks', label: 'Blocks JSON template', type: 'textarea', required: true, rows: 10, placeholder: '[{"type":"section","text":{"type":"mrkdwn","text":"*{{subject}}*\\n{{body}}"}}]' },
        ],
      },
      {
        id: 'attachments',
        label: 'Attachments',
        description: 'Slack legacy attachments array with fallback text.',
        templateFields: [
          { key: 'text', label: 'Fallback text', type: 'textarea', required: true, rows: 3, placeholder: '{{subject}}' },
          { key: 'attachments', label: 'Attachments JSON template', type: 'textarea', required: true, rows: 10, placeholder: '[{"color":"danger","title":"{{subject}}","text":"{{body}}"}]' },
        ],
      },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'Slack Incoming Webhooks', url: 'https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks/' }],
    defaultMessageType: 'text',
  },
  dingtalk: {
    provider: 'dingtalk',
    label: 'DingTalk Robot',
    category: 'office_bot',
    description: 'DingTalk custom robot webhook messages.',
    configFields: [
      { key: 'atMobiles', label: '@ mobile numbers', type: 'tags', placeholder: '13800138000' },
      { key: 'atUserIds', label: '@ user IDs', type: 'tags', placeholder: 'manager001' },
      { key: 'isAtAll', label: '@ all members', type: 'boolean', defaultValue: false },
    ],
    secretFields: [webhookUrlField, { key: 'signingKey', label: 'Signing secret', type: 'string', secret: true, placeholder: 'SECxxxxxxxxxxxxxxxx' }],
    messageTypes: [
      { id: 'text', label: 'Text', description: 'DingTalk text message.', templateFields: [{ key: 'content', label: 'Content template', type: 'textarea', required: true, rows: 5, placeholder: '{{subject}}\n{{body}}' }] },
      { id: 'markdown', label: 'Markdown', description: 'DingTalk markdown message.', templateFields: [{ key: 'title', label: 'Title template', type: 'string', required: true, placeholder: '{{subject}}' }, { key: 'text', label: 'Markdown template', type: 'textarea', required: true, rows: 8, placeholder: '### {{subject}}\n\n{{body}}' }] },
      { id: 'link', label: 'Link', description: 'DingTalk link card.', templateFields: [{ key: 'title', label: 'Title template', type: 'string', required: true, placeholder: '{{subject}}' }, { key: 'text', label: 'Text template', type: 'textarea', required: true, rows: 4, placeholder: '{{body}}' }, { key: 'messageUrl', label: 'Message URL', type: 'url', required: true, placeholder: 'https://tikeo.example.com/instances/{{resourceId}}' }, { key: 'picUrl', label: 'Picture URL', type: 'url' }] },
      { id: 'actionCard', label: 'ActionCard', description: 'DingTalk action card.', templateFields: [{ key: 'title', label: 'Title template', type: 'string', required: true, placeholder: '{{subject}}' }, { key: 'text', label: 'Markdown template', type: 'textarea', required: true, rows: 8, placeholder: '### {{subject}}\n\n{{body}}' }, { key: 'singleTitle', label: 'Button title', type: 'string', placeholder: 'Open Tikeo' }, { key: 'singleURL', label: 'Button URL', type: 'url', placeholder: 'https://tikeo.example.com/instances/{{resourceId}}' }] },
      { id: 'feedCard', label: 'FeedCard', description: 'DingTalk feed card. Configure links JSON.', templateFields: [{ key: 'links', label: 'Links JSON template', type: 'textarea', required: true, rows: 8, placeholder: '[{"title":"{{subject}}","messageURL":"https://tikeo.example.com/instances/{{resourceId}}","picURL":""}]' }] },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'DingTalk custom robot', url: 'https://open.dingtalk.com/document/group/custom-robot-access' }, { label: 'DingTalk robot message types', url: 'https://open.dingtalk.com/document/development/robot-message-type' }],
    defaultMessageType: 'markdown',
  },
  feishu: {
    provider: 'feishu',
    label: 'Feishu/Lark Bot',
    category: 'office_bot',
    description: 'Feishu/Lark custom bot webhook messages.',
    configFields: [],
    secretFields: [webhookUrlField, { key: 'signingKey', label: 'Signing secret', type: 'string', secret: true, placeholder: 'SECxxxxxxxxxxxxxxxx' }],
    messageTypes: [
      { id: 'text', label: 'Text', description: 'Plain text custom bot message.', templateFields: [{ key: 'text', label: 'Text template', type: 'textarea', required: true, rows: 5, placeholder: '{{subject}}\n{{body}}' }] },
      { id: 'post', label: 'Rich text post', description: 'Feishu/Lark post message.', templateFields: [{ key: 'title', label: 'Title template', type: 'string', required: true, placeholder: '{{subject}}' }, { key: 'content', label: 'Post content JSON template', type: 'textarea', required: true, rows: 10, placeholder: '[[{"tag":"text","text":"{{body}}"}]]' }] },
      { id: 'image', label: 'Image', description: 'Feishu/Lark image message using image_key.', templateFields: [{ key: 'imageKey', label: 'Image key template', type: 'string', required: true, placeholder: 'img_v3_...' }] },
      { id: 'share_chat', label: 'Share chat', description: 'Feishu/Lark share_chat message using share_chat_id.', templateFields: [{ key: 'shareChatId', label: 'Share chat ID template', type: 'string', required: true, placeholder: 'oc_...' }] },
      { id: 'interactive', label: 'Interactive card', description: 'One-way custom bot card message.', templateFields: [{ key: 'card', label: 'Card JSON template', type: 'textarea', required: true, rows: 12, placeholder: '{"header":{"title":{"tag":"plain_text","content":"{{subject}}"}},"elements":[{"tag":"div","text":{"tag":"lark_md","content":"{{body}}"}}]}' }] },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'Feishu custom bot', url: 'https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot' }, { label: 'Feishu card with custom bot', url: 'https://open.feishu.cn/document/common-capabilities/message-card/getting-started/send-message-cards-with-a-custom-bot' }],
    defaultMessageType: 'text',
  },
  wechat_work: {
    provider: 'wechat_work',
    label: 'WeCom Bot',
    category: 'office_bot',
    description: 'WeCom / WeChat Work group robot webhook messages.',
    configFields: [
      { key: 'mentionedList', label: 'Mentioned user IDs', type: 'tags', placeholder: '@all or user IDs' },
      { key: 'mentionedMobileList', label: 'Mentioned mobile numbers', type: 'tags', placeholder: '13800138000' },
    ],
    secretFields: [webhookUrlField],
    messageTypes: [
      { id: 'text', label: 'Text', description: 'WeCom text message.', templateFields: [{ key: 'content', label: 'Content template', type: 'textarea', required: true, rows: 5, placeholder: '{{subject}}\n{{body}}' }] },
      { id: 'markdown', label: 'Markdown', description: 'WeCom markdown message.', templateFields: [{ key: 'content', label: 'Markdown template', type: 'textarea', required: true, rows: 8, placeholder: '### {{subject}}\n{{body}}' }] },
      { id: 'markdown_v2', label: 'Markdown v2', description: 'WeCom markdown_v2 message.', templateFields: [{ key: 'content', label: 'Markdown v2 template', type: 'textarea', required: true, rows: 8, placeholder: '# {{subject}}\n{{body}}' }] },
      { id: 'image', label: 'Image', description: 'Image message using base64/md5 fields.', templateFields: [{ key: 'base64', label: 'Image base64 template', type: 'textarea', required: true, rows: 6 }, { key: 'md5', label: 'Image MD5 template', type: 'string', required: true }] },
      { id: 'news', label: 'News', description: 'News/articles message.', templateFields: [{ key: 'articles', label: 'Articles JSON template', type: 'textarea', required: true, rows: 8, placeholder: '[{"title":"{{subject}}","description":"{{body}}","url":"https://tikeo.example.com/instances/{{resourceId}}"}]' }] },
      { id: 'file', label: 'File', description: 'File message using media_id obtained from WeCom upload API.', templateFields: [{ key: 'media_id', label: 'Media ID template', type: 'string', required: true, placeholder: 'MEDIA_ID_FROM_WECOM_UPLOAD' }] },
      { id: 'voice', label: 'Voice', description: 'Voice message using media_id obtained from WeCom upload API.', templateFields: [{ key: 'media_id', label: 'Media ID template', type: 'string', required: true, placeholder: 'MEDIA_ID_FROM_WECOM_UPLOAD' }] },
      { id: 'template_card', label: 'Template card', description: 'WeCom template_card message for richer notices.', templateFields: [{ key: 'templateCard', label: 'Template card JSON template', type: 'textarea', required: true, rows: 12, placeholder: '{"card_type":"text_notice","main_title":{"title":"{{subject}}","desc":"{{body}}"},"card_action":{"type":1,"url":"https://tikeo.example.com/instances/{{resourceId}}"}}' }] },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'WeCom group robot', url: 'https://developer.work.weixin.qq.com/document/path/91770' }],
    defaultMessageType: 'markdown',
  },
  pagerduty: {
    provider: 'pagerduty',
    label: 'PagerDuty Events API',
    category: 'incident',
    description: 'PagerDuty Events API v2 incident lifecycle events.',
    configFields: [
      { key: 'dedupKey', label: 'Dedup key template', type: 'string', placeholder: '{{dedupeKey}}' },
      { key: 'source', label: 'Source template', type: 'string', defaultValue: 'tikeo', placeholder: 'tikeo' },
      { key: 'severity', label: 'PagerDuty severity', type: 'select', options: [{ value: 'info', label: 'Info' }, { value: 'warning', label: 'Warning' }, { value: 'error', label: 'Error' }, { value: 'critical', label: 'Critical' }] },
      { key: 'timestamp', label: 'Event timestamp template', type: 'string', placeholder: '{{triggeredAt}}' },
      { key: 'component', label: 'Component template', type: 'string', placeholder: '{{resourceType}}' },
      { key: 'group', label: 'Group template', type: 'string' },
      { key: 'class', label: 'Class template', type: 'string', placeholder: '{{eventType}}' },
      { key: 'client', label: 'Client template', type: 'string', placeholder: 'tikeo' },
      { key: 'clientUrl', label: 'Client URL template', type: 'url', placeholder: 'https://tikeo.example.com/instances/{{resourceId}}' },
      { key: 'links', label: 'Links JSON template', type: 'textarea', rows: 6, placeholder: '[{"href":"https://tikeo.example.com/instances/{{resourceId}}","text":"Open Tikeo"}]' },
      { key: 'images', label: 'Images JSON template', type: 'textarea', rows: 6, placeholder: '[{"src":"https://example.invalid/chart.png","href":"https://tikeo.example.com","alt":"chart"}]' },
      { key: 'customDetails', label: 'Custom details JSON template', type: 'textarea', rows: 8, placeholder: '{"eventType":"{{eventType}}","resourceId":"{{resourceId}}"}' },
    ],
    secretFields: [{ key: 'routingKey', label: 'Routing / integration key', type: 'string', required: true, secret: true, placeholder: 'PAGERDUTY_ROUTING_KEY' }],
    messageTypes: [
      { id: 'trigger', label: 'Trigger', description: 'Create or update a PagerDuty alert.', templateFields: [{ key: 'summary', label: 'Summary template', type: 'string', required: true, placeholder: '{{subject}}' }] },
      { id: 'acknowledge', label: 'Acknowledge', description: 'Acknowledge an existing event by dedup key.', templateFields: [] },
      { id: 'resolve', label: 'Resolve', description: 'Resolve an existing event by dedup key.', templateFields: [] },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'PagerDuty Events API v2', url: 'https://developer.pagerduty.com/docs/events-api-v2-overview' }, { label: 'Send an alert event', url: 'https://developer.pagerduty.com/docs/send-alert-event' }],
    defaultMessageType: 'trigger',
  },
  email: {
    provider: 'email',
    label: 'SMTP Email',
    category: 'email',
    description: 'SMTP email delivery. Runtime currently sends text/plain mail.',
    configFields: [
      { key: 'to', label: 'Recipients', type: 'emailList', required: true, placeholder: 'ops@example.com' },
      { key: 'from', label: 'From address', type: 'string', placeholder: 'tikeo@example.com' },
      { key: 'username', label: 'SMTP username', type: 'string' },
    ],
    secretFields: [
      { key: 'smtpUrl', label: 'SMTP URL', type: 'string', required: true, secret: true, placeholder: 'smtp+starttls://smtp.example.com:587' },
      { key: 'password', label: 'SMTP password', type: 'string', secret: true, placeholder: 'SMTP password' },
    ],
    messageTypes: [
      { id: 'plain', label: 'Plain text', description: 'Text/plain email body.', templateFields: [{ key: 'subject', label: 'Subject template', type: 'string', required: true, placeholder: '[tikeo/{{severity}}] {{subject}}' }, { key: 'body', label: 'Body template', type: 'textarea', required: true, rows: 8, placeholder: '{{body}}\n\nResource: {{resourceType}}/{{resourceId}}' }] },
      { id: 'html', label: 'HTML template', description: 'Stored template shape for future HTML delivery; current runtime falls back to text body.', templateFields: [{ key: 'subject', label: 'Subject template', type: 'string', required: true, placeholder: '[tikeo/{{severity}}] {{subject}}' }, { key: 'html', label: 'HTML template', type: 'textarea', rows: 10, placeholder: '<h1>{{subject}}</h1><p>{{body}}</p>' }, { key: 'body', label: 'Text fallback template', type: 'textarea', required: true, rows: 6, placeholder: '{{body}}' }] },
    ],
    templateVariables: DEFAULT_TEMPLATE_VARIABLES,
    docs: [{ label: 'SMTP RFC 5321', url: 'https://datatracker.ietf.org/doc/rfc5321/' }, { label: 'Internet Message Format RFC 5322', url: 'https://datatracker.ietf.org/doc/rfc5322/' }],
    defaultMessageType: 'plain',
  },
};

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}


function exampleTemplate(provider: string, messageType: string): Record<string, unknown> {
  if (provider === 'slack' && messageType === 'blockKit') return { messageType, text: '[tikeo] {{subject}}', blocks: [{ type: 'section', text: { type: 'mrkdwn', text: '*{{subject}}*\n{{body}}' } }] };
  if (provider === 'slack' && messageType === 'attachments') return { messageType, text: '[tikeo] {{subject}}', attachments: [{ color: '#439FE0', title: '{{subject}}', text: '{{body}}' }] };
  if (provider === 'slack') return { messageType: 'text', text: '[tikeo/{{severity}}] {{subject}}\n{{body}}' };
  if (provider === 'dingtalk' && messageType === 'markdown') return { messageType, title: '{{subject}}', text: '### {{subject}}\n\n{{body}}' };
  if (provider === 'dingtalk' && messageType === 'link') return { messageType, title: '{{subject}}', text: '{{body}}', messageUrl: 'https://tikeo.example.com/instances/{{resourceId}}', picUrl: 'https://tikeo.example.com/logo.png' };
  if (provider === 'dingtalk' && messageType === 'actionCard') return { messageType, title: '{{subject}}', text: '### {{subject}}\n\n{{body}}', singleTitle: 'Open Tikeo', singleURL: 'https://tikeo.example.com/instances/{{resourceId}}' };
  if (provider === 'dingtalk' && messageType === 'feedCard') return { messageType, links: [{ title: '{{subject}}', messageURL: 'https://tikeo.example.com/instances/{{resourceId}}', picURL: 'https://tikeo.example.com/logo.png' }] };
  if (provider === 'dingtalk') return { messageType: 'text', content: '{{subject}}\n{{body}}' };
  if (provider === 'feishu' && messageType === 'post') return { messageType, title: '{{subject}}', content: [[{ tag: 'text', text: '{{body}}' }]] };
  if (provider === 'feishu' && messageType === 'image') return { messageType, imageKey: 'img_v3_example_key' };
  if (provider === 'feishu' && messageType === 'share_chat') return { messageType, shareChatId: 'oc_example_chat_id' };
  if (provider === 'feishu' && messageType === 'interactive') return { messageType, card: { header: { title: { tag: 'plain_text', content: '{{subject}}' } }, elements: [{ tag: 'div', text: { tag: 'lark_md', content: '{{body}}' } }] } };
  if (provider === 'feishu') return { messageType: 'text', text: '{{subject}}\n{{body}}' };
  if (provider === 'wechat_work' && messageType === 'markdown') return { messageType, content: '### {{subject}}\n{{body}}' };
  if (provider === 'wechat_work' && messageType === 'markdown_v2') return { messageType, content: '# {{subject}}\n{{body}}' };
  if (provider === 'wechat_work' && messageType === 'image') return { messageType, base64: 'iVBORw0KGgo=', md5: 'd41d8cd98f00b204e9800998ecf8427e' };
  if (provider === 'wechat_work' && messageType === 'news') return { messageType, articles: [{ title: '{{subject}}', description: '{{body}}', url: 'https://tikeo.example.com/instances/{{resourceId}}' }] };
  if (provider === 'wechat_work' && (messageType === 'file' || messageType === 'voice')) return { messageType, media_id: 'MEDIA_ID_FROM_WECOM_UPLOAD' };
  if (provider === 'wechat_work' && messageType === 'template_card') return { messageType, templateCard: { card_type: 'text_notice', main_title: { title: '{{subject}}', desc: '{{body}}' }, card_action: { type: 1, url: 'https://tikeo.example.com/instances/{{resourceId}}' } } };
  if (provider === 'wechat_work') return { messageType: 'text', content: '{{subject}}\n{{body}}' };
  if (provider === 'pagerduty' && messageType === 'acknowledge') return { messageType, customDetails: { eventType: '{{eventType}}', resourceId: '{{resourceId}}' } };
  if (provider === 'pagerduty' && messageType === 'resolve') return { messageType, customDetails: { eventType: '{{eventType}}', resourceId: '{{resourceId}}' } };
  if (provider === 'pagerduty') return { messageType: 'trigger', summary: '{{subject}}', customDetails: { body: '{{body}}', eventType: '{{eventType}}' } };
  if (provider === 'email' && messageType === 'html') return { messageType, subject: '[tikeo/{{severity}}] {{subject}}', html: '<h1>{{subject}}</h1><p>{{body}}</p>', body: '{{body}}' };
  if (provider === 'email') return { messageType: 'plain', subject: '[tikeo/{{severity}}] {{subject}}', body: '{{body}}\n\nResource: {{resourceType}}/{{resourceId}}' };
  return { messageType: 'json', body: { text: '{{subject}}', body: '{{body}}', eventType: '{{eventType}}' } };
}

function directWebhookUrl(provider: string, messageType: string): string {
  if (provider === 'slack') return `https://hooks.slack.com/services/T00000000/B00000000/${messageType.replace(/[^A-Za-z0-9]+/g, '_').toUpperCase()}_WEBHOOK`;
  if (provider === 'dingtalk') return 'https://oapi.dingtalk.com/robot/send?access_token=xxxxxxxx';
  if (provider === 'feishu') return 'https://open.feishu.cn/open-apis/bot/v2/hook/xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx';
  if (provider === 'wechat_work') return 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx';
  return `https://hooks.example.com/tikeo/${provider}/${messageType}`;
}

function directSigningSecret(provider: string, messageType: string): string {
  return `SEC_${provider.toUpperCase()}_${messageType.replace(/[^A-Za-z0-9]+/g, '_').toUpperCase()}_SIGNING_SECRET`;
}

function exampleSecretRefs(provider: string, messageType: string): Record<string, unknown> {
  if (provider === 'slack') return { url: directWebhookUrl(provider, messageType) };
  if (provider === 'dingtalk') return { url: directWebhookUrl(provider, messageType), signingKey: directSigningSecret(provider, messageType) };
  if (provider === 'feishu') return { url: directWebhookUrl(provider, messageType), signingKey: directSigningSecret(provider, messageType) };
  if (provider === 'wechat_work') return { url: directWebhookUrl(provider, messageType) };
  if (provider === 'pagerduty') return { routingKey: `PAGERDUTY_${messageType.replace(/[^A-Za-z0-9]+/g, '_').toUpperCase()}_ROUTING_KEY` };
  if (provider === 'email') return { smtpUrl: 'smtp+starttls://smtp.example.com:587', password: `SMTP_${messageType.replace(/[^A-Za-z0-9]+/g, '_').toUpperCase()}_PASSWORD` };
  return { url: directWebhookUrl(provider, messageType), authorization: 'Bearer direct-channel-token' };
}

function exampleConfig(provider: string, messageType: string): Record<string, unknown> {
  if (provider === 'dingtalk') return { messageType, isAtAll: false };
  if (provider === 'wechat_work') return { messageType, mentionedList: [], mentionedMobileList: [] };
  if (provider === 'pagerduty') return { messageType, source: 'tikeo', severity: 'critical', dedupKey: '{{dedupeKey}}' };
  if (provider === 'email') return { messageType, to: ['ops@example.com'], from: 'tikeo@example.com' };
  return { messageType };
}

function generatedExample(provider: string, messageType: string): ProviderMessageTypeExample {
  return {
    name: `${provider} ${messageType} smoke`,
    description: '示例：按渠道私密配置保存，真实值保存后立即生效；也可改成 env:NAME 兼容引用。',
    config: exampleConfig(provider, messageType),
    secretRefs: exampleSecretRefs(provider, messageType),
    template: exampleTemplate(provider, messageType),
    sample: {
      subject: 'Tikeo smoke test',
      body: 'A notification channel test was sent from the configuration drawer.',
      eventType: 'notification.channel_test',
      resourceType: 'notification_channel',
      resourceId: 'channel-example',
      severity: 'info',
    },
  };
}

function parseExamples(value: unknown): ProviderMessageTypeExample[] {
  return asArray(value).filter((item): item is ProviderMessageTypeExample => Boolean(item && typeof item === 'object' && 'name' in item));
}

function parseFields(value: unknown): ProviderFieldSchema[] {
  return asArray(value).filter((item): item is ProviderFieldSchema => Boolean(item && typeof item === 'object' && 'key' in item && 'label' in item));
}

function normalizeProviderField(field: ProviderFieldSchema): ProviderFieldSchema {
  if (field.secret && ['url', 'webhookUrl', 'webhook_url'].includes(field.key)) {
    return {
      ...field,
      label: '机器人/Webhook 地址',
      placeholder: field.placeholder ?? 'https://hooks.example.com/tikeo',
      help: field.help ?? '可直接填写真实值（本渠道 Webhook URL），保存后立即生效且响应不会回显；也支持 env:NAME 兼容引用。',
    };
  }
  return field;
}

function parseNormalizedFields(value: unknown): ProviderFieldSchema[] {
  return parseFields(value).map(normalizeProviderField);
}

function parseMessageTypes(value: unknown): ProviderMessageTypeSchema[] {
  return asArray(value).filter((item): item is ProviderMessageTypeSchema => Boolean(item && typeof item === 'object' && 'id' in item && 'label' in item));
}

function parseDocs(value: unknown): ProviderDocLink[] {
  return asArray(value).filter((item): item is ProviderDocLink => Boolean(item && typeof item === 'object' && 'url' in item && 'label' in item));
}

function templateRecord(type?: NotificationChannelTypeSummary): Record<string, unknown> {
  return type?.template && typeof type.template === 'object' && !Array.isArray(type.template) ? type.template : {};
}

export function providerSchemaFor(type?: NotificationChannelTypeSummary | null, provider?: string): ProviderSchema {
  const key = type?.type ?? provider ?? 'webhook';
  const fallback = fallbackSchemas[key] ?? fallbackSchemas.webhook;
  const template = templateRecord(type ?? undefined);
  const parsedMessageTypes = parseMessageTypes(template.messageTypes);
  const rawMessageTypes = parsedMessageTypes.length > 0 ? parsedMessageTypes : fallback.messageTypes;
  const parsedConfigFields = parseNormalizedFields(template.configFields);
  const parsedSecretFields = parseNormalizedFields(template.secretFields);
  const messageTypes = rawMessageTypes.map((item) => {
    const examples = parseExamples(item.examples);
    return { ...item, examples: examples.length > 0 ? examples : [generatedExample(key, item.id)] };
  });
  return {
    ...fallback,
    provider: key,
    label: type?.label ?? fallback.label,
    category: type?.category ?? fallback.category,
    description: type?.description ?? fallback.description,
    configFields: parsedConfigFields.length > 0 ? parsedConfigFields : fallback.configFields,
    secretFields: parsedSecretFields.length > 0 ? parsedSecretFields : fallback.secretFields,
    messageTypes,
    templateVariables: asArray(template.templateVariables).filter((item): item is string => typeof item === 'string').length > 0
      ? asArray(template.templateVariables).filter((item): item is string => typeof item === 'string')
      : fallback.templateVariables,
    docs: parseDocs(template.docs).length > 0 ? parseDocs(template.docs) : fallback.docs,
    defaultMessageType: typeof template.defaultMessageType === 'string' ? template.defaultMessageType : fallback.defaultMessageType,
  };
}

export function providerSchemasFor(types: NotificationChannelTypeSummary[]): ProviderSchema[] {
  return types.map((type) => providerSchemaFor(type));
}

export function findMessageType(schema: ProviderSchema, messageType?: string): ProviderMessageTypeSchema {
  return schema.messageTypes.find((item) => item.id === messageType) ?? schema.messageTypes[0];
}
