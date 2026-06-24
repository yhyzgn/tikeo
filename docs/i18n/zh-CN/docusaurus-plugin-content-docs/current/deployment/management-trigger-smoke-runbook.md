---
title: Management trigger smoke 运行手册
description: 面向贡献者的 Management API create/trigger 端到端 smoke 运行与排障说明。
---

# Management trigger smoke 运行手册

`scripts/management-trigger-e2e-smoke.sh` 是 SDK Management API 创建并触发作业链路的贡献者 smoke。它不是模拟测试：脚本会启动真实本地 `tikeo` server，创建真实 app-scoped 机器凭证，启动 Node.js demo worker 并通过出站 Worker Tunnel 注册，然后用 Node.js SDK `ManagementClient` 创建作业、执行 `apiTrigger`，最后验证实例结果和持久化日志。

当你修改 Management API 鉴权、SDK helper 名称、Worker Tunnel 注册、实例结果持久化、任务日志、Node.js demo worker 或 CI 的 `other-cross-language-smoke` 时，应运行这个 smoke。

## smoke 验证什么

脚本事实来源是 `scripts/management-trigger-e2e-smoke.sh` 与 `deploy/smoke/lib/tikeo-smoke-lib.sh`，覆盖真实路径而不是伪造 HTTP：

- 使用 `serve --config "$SERVER_CONFIG"` 启动本地 server，并在 `DB_PATH` 下生成隔离 SQLite；
- 用 `tikeo_smoke_wait_for_http server "$API_URL/readyz"` 验证 readiness；
- 通过 `/api/v1/namespaces`、`/api/v1/apps`、`/api/v1/worker-pools` seed namespace、app 和 worker pool；
- 通过 `POST /api/v1/management/service-accounts`、`POST /api/v1/management/api-keys` 和 `x-tikeo-api-key` 验证机器到机器凭证；
- 以 `TIKEO_WORKER_CONNECT=1`、`TIKEO_WORKER_ENDPOINT`、`TIKEO_WORKER_NAMESPACE`、`TIKEO_WORKER_APP`、`TIKEO_WORKER_POOL` 启动真实 Node.js demo worker；
- 使用 `ManagementClient`、`apiJob`、`apiTrigger`、`createJob`、`triggerJob` 创建并触发 API 作业；
- 通过 `/api/v1/instances/$instance_id`、`/api/v1/instances/$instance_id/logs`、`result.success` 和 `nodejs demo echo processed` 验证实例和日志。

重要 case ID 包括 `management-scope-seed`、`management-sdk-api-key`、`management-worker-online`、`management-sdk-create-trigger`、`management-instance-result`。通过时脚本会调用 `tikeo_smoke_finalize_report`，并输出 `management trigger e2e report:` 与 `management trigger e2e evidence:`。

## 前置条件

从仓库根目录运行。脚本会通过 `tikeo_smoke_need_cmd` 检查这些命令：

```bash
cargo --version
bun --version
python3 --version
curl --version
```

默认端口必须空闲，或通过环境变量覆盖：

```bash
export TIKEO_HTTP_URL=http://127.0.0.1:19093
export TIKEO_WORKER_ENDPOINT=http://127.0.0.1:19993
```

脚本可以自行构建 `target/debug/tikeo`。本地快速循环建议先构建，再复用二进制：

