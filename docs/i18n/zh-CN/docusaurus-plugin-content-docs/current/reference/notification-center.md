---
title: 通知中心参考
description: Tikeo 通知中心 API、配置、存储、事件、脱敏、重试、DLQ 与 UI 的源码证据参考。
---

# 通知中心参考

本页是通用 Notification Center 实现的 operator reference，配合 [通知用户指南](../user-guide/notifications) 和 [告警用户指南](../user-guide/alerts) 使用。

主要证据源：

- 设计与边界：`design/notification-center-alerting-plan.md`
- 物化与投递：`crates/tikeo-server/src/notification.rs`
- HTTP routes 与 OpenAPI annotations：`crates/tikeo-server/src/http/routes/notifications.rs`、`crates/tikeo-server/src/http/routes/notification_templates.rs`
- 存储 repository 与脱敏：`crates/tikeo-storage/src/repository/notification.rs`、`crates/tikeo-storage/src/repository/notification_template.rs`
- 实体/迁移：`crates/tikeo-storage/src/entities/notification_*.rs`、`crates/tikeo-storage/src/migration/notification_center.rs`
- 配置默认值：`crates/tikeo-config/src/lib.rs`、`config/dev.toml`、`config/container.toml`
- Web UI：`web/src/pages/NotificationCenterPage.tsx`、`web/src/api/notifications.ts`

## 领域模型

| Record | Table | 用途 |
| --- | --- | --- |
| Channel | `notification_channels` | 带 scope、provider、脱敏 config、secret refs 和 safety policy 的可复用出站目的地。 |
| Policy/subscription | `notification_policies` | owner/event filter 到有序 channel refs 和可选 template refs 的映射。 |
| Template | `notification_templates` | 可复用 provider/message-type 模板 body，支持安全变量渲染与预览。 |
| Message | `notification_messages` | 从源事件和策略产生的标准化出站消息。 |
| Delivery attempt | `notification_delivery_attempts` | 一个 message/channel 的一次 provider 投递尝试，带 retry 和 DLQ 状态。 |

迁移遵循 soft-link 约定，不使用数据库级 foreign key。`policy_id`、`message_id`、`channel_id` 等由 repository 和 service 显式检查。

## 配置

通用投递 worker 位于 `notification_delivery`。

```toml
[notification_delivery]
enabled = true
# 可选。用于通知卡片按钮的外部可访问 Web 基地址。
# public_console_base_url = "https://tikeo.example.com"
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300
```

| Key | 默认值 | 环境变量 | 含义 |
| --- | --- | --- | --- |
| `notification_delivery.enabled` | `true` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | 启动通用投递 worker。 |
| `notification_delivery.public_console_base_url` | 未设置 | `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL` | 可选的外部可访问 Web 基地址，用于把 `/public/instances/{id}/console` 转成飞书/Lark 卡片按钮可直接打开的绝对 URL。 |
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
| `notifications:test` | 发送已保存渠道的测试通知，并执行 retry-due delivery scan。 |

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
| `GET /api/v1/notification-templates` | 按 provider/message-type/enabled 过滤列出可复用模板。 | `notifications:read` |
| `POST /api/v1/notification-templates` | 创建 provider-specific 可复用模板。 | `notifications:manage` |
| `GET /api/v1/notification-templates/{id-or-key}` | 按 id 或 `templateKey` 读取模板。 | `notifications:read` |
| `PATCH /api/v1/notification-templates/{id}` | 更新模板元数据、body、variables 或 enabled。 | `notifications:manage` |
| `DELETE /api/v1/notification-templates/{id}` | 删除模板行；policy 是 soft link，删除前应先更新引用策略。 | `notifications:manage` |
| `POST /api/v1/notification-templates/{id}/render` | 用 sample JSON 渲染已存储模板或未保存草稿，不执行 provider 投递；`{id}` 可为生成 id、`templateKey`，或在请求体提供 `provider`、`messageType`、`template` 时作为草稿 key。 | `notifications:read` |
| `GET /api/v1/jobs/{job}/notification-bindings` | 列出 Job 级通知绑定，底层为 Job-owned `notification_policies`。 | `jobs:read` + `notifications:read` |
| `POST /api/v1/jobs/{job}/notification-bindings` | 创建 Job 通知绑定，支持成功、失败、总是、取消、重试和高级事件列表。 | `jobs:write` + `notifications:manage` |
| `GET/PATCH/DELETE /api/v1/jobs/{job}/notification-bindings/{binding}` | 读取、更新或删除 Job 通知绑定；owner 不匹配返回 404。 | `jobs:read/write` + `notifications:read/manage` |
| `POST /api/v1/jobs/{job}/notification-bindings:validate` | 校验渠道、模板 provider 兼容性和展开后的 Job 事件类型。 | `jobs:read` + `notifications:read` |
| `POST /api/v1/jobs/{job}/notification-bindings:preview` | 对样例 Job 实例上下文渲染模板，不发送消息。 | `jobs:read` + `notifications:read` |
| `GET /api/v1/notification-messages` | 列出标准化消息。 | `notifications:read` |
| `GET /api/v1/notification-messages/{id}/trace` | 查看消息、policy、投递 attempts、Job/实例上下文和脱敏执行日志摘要。 | `notifications:read`，可解析 Job 时还会做租户 scope 检查 |
| `GET /api/v1/notification-delivery-attempts` | 列出投递尝试。 | `notifications:read` |
| `GET /api/v1/notification-delivery-attempts:queue-status` | 统计 retry/DLQ 并返回最近 dead letters。 | `notifications:read` |
| `POST /api/v1/notification-delivery-attempts:retry-due` | 处理 due attempts。 | `notifications:test` |

