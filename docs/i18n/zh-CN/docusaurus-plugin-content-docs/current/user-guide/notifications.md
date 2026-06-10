---
title: 通知用户指南
description: Tikeo 通知中心的渠道、策略、消息、投递重试、DLQ、UI 检查和告警边界运维指南。
---

# 通知用户指南

通知中心是 Tikeo 的可复用出站投递层。需要把作业生命周期、治理事件或运维消息发送到 Slack、钉钉、飞书/Lark、企业微信、PagerDuty、Email、通用 Webhook 或插件提供的 Webhook-compatible provider 时，应使用通知中心。

边界必须清楚：

- **通知中心** 负责可复用出站渠道、策略/订阅、标准化消息、投递尝试、重试和 DLQ。当前证据源包括 `crates/tikeo-server/src/notification.rs`、`crates/tikeo-server/src/http/routes/notifications.rs`、`crates/tikeo-storage/src/repository/notification.rs` 和 `web/src/pages/NotificationCenterPage.tsx`。
- **告警** 负责异常条件规则、告警事件、incident-like 状态、silence/recovery/suppression 语义，以及兼容阶段的告警投递账本。普通作业完成消息不要建成告警规则。

## 什么时候使用通知

| 场景 | 推荐 event family | 示例事件 |
| --- | --- | --- |
| 作业成功后发确认消息 | `job_instance` | `job_instance.succeeded` |
| 作业终态失败或重试耗尽 | `job_instance` | `job_instance.failed`, `job_instance.retry_exhausted` |
| 广播执行部分失败 | `job_instance` | `job_instance.partial_failed` |
| 没有匹配 Worker | `job_instance` | `job_instance.no_eligible_worker` |
| 脚本治理阻断执行 | `job_instance` 或 `script_governance` | `job_instance.script_governance_failure` |
| 告警系统产生事件 | `alert` | `alert.firing`、`alert.recovered` 是已接受的策略族概念，但当前尚未由通知中心物化。 |

需要条件评估、去重、静默、恢复、异常历史或 incident review 时使用告警；只需要出站消息时使用通知中心。

## Provider 类型

内置渠道类型来自 `crates/tikeo-server/src/http/routes/notifications.rs` 中的 `builtin_channel_types()`。

| Provider | 必填 config keys | Secret config keys | 说明 |
| --- | --- | --- | --- |
| `webhook` | `url` | `authorization` | 通用 JSON POST。 |
| `slack` | `url` | 无 | Slack incoming webhook。 |
| `dingtalk` | `url` | `signingKey` | 钉钉机器人。 |
| `feishu` | `url` | `signingKey` | 飞书/Lark 机器人。 |
| `wechat_work` | `url` | 无 | 企业微信/WeCom 机器人。 |
| `pagerduty` | `routingKey` | `routingKey` | PagerDuty Events v2。 |
| `email` | `smtpUrl`, `to` | `password`, `smtpUrl` | SMTP 邮件投递。 |
| 插件类型 | 通常为 `url` | 插件定义 | 兼容使用插件 alert channel metadata。 |

Webhook-style provider 接受 `url`、`webhookUrl` 或 `webhook_url`。PagerDuty 接受 `routingKey`、`routing_key`、`integrationKey` 或 `integration_key`。Email 接受 `to` 或 `recipients`；SMTP endpoint 可以来自 `config.smtpUrl`、`config.smtp_url`、`config.url`、`secretRefs.smtpUrl`、`secretRefs.smtp_url`、`secretRefs.url`、`config.smtpUrlSecretRef`、`config.smtp_url_secret_ref`、`secretRefs.smtpUrlSecretRef` 或 `secretRefs.smtp_url_secret_ref`。SMTP auth password 使用 `config.passwordSecretRef`、`config.password_secret_ref`、`secretRefs.password`、`secretRefs.passwordSecretRef` 或 `secretRefs.password_secret_ref`。

