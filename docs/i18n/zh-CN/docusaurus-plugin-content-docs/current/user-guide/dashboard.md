---
title: Dashboard 用户指南
description: Tikeo 实时运维驾驶舱的人类操作指南。
---

# Dashboard 用户指南

在触发任务、发布新 Worker、修改通知渠道或排查事故前，把 Dashboard 当作实时运维驾驶舱。它把 Jobs、实例、Workers、调度队列、通知中心投递、集群诊断和审计活动聚合在一个页面里。

![Dashboard 用户指南截图](pathname:///img/screenshots/dashboard.svg)

## 页面展示什么

| 区域 | 产品数据来源 | 用来判断什么 |
| --- | --- | --- |
| KPI 条 | Jobs、实例摘要、Worker 快照 | 平台是否有启用任务、活跃实例、在线 Worker、广播负载或可见失败。 |
| 12 小时执行趋势 | 近期 Job 实例 | 执行量或失败是在增长、孤立发生，还是处于安静状态。绿色段表示成功，红色段表示失败。 |
| 实例状态分布 | 实例 status 与 result success/failure | 当前主要是等待、派发中、运行中、重试、成功、失败还是取消。 |
| 任务计划图 | Job `scheduleType`、`scheduleExpr`、processor/script 绑定和启用状态 | 哪些 Cron/fixed/API 计划正在生效，以及每个计划绑定了哪个处理器或脚本。 |
| 队列压力 | Dispatch queue overview | 继续触发任务前，pending/running 工作是否正在积压。 |
| 通知投递 | Alert delivery queue status | Provider 投递是健康、等待重试、失败，还是进入死信。 |
| HA / 网关 | Cluster diagnostics 与 Smart Gateway diagnostics | Server HA、Worker gateway 本地性、远端 gateway 容量和 outbox 总量是否安全。 |
| Worker Mesh 分布 | 按 namespace/app/cluster/region 聚合的 Worker 快照 | 哪些应用作用域有在线容量，哪些作用域缺少 master 或足够 Worker。 |
| 能力覆盖 | 结构化 Worker capabilities | 在线 Worker 实际广告了哪些 SDK processor、脚本 runner、插件 processor 和标签。 |
| 审计活动 | 最近审计日志页 | 最近哪些操作人或 API Key 做了变更，以及这些操作成功还是失败。 |
| 风险信号 | 从失败、队列、通知投递、Worker 数和集群状态派生 | 当前是否可以继续常规操作，还是应该先暂停并排查。 |

这个页面以读取和判断为主。某个面板需要细节时，使用页面按钮跳转到 Jobs、Instances、Workers、Dispatch Queue、Security、Notifications 或 Audit。

## 实时刷新行为

Dashboard 路由激活时会打开多条 Server-Sent Events 流：

| Stream | 用途 |
| --- | --- |
| `/api/v1/instances/stream` | 刷新 Jobs、实例趋势和实例状态面板。 |
| `/api/v1/workers/stream` | 刷新 Worker 数、Worker Mesh 和能力覆盖。 |
| `/api/v1/dispatch-queue/stream` | 刷新队列压力。 |

它还会用 3 秒 fallback refresh 刷新 REST 面板，包括集群诊断、通知投递队列状态、审计日志和 Job instance 历史。如果页面状态滞后，需要同时验证 REST 访问和 SSE 代理行为。参见 [SSE 实时刷新部署注意事项](../deployment/sse-realtime)。

## 典型操作流程

1. 生产变更前先打开 Dashboard。
2. 先看风险信号和 KPI 条。如果失败、队列积压或通知死信非零，暂停大范围发布。
3. 查看 12 小时执行趋势和状态环图。如果失败集中在最新时间桶，先进入 Instances 排查，不要继续触发更多任务。
4. 查看队列压力。如果 pending/running 正在积累，进入 Dispatch Queue 检查 ownership 和 Worker eligibility。
5. 查看 Worker Mesh 和能力覆盖。如果所需 processor 或 script runner 不存在，先修 Worker 部署，再改 Job。
6. 查看 HA / 网关。Raft 部署下确认 diagnostics 未 degraded，outbox 总量没有异常增长。
7. 依赖事故通知前，先确认通知投递面板健康。
8. 部署后保持 Dashboard 打开，直到趋势、队列、Worker 和通知面板稳定。

## 决策表

| 场景 | 人的判断 | 需要收集的证据 |
| --- | --- | --- |
| 首次配置 | 先跑一个小规模验收任务，再增加更多计划。 | Dashboard 截图、Job id、instance id、Worker id、审计事件。 |
| 队列积压 | 理解 dispatch 压力前，不要继续触发批量任务。 | 队列状态、最旧 queued item、Worker 能力覆盖、实例日志。 |
| Worker 发布 | 对比发布前后的 Worker Mesh 分布和能力覆盖。 | Worker ids、namespace/app、cluster/region、结构化能力标签。 |
| 通知问题 | 把 provider 投递证据与任务执行成功分开判断。 | 投递队列状态、delivery attempt id、provider 响应/错误、相关 alert/job id。 |
| Server HA 事故 | Dashboard 看总览，然后进入集群诊断和 HA runbook。 | `/api/v1/cluster/diagnostics`、outbox totals、shard/queue metrics、Kubernetes event window。 |
| 生产发布 | 一次只改一个维度，并对比前后状态。 | 版本 diff、Dashboard 前后截图、审计链路、smoke 结果。 |

## API 与实现锚点

Dashboard 由以下代码和 API 面支撑：

| Surface | Anchor |
| --- | --- |
| React 页面 | `web/src/pages/Dashboard.tsx` |
| Jobs | `/api/v1/jobs` |
| Job instances | `/api/v1/jobs/{jobId}/instances` |
| Instance stream | `/api/v1/instances/stream` |
| Workers | `/api/v1/workers` 与 `/api/v1/workers/stream` |
| Dispatch queue | `/api/v1/dispatch-queue` 与 `/api/v1/dispatch-queue/stream` |
| Cluster diagnostics | `/api/v1/cluster/diagnostics` |
| Alert delivery queue | `/api/v1/alert-delivery-attempts:queue-status` |
| Audit logs | `/api/v1/audit-logs?page_size=8` |

## 验收

- Worker 连接、断开或能力变化时，页面会更新。
- Job instance 创建或完成时，页面会更新。
- Dispatch Queue 变化无需完整浏览器刷新即可看到。
- 通知投递死信或重试能在通知面板看到。
- 特权变更后，最近审计活动出现对应记录。
- 只读用户可以查看证据，但不能通过链接页面执行特权变更。

## 故障排查

| 现象 | 处理方式 |
| --- | --- |
| Dashboard 能打开但实时面板不更新 | 检查 `/api/v1/instances/stream`、`/api/v1/workers/stream` 和 `/api/v1/dispatch-queue/stream`，再应用 SSE 代理检查清单。 |
| 队列压力高但 Worker 在线 | 对比 Worker 能力覆盖与 Job processor/script 绑定；在线 Worker 可能不 eligible。 |
| 通知面板出现死信 | 打开 Notification Center 投递记录，检查 provider 响应、凭据、模板渲染和重试策略。 |
| HA / 网关面板 degraded | 打开 Server HA runbook，检查 `/api/v1/cluster/diagnostics`、outbox totals 和 Worker gateway locality。 |
| 审计活动没有显示近期变更 | 确认用户/API Key 有权限，再检查 Audit 页面过滤条件和 Server 日志。 |
| 页面看起来为空 | 确认 RBAC、bootstrap/login 状态，以及环境中是否已有 Jobs、Workers、instances、queue records 或 audit logs。 |

## 生产检查清单

- [ ] SSE stream routes 能通过 Web 控制台相同的 proxy/Ingress 工作。
- [ ] 运维人员知道 Dashboard 是总览页，细节要进入 Instances、Workers、Dispatch Queue、Notifications、Security 和 Audit。
- [ ] 事故记录包含 Dashboard 状态，以及被调查面板背后的精确对象 id。
- [ ] HA 部署发布时归档 cluster diagnostics、outbox/queue metrics 和 Dashboard 前后证据。
- [ ] 通知投递证据与任务执行证据分开采集。
