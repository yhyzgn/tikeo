# Instances 用户指南

Instances 页面由 `web/src/pages/InstancesPage.tsx` 实现。它展示 Jobs 与 Workflows 产生的执行记录，包括每次 attempt 的节点、广播结果、日志、取消操作与 live stream 更新。

## 源码对应的数据路径

页面通过 `/api/v1/jobs` 读取 Job，通过 `/api/v1/jobs/{job}/instances` 读取实例，通过 `/api/v1/instances/{instance}` 读取详情，通过 `/api/v1/instances/{instance}/attempts` 读取 attempts，通过 `/api/v1/instances/{instance}/logs` 读取日志。它还监听 `/api/v1/instances/stream`，并可为选中的 instance 打开日志流。

## 读取状态

状态以 tag 展示。`pending` 表示 scheduler 尚未完成派发。`running` 表示 Worker 已接受工作或日志正在到达。`succeeded`、`failed` 与 `partial_failed` 是终态证据；broadcast instance 在部分执行节点成功、部分失败时会是 partial。

## 日志与执行节点

单 Worker 执行时，页面显示 instance result 或 latest log 中的 Worker。广播执行时，`InstancesPage.tsx` 会按 attempt 构造 execution result node，并按 `workerId` 分组日志。可复制 Worker ID，然后到 Workers 页面和 Worker lifecycle history 中交叉验证。

## 取消边界

取消走 Management API，不是浏览器本地开关。只有 instance 仍处于 active 且 RBAC 允许执行控制时才取消。取消后刷新详情抽屉并检查日志，因为 Worker 可能在请求被接受后继续上报 cleanup 或失败证据。


## 验收检查清单

验收时至少准备一个 succeeded 单节点实例、一个 failed 实例、一个 broadcast 或 partial_failed 实例，并分别打开详情、attempts 与 logs。确认 Worker ID、result message、log count、取消按钮权限和 SSE 刷新都来自后端证据。若页面只显示本地 mock 或空状态，必须补真实 API/stream 验证。
