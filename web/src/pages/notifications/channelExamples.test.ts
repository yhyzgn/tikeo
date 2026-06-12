import { describe, expect, test } from 'bun:test';

import type { NotificationChannelTypeSummary } from '../../api/notifications';
import { channelConfigExamples, channelExampleRows } from './channelExamples';
import { providerSchemaFor } from './providerSchema';

const builtInTypes: NotificationChannelTypeSummary[] = ['webhook', 'slack', 'dingtalk', 'feishu', 'wechat_work', 'pagerduty', 'email'].map((type) => ({
  type,
  label: type,
  category: 'test',
  targetKind: type === 'email' ? 'email' : 'webhook',
  description: type,
  requiredConfigKeys: [],
  requiredTargetKeys: [],
  secretConfigKeys: [],
  supportsTestSend: true,
  pluginProvided: false,
  template: {},
}));

describe('notification channel configuration examples', () => {
  test('provides directly usable example data for every built-in provider message type', () => {
    const examples = channelConfigExamples(builtInTypes);

    for (const type of builtInTypes) {
      const schema = providerSchemaFor(type, type.type);
      for (const messageType of schema.messageTypes) {
        const matches = examples.filter((item) => item.provider === type.type && item.messageType === messageType.id);
        expect(matches.length, `${type.type}/${messageType.id} should have 1-2 configured examples`).toBeGreaterThanOrEqual(1);
        expect(matches.length, `${type.type}/${messageType.id} should have no more than 2 examples`).toBeLessThanOrEqual(2);
        for (const example of matches) {
          expect(example.config).toMatchObject({ messageType: messageType.id });
          expect(example.template).toMatchObject({ messageType: messageType.id });
          expect(example.sample).toMatchObject({ eventType: 'notification.channel_test' });
        }
      }
    }
  });

  test('example secret refs stay as env placeholders and never include raw secrets', () => {
    const rendered = JSON.stringify(channelConfigExamples(builtInTypes));

    expect(rendered).toContain('env:SLACK_WEBHOOK_URL');
    expect(rendered).toContain('env:DINGTALK_WEBHOOK_URL');
    expect(rendered).toContain('env:FEISHU_WEBHOOK_URL');
    expect(rendered).toContain('env:WECOM_WEBHOOK_URL');
    expect(rendered).toContain('env:PAGERDUTY_ROUTING_KEY');
    expect(rendered).toContain('env:TIKEO_SMTP_URL');
    expect(rendered).not.toContain('xoxb-');
    expect(rendered).not.toContain('hooks.slack.com/services/');
    expect(rendered).not.toContain('top-secret');
  });

  test('flattens examples into rows for the notification center use-case tab', () => {
    const rows = channelExampleRows(builtInTypes);

    expect(rows.length).toBeGreaterThan(0);
    expect(rows[0]).toHaveProperty('key');
    expect(rows[0]).toHaveProperty('name');
    expect(rows[0]).toHaveProperty('provider');
    expect(rows[0]).toHaveProperty('messageType');
  });
});
