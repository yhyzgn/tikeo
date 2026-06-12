---
title: 集成概览
description: Tikeo 与 API、Worker Tunnel、观测、身份、部署和通知系统的集成操作入口。
---

# 集成概览

Tikeo 的集成按运行边界划分。Server 负责调度、状态、治理、HTTP API 和审计；Worker 负责执行；部署系统负责交付与回滚；观测系统负责展示证据；通知系统负责把事件投递到外部渠道。

## 集成地图

| 集成 | 方向 | 主要契约 | 用途 |
|---|---|---|---|
| HTTP API / OpenAPI | 外部工具 ↔ Tikeo Server | `/api-docs/openapi.json`、`/api/v1/**` | 管理 Job、实例、租户、脚本、通知、身份和 GitOps。 |
| Worker Tunnel / protobuf | Worker → Tikeo Server | `crates/tikeo-proto/proto/tikeo/worker/v1/worker.proto` | Worker 主动连接、心跳、接收 `DispatchTask`、回传日志和结果。 |
| Prometheus / Grafana | Tikeo Server → 观测系统 | `/metrics`、`/api/v1/metrics/summary` | 调度队列、Worker、实例、脚本治理和 workflow SLO。 |
| OpenTelemetry | Tikeo Server → Collector | `observability.tracing`、OTLP HTTP endpoint | 导出 trace，关联请求和后台处理。 |
| OIDC | 身份提供方 ↔ Tikeo Server | `/api/v1/auth/oidc/authorize`、`/api/v1/auth/oidc/callback`、`/api/v1/oidc-identities` | 把外部身份映射成本地 session、角色和 scope。 |
| Terraform / GitOps | IaC 工具 ↔ Tikeo Server | `/api/v1/gitops/manifest`、`/api/v1/gitops/diff` | 导出/校验声明式资源，支持 drift 检查。 |
| Kubernetes / Helm | 集群 → Tikeo 运行时 | `deploy/k8s/tikeo.yaml`、`deploy/helm/tikeo/values.yaml` | 部署 Server、Web、探针、配置和 Secret。 |
| 入站 Webhook 触发 | 外部系统 → Tikeo Server | `POST /api/v1/events/webhooks/{job}:trigger` | 把外部事件转成 Job Instance。 |
| 出站通知渠道 | Tikeo Server → 外部系统 | `/api/v1/notification-channels`、`notification_channels` | 投递到 webhook、Slack、钉钉、飞书、企微、PagerDuty、email。 |

## 前置条件

本地验证集成前，先启动 Server 并确认 API 可用：

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

另开终端：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

需要登录的 API 示例统一使用本地 token 占位符：

```bash
export TIKEO_TOKEN='<local-admin-token>'
```

不要在命令历史、文档或截图中保留真实凭据。

## HTTP API / OpenAPI

管理 API 的稳定入口是 `/api/v1/**`。OpenAPI JSON 可用于生成客户端、检查路由是否加载、或对比版本差异。

```bash
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json \
  | python3 -m json.tool >/tmp/tikeo-openapi.pretty.json
```

常用管理路径：

| 能力 | 路径 |
|---|---|
| Job 列表/创建 | `GET/POST /api/v1/jobs` |
| Job 触发 | `POST /api/v1/jobs/{job}:trigger` |
| 实例详情 | `GET /api/v1/instances/{instance}` |
| 实例日志 | `GET /api/v1/instances/{instance}/logs` |
| 租户资源 | `/api/v1/namespaces`、`/api/v1/apps`、`/api/v1/worker-pools` |
| API key | `/api/v1/management/service-accounts`、`/api/v1/management/api-keys` |
| GitOps | `/api/v1/gitops/manifest`、`/api/v1/gitops/diff` |

验收：OpenAPI 中应包含 `/api/v1/jobs`、`/api/v1/jobs/{job}:trigger` 和 `/api/v1/jobs/{job}/instances`。

## Worker Tunnel / protobuf

Worker 通过 Worker Tunnel 主动连接 Server，不要求 Server 主动连回 Worker。协议定义位于 `crates/tikeo-proto/proto/tikeo/worker/v1/worker.proto`，核心消息包括 Worker 注册、心跳、`DispatchTask`、日志和结果回传。

本地联调常用脚本：

```bash
scripts/dev-integration-seed.sh
scripts/start-java-demo-workers.sh
```

多语言 smoke 可按需要运行：

```bash
deploy/smoke/cross-language-worker-parity-smoke.sh
```

验收：`GET /api/v1/workers` 能看到在线 Worker；创建 `processorName` 匹配的 Job 后，实例不应长期停留 pending。

## Prometheus 与指标摘要

Prometheus 文本指标入口是 `/metrics`；控制台和排障常用摘要入口是 `/api/v1/metrics/summary`。

