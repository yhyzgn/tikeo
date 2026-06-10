# Dashboard 用户指南

Dashboard 是登录后的第一块运行视图。它由 `web/src/pages/Dashboard.tsx` 实现，只汇总 Web 控制台已经从 Management API 与 Server-Sent Events stream 读取到的数据。请把它当成实时排障看板，而不是单独的配置来源。

## 源码对应的数据路径

`web/src/pages/Dashboard.tsx` 通过 `/api/v1/jobs` 读取 Jobs，通过 `/api/v1/workers` 读取 Worker 状态，并按 Job 调用 `/api/v1/jobs/{job}/instances` 汇总实例。它还监听 `/api/v1/instances/stream` 与 `/api/v1/workers/stream`，实例或 Worker 变化时会刷新卡片。后端也提供 `/api/v1/metrics/summary` 与 `/api/v1/cluster`，用于把 Dashboard 症状和原始 API 证据对齐。

## 卡片含义

可见卡片统计任务总数、启用任务、等待实例、在线 Worker 和广播实例。它们来自控制台其他页面同样使用的 `JobSummary`、`JobInstanceSummary` 与 `WorkerListResponse` 类型。如果数字异常，应先打开 Jobs、Instances 或 Workers 页面查证，再修改配置。

## 操作流程

先看在线 Worker 数。如果为零，进入 Workers 确认 Worker Tunnel 会话。如果 Worker 在线但 pending 实例持续增长，进入 Jobs 检查 processor name、worker pool、调度建议与触发模式。如果出现 failed 或 partial broadcast execution，打开 Instances 按执行节点查看日志。

## 边界与注意事项

Dashboard 不创建、不更新、不重试、不取消、不审批任何资源。它是只读界面，背后是现有 API 和 stream。stream frame 异常或暂时不可用时，`Dashboard.tsx` 会静默回退到周期刷新，因此数字停滞通常说明连接或 API 有问题，而不是存在 Dashboard 私有状态。
