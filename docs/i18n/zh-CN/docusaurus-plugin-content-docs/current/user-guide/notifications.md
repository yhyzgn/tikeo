---
title: 通知用户指南
description: Tikeo 通知中心的渠道、策略、消息、投递重试、DLQ、UI 检查和告警边界运维指南。
---

# 通知用户指南

通知中心是 Tikeo 的可复用出站投递层。需要把作业生命周期、治理事件或运维消息发送到 Slack、钉钉、飞书/Lark、企业微信、PagerDuty、Email、通用 Webhook 或插件提供的 Webhook-compatible provider 时，应使用通知中心。

边界必须清楚：

- **通知中心** 负责可复用出站渠道、策略/订阅、标准化消息、投递尝试、重试和 DLQ。当前证据源包括 `crates/tikeo-server/src/notification.rs`、`crates/tikeo-server/src/http/routes/notification_providers.rs`、`crates/tikeo-storage/src/repository/notification.rs` 和 `web/src/pages/NotificationCenterPage.tsx`。
- **告警** 负责异常条件规则、告警事件、incident-like 状态、silence/recovery/suppression 语义，以及兼容阶段的告警投递账本。普通作业完成消息不要建成告警规则。

## 什么时候使用通知

| 场景 | 推荐 event family | 示例事件 |
| --- | --- | --- |
| 作业成功后发确认消息 | `job_instance` | `job_instance.succeeded` |
| 作业终态失败或重试耗尽 | `job_instance` | `job_instance.failed`, `job_instance.retry_exhausted` |
| 广播执行部分失败 | `job_instance` | `job_instance.partial_failed` |
| 没有匹配 Worker | `job_instance` | `job_instance.no_eligible_worker` |
| 脚本治理阻断执行 | `job_instance` 或 `script_governance` | `job_instance.script_governance_failure` |
| 告警系统产生事件 | `alert` | `alert.firing`、`alert.recovered` 已由 Notification Center 物化为消息和投递尝试，可匹配 `global` 或 `alert_rule` 策略。 |

需要条件评估、去重、静默、恢复、异常历史或 incident review 时使用告警；只需要出站消息时使用通知中心。

## Provider 类型

内置渠道类型来自 `crates/tikeo-server/src/http/routes/notification_providers.rs` 中的 `builtin_channel_types()`。

| Provider | 非密钥配置 | Target / secret refs | 抽屉暴露的消息类型 |
| --- | --- | --- | --- |
| `webhook` | 无必填 | `secretRefs.url`，可选 `authorization` | `json` |
| `slack` | 可选 `threadTs` | `secretRefs.url` | `text`, `blockKit`, `attachments` |
| `dingtalk` | `atMobiles`, `atUserIds`, `isAtAll` | `secretRefs.url`，可选 `signingKey` | `text`, `markdown`, `link`, `actionCard`, `feedCard` |
| `feishu` | 无必填 | `secretRefs.url`，可选 `signingKey` | `text`, `post`, `image`, `share_chat`, `interactive` |
| `wechat_work` | mention lists | `secretRefs.url` | `text`, `markdown`, `markdown_v2`, `image`, `news`, `file`, `voice`, `template_card` |
| `pagerduty` | `source`, `component`, `group`, `class`, `client`, `clientUrl`, `links`, `images`, `customDetails` | `secretRefs.routingKey` / aliases | `trigger`, `acknowledge`, `resolve` |
| `email` | `to`/`recipients`, `from`, `username` | `secretRefs.smtpUrl`，可选 `password` | `plain`，已存储的 `html` 形状；runtime text/plain |
| 插件类型 | 插件定义 | 插件定义 | 插件定义 |

Webhook-style provider 接受 `url`、`webhookUrl` 或 `webhook_url` 作为 target key，但内置 provider 的 UI 和校验优先使用 `secretRefs`。PagerDuty 通过 `secretRefs` 接受 `routingKey`、`routing_key`、`integrationKey` 或 `integration_key`。Email 接受 `to` 或 `recipients`；SMTP endpoint 可以来自 `secretRefs.smtpUrl`、`secretRefs.smtp_url`、`secretRefs.url`、`config.smtpUrlSecretRef`、`config.smtp_url_secret_ref`、`secretRefs.smtpUrlSecretRef` 或 `secretRefs.smtp_url_secret_ref`。SMTP auth password 使用 `config.passwordSecretRef`、`config.password_secret_ref`、`secretRefs.password`、`secretRefs.passwordSecretRef` 或 `secretRefs.password_secret_ref`。

Notification Center 当前运行时 secret 解析支持直接配置私密值，保存在服务端并立即生效而无需重启服务。同时为了部署兼容性，也支持通过 `env:NAME` 引用或裸环境变量名从 Server 进程环境变量中读取。

