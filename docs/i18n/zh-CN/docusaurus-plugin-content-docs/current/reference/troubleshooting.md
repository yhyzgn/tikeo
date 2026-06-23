---
title: 排障指南
description: Tikeo 本地和部署环境的首轮排障手册。
---

# 排障指南

先证明问题发生在哪一层，再修改配置或代码。推荐顺序是：进程 → 健康检查 → 存储/migration → 认证/RBAC → Worker Tunnel → Job/实例 → Web 控制台 → 审计和通知。

## 前置条件

从仓库根目录执行命令。先确认工具：

```bash
cargo --version
curl --version
python3 --version
```

Web 和 docs 相关排障再确认 Bun：

```bash
bun --version
```

默认本地 API：

```bash
export TIKEO_HTTP_URL=http://127.0.0.1:9090
```

需要认证的接口使用本地 token：

```bash
export TIKEO_TOKEN='<local-admin-token>'
```

不要把真实 token 粘贴到日志、截图或文档。

## 1. Server 无法启动

用最小命令启动：

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

优先检查：

- `config/dev.toml` 是否能解析；
- `server.listen_addr` 端口是否被占用；
- `server.worker_tunnel_addr` 端口是否被占用；
- `storage.database.*` 指向的 SQLite 文件或外部数据库是否可写；
- migration 是否失败；
- 环境变量覆盖是否把地址、TLS、数据库或认证配置改错。

常用定位命令：

```bash
ss -ltnp | grep -E ':(9090|9998)\b' || true
ls -l .dev/tikeo-dev.db 2>/dev/null || true
```

## 2. 健康检查失败

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

判断方式：

| 结果 | 含义 | 下一步 |
|---|---|---|
| `healthz` 失败 | HTTP Server 不可达或进程未监听 | 看启动日志、端口、容器端口映射、Ingress/LB。 |
| `healthz` 成功，`readyz` 失败 | 进程活着，但依赖未就绪 | 看存储连接、migration、外部数据库、配置覆盖。 |
| 两者都成功，业务 API 失败 | 进入认证、权限、scope 或业务层排查 | 查看 API 响应 envelope、Server 日志和审计。 |

Kubernetes 中 readiness probe 应使用 `/readyz`，liveness probe 使用 `/healthz`。不要把 SSE stream 或写入接口当探针。

## 3. 认证和权限失败

常见 API 返回：401、403，或 envelope 中出现权限相关 message。

检查顺序：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/status

curl -fsS http://127.0.0.1:9090/api/v1/auth/me \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

重点确认：

- bootstrap 是否已创建 Owner；
- 本地登录是否启用；
- token 是否过期；
- 角色是否包含目标资源权限，例如 `jobs:read`、`jobs:write`、`instances:execute`、`tenants:manage`、`notifications:read`；
- API key 的 scope 是否允许目标 namespace/app/worker pool。

API key smoke 可参考：

```bash
deploy/smoke/sdk-api-key-live-smoke.sh
```

## 4. Worker 不可见或任务 pending

先看 Worker 列表：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workers \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"

curl -fsS http://127.0.0.1:9090/api/v1/workers/history \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

排查要点：

- Worker 是否能访问 Worker Tunnel 地址，默认本地 Worker Tunnel 监听端口是 `9998`；客户端应连接可路由地址，例如 `http://127.0.0.1:9998`；
- Worker 是否设置了正确 namespace、app、worker pool；
- Worker 广告的 `processorName` / capability 是否与 Job 绑定一致；
- generation/fencing token 是否拒绝陈旧心跳或结果；
- Worker 是真实连接模式，不是 dry-run 输出。

不要用过宽 wildcard capability 让任务“看起来能跑”。正确修复点通常是 Job 的 `processorName`、`processorType`、`scriptId`、worker pool assignment，或 Worker runtime 安装。

## 5. Job 创建或触发失败

创建 Job 的关键路径：

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/jobs \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "namespace":"default",
    "app":"default",
    "name":"troubleshoot-echo",
    "scheduleType":"api",
    "processorName":"demo.echo",
    "enabled":true
  }'
```

触发路径：

```bash
curl -fsS -X POST "http://127.0.0.1:9090/api/v1/jobs/${TIKEO_JOB_ID}:trigger" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"triggerType":"api","executionMode":"single"}'
```

检查实例：

```bash
curl -fsS "http://127.0.0.1:9090/api/v1/instances/${TIKEO_INSTANCE_ID}" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"

curl -fsS "http://127.0.0.1:9090/api/v1/instances/${TIKEO_INSTANCE_ID}/logs" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"

curl -fsS "http://127.0.0.1:9090/api/v1/instances/${TIKEO_INSTANCE_ID}/attempts" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

常见错误：

