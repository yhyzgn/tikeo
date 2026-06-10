---
title: 通知中心参考
description: Tikeo 通知中心 API、配置、存储、事件、脱敏、重试、DLQ 与 UI 的源码证据参考。
---

# 通知中心参考

本页是通用 Notification Center 实现的 source-backed reference，配合 [通知用户指南](../user-guide/notifications) 和 [告警用户指南](../user-guide/alerts) 使用。

主要证据源：

- 设计与边界：`design/notification-center-alerting-plan.md`
- 物化与投递：`crates/tikeo-server/src/notification.rs`
- HTTP routes 与 OpenAPI annotations：`crates/tikeo-server/src/http/routes/notifications.rs`
- 存储 repository 与脱敏：`crates/tikeo-storage/src/repository/notification.rs`
- 实体/迁移：`crates/tikeo-storage/src/entities/notification_*.rs`、`crates/tikeo-storage/src/migration/notification_center.rs`
- 配置默认值：`crates/tikeo-config/src/lib.rs`、`config/dev.toml`、`config/container.toml`
- Web UI：`web/src/pages/NotificationCenterPage.tsx`、`web/src/api/notifications.ts`

## 领域模型

| Record | Table | 用途 |
| --- | --- | --- |
| Channel | `notification_channels` | 带 scope、provider、脱敏 config、secret refs 和 safety policy 的可复用出站目的地。 |
| Policy/subscription | `notification_policies` | owner/event filter 到有序 channel refs 和可选 template refs 的映射。 |
| Message | `notification_messages` | 从源事件和策略产生的标准化出站消息。 |
| Delivery attempt | `notification_delivery_attempts` | 一个 message/channel 的一次 provider 投递尝试，带 retry 和 DLQ 状态。 |

迁移遵循 soft-link 约定，不使用数据库级 foreign key。`policy_id`、`message_id`、`channel_id` 等由 repository 和 service 显式检查。

## 配置

通用投递 worker 位于 `notification_delivery`。

```toml
[notification_delivery]
enabled = true
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300
```

| Key | 默认值 | 环境变量 | 含义 |
| --- | --- | --- | --- |
| `notification_delivery.enabled` | `true` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | 启动通用投递 worker。 |
| `notification_delivery.interval_seconds` | `60` | `TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS` | due-attempt 扫描间隔。 |
| `notification_delivery.batch_size` | `50` | `TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE` | 每轮最大扫描数量。 |
| `notification_delivery.max_attempts` | `3` | `TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS` | 进入 dead-letter 前最大尝试次数。 |
| `notification_delivery.backoff_seconds` | `300` | `TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS` | 失败后的退避秒数。 |

手动 retry endpoint 会 clamp 参数：`limit <= 500`，`maxAttempts` 在 `1..20`，`backoffSeconds` 在 `1..86400`。

## RBAC

迁移 seed 了这些权限：

| Permission | 用途 |
| --- | --- |
| `notifications:read` | 列出 channel types、channels、policies、messages、delivery attempts 和 queue status。 |
| `notifications:manage` | 创建、更新、删除 channels 和 policies。 |
| `notifications:test` | 执行 retry-due delivery scan。 |

Web route `/notifications` 同样要求 `notifications:read`。

## API envelope

所有 endpoint 使用共享 envelope：

```json
{"code":0,"message":"success","data":{}}
```

示例只允许占位值。不要在文档中放真实 token、带 secret 的 webhook URL、SMTP password、PagerDuty routing key 或 authorization header。

## Endpoint 汇总

| Method/path | 用途 | 权限 |
| --- | --- | --- |
| `GET /api/v1/notification-channel-types` | Built-in provider metadata 与 enabled plugin channel types。 | `notifications:read` |
| `GET /api/v1/notification-channels` | 列出渠道，支持 scope/provider/enabled filters。 | `notifications:read` |
| `POST /api/v1/notification-channels` | 创建渠道。 | `notifications:manage` |
| `GET /api/v1/notification-channels/{id}` | 读取一个脱敏 channel summary。 | `notifications:read` |
| `PATCH /api/v1/notification-channels/{id}` | 更新渠道。 | `notifications:manage` |
| `DELETE /api/v1/notification-channels/{id}` | 仅在没有 policy 引用时删除。 | `notifications:manage` |
| `GET /api/v1/notification-policies` | 按 owner/event/enabled filter 列出策略。 | `notifications:read` |
| `POST /api/v1/notification-policies` | 创建策略。 | `notifications:manage` |
| `GET /api/v1/notification-policies/{id}` | 读取策略。 | `notifications:read` |
| `PATCH /api/v1/notification-policies/{id}` | 更新策略。 | `notifications:manage` |
| `DELETE /api/v1/notification-policies/{id}` | 删除策略。 | `notifications:manage` |
| `POST /api/v1/notification-policies/{id}:validate` | 验证 channel refs。 | `notifications:read` |
| `GET /api/v1/notification-messages` | 列出标准化消息。 | `notifications:read` |
| `GET /api/v1/notification-delivery-attempts` | 列出投递尝试。 | `notifications:read` |
| `GET /api/v1/notification-delivery-attempts:queue-status` | 统计 retry/DLQ 并返回最近 dead letters。 | `notifications:read` |
| `POST /api/v1/notification-delivery-attempts:retry-due` | 处理 due attempts。 | `notifications:test` |

