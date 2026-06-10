---
title: Management OpenAPI 参考
description: Tikeo HTTP 管理 API、OpenAPI 文档路由、app 级 SDK API key，以及 SDK helper 使用的任务/实例端点参考。
---

# Management OpenAPI 参考

本页从 `crates/tikeo-server/src/http/openapi.rs`、
`crates/tikeo-server/src/http/router.rs`，以及
`crates/tikeo-server/src/http/routes/` 下的 handler 整理而来。运行时
OpenAPI JSON 暴露在 `/api-docs/openapi.json`，用于生成 client、做 API
兼容性检查和 CI 策略审查。Server 二进制本身不提供浏览器文档 UI，因此本
Docusaurus 页面是 JSON 文档的人类可读补充。

所有业务 HTTP 响应都使用共享 `ApiResponse` envelope：`code` 是成功判断
字段，`message` 是可读结果，`data` 即使为 `null` 也必须显式存在。SDK
management client 使用 app 级 API key，通过 `x-tikeo-api-key` 认证，
通常从 `TIKEO_MANAGEMENT_API_KEY` 注入。不要把浏览器/OIDC session 当作
机器 SDK 凭据复用。

## 源文件与运行时路由

| 契约 | 源文件 |
| --- | --- |
| OpenAPI assembly | `crates/tikeo-server/src/http/openapi.rs` |
| HTTP router 和 `/api-docs/openapi.json` | `crates/tikeo-server/src/http/router.rs` |
| Job / instance DTO | `crates/tikeo-server/src/http/dto.rs` |
| Job / instance handler | `crates/tikeo-server/src/http/routes/jobs.rs` |
| SDK API-key handler | `crates/tikeo-server/src/http/sdk_api_keys.rs` |

OpenAPI 路由挂在 `/api/v1` 外部，所以运维检查可以直接抓取
`/api-docs/openapi.json`，不需要猜测 API 版本前缀。管理操作本身仍位于
`/api/v1` 下，并受 handler 所选择的认证与授权边界保护。

## SDK management 认证边界

SDK create/trigger 流程是机器到机器调用。管理员先创建 Service Account，
再签发有 scope 的 SDK API key；worker 或自动化工具在 `x-tikeo-api-key`
中发送该 key。key 被限定到 namespace/app 和可选 worker-pool 边界；它不
是用户 session token，也不应该保存到浏览器状态中。

默认 helper 语义刻意保持收敛：

- 创建 helper 构造带 API schedule 的 `CreateJobRequest`。
- 触发 helper 构造 `triggerType=api` 的 `TriggerJobRequest`。
- 默认触发路径使用 `executionMode=single`。
- 广播必须使用显式 broadcast helper 和 `broadcastSelector`。

## Post api v1 jobs

`POST /api/v1/jobs` 创建 Job 定义，并在共享 `ApiResponse` envelope 中返回。
不同语言的 SDK helper 名称不同，但创建 API 触发 processor job 时都映射
到这个端点：`ManagementCreateJobRequest::api`、`APIJob`、
`CreateJobRequest.api`、`api_job` 和 `apiJob`。

请求体由 `CreateJobRequest` 表示。对 SDK helper 来说，关键字段是 client
携带的 namespace/app scope、任务名、processor name，以及
`scheduleType=api`。服务端仍会执行常规 RBAC、scope、worker-pool、schedule、
canary 和脚本绑定校验；helper 不会绕过调度状态机。

## Post api v1 jobs job trigger

`POST /api/v1/jobs/{job}:trigger` 为手动/API 执行创建 Job Instance。OpenAPI
路径记录为 `/api/v1/jobs/{job}:trigger`，即使 Axum 内部通过兼容路由解析
action 后缀。SDK 默认 trigger helper 映射到这里，并发送 `triggerType=api`
和 `executionMode=single`。

只有意图是对所有匹配 Worker 扇出时才使用 broadcast helper。广播 payload
会设置 `executionMode=broadcast` 并包含 `broadcastSelector`，让代码审查
和审计日志可以把 fan-out 与默认 single-worker 行为区分开。

## Get api v1 instances instance

`GET /api/v1/instances/{instance}` 返回 create/trigger 后的当前实例摘要。SDK
示例会直接或通过 helper 轮询这个端点，等待 `succeeded` 或 `failed` 等终态。
响应包含 trigger type、execution mode、result summary 和调度元数据，用于
确认 Server 是通过 Worker Tunnel 派发，而不是自行执行用户代码。

验证 API-trigger 冒烟时，在 `/api/v1/jobs/{job}:trigger` 后使用该端点确认
实例属于预期 namespace/app，并且最终结果来自真实 Worker。

## Get api v1 instances instance logs

`GET /api/v1/instances/{instance}/logs` 返回持久化任务日志。日志由 Worker 通过
Worker Tunnel 携带 assignment-token authority 写入，再通过 HTTP 管理 API
呈现。这个端点是用户侧证明任务由 Worker 处理的最直接证据。

端到端证据应把实例轮询和日志读取配对使用，寻找 processor 专属日志。Management
API trigger smoke 正是这样做：先验证实例结果，再验证 worker log evidence。

## SDK 文档端点清单

SDK 页面应把 helper 行为链接到上面的精确锚点：

- 创建任务 helper → [`POST /api/v1/jobs`](./management-openapi#post-api-v1-jobs)
- 触发 helper → [`POST /api/v1/jobs/{job}:trigger`](./management-openapi#post-api-v1-jobs-job-trigger)
- 轮询实例 → [`GET /api/v1/instances/{instance}`](./management-openapi#get-api-v1-instances-instance)
- 查看日志 → [`GET /api/v1/instances/{instance}/logs`](./management-openapi#get-api-v1-instances-instance-logs)

这些链接必须保持 source-backed。如果要文档化新的 SDK helper，先确认它已在
提交的 SDK 源码中存在，并且序列化 payload 与 OpenAPI 请求契约一致。
