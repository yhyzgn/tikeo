import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const routesSource = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const clientSource = readFileSync(new URL('../../api/notifications.ts', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../NotificationCenterPage.tsx', import.meta.url), 'utf8');

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
      'createNotificationChannel',
      'updateNotificationChannel',
      'deleteNotificationChannel',
      'createNotificationPolicy',
      'updateNotificationPolicy',
      'deleteNotificationPolicy',
      'validateNotificationPolicy',
    ]) {
      expect(clientSource).toContain(token);
      expect(pageSource).toContain(token);
    }
    expect(pageSource).toContain('channelDrawerOpen');
    expect(pageSource).toContain('policyDrawerOpen');
    expect(pageSource).toContain('新建渠道');
    expect(pageSource).toContain('新建策略');
    expect(pageSource).toContain('校验');
    expect(pageSource).toContain('删除');
  });

  test('does not overclaim vault secret resolution for notification channels', () => {
    expect(pageSource).toContain('当前运行时解析 env: 前缀或环境变量名');
    expect(pageSource).not.toContain('env 或 vault');
    expect(pageSource).not.toContain('vault 路径');
  });
});