当前源码没有独立的 `POST /api/v1/notification-channels/{id}:test` endpoint。Channel type metadata 现在返回 `supportsTestSend: false`；已实现的 operator action 是 generic retry-due scan。

## Channel 字段

`CreateNotificationChannelRequest`：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `scopeType` | string | 是 | `global`、`namespace`、`app` 或 `worker_pool`。 |
| `namespace` | string/null | 否 | Scope qualifier。 |
| `app` | string/null | 否 | Scope qualifier。 |
| `workerPool` | string/null | 否 | Scope qualifier。 |
| `name` | string | 是 | 不能为空，scope 维度内唯一。 |
| `provider` | string | 是 | lowercase slug，built-in 或 enabled plugin type。 |
| `enabled` | boolean | 否 | 默认 `true`。Disabled channel 不投递。 |
| `config` | object | 否 | Provider config；summary 会脱敏。 |
| `secretRefs` | object | 否 | Secret references；`secretRefsJson` 不返回。 |
| `safetyPolicy` | object/null | 否 | 本地 smoke transport override。 |

Provider validation：

- Webhook-style provider 需要 `url`、`webhookUrl` 或 `webhook_url`。
- PagerDuty 需要 `routingKey`、`routing_key`、`integrationKey` 或 `integration_key`。
- Email 需要 `to` 或 `recipients`，并通过直接配置或 secret ref 提供 SMTP URL/config。运行时接受与 metadata 对齐的 `secretRefs.password` 作为 SMTP password reference alias，也接受 `passwordSecretRef` / `password_secret_ref`；SMTP URL reference alias 包括 `smtpUrl`、`smtp_url`、`url`、`smtpUrlSecretRef` 和 `smtp_url_secret_ref`。
- 本实现中的 secret 解析基于环境变量：`env:NAME` 和裸 `NAME` 都会从进程环境变量读取。

## 脱敏行为

`NotificationChannelSummary` 包含 `configJson`、`targetRedacted`、`targetConfigured` 和 `secretConfigured`。脱敏逻辑在 `crates/tikeo-storage/src/repository/notification.rs`：

- key 包含 `secret`、`token`、`password`、`authorization` 或 routing-key 变体时替换为 `***redacted***`。
- URL-like 值只保留 scheme、host、可选 port 和 `...` path。
- `config.headers` 下的值全部脱敏，包括 `X-API-Key` 这类不含明显 secret 词的 header 名。
- `secret_refs_json` 使用 `skip_serializing`，不应出现在 API response。
- UI 和日志应使用 `targetRedacted`，不要使用原始 provider config。

## Policy 字段

`CreateNotificationPolicyRequest`：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `ownerType` | string | 是 | API 接受 `global`、`namespace`、`app`、`job`、`workflow`、`workflow_node`、`alert_rule`、`worker_pool`；当前运行时只为 `job_instance` 匹配 `global`、`namespace`、`app`、`job`。 |
| `ownerId` | string/null | 否 | Soft-linked owner。 |
| `name` | string | 是 | 不能为空。 |
| `eventFamily` | string | 是 | API 接受 `job_instance`、`workflow`、`alert`、`worker`、`script_governance`；当前运行时物化只实现 `job_instance`。 |
| `eventFilter` | object | 否 | 作业 materializer 支持 `statuses` 和 `eventTypes`/`event_types`。 |
| `channelRefs` | array | 是 | 有序渠道引用，不能为空。 |
| `templateRef` | string/null | 否 | Soft link；当前 materializer 使用内置渲染。 |
| `severity` | string | 是 | 为空时 service 侧会从事件推导默认 severity。 |
| `enabled` | boolean | 否 | 默认 `true`。 |
| `dedupeSeconds` | integer | 否 | 默认 `300`。 |