内置 channel type metadata 返回 `supportsTestSend=true`。使用 `POST /api/v1/notification-channels/{id}/test-send`、列表行 **测试** 或编辑抽屉 **测试** 来验证某条已保存、已启用渠道；`POST /api/v1/notification-delivery-attempts:retry-due` 则用于 generic due-attempt worker scan。

## Job 通知绑定与消息 Trace

Job 通知绑定是面向任务操作者的配置层，不是新的投递系统。它复用 `notification_policies(owner_type='job', owner_id=job.id, event_family='job_instance')`，并把 `trigger`、`eventTypes`、`channelIds`、`templateRef`、日志链接/摘要配置写入 policy filter/channel refs。

运行时仍由 `NotificationCenter::emit_job_instance_event` 物化消息和投递 attempts。Job 消息 payload 会携带 `jobId`、`jobName`、`namespace`、`app`、`instanceId`、`status`、`triggerType`、`executionMode`、`startedAt`、`finishedAt`、`workerId`、`operatorName`、`operatorType`、`reason`、`logsUrl`、`consoleUrl` 以及嵌套 `job` / `instance` / `operator` / `logs` 对象。

`GET /api/v1/notification-messages/{id}/trace` 用于排障某条通知消息：响应包含标准化 message、policy、delivery attempts、Job/实例上下文，以及最近 80 行执行日志摘要。日志展示层会对 password/token/secret/authorization/routingKey/signingKey 等常见 key-value 片段做脱敏；trace 不会调用外部 provider，也不会回显渠道私密配置。


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
| `secretRefs` | object | 否 | 这条渠道记录自己的 Secret references；`secretRefsJson` 不返回。 |
| `safetyPolicy` | object/null | 否 | 本地 smoke transport override。 |

Provider validation：

- Webhook-style provider 需要 `url`、`webhookUrl` 或 `webhook_url`。
- PagerDuty 需要 `routingKey`、`routing_key`、`integrationKey` 或 `integration_key`。
- Email 需要 `to` 或 `recipients`，并通过直接配置或 secret ref 提供 SMTP URL/config。运行时接受与 metadata 对齐的 `secretRefs.password` 作为 SMTP password reference alias，也接受 `passwordSecretRef` / `password_secret_ref`；SMTP URL reference alias 包括 `smtpUrl`、`smtp_url`、`url`、`smtpUrlSecretRef` 和 `smtp_url_secret_ref`。
- Secret 解析支持直接配置私密值，保存在服务端并立即生效而无需重启服务。同时为了部署兼容性，也支持通过 `env:NAME` 引用或裸环境变量名从 Server 进程环境变量中读取。

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
| `ownerType` | string | 是 | API 接受 `global`、`namespace`、`app`、`job`、`workflow`、`workflow_node`、`alert_rule`、`worker_pool`；当前运行时为 `job_instance` 匹配 `global`/`namespace`/`app`/`job`，为 `alert` 匹配 `global`/`alert_rule`，为 workflow notification 节点匹配 `global`/`workflow`/`workflow_node`。 |
| `ownerId` | string/null | 否 | Soft-linked owner。 |
| `name` | string | 是 | 不能为空。 |
| `eventFamily` | string | 是 | API 接受 `job_instance`、`workflow`、`alert`、`worker`、`script_governance`；当前运行时已实现 job instance、alert event 与 workflow notification-node request 物化。 |
| `eventFilter` | object | 否 | 作业 materializer 支持 `statuses` 和 `eventTypes`/`event_types`；workflow notification-node materializer 还支持 `workflowIds` 和 `nodeKeys`。 |
| `channelRefs` | array | 是 | 有序渠道引用，不能为空。 |
| `templateRef` | string/null | 否 | 指向 `notification_templates.id` 或 `templateKey` 的 soft link。`job_instance` 物化时会加载 enabled 的存储模板，可覆盖 subject/body 并写入 `payload.template`；缺失/disabled 引用为兼容性会被忽略。 |
| `severity` | string | 是 | 为空时 service 侧会从事件推导默认 severity。 |
| `enabled` | boolean | 否 | 默认 `true`。 |
| `dedupeSeconds` | integer | 否 | 默认 `300`。 |