### 每条渠道自己的 secretRefs

每一条通知渠道记录都拥有自己的 `secretRefs` 对象。不要配置一个全局 `FEISHU_WEBHOOK_URL`、Slack webhook、PagerDuty routing key 或 SMTP 凭据并让所有渠道共用。生产环境应使用能体现业务目的地或 provider/message type 的稳定名称，例如 `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_WEBHOOK_URL`、`env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_SIGNING_KEY` 或 `env:TIKEO_NOTIFICATION_CHANNEL_ONCALL_PAGERDUTY_ROUTING_KEY`。内置 seed/生成示例使用 `env:TIKEO_NOTIFICATION_CHANNEL_<PROVIDER>_<MESSAGE_TYPE>_<PURPOSE>`，就是为了明确这些引用是 channel-scoped。

如果插件或 app-style provider 需要 `appId`、`appSecret` 一类凭据，也放在同一条渠道记录的 `secretRefs` 中，不要移到 Server 全局配置。当前内置飞书/Lark 自定义机器人使用 `secretRefs.url` 和可选 `secretRefs.signingKey`。

## 快速路径：channel → template → policy → event → delivery

这条路径用于把一个 Job 失败策略投递到一个可复用出站渠道。命令可以串联执行：每一步都从上一步响应中取 ID。真实 webhook URL、authorization、SMTP password、PagerDuty routing key 等必须由 Server 进程环境提供，并通过 `secretRefs` 引用；不要写入 channel `config`、模板、截图、工单或示例。

前置条件：

- 当前 token 具备 `notifications:manage`；手动 retry scan 还需要 `notifications:test`。
- Server 进程能读取这条渠道自己的 `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_URL`，以及可选的 `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_AUTH`。
- 已确定要匹配的 namespace/app 或 job owner。
- 内置 provider 会返回 `supportsTestSend=true`；保存渠道后，可在编辑抽屉使用 **发一条试试**，或调用 `POST /api/v1/notification-channels/{id}/test-send` 验证已保存渠道，响应只展示脱敏结果。上线验收仍应同时检查策略校验、消息、投递 attempts 和 retry/DLQ 状态。

```bash
export TOKEN='<operator-bearer-token>'

CHANNEL_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-channels \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "scopeType": "app",
    "namespace": "prod",
    "app": "billing",
    "name": "billing-ops-webhook",
    "provider": "webhook",
    "enabled": true,
    "config": {"messageType": "json"},
    "secretRefs": {
      "url": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_URL",
      "authorization": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_AUTH"
    }
  }' | jq -r '.data.id')"

test -n "${CHANNEL_ID}" && test "${CHANNEL_ID}" != "null"

TEMPLATE_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "templateKey": "ops.webhook.failure",
    "name": "Ops webhook failure",
    "provider": "webhook",
    "messageType": "json",
    "enabled": true,
    "body": {
      "subject": "[{{severity}}] {{subject}}",
      "body": "{{body}}",
      "eventType": "{{eventType}}",
      "resourceId": "{{resourceId}}"
    },
    "variables": {"severity": "critical"}
  }' | jq -r '.data.id')"

test -n "${TEMPLATE_ID}" && test "${TEMPLATE_ID}" != "null"

curl -fsS -X POST "http://127.0.0.1:9090/api/v1/notification-templates/${TEMPLATE_ID}/render" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"sample":{"subject":"Nightly failed","body":"exit 2","eventType":"job_instance.failed","resourceId":"instance-demo","severity":"critical"}}' | jq .data

POLICY_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-policies \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "$(python3 - <<'PYJSON'
import json, os
print(json.dumps({
  "ownerType": "app",
  "ownerId": "prod/billing",
  "name": "billing terminal failures",
  "eventFamily": "job_instance",
  "eventFilter": {
    "eventTypes": ["job_instance.failed", "job_instance.retry_exhausted"],
    "statuses": ["failed", "retry_exhausted"]
  },
  "channelRefs": [{"channelId": os.environ["CHANNEL_ID"]}],
  "templateRef": os.environ["TEMPLATE_ID"],
  "severity": "critical",
  "enabled": True,
  "dedupeSeconds": 300
}, separators=(",", ":")))
PYJSON
)" | jq -r '.data.id')"

test -n "${POLICY_ID}" && test "${POLICY_ID}" != "null"

curl -fsS -X POST "http://127.0.0.1:9090/api/v1/notification-policies/${POLICY_ID}:validate" \
  -H "Authorization: Bearer ${TOKEN}" | jq .data
```

随后触发一个同 owner scope 的 Job，使它进入 `failed` 或 `retry_exhausted`，再检查消息和投递：