```bash
curl -fsS http://127.0.0.1:9090/metrics | head

curl -fsS http://127.0.0.1:9090/api/v1/metrics/summary \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

重点指标包括：

- `tikeo_dispatch_queue_pending_age_seconds`
- `tikeo_dispatch_queue_dispatch_latency_seconds`
- `tikeo_workers_online_current`
- `tikeo_job_instances_current`
- `tikeo_job_instance_success_ratio`
- `tikeo_script_governance_failures_current`
- `tikeo_workflow_instances_current`

生产建议：Prometheus scrape 使用 `/metrics`；健康探针使用 `/readyz` 或 `/healthz`，不要使用 SSE 或业务写入路径。

## OpenTelemetry trace

`config/dev.toml` 中 `observability.tracing.enabled` 默认是 `false`。启用时至少配置 OTLP HTTP endpoint：

```toml
[observability.tracing]
enabled = true
headers = []
otlp_endpoint = "http://otel-collector:4318/v1/traces"
```

生产检查：

- collector 地址从 Server 网络命名空间可达；
- header 不含明文密钥；
- trace 采样、保留周期和数据脱敏符合组织要求；
- collector 未就绪时，Server 日志能显示可排查的导出错误。

## OIDC 身份集成

OIDC 配置位于 `[auth.oidc]`。本地默认关闭：

```toml
[auth.oidc]
enabled = false
scopes = ["openid", "profile", "email"]
```

相关 API：

| 用途 | 路径 |
|---|---|
| 查看认证状态 | `GET /api/v1/auth/status` |
| 生成授权地址 | `GET /api/v1/auth/oidc/authorize` |
| 完成回调 | `GET /api/v1/auth/oidc/callback` |
| 管理外部身份映射 | `GET/POST/DELETE /api/v1/oidc-identities` |

验收：`/api/v1/auth/status` 中 `oidc.enabled` 与配置一致；`client_secret_configured` 只返回布尔值，不返回密钥本身。

## Terraform、GitOps 与 Kubernetes

GitOps API 可导出现有 manifest 并执行 diff：

```bash
curl -fsS "http://127.0.0.1:9090/api/v1/gitops/manifest?namespace=demo&app=orders" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

相关 smoke：

```bash
deploy/smoke/gitops-live-smoke.sh
deploy/smoke/terraform-provider-live-smoke.sh
deploy/smoke/k8s-operator-dry-run-smoke.sh
```

部署文件入口：

- `deploy/k8s/tikeo.yaml`
- `deploy/helm/tikeo/values.yaml`
- `docker-compose.yml`
- `docker-compose.postgres.yml`
- `docker-compose.mysql.yml`

生产检查：

- readiness 使用 `/readyz`，liveness 使用 `/healthz`；
- Secret 由集群密钥系统注入，不写入 manifest 明文；
- 变更前后保留 manifest diff；
- 回滚路径在 staging 验证过。

## 入站 Webhook 触发

入站 Webhook 是外部系统调用 Tikeo 的触发入口：

```bash
curl -fsS -X POST "http://127.0.0.1:9090/api/v1/events/webhooks/${TIKEO_JOB_ID}:trigger" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "source": "ci",
    "eventType": "build.finished",
    "payload": {"status":"passed"}
  }'
```

签名字段：`secretRef`、`signature`、`timestamp`、`nonce`。`secretRef` 只应引用环境变量，例如 `env:TIKEO_WEBHOOK_SECRET`；不要传递真实密钥文本。

验收：响应中 `data.accepted=true`，`data.triggerType=webhook`，并能在实例日志中看到 `webhook_event_source`。

## 出站通知渠道

出站通知渠道用于把 Tikeo 内部事件投递到外部系统，方向与入站 Webhook 相反。相关页面和 API：

| 能力 | 路径 |
|---|---|
| 渠道类型 | `GET /api/v1/notification-channel-types` |
| 渠道 | `GET/POST/PATCH/DELETE /api/v1/notification-channels` |
| 策略 | `GET/POST/PATCH/DELETE /api/v1/notification-policies` |
| 模板 | `GET/POST/PATCH/DELETE /api/v1/notification-templates` |
| 队列状态 | `GET /api/v1/notification-delivery-attempts:queue-status` |
| 处理 due attempts | `POST /api/v1/notification-delivery-attempts:retry-due` |

生产检查：Webhook URL、Slack incoming webhook、PagerDuty routing key、SMTP URL/password、飞书/钉钉签名密钥、插件型 `appId`/`appSecret` 等都放在每条渠道记录自己的 `secretRefs`/env 引用中；不同渠道使用不同引用名，UI 和 API 返回值只应展示脱敏目标。

## 排障

| 集成 | 常见现象 | 检查 |
|---|---|---|
| OpenAPI | `/api-docs/openapi.json` 404/空响应 | Server 路由是否启动；是否访问了正确端口。 |
| Worker Tunnel | Worker online 后任务仍 pending | `processorName`、`processorType`、namespace/app/pool 是否匹配；Worker 是否广告对应 capability。 |
| Prometheus | scrape 成功但没有业务指标 | 先触发至少一个 Job 或 workflow，再查看 `/metrics`。 |
| OTel | collector 无 trace | `observability.tracing.enabled`、`otlp_endpoint`、网络和 collector 日志。 |
| OIDC | callback 失败 | issuer URL、client ID、client secret、redirect URI、state 是否一致。 |
| GitOps/Terraform | diff 与预期不一致 | 确认 namespace/app 过滤条件和当前 API 返回。 |
| 入站 Webhook | `webhook nonce was already used` | nonce 不能复用；重放请求会被拒绝。 |
| 出站通知 | delivery 进入 DLQ | 查看 `/api/v1/notification-delivery-attempts`、渠道 enabled 状态和脱敏 target。 |

## 清理/生产检查清单

- 本地演示结束后删除临时 Job、worker pool、API key、通知渠道和 OIDC 映射。
- 生产环境只开放必需端口：Web/API、Worker Tunnel、Prometheus scrape 或 ingress。
- 所有跨网络调用使用 HTTPS；Worker Tunnel 按环境启用 TLS/mTLS。
- 健康检查不要使用写入接口、SSE stream 或长连接端点。
- 入站 Webhook 与出站通知渠道分开配置、分开审计、分开轮换密钥。
- 发布前保留 smoke 输出、OpenAPI diff、部署 manifest diff 和回滚步骤。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