`PATCH` 还接受 `throttle`、`quietHours`、`escalation` JSON。当前 job-event materialization 只实现事件 filter 和 dedupe；在 service 代码实现前，不要宣称 throttle/quiet-hours/escalation 已完整执行。

## Template 字段与渲染

`notification_templates` 保存 provider-specific 可复用模板。API 形态：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `templateKey` | string | 是 | 稳定操作员 key；只允许字母、数字、点、下划线、短横线，最长 128 字节。 |
| `name` | string | 是 | 面向操作员的名称。 |
| `description` | string/null | 否 | 可选描述。 |
| `provider` | string | 是 | 内置或插件 provider slug。 |
| `messageType` | string | 是 | 内置 provider 会校验该消息类型是否受支持。 |
| `enabled` | boolean | 否 | disabled 模板保留在库中，但 runtime materialization 会跳过。 |
| `body` | object | 否 | Provider-specific 消息模板 body；必填字段来自 provider metadata。 |
| `variables` | object | 否 | 变量说明或默认值 metadata；不是 secret 存储。 |

Render dry-run endpoint 使用与投递 payload 渲染一致的变量替换引擎，只返回渲染后的 JSON body，不解析渠道 secret refs，也不会调用外部 provider。已存储模板可按 id 或 `templateKey` 渲染；未保存草稿也可以在请求体提供 `provider`、`messageType` 和 `template` object，此时路径片段只作为草稿 key 参与校验。支持变量包括 `{{subject}}`、`{{body}}`、`{{eventType}}`、`{{resourceType}}`、`{{resourceId}}`、`{{severity}}`、`{{messageId}}`、`{{policyId}}`、`{{dedupeKey}}`、`{{triggeredAt}}`、`{{createdAt}}`。

当 job-instance policy 通过 `id` 或 `templateKey` 引用 enabled 模板时，materializer 会在插入 message 前渲染模板。`subject`/`title` 可覆盖标准化 subject；`body`/`text`/`content` 可覆盖标准化 body；完整渲染 JSON 会连同 `templateRef`、`templateKey` 写入 `payload.template`。Provider renderer 会优先使用 `payload.template`，再回退到渠道 inline `config.template`，因此一个 enabled 存储模板可以驱动 Slack/钉钉/飞书/企业微信/PagerDuty/webhook/email 的 payload 形状，而不会被渠道默认值静默遮蔽，也不复制渠道密钥。

模板行绝不存储 provider 凭据。Webhook URL、签名密钥、PagerDuty routing key、SMTP URL、SMTP password、authorization header、自定义 secret header，以及 `appId`/`appSecret` 这类 app-style 凭据，都必须留在所属渠道记录自己的 `secretRefs`。

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

## Provider schema 与投递行为

`GET /api/v1/notification-channel-types` 返回渠道抽屉使用的 schema metadata。metadata 区分非密钥的 `requiredConfigKeys` 与目标类 `requiredTargetKeys`：Webhook URL、PagerDuty routing key、SMTP URL 等目标/凭据应优先放在该渠道记录自己的 `secretRefs`，而不是 raw config。后端也会校验内置 provider 的 `messageType` 和必填模板字段。内置 seed/API 示例使用 `env:TIKEO_NOTIFICATION_CHANNEL_FEISHU_INTERACTIVE_WEBHOOK_URL` 这类 channel-scoped 引用，而不是 `env:FEISHU_WEBHOOK_URL` 这类共享引用。

当前抽屉与投递 renderer 暴露的官方文档对齐能力：

