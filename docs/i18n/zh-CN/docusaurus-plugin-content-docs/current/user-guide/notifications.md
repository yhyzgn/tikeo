---
title: 通知中心用户指南
description: Tikeo notifications 控制台页面的人类操作指南。
---

# 通知中心用户指南

用通知中心配置可复用投递渠道、提供方消息模板、策略规则、Job/Workflow 绑定、投递记录、重试和死信可见性。它与告警不同：告警决定规则是否触发，通知负责把消息送达。

![通知中心用户指南 截图](pathname:///img/screenshots/notifications.svg)

## 前置条件

- 你可以登录 Tikeo 控制台，并且当前角色拥有此页面的读取权限。
- 在变更运行时对象前，已经明确目标 namespace/app。
- 做现场验收时，至少存在一个近期实例、Worker session 或审计事件。
- 生产变更前，先写好回滚说明和期望观察结果，再保存。

## When to use / 何时使用

- Job 需要在运行、重试、成功、失败或 always 时通知。
- Workflow 需要发送提供方专属卡片。
- 告警需要复用渠道而不是重复 webhook 密钥。
- 操作人需要投递记录和测试发送证据。

## Key areas / 关键区域

| 区域 | 先看什么 |
| --- | --- |
| 渠道 | 提供方、作用域、secretRefs、目标、supportsTestSend=true 与脱敏摘要。 |
| 模板 | 提供方消息类型：blockKit、actionCard、feedCard、interactive、share_chat、markdown_v2、template_card。 |
| 策略 | 事件过滤、归属作用域、templateRef、重试、去重与路由规则。 |
| 投递 | 渲染 payload、状态、重试次数、提供方响应与死信处理。 |

## Typical workflow / 典型流程

1. Create or select a channel and test it if the provider supports test send.
2. Create a template with variable mapping for instance id, status, operator, time, trigger type, and public console URL.
3. Create a policy that binds event types to a channel/template pair.
4. Bind the policy to a Job or Workflow event.
5. Trigger a real instance and inspect delivery attempts, not only provider chat history.

## 决策表

| 场景 | 人的判断 | 需要收集的证据 |
| --- | --- | --- |
| 首次配置 | 使用最小作用域，并只跑一次小规模验收。 | 截图、对象 id、实例 id、审计事件。 |
| 事故处理 | 在理解失败对象前，暂停高风险变更。 | 时间线、attempt、日志、投递记录。 |
| 生产发布 | 一次只改一个维度，并对比前后状态。 | 版本 diff、Dashboard 健康、审计链路。 |
| 回滚 | 优先回到已知版本，而不是临场乱改。 | 旧版本 id、回滚审计、新验收运行。 |


## 快速路径：channel → template → policy → event → delivery

最安全的快速路径是可串联的：先创建渠道，再创建模板，再创建策略，把策略绑定到执行事件，最后查看投递记录。密钥放在 `secretRefs` 中；API 摘要只能展示脱敏引用，不能返回原始 webhook URL 或 token。

```bash
CHANNEL_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-channels \
  -H "authorization: Bearer $TIKEO_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"prod-feishu","provider":"feishu","secretRefs":{"webhook":"secret://tikeo/feishu/webhook"},"supportsTestSend":true}' | jq -r '.data.id')"

TEMPLATE_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H "authorization: Bearer $TIKEO_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"job-failure-card","provider":"feishu","messageType":"interactive","templateRef":"builtin.job.failure.card"}' | jq -r '.data.id')"

POLICY_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-policies \
  -H "authorization: Bearer $TIKEO_TOKEN" \
  -H 'content-type: application/json' \
  -d "{\"name\":\"job-failure\",\"channelId\":\"$CHANNEL_ID\",\"templateId\":\"$TEMPLATE_ID\",\"eventTypes\":[\"job_instance.failed\"],\"enabled\":true}" | jq -r '.data.id')"
```

当前提供方消息族包括 Slack 的 `blockKit`，钉钉的 `actionCard` 与 `feedCard`，飞书的 `interactive` 与 `share_chat`，企业微信的 `markdown_v2`，微信生态的 `template_card`，PagerDuty 事件，邮件，通用 webhook payload，以及 plugin webhook 适配器。

## 验收 Verify

- 页面展示的是当前对象，而不是浏览器缓存中的旧状态。
- 只读用户可以查看证据，但不能执行特权变更。
- 一次真实操作会产生可见审计事件，并在相关场景产生实例或投递记录。
- 控制台链接复制到事故记录后，仍能定位同一个对象。

## 故障排查

| 现象 | 处理方式 |
| --- | --- |
| 页面看起来为空 | 先检查 namespace/app 过滤和角色权限，不要直接判断数据丢失。 |
| 对象存在但按钮禁用 | 检查 RBAC、对象状态以及操作是否跨越作用域边界。 |
| UI 结果与聊天/邮件不一致 | 先相信 Tikeo 投递记录和实例证据，再对比提供方历史。 |
| 时间顺序混乱 | 使用 Server 时间戳、attempt 编号和审计 request id，而不是本地浏览器顺序。 |

## 参考锚点

本指南刻意把 API 细节放在附录中。如果需要排查实现或自动化相同流程，可使用这些锚点：`Notifications`、`crates/tikeo-server/src/http/routes/notifications.rs`、`crates/tikeo-server/src/http/routes/notification_templates.rs`、`notification_templates`、`/api/v1/notification-templates`、`/api/v1/notification-templates/{id}/render`、`templateRef`、`blockKit`、`actionCard`、`feedCard`、`interactive`、`share_chat`、`markdown_v2`、`template_card`、`PagerDuty`、`supportsTestSend=true`。

## 生产检查清单

- [ ] 归属作用域和运维责任人明确。
- [ ] 变更有小规模验收路径和回滚说明。
- [ ] 证据包含对象 id、时间、操作人、状态以及相关实例或投递 id。
- [ ] 离开控制台的公开链接使用已配置平台 URL。
- [ ] 团队清楚本页描述的是执行、通知、告警还是治理语义。
