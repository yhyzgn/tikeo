---
title: 作业用户指南
description: Tikeo jobs 控制台页面的人类操作指南。
---

# 作业用户指南

用 Jobs 定义可复用执行契约：归属作用域、Processor 或脚本绑定、调度、重试行为、Worker 资格、通知绑定、版本历史以及手动/API 触发。

![作业用户指南 截图](pathname:///img/screenshots/jobs.svg)

## 前置条件

- 你可以登录 Tikeo 控制台，并且当前角色拥有此页面的读取权限。
- 在变更运行时对象前，已经明确目标 namespace/app。
- 做现场验收时，至少存在一个近期实例、Worker session 或审计事件。
- 生产变更前，先写好回滚说明和期望观察结果，再保存。

## When to use / 何时使用

- 创建新的 API、cron、fixed-rate 或一次性任务。
- 调整重试、调度或 Worker 目标。
- 手动触发单实例或广播执行。
- 修改高影响任务前做影响审查。

## Key areas / 关键区域

| 区域 | 先看什么 |
| --- | --- |
| 定义表单 | 名称、namespace、app、调度类型、processor/script/plugin 绑定、超时、重试与 misfire。 |
| 目标面板 | Worker pool、标签、region、cluster、broadcastSelector 与调度建议。 |
| 版本抽屉 | 不可变变更历史、作者、创建时间、diff 与回滚入口。 |
| 触发面板 | 单实例触发、广播触发、API 参数以及创建后的实例链接。 |

## Typical workflow / 典型流程

1. Choose namespace and app before any execution details.
2. Select the executor binding and verify at least one Worker advertises that capability.
3. Set retry and timeout based on failure class, not by copying another Job blindly.
4. Save, inspect version history, then open scheduling advice.
5. Trigger a single run first; use broadcast only after selector preview is correct.

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

本指南刻意把 API 细节放在附录中。如果需要排查实现或自动化相同流程，可使用这些锚点：`Jobs`、`web/src/pages/JobsPage.tsx`、`/api/v1/jobs`、`/api/v1/jobs/{job}:trigger`、`triggerType=api`、`executionMode=single`、`broadcastSelector`。

## 生产检查清单

- [ ] 归属作用域和运维责任人明确。
- [ ] 变更有小规模验收路径和回滚说明。
- [ ] 证据包含对象 id、时间、操作人、状态以及相关实例或投递 id。
- [ ] 离开控制台的公开链接使用已配置平台 URL。
- [ ] 团队清楚本页描述的是执行、通知、告警还是治理语义。
