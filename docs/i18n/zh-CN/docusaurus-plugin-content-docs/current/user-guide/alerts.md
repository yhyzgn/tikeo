---
title: 告警用户指南
description: Tikeo 告警规则、事件、静默/恢复/抑制语义、兼容投递和通知中心边界运维指南。
---

# 告警用户指南

告警用于需要 incident-like 语义的异常条件。通知中心用于可复用出站投递。保持边界清晰，才能避免普通生命周期消息变成噪音 incident，也避免 alert rule 变成分散的 secret 仓库。

告警证据源包括 `crates/tikeo-server/src/alert.rs`、`crates/tikeo-server/src/http/routes/alerts.rs`、`crates/tikeo-storage/src/entities/alert_rule.rs`、`crates/tikeo-storage/src/entities/alert_event.rs` 和 `crates/tikeo-storage/src/repository/alert.rs`。通知中心迁移边界记录在 `design/notification-center-alerting-plan.md`。

## 告警 vs 通知中心

| 能力 | 告警 | 通知中心 |
| --- | --- | --- |
| 核心职责 | 发现异常条件并保留事件历史。 | 把消息投递到可复用目的地。 |
| 核心记录 | `alert_rules`、`alert_events`、alert-specific delivery attempts。 | `notification_channels`、`notification_policies`、`notification_messages`、generic delivery attempts。 |
| 语义 | severity、condition、dedupe、silence、suppression、recovery、escalation intent。 | channel config、owner/event subscription、message materialization、retry、DLQ。 |
| 示例 | 脚本治理失败产生 critical alert event。 | 作业成功发送可选 Slack 消息。 |
| Secret 姿态 | 兼容阶段规则仍可能包含 inline channels。 | 渠道复用、target 脱敏，API 不返回 `secretRefsJson`。 |

需要异常条件审查时使用告警；只需要出站消息时使用通知中心。

## 生命周期词汇

| Status | 含义 |
| --- | --- |
| `firing` | 规则条件产生 active event，未被 suppress 或 silence。 |
| `suppressed` | 条件出现但被阈值/去重逻辑抑制。 |
| `silenced` | 当前仍处于 silence window，不应 page operator。 |
| `recovered` | 之前的 event 已恢复/解决。 |

事件 summary 代码会统计 `firing`、`suppressed`、`silenced` 和 `recovered`。

## 当前告警 API

所有业务响应使用 `{code,message,data}` envelope。

| Method/path | 用途 | 当前权限 |
| --- | --- | --- |
| `GET /api/v1/alert-rules` | 列出告警规则。 | `audit:read` |
| `POST /api/v1/alert-rules` | 创建告警规则。 | `audit:manage` |
| `GET /api/v1/alert-rules/{id}/delivery-status` | 检查规则 inline channel readiness。 | `audit:read` |
| `GET /api/v1/alert-events` | 列出告警事件。 | `audit:read` |
| `GET /api/v1/alert-events:summary` | 统计和分组告警事件。 | `audit:read` |
| `POST /api/v1/alert-events/{id}/resolve` | 标记 recovered/resolved。 | `audit:manage` |
| `GET /api/v1/alert-delivery-attempts` | 列出 alert-specific 投递尝试。 | `audit:read` |
| `GET /api/v1/alert-delivery-attempts:queue-status` | 告警 retry/DLQ summary。 | `audit:read` |
| `POST /api/v1/alert-delivery-attempts:retry-due` | 扫描 due alert delivery retries。 | `audit:manage` |

通用通知中心端点见 [通知中心参考](../reference/notification-center)。

## 安全创建告警规则

Alert rule 当前保存 `condition_json` 和兼容 `channels_json`。示例只展示脱敏/占位形状，生产应优先把可复用目的地迁到 Notification Center channel。

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/alert-rules \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "script governance critical failures",
    "severity": "critical",
    "condition": {
      "resourceType": "job_instance",
      "failureClass": "script_governance_failure"
    },
    "channels": [
      {
        "type": "webhook",
        "enabled": true,
        "url": "https://hooks.example.invalid/tikeo/alerts",
        "authorization": "env:TIKEO_ALERT_SECRET_WEBHOOK_AUTH"
      }
    ],
    "enabled": true,
    "dedupeSeconds": 300
  }'
```

不要在 `url`、`authorization`、`password`、`routingKey` 或类似字段里放真实 secret。

## Delivery readiness 与队列操作

上线前先检查 delivery status：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/alert-rules/alert-rule-example/delivery-status \
  -H 'Authorization: Bearer <operator-token>'
```

响应包含 `ready`、`channelCount`、每个 channel 的 provider、targetConfigured、secretConfigured、enabled、targetRedacted、transportSecurity 和 issues。

Alert retry 默认值与 Notification Center 分离：

| Key | 默认值 |
| --- | --- |
| `alert_retry.enabled` | `true` |
| `alert_retry.interval_seconds` | `60` |
| `alert_retry.batch_size` | `50` |
| `alert_retry.max_attempts` | `3` |
| `alert_retry.backoff_seconds` | `300` |

手动扫描：

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/alert-delivery-attempts:retry-due \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"limit":50,"maxAttempts":3,"backoffSeconds":300}'
```

## Silence、suppression 与 recovery runbook

| 情况 | 含义 | 操作 |
| --- | --- | --- |
| 很多 `suppressed` | 条件重复或低于阈值，被去重/阈值逻辑压制。 | 先检查规则是否过宽，再扩大投递。 |
| 事件为 `silenced` | `silenced_until` 仍在未来。 | 确认 silence owner 和窗口，不要绕过。 |
| 问题已修复 | 事件应记录为 `recovered`。 | 用 resolve endpoint 或对应 UI/API 留下恢复证据。 |
| 告警投递失败 | Provider target、secret、安全策略或 transport 有问题。 | 先看 delivery-status 和 queue-status，修复后 retry-due。 |
| 规则对普通成功/取消 page | 把 lifecycle notifier 错建成 alert。 | 迁到通知中心。 |

## UI 与运维位置

`web/src/routes.tsx` 把 `/alerts` 作为 observability 组下的 **告警事件**，权限为 `audit:read`。它是 incident review 面：规则状态、告警事件、投递历史和恢复证据属于这里。`/notifications` 是出站投递中心：可复用目的地、订阅、消息、重试队列和 DLQ 检查属于那里。

## 迁移指导

目标架构是：**告警产生 notification messages，通知中心拥有 channels 和 delivery**。兼容阶段：

- `alert_rules.channels_json` 和 `alert_delivery_attempts` 仍是当前行为。
- 新的可复用目的地应建成 Notification Center channel。
- 文档不要让 operator 把同一个 webhook token 复制到每条 alert rule。
- 普通 job/workflow 状态消息使用 notification policy，不使用 alert rule。
- `firing`、`suppressed`、`silenced`、`recovered` 仍属于告警语义。

## 排障检查

- 权限错误时注意：读取需要 `audit:read`；创建规则、恢复事件和重试扫描需要 `audit:manage`。
- delivery-status 提示 provider 未注册时，改用 built-in provider 或 enabled plugin alert channel type。
- Email 的 `smtp://` 被拒绝时，在非本地 smoke 场景应使用安全 SMTP。
- retry/DLQ 增长时，分别检查 `alert_retry` 和 `notification_delivery`，它们是不同 worker 和队列。
- 如果文档或 runbook 把 alert rule 与 notification channel 当成同义词，应先修正文案再给生产 operator 使用。
