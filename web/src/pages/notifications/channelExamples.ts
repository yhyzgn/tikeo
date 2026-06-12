import type { NotificationChannelTypeSummary } from '../../api/notifications';
import { providerSchemaFor, type ProviderMessageTypeExample } from './providerSchema';

export interface ChannelConfigExample {
  key: string;
  provider: string;
  providerLabel: string;
  category: string;
  messageType: string;
  messageTypeLabel: string;
  name: string;
  description: string;
  suggestedChannelName: string;
  config: Record<string, unknown>;
  secretRefs: Record<string, unknown>;
  template: Record<string, unknown>;
  sample: Record<string, unknown>;
}

export type ChannelConfigExampleRow = ChannelConfigExample;

const builtInTypeStubs: NotificationChannelTypeSummary[] = [
  ['webhook', 'Generic Webhook', 'webhook', 'HTTP webhook'],
  ['slack', 'Slack Incoming Webhook', 'office_bot', 'Slack robot webhook'],
  ['dingtalk', 'DingTalk Robot', 'office_bot', 'DingTalk robot webhook'],
  ['feishu', 'Feishu/Lark Bot', 'office_bot', 'Feishu/Lark bot webhook'],
  ['wechat_work', 'WeCom Bot', 'office_bot', 'WeChat Work/WeCom robot webhook'],
  ['pagerduty', 'PagerDuty Events API', 'incident', 'PagerDuty Events v2 integration'],
  ['email', 'SMTP Email', 'email', 'SMTP email delivery'],
].map(([type, label, category, description]) => ({
  type,
  label,
  category,
  targetKind: type === 'email' ? 'email' : 'webhook',
  description,
  requiredConfigKeys: [],
  requiredTargetKeys: [],
  secretConfigKeys: [],
  supportsTestSend: true,
  pluginProvided: false,
  template: {},
}));

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function sampleValue(example: ProviderMessageTypeExample): Record<string, unknown> {
  return {
    subject: 'Tikeo smoke test',
    body: 'A notification channel test was sent from the configuration drawer.',
    eventType: 'notification.channel_test',
    resourceType: 'notification_channel',
    resourceId: 'channel-example',
    severity: 'info',
    ...objectValue(example.sample),
  };
}

function exampleName(provider: string, messageType: string, example: ProviderMessageTypeExample, index: number): string {
  const name = example.name.trim();
  return name || `${provider} ${messageType} example ${index + 1}`;
}

function normalizeExample(
  type: NotificationChannelTypeSummary,
  messageType: { id: string; label: string; examples?: ProviderMessageTypeExample[] },
  example: ProviderMessageTypeExample,
  index: number,
): ChannelConfigExample {
  const config = { ...objectValue(example.config), messageType: messageType.id };
  const template = { ...objectValue(example.template), messageType: messageType.id };
  const name = exampleName(type.type, messageType.id, example, index);
  return {
    key: `${type.type}:${messageType.id}:${index}`,
    provider: type.type,
    providerLabel: type.label,
    category: type.category,
    messageType: messageType.id,
    messageTypeLabel: messageType.label,
    name,
    description: example.description ?? 'Safe smoke-test configuration data. Replace env: references with deployment secrets before sending.',
    suggestedChannelName: name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || `${type.type}-${messageType.id}-example`,
    config,
    secretRefs: objectValue(example.secretRefs),
    template,
    sample: sampleValue(example),
  };
}

export function channelConfigExamples(types: NotificationChannelTypeSummary[]): ChannelConfigExample[] {
  const sourceTypes = types.length > 0 ? types : builtInTypeStubs;
  return sourceTypes.flatMap((type) => {
    const schema = providerSchemaFor(type, type.type);
    return schema.messageTypes.flatMap((messageType) => (
      (messageType.examples ?? [])
        .slice(0, 2)
        .map((example, index) => normalizeExample(type, messageType, example, index))
    ));
  });
}

export function channelExampleRows(types: NotificationChannelTypeSummary[]): ChannelConfigExampleRow[] {
  return channelConfigExamples(types);
}
