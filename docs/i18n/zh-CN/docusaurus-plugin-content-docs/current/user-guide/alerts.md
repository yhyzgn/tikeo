---
title: 告警用户指南
description: Tikeo alerts 控制台页面的人类操作指南。
---

# 告警用户指南

用 Alerts 定义会触发、恢复、静默并路由事故的条件。告警可以复用通知中心渠道投递，但生命周期是规则驱动，而不是模板驱动。

![告警用户指南 截图](pathname:///img/screenshots/alerts.svg)

## 前置条件

- 你可以登录 Tikeo 控制台，并且当前角色拥有此页面的读取权限。
- 在变更运行时对象前，已经明确目标 namespace/app。
- 做现场验收时，至少存在一个近期实例、Worker session 或审计事件。
- 生产变更前，先写好回滚说明和期望观察结果，再保存。

## When to use / 何时使用

- 集群健康或队列压力需要叫醒负责人。
- 重复失败类型需要静默窗口。
- 恢复事件需要闭环通知。
- 需要区分告警责任人与作业通知责任人。

## Key areas / 关键区域

| 区域 | 先看什么 |
| --- | --- |
| 规则 | 指标/事件条件、严重级别、作用域、持续时间、触发与恢复语义。 |
| 静默 | 维护窗口、负责人、原因、过期时间与审计。 |
| 路由 | 关联通知渠道或策略、去重和升级目标。 |
| 历史 | 触发、更新、恢复、抑制与投递证据。 |

## Typical workflow / 典型流程

1. Define the condition in terms a human can verify.
2. Choose severity and owner before choosing a delivery channel.
3. Attach a Notification Center channel or policy if humans must be reached.
4. Create a bounded silence for maintenance rather than disabling the rule.
5. After recovery, review alert history and linked deliveries.

## 决策表

| 场景 | 人的判断 | 需要收集的证据 |
| --- | --- | --- |
| 首次配置 | 使用最小作用域，并只跑一次小规模验收。 | 截图、对象 id、实例 id、审计事件。 |
| 事故处理 | 在理解失败对象前，暂停高风险变更。 | 时间线、attempt、日志、投递记录。 |
| 生产发布 | 一次只改一个维度，并对比前后状态。 | 版本 diff、Dashboard 健康、审计链路。 |
| 回滚 | 优先回到已知版本，而不是临场乱改。 | 旧版本 id、回滚审计、新验收运行。 |

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

本指南刻意把 API 细节放在附录中。如果需要排查实现或自动化相同流程，可使用这些锚点：`Alerts`、`/api/v1/alerts`、`Notification Center`、`supportsTestSend=true`。

## 生产检查清单

- [ ] 归属作用域和运维责任人明确。
- [ ] 变更有小规模验收路径和回滚说明。
- [ ] 证据包含对象 id、时间、操作人、状态以及相关实例或投递 id。
- [ ] 离开控制台的公开链接使用已配置平台 URL。
- [ ] 团队清楚本页描述的是执行、通知、告警还是治理语义。