```bash
curl -fsS 'http://127.0.0.1:9090/api/v1/notification-messages?eventFamily=job_instance' \
  -H "Authorization: Bearer ${TOKEN}" | jq '.data.items[0]'

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts \
  -H "Authorization: Bearer ${TOKEN}" | jq '.data.items[0]'

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H "Authorization: Bearer ${TOKEN}" | jq .data
```

验收完成的标志：策略校验返回 `valid=true`，匹配事件生成标准化 message，delivery attempt 引用 `${CHANNEL_ID}`，attempt 状态进入 `delivered`、`retry_pending` 或 `dead_letter`，且目标信息保持脱敏。


## Job 级通知绑定

如果规则只属于某个具体 Job，优先在 **任务 → 通知配置** 抽屉中配置。抽屉底层仍写入 `ownerType=job` 的 Notification Center policy，但操作者不需要手动拼 owner、job id 和 `job_instance.*` 事件列表。

推荐流程：

1. 在 **通知中心 → 通知渠道** 创建并测试可复用渠道。
2. 如需富文本卡片，在 **通知中心 → 模板** 创建启用的 provider 模板。
3. 打开 **任务** 页面，在目标任务行点击 **通知配置**。
4. 选择触发预设：失败、成功、总是、取消、重试中、重试耗尽，或高级事件列表。
5. 选择渠道和模板。抽屉会按渠道 provider 过滤模板，服务端也会再次校验模板 provider 与渠道是否兼容。
6. 保存前先执行 **校验** 和 **预览**。预览只渲染样例 payload，不会真正发送外部消息。
7. 真实运行后，在 **通知中心 → 消息 → 详情** 查看标准化消息、投递 attempts、Job/实例上下文和执行日志透传。

Job 消息模板除通用变量外，还支持 `{{jobId}}`、`{{jobName}}`、`{{namespace}}`、`{{app}}`、`{{instanceId}}`、`{{status}}`、`{{triggerType}}`、`{{executionMode}}`、`{{startedAt}}`、`{{finishedAt}}`、`{{workerId}}`、`{{operatorName}}`、`{{operatorType}}` 和 `{{logsUrl}}`。

## 安全渠道创建示例

响应统一使用 `{code,message,data}` envelope。上面的快速路径是推荐复制流程；下面只保留核心安全形状：凭据放在 `secretRefs`，不要写进 `config`。

```json
{
  "scopeType": "app",
  "namespace": "prod",
  "app": "billing",
  "name": "billing-ops-webhook",
  "provider": "webhook",
  "enabled": true,
  "config": {"messageType": "json"},
  "secretRefs": {
    "url": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_URL",
    "authorization": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_AUTH"
  }
}
```

响应中的 `id` 由存储生成。`secretRefsJson` 不序列化返回，`configJson` 会被 `NotificationChannelSummary::from()` 脱敏。

## 可复用模板

当多个策略需要复用同一套消息正文，或希望编辑消息体但不触碰渠道凭据时，使用模板。模板包含 `templateKey`、`provider`、`messageType`、`body`、可选 `variables` 和 `enabled`。对内置 provider，抽屉和后端都会基于 provider metadata 校验消息类型和必填 body 字段。

模板渲染是安全 token 替换器，不是任意表达式引擎。未知 token（例如 `{{env.SECRET}}`）、畸形分隔符以及不合法的 JSON array/object 字段都会在保存或预览前被拒绝。

安全 Slack Block Kit 模板示例：

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "templateKey": "ops.slack.failure",
    "name": "Ops Slack failure",
    "provider": "slack",
    "messageType": "blockKit",
    "enabled": true,
    "body": {
      "subject": "[{{severity}}] {{subject}}",
      "body": "{{body}}",
      "text": "{{subject}}",
      "blocks": [
        {"type":"section","text":{"type":"mrkdwn","text":"*{{subject}}*\n{{body}}"}}
      ]
    },
    "variables": {"severity": "critical"}
  }'