```bash
cargo build --bin tikeo
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

需要完全自包含运行时：

```bash
scripts/management-trigger-e2e-smoke.sh
```

## 常用环境变量

| 变量 | 默认值 | 用途 |
|---|---|---|
| `TIKEO_MANAGEMENT_TRIGGER_RUN_ID` | `management-trigger-e2e-<UTC>-<pid>` | 固定报告文件名前缀。 |
| `TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR` | `.dev/reports/$RUN_ID` | 覆盖证据目录。 |
| `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER` | `1` | 已执行 `cargo build --bin tikeo` 后可设为 `0`。 |
| `TIKEO_MANAGEMENT_TRIGGER_NAMESPACE` | `sdk-smoke` | SDK key 所属 namespace。 |
| `TIKEO_MANAGEMENT_TRIGGER_APP` | `management` | SDK key 与 job 所属 app。 |
| `TIKEO_MANAGEMENT_TRIGGER_WORKER_POOL` | `nodejs-blue` | demo worker 使用的 worker pool。 |
| `TIKEO_MANAGEMENT_TRIGGER_CLIENT_INSTANCE_ID` | `nodejs-management-trigger-smoke` | 期望的 worker `clientInstanceId`。 |
| `TIKEO_HTTP_URL` | `http://127.0.0.1:19093` | Server API 地址。 |
| `TIKEO_WORKER_ENDPOINT` | `http://127.0.0.1:19993` | demo worker 主动连接的 Worker Tunnel 地址。 |

固定证据路径示例：

```bash
export TIKEO_MANAGEMENT_TRIGGER_RUN_ID=management-trigger-e2e-local
export TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR=.dev/reports/management-trigger-e2e-local
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

## 证据目录

默认输出在 `.dev/reports/management-trigger-e2e-*` 下。脚本结束时会打印：

```text
management trigger e2e report: .dev/reports/management-trigger-e2e-.../management-trigger-e2e-....json
management trigger e2e evidence: .dev/reports/management-trigger-e2e-...
```

关键文件：

| 文件模式 | 含义 |
|---|---|
| `*-config.toml` | 生成的 server 配置，包含隔离 SQLite 与本地明文 listener。 |
| `*-server.log` | server 启动、鉴权、存储、派发日志。 |
| `*-nodejs-worker.log` | Node.js demo worker 与 Worker Tunnel 日志。 |
| `*-service-account.json` | Service Account 创建响应。 |
| `*-api-key.json` | API key 创建响应，留在 `.dev/`，不要提交。 |
| `*-sdk-key-jobs-list.json` | `x-tikeo-api-key` 可以访问 jobs 的证据。 |
| `*-sdk-create-trigger.json` | SDK `ManagementClient` 创建/触发输出。 |
| `*-instance.json` | 最终实例状态。 |
| `*-instance-logs.json` | 持久化实例日志。 |
| `*-cases.jsonl` | 单个 smoke case 记录。 |
| `*.json` report | `tikeo_smoke_finalize_report` 聚合结果。 |
| `*-summary.json` | 证据文件索引。 |

## 排障路径

1. **Server 未到 `/readyz`**：看 `*-server.log`、`*-config.toml`，确认 `TIKEO_HTTP_URL` / `TIKEO_WORKER_ENDPOINT` 端口未被占用。
2. **Service Account 或 API key 创建失败**：检查 `/api/v1/management/service-accounts`、`/api/v1/management/api-keys`、`x-tikeo-api-key`、bootstrap 登录和 RBAC scope。
3. **Worker 未 online**：看 `*-nodejs-worker.log`、`TIKEO_WORKER_CONNECT=1`、`TIKEO_WORKER_ENDPOINT`、`/api/v1/workers`、`clientInstanceId`、namespace/app/pool，以及 `structuredCapabilities.normalProcessors` 是否包含 `demo.echo`。
4. **SDK create/trigger 失败**：检查 `sdks/nodejs/tikeo/src/management.ts`、`apiJob`、`apiTrigger`、`/api/v1/jobs`、`/api/v1/jobs/{job}:trigger` 和默认 `executionMode=single`。
5. **实例未成功**：检查 dispatcher 日志、`/api/v1/instances/$instance_id`、`/api/v1/instances/$instance_id/logs`、`result.success`，以及结果和日志中是否都有 `nodejs demo echo processed`。

不要把 dry-run worker 当成通过。脚本刻意不使用 `TIKEO_WORKER_DRY_RUN=1`；worker 必须通过真实 Worker Tunnel 主动出站连接。
