# Workers 用户指南

Workers 页面由 `web/src/pages/WorkersPage.tsx` 实现。它展示 Worker Tunnel 连接、structured capabilities、持久化生命周期历史和当前派发容量。要确认 `DispatchTask` 是否能到达合格 Worker，应先看这个页面。

## 源码对应的数据路径

页面读取 `/api/v1/workers`、`/api/v1/workers/history` 与 Worker SSE stream。实际执行使用 protobuf 参考中记录的 Worker Tunnel 协议：`WorkerTunnelService`、`OpenTunnel`、`RegisterWorker`、`Heartbeat`、`DispatchTask`、`TaskLog`、`TaskResult` 与 `TaskCheckpoint`。

## 理解 Worker Tunnel 状态

Worker 主动出站连接 Server；Server 不要求业务 Worker 暴露入站端口。在线状态来自 active tunnel registry，持久化 snapshot 则在重连或 Server 重启后保留近期可见性。Worker 从 live list 消失时，先查看 lifecycle events，再判断容量是否真的全部丢失。

## 能力与路由检查

Structured capabilities 是路由契约。SDK processors、script runners、labels、worker pool、namespace、app、region 或 cluster 必须由 Worker 声明后，Job 才应依赖这些条件。Worker 不应广告自己无法执行的 sandbox 或 script runner。

## 调度队列交接

页面链接到 dispatch queue 以便更深入排查调度。如果 Jobs 有 pending instances 而 Workers 看似健康，应把 Job processor binding 与 broadcast selector 要求和 Worker table 对齐。如果没有 Worker 匹配，应修复 Worker registration 或 Job scope，而不是盲目重试。


## 验收检查清单

验收时至少启动一个真实 Worker Tunnel 连接，查看 live list、history、structuredCapabilities、labels 与 dispatch queue 是否一致。断开 Worker 后检查 persisted snapshot 和 lifecycle event 是否保留。若 Job 需要某个 processor、script runner 或 label，必须在 Worker 表中看到结构化声明，否则不要把它记为可调度容量。


## 持续维护要求

后续修改本页面时，必须同时核对对应源码、接口路径、RBAC 行为和自动化测试。文档不能为了看起来完整而描述尚未实现的按钮、字段或后端能力；如果验收发现差异，应把差异转成补丁、测试或明确风险记录。