| 现象 | 原因 | 处理 |
|---|---|---|
| 创建 Job 返回 400 | `scheduleType` 无效，或 `processorName` 与 `scriptId` 同时设置 | 使用 `api`、`cron`、`fixed_rate` 等受支持类型；二选一绑定 processor 或 script。 |
| 触发返回 404 | Job ID 不存在或路径拼错 | 确认路径是 `/api/v1/jobs/{job}:trigger`。 |
| pending 时间过长 | 没有合格 Worker | 看 `/api/v1/workers` 和 `/api/v1/jobs/{job}/scheduling-advice`。 |
| failed/partial_failed | Worker 执行失败或脚本治理失败 | 看实例日志、attempts、审计记录。 |

## 6. 入站 Webhook 触发失败

入站 Webhook 路径是：

```text
POST /api/v1/events/webhooks/{job}:trigger
```

它用于外部系统触发 Tikeo Job，方向是“外部系统 → Tikeo”。不要和“出站通知渠道”混用；出站通知是 Tikeo 投递到 webhook、Slack、钉钉、飞书、企微、PagerDuty 或 email。

无签名本地测试：

```bash
curl -fsS -X POST "http://127.0.0.1:9090/api/v1/events/webhooks/${TIKEO_JOB_ID}:trigger" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"source":"local","eventType":"demo.event","payload":{"ok":true}}'
```

签名相关失败：

| message | 检查 |
|---|---|
| `webhook signature requires secretRef` | 提供了签名字段但没有 `secretRef`。 |
| `webhook timestamp is outside replay window` | timestamp 与 Server 当前时间相差超过 300 秒。 |
| `webhook nonce was already used` | nonce 已经成功使用过，重放被拒绝。 |
| `webhook secretRef is not resolvable` | `secretRef` 未使用可解析环境变量，例如 `env:TIKEO_WEBHOOK_SECRET`。 |
| `webhook signature verification failed` | 签名计算输入不一致。 |

## 7. 出站通知失败或 DLQ 增长

通知中心相关路径：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

排查顺序：

1. channel 是否 enabled；
2. policy 是否引用至少一个 channel；
3. provider 目标是否通过 `secretRefs` 或环境变量配置；
4. `targetRedacted` 是否指向预期外部系统；
5. `retryState` 是 `retry_pending`、`retry_consumed` 还是 `dead_letter`；
6. `notification_delivery.enabled`、`interval_seconds`、`batch_size`、`max_attempts` 是否符合预期。

## 8. Web 控制台不刷新

控制台实时刷新使用 SSE。先确认普通 API 可用，再查 SSE：

```bash
curl -N \
  -H 'Accept: text/event-stream' \
  "http://127.0.0.1:9090/api/v1/workers/stream?token=${TIKEO_TOKEN}"
```

常见原因：

- 代理缓冲了 `text/event-stream`；
- nginx/LB/WAF idle timeout 低于 keep-alive 周期；
- query token 被 rewrite 或过滤；
- `/api/v1/**/stream` 被 CDN 缓存或 challenge 页面拦截；
- 浏览器不同源访问时 CORS/origin 配置不一致。

更多部署配置见 [SSE 实时刷新部署注意事项](../deployment/sse-realtime)。

## 9. Docker 或 CI 构建慢

Server 镜像会编译 Rust workspace，冷启动 runner 上明显慢于 Web 镜像。先看日志是否仍在下载依赖或编译 crate，不要把“耗时”直接判定为失败。

常用本地验证：

```bash
cargo build --bin tikeo
cargo test --workspace
```

文档站或 Web 相关命令按项目约定使用 Bun：

```bash
cd docs
bun install
bun run build
```

## 10. 升级问题时带什么证据

提交问题前，请收集：

- Tikeo commit 或镜像 tag；
- 配置文件路径和相关配置片段，密钥脱敏；
- 数据库后端和 migration 日志；
- `curl -fsS http://127.0.0.1:9090/healthz` 输出；
- `curl -fsS http://127.0.0.1:9090/readyz` 输出；
- Worker SDK 语言、版本、启动命令和 Worker 日志；
- 相关 Job ID、Instance ID、attempt ID、audit ID；
- API 响应 envelope；
- 如果涉及通知，附上 message ID、delivery attempt ID、`targetRedacted` 和 `retryState`。

## 保留现场

排障时优先保留日志、配置路径、命令输出和相关 ID。不要先清空数据库或重启所有服务，否则会丢失 migration、fencing、policy、attempt 和 audit 证据。能复现的问题应固化为脚本或测试。

## 清理/生产检查清单

- 本地临时环境可停止 Server 后删除 `.dev/tikeo-dev.db .dev/tikeo-dev.db-shm .dev/tikeo-dev.db-wal`。
- 不在生产执行会改写数据库的本地演示脚本。
- 不把真实 token、Webhook URL、SMTP 密码、PagerDuty routing key 或 Authorization header 放进排障材料。
- 生产探针只使用 `/healthz` 和 `/readyz`。
- 修复后至少重新验证：健康检查、目标 API、一个 Worker 派发链路、实例日志和审计记录。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