```

绑定到策略前先 dry-run 渲染：

```bash
curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-templates/ops.slack.failure/render \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"sample":{"subject":"Nightly failed","body":"exit 2","severity":"critical","eventType":"job_instance.failed"}}'
```

把 policy 的 `templateRef` 设置为模板 `id` 或 `templateKey`。作业实例物化时，enabled 模板可以覆盖标准化 message 的 `subject` 和 `body`，并把渲染后的 provider body 写入 `payload.template`。provider 投递会优先使用这个渲染模板，再回退到渠道 inline `config.template`，因此策略选择的 enabled 存储模板不会被渠道默认值静默遮蔽。缺失或 disabled 的模板引用会为了兼容性被忽略，因此生产策略应引用已存在、enabled 的模板，并在 UI 中预览确认。

模板不是 secret store。Webhook URL、签名密钥、PagerDuty routing key、SMTP URL/password、authorization header 和自定义 header 值必须放在 channel `secretRefs`。

需要 provider-specific 字段的富消息类型在没有 channel inline template 或 enabled policy template 时会 fail closed。钉钉 `link`/`actionCard`/`feedCard`、飞书 `image`/`share_chat`、企业微信 `image`/`news`/`file`/`voice`/`template_card` 必须由 operator 提供真实模板字段；Tikeo 不会为生产投递合成占位 URL 或假 media ID。

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
    "channelRefs": [{"channelId": "${CHANNEL_ID}"}],
    "templateRef": null,
    "severity": "critical",
    "enabled": true,
    "dedupeSeconds": 300
  }'
```

创建后验证：

```bash
curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-policies/${POLICY_ID}:validate \
  -H 'Authorization: Bearer <operator-token>'
```

## Owner 与事件语义

API 当前接受 owner 类型：`global`、`namespace`、`app`、`job`、`workflow`、`workflow_node`、`alert_rule`、`worker_pool`；接受 event family：`job_instance`、`workflow`、`alert`、`worker`、`script_governance`。当前运行时已实现 `job_instance` 策略（`global`、`namespace`、`app`、`job` owner）、alert 事件（`global`、`alert_rule` owner）以及 workflow notification 节点（`global`、`workflow`、`workflow_node` owner）物化。

对作业实例事件，当前 materializer 匹配：所有 `global`；`ownerId` 等于 namespace 的 `namespace`；`ownerId` 等于 app 或 `namespace/app` 的 `app`；`ownerId` 等于 job id 的 `job`。Filter 会用 `statuses` 匹配稳定状态 token，用 `eventTypes` 或 `event_types` 匹配完整事件名。

对 workflow notification 节点，运行时事件名是 `workflow_node.notification_requested`，状态 token 是 `notification_requested`。`workflow` policy 用 `ownerId=<workflow id>` 匹配；`workflow_node` policy 支持 `ownerId=<workflow id>:<node key>`、节点实例 id 或节点 key；`eventFilter.workflowIds` 和 `eventFilter.nodeKeys` 可进一步限制。节点自身的 `config.channelRefs` 会被编译/更新为 workflow_node policy，并生成消息和投递尝试。

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

打开 `/notifications` 的 **Notification Center / 通知中心**。页面由 `web/src/pages/NotificationCenterPage.tsx`、`web/src/pages/notifications/TemplateDrawer.tsx` 和 `web/src/api/notifications.ts` 支撑，支持脱敏检查以及受权限控制的渠道、模板、策略创建、编辑、删除、渲染预览和校验。

| Tab | 检查内容 |
| --- | --- |
| Channels | 名称、provider、scope、脱敏 target、secret 是否配置、enabled，以及创建/编辑/删除抽屉；被策略引用时由后端拒绝删除。 |
| Templates | provider、message type、enabled、schema-driven body 字段、variables JSON、后端渲染预览、创建/编辑/删除，并且不展示 secret 字段。 |
| Policies | owner、event family、severity、dedupe seconds、enabled、创建/编辑/删除、渠道多选、模板选择器、JSON 事件过滤和策略校验。 |
| Delivery | 总尝试数、delivered、retry pending、retry consumed、DLQ、failed、最近 DLQ、Retry due。 |
| Messages | 最近标准化消息、事件、资源、subject、状态、创建时间。 |

常规渠道和策略 CRUD/校验可直接使用 UI；自动化、批量变更或表单尚未优化的字段继续使用 Management API。

## 故障排查

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


## 生产检查清单

- [ ] 所有 provider token、Webhook URL、签名密钥、routing key、SMTP URL、SMTP password、authorization header 和自定义 header secret 都通过 `secretRefs` 引用；原始 secret 不进入 channel `config`、模板、文档、截图或 shell 历史。
- [ ] 渠道作用域与影响范围匹配：global channel 应很少使用，优先使用 namespace/app channel，高噪声 workload 使用 job 级策略。
- [ ] 富消息模板为 provider 必填字段提供真实值；生产投递时不能由 Tikeo 合成占位链接、假 media id 或缺失 card 字段。
- [ ] Job policy 明确区分 retry noise、终态失败、部分失败和无可用 Worker，避免把所有失败都投递给同一个高优先级渠道。
- [ ] Operator 掌握 retry/DLQ runbook，包括 `notification_delivery.*` 默认值、bounded `retry-due` 扫描，以及在创建替代事件前检查 `dead_letter` 原因。
- [ ] Alert rule 与 notification policy 的命名能清楚区分 incident 语义和普通生命周期消息。