`PATCH` 还接受 `throttle`、`quietHours`、`escalation` JSON。当前 job-event materialization 只实现事件 filter 和 dedupe；在 service 代码实现前，不要宣称 throttle/quiet-hours/escalation 已完整执行。

## Message 与 attempt 字段

`NotificationMessageSummary` 包含 `sourceType`、`sourceId`、`policyId`、`eventType`、`resourceType`、`resourceId`、`severity`、`subject`、`body`、`payloadJson`、`dedupeKey`、可选 `traceId`、`status`、`createdAt`、`updatedAt`。

`NotificationDeliveryAttemptSummary` 包含 `messageId`、`policyId`、`channelId`、`provider`、`targetRedacted`、`attempt`、`delivered`、可选 `statusCode`、可选 `error`、`retryState`、可选 `nextRetryAt`、`createdAt`。

Retry 行为：加载 due attempts；达到 `maxAttempts` 进入 `dead_letter`；message/channel 缺失或 channel disabled 也进入 dead-letter；provider call 执行时当前 attempt 仍保持可重试，避免进程在投递前或投递中崩溃后丢失唯一 pending 行；provider result row 持久化后才把旧 attempt 标记为 `retry_consumed`；成功则 message delivered，失败则追加新的 `retry_pending` 或耗尽后 dead-letter。

## 作业事件契约

| Event type | Filter status | 默认 severity |
| --- | --- | --- |
| `job_instance.retry_scheduled` | `retry_scheduled` | `warning` |
| `job_instance.retry_exhausted` | `retry_exhausted` | `critical` |
| `job_instance.succeeded` | `succeeded` | `info` |
| `job_instance.failed` | `failed` | `critical` |
| `job_instance.partial_failed` | `partial_failed` | `critical` |
| `job_instance.cancelled` | `cancelled` | `warning` |
| `job_instance.no_eligible_worker` | `no_eligible_worker` | `critical` |
| `job_instance.script_governance_failure` | `script_governance_failure` | `critical` |

`JobNotificationEvent::from_terminal_status()` 只映射 `succeeded`、`failed`、`partial_failed`、`cancelled`；pending、dispatching、running 不产生 terminal notification。

## Provider 投递行为

| Provider | 投递行为 |
| --- | --- |
| `webhook` | POST provider-neutral JSON payload。 |
| `slack` | POST `{text}` 简要消息。 |
| `dingtalk` | POST text message。 |
| `feishu` | POST 飞书/Lark text message。 |
| `wechat_work` | POST 企业微信 text message。 |
| `pagerduty` | POST Events API v2 trigger；未配置 URL 时默认 `https://events.pagerduty.com/v2/enqueue`。 |
| `email` | 复用 `AlertDispatcher` 的 email 分支。 |
| plugin webhook | POST provider-neutral JSON，带配置 headers。 |

URL safety 使用 `alert::validate_webhook_url()`。生产应使用 HTTPS/public target；`safetyPolicy.allowInsecureLoopback` 仅用于明确的本地 smoke。

## UI 参考

`web/src/pages/NotificationCenterPage.tsx` 并行加载：channel types、channels、policies、messages 和 queue-status。页面展示渠道数、策略数、待重试数和 DLQ 数；Tabs 展示 channels、policies、delivery/DLQ 与最近 20 条 messages，并提供 **Retry due** 操作。拥有 `notifications:manage` 的 operator 可创建、编辑、删除 channels/policies；策略校验调用 `POST /api/v1/notification-policies/{id}:validate`。

## 可安全复制的静态命令

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-channel-types \
  -H 'Authorization: Bearer <operator-token>'

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H 'Authorization: Bearer <operator-token>'

curl -fsS 'http://127.0.0.1:9090/api/v1/notification-messages?status=failed' \
  -H 'Authorization: Bearer <operator-token>'
```

这些命令需要运行中的认证 Server；文档检查只验证命令语法和源码路径，端到端执行应放到带测试凭据的 smoke 中。

## 告警边界

告警应成为 notification messages 的 producer，而不是可复用 provider credentials 的 owner。迁移完成前保持：alert rule 是异常条件 evaluator；Notification Center 拥有新的可复用出站目的地；`alert_rules.channels_json` 是兼容行为，不是复制 secret 的推荐位置；普通 job success/failure/cancel 消息属于 notification policy；`firing`、`suppressed`、`silenced`、`recovered` 属于告警。