| Provider | 消息类型与关键字段 | Secret/ref 行为 |
| --- | --- | --- |
| `webhook` | `json` body template。 | 每条渠道自己的 `secretRefs.url`，可选 `secretRefs.authorization` 或 `secretRefs.headers.*`。 |
| `slack` | `text`、`blockKit` (`blocks`)、`attachments`；已知父消息 timestamp 时，可选 `threadTs` 会映射为 webhook thread reply 的 Slack `thread_ts`。 | Incoming webhook URL 应作为每条渠道自己的 secret ref。 |
| `dingtalk` | `text`、`markdown`、`link`、支持单按钮或 `btns` JSON 的 `actionCard`、`feedCard`；`atMobiles`、`atUserIds`、`isAtAll`。 | 每条渠道自己的 webhook URL；可选每条渠道自己的 `signingKey` 会按 timestamp/HMAC 给 URL 签名。 |
| `feishu` | `text`、`post`、`image` (`image_key`)、`share_chat` (`share_chat_id`)、`interactive` card。 | 每条渠道自己的 webhook URL；可选每条渠道自己的 `signingKey` 会添加 body `timestamp`/`sign`。插件型 app 凭据 `appId`/`appSecret` 也放在同一条渠道记录的 `secretRefs`。 |
| `wechat_work` | `text`、`markdown`、`markdown_v2`、`image`、`news`、`file`、`voice`、`template_card`；text-compatible 消息支持 mentions。 | Webhook URL 用每条渠道自己的 secret ref。 |
| `pagerduty` | Events API `trigger`、`acknowledge`、`resolve`；payload 字段包括 `source`、`component`、`group`、`class`、`client`、`client_url`、`links`、`images`、`custom_details`。 | Routing/integration key 必须通过这条渠道自己的 `secretRefs.routingKey` / aliases 提供。 |
| `email` | `plain` text 和已存储的 `html` 模板形状；当前 runtime 仍通过 SMTP adapter 发送 text/plain。 | SMTP URL/password 应使用每条渠道自己的 secret refs。 |
| plugin webhook | 默认 provider-neutral JSON，除非插件 metadata 提供自定义模板。 | 由插件定义。 |

需要 URL、media ID、card、link 或 image/chat identifier 的富 provider 消息会在缺少 channel inline `config.template` 且缺少 enabled policy `templateRef` 渲染出的 `payload.template` 时 fail closed。覆盖钉钉 `link`/`actionCard`/`feedCard`、飞书 `image`/`share_chat`、企业微信 `image`/`news`/`file`/`voice`/`template_card`；运行时不会生成占位 provider payload。

URL safety 使用 `alert::validate_webhook_url()`。生产应使用 HTTPS/public target；`safetyPolicy.allowInsecureLoopback` 仅用于明确的本地 smoke。

内置 schema 的官方/标准参考包括 Slack incoming webhooks 与 `chat.postMessage` thread 字段语义、钉钉自定义机器人与安全设置、飞书自定义机器人和消息卡片自定义机器人文档、企业微信群机器人、PagerDuty Events API v2、generic HTTP/JSON 的 IETF RFC 9110/8259，以及 Email/SMTP 相关 RFC 5321/5322/2045/4954/6409。

## UI 参考

`web/src/pages/NotificationCenterPage.tsx` 并行加载：channel types、channels、policies、templates、messages 和 queue-status。页面展示渠道数、策略数、模板数、待重试数和 DLQ 数；Tabs 展示 channels、templates、policies、delivery/DLQ 与最近 20 条 messages，并提供 **Retry due** 操作。拥有 `notifications:manage` 的 operator 可创建、编辑、删除并 render-preview templates；创建/编辑/删除 channels 和 policies；策略校验调用 `POST /api/v1/notification-policies/{id}:validate`。模板抽屉由 provider/message-type schema 驱动，刻意不展示 `secretRefsJson` 或 provider secret fields。

## 可安全复制的静态命令

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-channel-types \
  -H 'Authorization: Bearer <operator-token>'

curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "templateKey": "ops.slack.failure",
    "name": "Ops Slack failure",
    "provider": "slack",
    "messageType": "blockKit",
    "body": {
      "subject": "[{{severity}}] {{subject}}",
      "body": "{{body}}",
      "text": "{{subject}}",
      "blocks": [
        {"type":"section","text":{"type":"mrkdwn","text":"*{{subject}}*\n{{body}}"}}
      ]
    }
  }'

curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-templates/ops.slack.failure/render \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"sample":{"subject":"Nightly failed","body":"exit 2","severity":"critical"}}'

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H 'Authorization: Bearer <operator-token>'

curl -fsS 'http://127.0.0.1:9090/api/v1/notification-messages?status=failed' \
  -H 'Authorization: Bearer <operator-token>'
```

这些命令需要运行中的认证 Server；文档检查只验证命令语法和源码路径，端到端执行应放到带测试凭据的 smoke 中。

## 告警边界

告警应成为 notification messages 的 producer，而不是可复用 provider credentials 的 owner。迁移完成前保持：alert rule 是异常条件 evaluator；Notification Center 拥有新的可复用出站目的地；`alert_rules.channels_json` 是兼容行为，不是复制 secret 的推荐位置；普通 job success/failure/cancel 消息属于 notification policy；`firing`、`suppressed`、`silenced`、`recovered` 属于告警。


渲染器采用 fail-closed 的安全 token 替换：创建、更新与渲染预览都会拒绝 `{{env.SECRET}}` 这类未知 token、未打开的 `}}` 或未闭合的 `{{`。标注为 JSON array/object 的 provider 字段也会在校验阶段解析，避免畸形 Block Kit、钉钉 feed card、飞书 card、企业微信 card、PagerDuty links/images 或 webhook JSON body 进入真实投递。