Notification Center 当前运行时 secret 解析只支持 `env:` 引用或裸环境变量名，并从进程环境读取。不要在 `config` 或 `secretRefs` 中填写真实密钥值。

## 设置流程

1. **检查权限。** `/notifications` 需要 `notifications:read`；创建/更新渠道和策略需要 `notifications:manage`；手动扫描重试需要 `notifications:test`。
2. **创建渠道。** 渠道是可复用目的地，scope 可以是 `global`、`namespace`、`app` 或 `worker_pool`。
3. **创建策略。** 策略绑定 owner、event family、filter、severity、dedupe window 和有序渠道引用。
4. **验证策略。** validation 会检查引用渠道是否存在且启用。
5. **等待或触发源事件。** 已实现的作业生命周期事件通过 `NotificationCenter::emit_job_instance_event()` 物化消息。
6. **检查消息和投递尝试。** UI 展示近期消息与队列状态；API 支持过滤。
7. **处理重试和 DLQ。** 后台 worker 会扫描 due attempts；也可以用 retry-due endpoint 做受控扫描。

## 安全渠道创建示例

响应统一使用 `{code,message,data}` envelope。示例只使用占位 URL 和 secret ref；不要把真实 webhook token、SMTP password、routing key 或 authorization header 写入文档、截图、工单或聊天。

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-channels \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "scopeType": "app",
    "namespace": "prod",
    "app": "billing",
    "name": "billing-ops-webhook",
    "provider": "webhook",
    "enabled": true,
    "config": {"url": "https://hooks.example.invalid/tikeo/billing"},
    "secretRefs": {"authorization": "env:TIKEO_NOTIFICATION_WEBHOOK_AUTH"}
  }'
```

响应中的 `id` 由存储生成。`secretRefsJson` 不序列化返回，`configJson` 会被 `NotificationChannelSummary::from()` 脱敏。

## 安全策略创建示例

`channelRefs` 可以是字符串，也可以是包含 `channelId`、`channel_id` 或 `id` 的对象。

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-policies \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "ownerType": "app",
    "ownerId": "prod/billing",
    "name": "billing terminal failures",
    "eventFamily": "job_instance",
    "eventFilter": {
      "eventTypes": ["job_instance.failed", "job_instance.retry_exhausted"],
      "statuses": ["failed", "retry_exhausted"]
    },
    "channelRefs": [{"channelId": "notification-channel-example"}],
    "templateRef": null,
    "severity": "critical",
    "enabled": true,
    "dedupeSeconds": 300
  }'
```

创建后验证：

```bash
curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-policies/notification-policy-example:validate \
  -H 'Authorization: Bearer <operator-token>'
```

## Owner 与事件语义

API 当前接受 owner 类型：`global`、`namespace`、`app`、`job`、`workflow`、`workflow_node`、`alert_rule`、`worker_pool`；接受 event family：`job_instance`、`workflow`、`alert`、`worker`、`script_governance`。当前运行时物化只对 `job_instance` 策略实现，并只匹配 `global`、`namespace`、`app`、`job` owner。

对作业实例事件，当前 materializer 匹配：所有 `global`；`ownerId` 等于 namespace 的 `namespace`；`ownerId` 等于 app 或 `namespace/app` 的 `app`；`ownerId` 等于 job id 的 `job`。Filter 会用 `statuses` 匹配稳定状态 token，用 `eventTypes` 或 `event_types` 匹配完整事件名。

## 已实现作业事件

| Event type | 默认 severity | 含义 |
| --- | --- | --- |
| `job_instance.retry_scheduled` | `warning` | 失败后安排了下一次重试。 |
| `job_instance.retry_exhausted` | `critical` | 重试次数已耗尽。 |
| `job_instance.succeeded` | `info` | 实例终态成功。 |
| `job_instance.failed` | `critical` | 实例终态失败。 |
| `job_instance.partial_failed` | `critical` | 广播父实例部分失败。 |
| `job_instance.cancelled` | `warning` | 用户或系统取消实例。 |
| `job_instance.no_eligible_worker` | `critical` | 没有符合 capability 的 Worker。 |
| `job_instance.script_governance_failure` | `critical` | 脚本治理失败已物化。 |

