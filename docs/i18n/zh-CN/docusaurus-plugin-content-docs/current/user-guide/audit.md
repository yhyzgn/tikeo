# Audit 用户指南

Audit 页面由 `web/src/pages/AuditLogsPage.tsx` 实现。它是平台写操作、认证事件、脚本治理动作、派发相关事件和 failure reason 的治理证据界面。

## 源码对应的数据路径

页面通过 `/api/v1/audit-logs` 按服务端过滤读取日志，并通过 `/api/v1/audit-logs:export` 导出 JSON。导出路径保留相同 filter，并采用有上限的治理导出，而不是任意倾倒数据库表。

## 过滤模型

过滤条件包括 actor、action、resource type、resource id、failure reason 和 page size。URL query state 会持久化，所以复制 URL 可以保留当前审计调查视图。Result tag 区分成功与失败操作，失败行会在有数据时展示 failure reason。

## Before/after 与 trace 证据

行可能包含 before/after snapshot、trace ID、IP address、user agent 和 request identifiers。用 trace ID 可关联 API 错误和服务端日志。用 before/after snapshot 可确认实际变更，尤其是 Job scope move、script publication、API-Key rotation 与 RBAC edits。

## 导出使用

需要和运维或 release reviewer 共享证据时，导出当前 filter。导出格式是 JSON，暂不提供 CSV，因为治理数据的 redaction 与 content-type policy 更严格。请把导出文件当作敏感运维记录处理。


## 验收检查清单

验收时应至少过滤一个成功写操作、一个失败操作和一个带 trace id 的请求，并导出相同过滤条件的 JSON。导出后核对 `exported` 数量、filter metadata、before/after 字段和 failure reason 是否保留。若审计证据缺失，不应只看 UI 表格通过，而要回到服务端审计写入路径和权限检查补测试。