如果还会重试，不要把每次失败都当成 `job_instance.failed`；先发 `retry_scheduled`，终态才使用 `failed` 或 `retry_exhausted`。

## 队列、重试与 DLQ

通用投递尝试写入 `notification_delivery_attempts`。当前运行时会写入的 attempt retry state 是 `retry_pending`、`retry_consumed`、`delivered` 和 `dead_letter`；queue status 会把未知或历史状态计入 `failed` 桶。当前运行时会写入的 message status 是 `pending`、`delivered` 和 `dead_letter`；存储字段是字符串并预留未来状态。

`notification_delivery` 默认值：

| Key | 默认值 |
| --- | --- |
| `notification_delivery.enabled` | `true` |
| `notification_delivery.interval_seconds` | `60` |
| `notification_delivery.batch_size` | `50` |
| `notification_delivery.max_attempts` | `3` |
| `notification_delivery.backoff_seconds` | `300` |

手动扫描：

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-delivery-attempts:retry-due \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"limit":50,"maxAttempts":3,"backoffSeconds":300}'
```

## UI 工作流

打开 `/notifications` 的 **Notification Center / 通知中心**。页面由 `web/src/pages/NotificationCenterPage.tsx` 和 `web/src/api/notifications.ts` 支撑，支持脱敏检查以及受权限控制的渠道/策略创建、编辑、删除和校验。

| Tab | 检查内容 |
| --- | --- |
| Channels | 名称、provider、scope、脱敏 target、secret 是否配置、enabled，以及创建/编辑/删除抽屉；被策略引用时由后端拒绝删除。 |
| Policies | owner、event family、severity、dedupe seconds、enabled、创建/编辑/删除、渠道多选、JSON 事件过滤和策略校验。 |
| Delivery | 总尝试数、delivered、retry pending、retry consumed、DLQ、failed、最近 DLQ、Retry due。 |
| Messages | 最近标准化消息、事件、资源、subject、状态、创建时间。 |

常规渠道和策略 CRUD/校验可直接使用 UI；自动化、批量变更或表单尚未优化的字段继续使用 Management API。

## 排障 runbook

| 症状 | 检查 | 修复 |
| --- | --- | --- |
| `/notifications` 不可见 | 需要 `notifications:read`。 | 授权或切换 Owner/Operator。 |
| 创建渠道报 provider 错误 | 只接受 built-in 或 enabled plugin type。 | 调 `GET /api/v1/notification-channel-types` 确认类型。 |
| 提示缺少 target | provider 缺少必填 URL、routing key 或 SMTP/recipient。 | 补齐非敏感 target 和 secret refs。 |
| 删除渠道冲突 | 渠道被 enabled/已有策略引用。 | 先更新或删除引用策略。 |
| 策略验证失败 | channelRefs 缺失、错误或指向 disabled channel。 | 修正 ID 或启用渠道。 |
| attempts 卡在 `retry_pending` | 检查 worker 配置、`nextRetryAt` 和 queue status。 | 执行 bounded retry-due，确认后台 worker 启用。 |
| 进入 `dead_letter` | 次数耗尽、message/channel 缺失或 channel disabled。 | 修复上下文后由新事件生成新 message。 |
| Webhook URL 被拒绝 | URL safety policy 拒绝。 | 生产使用 HTTPS/public target；本地 smoke 才使用 loopback override。 |

## 告警边界清单

普通生命周期消息使用通知中心；异常条件和 incident review 使用告警。可复用目的地放在 Notification Center channel 中，不要把同一个 token 复制到多个 alert rule。任何 secret 都应通过 secret refs 管理，不能出现在示例或 UI 截图中。
