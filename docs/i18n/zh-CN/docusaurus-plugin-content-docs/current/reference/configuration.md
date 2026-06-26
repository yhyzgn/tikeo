---
title: 配置参考
description: Tikeo Server 与 Worker 的完整配置参考、环境变量、默认值、挂载目录、结构化数据库配置和部署说明。
---

# 配置参考

Tikeo 有两个配置面：

1. **Server 配置**控制控制面、存储、认证、集群、通知投递、TLS 和观测。Docker/Compose 使用 `config/tikeo.yml`，容器内挂载为 `/config/tikeo.yml`。
2. **Worker 配置**位于 SDK 或业务应用配置中。Java Spring Boot 暴露为 `tikeo.worker.*`；其他 SDK 使用等价 `WorkerConfig` 字段或 demo 环境变量。

Server 加载顺序：默认值、`tikeo serve --config <path>`/`TIKEO_CONFIG` 指定的文件、`TIKEO__...` 环境变量覆盖。例如 `storage.database.host` 对应 `TIKEO__STORAGE__DATABASE__HOST`。

正常部署优先编辑挂载配置文件。环境变量覆盖更适合 Kubernetes Secret、紧急覆盖或无法挂载文件的平台。

## 运行时文件与挂载路径

| 路径 | 用途 | 含义 | 挂载建议 |
| --- | --- | --- | --- |
| `/config/tikeo.yml` | Dockerfile、Compose、Kubernetes、Helm | `serve --config /config/tikeo.yml` 读取的 Server 配置。 | 从 host path、ConfigMap 或 Secret 只读挂载。 |
| `/config/tls` | TLS/mTLS | `transport_security.*` 引用的证书、私钥、CA。 | 只读挂载，不要把私钥打进镜像。 |
| `/data/tikeo.db` | SQLite 模式 | `storage.database.path=/data/tikeo.db` 对应的 SQLite 文件。 | SQLite 数据需要保留时持久化 `/data`。 |
| `/logs/tikeo.log` 和 `/logs/tikeo-error.log` | 可选文件日志 | 文件日志 sink 启用且指向 `/logs` 时生成。 | 可选；stdout 始终输出。 |
| `/etc/tikeo/tikeo.yml` | systemd/裸机 | 主机上的配置文件。 | root 或部署系统管理，进程可读。 |
| `/var/lib/tikeo` | systemd/裸机 | 本地持久状态，通常是 VM 上的 SQLite。 | `tikeo` 用户拥有；使用 SQLite 时纳入备份。 |
| `/var/log/tikeo` | systemd/裸机 | 主机文件日志。 | 启动前创建并按主机策略轮转。 |

## 仓库内置配置文件

| 文件 | 用途 | 说明 |
| --- | --- | --- |
| `config/tikeo.yml` | 生产/容器模板 | 唯一正式部署入口。默认 SQLite `/data/tikeo.db`，包含 PostgreSQL/MySQL/Raft/TLS 注释示例。 |
| `config/dev.yml` | 本地源码开发 | 保留快速 dev 路径，使用 `.dev/tikeo-dev.db`。 |

## 结构化数据库配置

Tikeo 使用结构化 `storage.database.*` 字段配置 Server 持久化。密码包含 `@`、`/`、`:`、`#` 时可以直接写普通值；Tikeo 会生成内部 sqlx/SeaORM connection URL 并自动转义凭据。

```yaml
storage:
  database:
    type: postgres
    host: postgres
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: disable
```


## Server 配置表

下面是 Server 的完整默认值表，包含配置项、环境变量、是否必填、默认值和说明。

| 配置项 | 环境变量 | 是否必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `server.listen_addr` | `TIKEO__SERVER__LISTEN_ADDR` | 否 | `0.0.0.0:9090` | HTTP API、健康检查、metrics、OpenAPI、Web API 目标绑定地址。 |
| `server.worker_tunnel_addr` | `TIKEO__SERVER__WORKER_TUNNEL_ADDR` | 否 | `0.0.0.0:9998` | gRPC/HTTP2 Worker Tunnel 绑定地址；Worker 主动连接。 |
| `storage.database.type` | `TIKEO__STORAGE__DATABASE__TYPE` | 否 | `sqlite` | `sqlite`、`postgres`、`mysql` 或 `cockroachdb`。 |
| `storage.database.path` | `TIKEO__STORAGE__DATABASE__PATH` | SQLite 模式 | `.dev/tikeo-dev.db`；生产模板 `/data/tikeo.db` | SQLite 文件路径，容器中要持久化 `/data`。 |
| `storage.database.host` | `TIKEO__STORAGE__DATABASE__HOST` | 网络数据库 | 省略时 `127.0.0.1` | PostgreSQL/MySQL/CockroachDB host。 |
| `storage.database.port` | `TIKEO__STORAGE__DATABASE__PORT` | 否 | Postgres `5432`、MySQL `3306` | 网络数据库端口。 |
| `storage.database.username` | `TIKEO__STORAGE__DATABASE__USERNAME` | 网络数据库通常必填 | 未设置 | 数据库用户名。 |
| `storage.database.password` | `TIKEO__STORAGE__DATABASE__PASSWORD` | 网络数据库通常必填 | 未设置 | 数据库密码；特殊字符不需要人工 URL 转义。 |
| `storage.database.database` | `TIKEO__STORAGE__DATABASE__DATABASE` | 网络数据库 | 省略时 `tikeo` | 数据库/schema 名。 |
| `storage.database.params.*` | 建议放文件 | 否 | SQLite 参数为空时使用 `mode=rwc` | 查询参数，例如 `sslmode=disable`。 |
| `storage.timestamp_offset` | `TIKEO__STORAGE__TIMESTAMP_OFFSET` | 否 | `+00:00` | 写入 DB 时间戳时使用的 offset。 |
| `cluster.mode` | `TIKEO__CLUSTER__MODE` | 否 | `standalone` | `standalone` 或 `raft`；多 Pod Server HA 用 raft。 |
| `cluster.node_id` | `TIKEO__CLUSTER__NODE_ID` | Raft 必填 | `standalone` | 稳定节点 id；Kubernetes 中用 pod name。 |
| `cluster.peers` | `TIKEO__CLUSTER__PEERS` | Raft 必填 | `[]` | 静态 peer 列表；数组结构建议放文件/Helm values。 |
| `cluster.transport_token` | `TIKEO__CLUSTER__TRANSPORT_TOKEN` | Raft 必填 | 未设置 | 内部 Raft/relay 通信 token，放 Secret。 |
| `cluster.scheduler_shard_map_version` | `TIKEO__CLUSTER__SCHEDULER_SHARD_MAP_VERSION` | 否 | `1` | 调度 shard map 版本。 |
| `cluster.scheduler_shard_count` | `TIKEO__CLUSTER__SCHEDULER_SHARD_COUNT` | 否 | `64` | 逻辑调度 shard 数。 |
| `auth.local_login_enabled` | `TIKEO__AUTH__LOCAL_LOGIN_ENABLED` | 否 | `true` | 本地账号密码登录开关。 |
| `auth.api_tokens.*` | `TIKEO__AUTH__API_TOKENS__*` | 否 | `43200`/`300`/`2592000` | API token 默认、最小、最大 TTL。 |
| `auth.oidc.*` | `TIKEO__AUTH__OIDC__*` | 启用 OIDC 时 | disabled / 未设置 | OIDC issuer、client id、client secret、scopes。 |
| `transport_security.http.*` | `TIKEO__TRANSPORT_SECURITY__HTTP__*` | 启用时 | TLS/mTLS 关闭 | HTTP listener TLS/mTLS 与证书路径。 |
| `transport_security.worker_tunnel.*` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__*` | 启用时 | TLS/mTLS 关闭 | Worker Tunnel TLS/mTLS 与证书路径。 |
| `observability.logging.root.level` | `TIKEO__OBSERVABILITY__LOGGING__ROOT__LEVEL` | 否 | `info` | 未设置 `RUST_LOG` 时的根日志过滤级别。 |
| `observability.logging.http.*` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__*` | 否 | header/body 关闭，`65536` bytes | HTTP 访问/明细日志策略。INFO 只打印摘要；完整 header/body 还需要开启 `include_headers`/`include_body` 并让 `tikeo_server::http::trace` 达到 DEBUG。 |
| `observability.logging.sql.*` | `TIKEO__OBSERVABILITY__LOGGING__SQL__*` | 否 | 关闭，`DEBUG`，值关闭，`250ms` | SQL 执行日志策略。常规运行保持关闭；排查存储查询时在 DEBUG 开启。`include_values` 可能暴露敏感数据。 |
| `observability.logging.channels.console.*` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__*` | 否 | 启用，`info` | console/stdout sink。 |
| `observability.logging.channels.file.*` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__*` 或模板中的 `TIKEO_LOG_PATH` | 否 | 禁用，`info`，`/logs` | 非阻塞 JSON 文件日志 sink，写入 `tikeo.log`。 |
| `observability.logging.channels.error-file.*` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ERROR_FILE__*` 或模板中的 `TIKEO_LOG_PATH` | 否 | 禁用，`error`，`/logs` | 非阻塞 JSON 错误日志 sink，写入 `tikeo-error.log`。 |
| `observability.logging.channels.elk.*` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__*` | 否 | 禁用，topic `ivs-dev` | 非阻塞批量 JSON-lines 转发到配置的日志采集器。 |
| `observability.tracing.*` | `TIKEO__OBSERVABILITY__TRACING__*` | tracing 启用时 | disabled / 未设置 | OTLP trace 导出开关、endpoint、headers。 |
| `alert_retry.*` | `TIKEO__ALERT_RETRY__*` | 否 | 开启，`60`，`50`，`3`，`300` | Alert retry worker 配置。 |
| `notification_delivery.*` | `TIKEO__NOTIFICATION_DELIVERY__*` | 否 | 开启，恢复扫描 `60`，`50`，`3`，`300` | 通知中心通用投递 worker；新 attempt 会立即唤醒本进程投递 worker，`interval_seconds` 是恢复扫描兜底；卡片链接设置 `public_console_base_url`。 |
| `alert_secrets.allow_env_refs` | `TIKEO__ALERT_SECRETS__ALLOW_ENV_REFS` | 否 | `true` | 允许 `env:NAME` secret 引用。 |
| `alert_secrets.env_prefix` | `TIKEO__ALERT_SECRETS__ENV_PREFIX` | 否 | `TIKEO_ALERT_SECRET_` | 期望的 env secret 前缀。 |
| `script_governance.release_signature_secret_ref` | `TIKEO__SCRIPT_GOVERNANCE__RELEASE_SIGNATURE_SECRET_REF` | 启用签名门禁时 | 未设置 | 脚本发布签名校验用 `env:NAME` secret ref。 |

## 通知中心投递

`notification_delivery.*` 控制通知中心通用投递 worker。新的通知 attempt 会立即唤醒本进程投递 worker；`notification_delivery.interval_seconds` 是信号丢失、进程重启、HA handoff 和 retry 场景的恢复扫描兜底，不是正常通知的目标延迟。需要让供应商卡片回链控制台时，把 `notification_delivery.public_console_base_url` 设置为外部可访问的 Web URL。供应商凭据保存在每条渠道配置中；这里的 Server 配置只控制投递 worker 行为和公开链接 base。

## Worker 配置表

| 配置项 / SDK 字段 | 环境变量 | 是否必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `tikeo.worker.enabled` | `TIKEO_WORKER_ENABLED` | 否 | `true` | Spring Boot 自动配置开关。 |
| `tikeo.worker.auto-startup` | `TIKEO_WORKER_AUTO_STARTUP` | 否 | `true` | Spring Boot 生命周期自动启动开关。 |
| `endpoint` / `tikeo.worker.endpoint` | `TIKEO_WORKER_ENDPOINT` | 真实连接时必填 | demo 常用 `http://127.0.0.1:9998` | Worker Tunnel 地址。 |
| `dry-run` | `TIKEO_WORKER_DRY_RUN` | 否 | `false` | 不打开真实 Worker Tunnel。 |
| `heartbeatEvery` / `heartbeat-interval-millis` | `TIKEO_WORKER_HEARTBEAT_INTERVAL_MILLIS` | 否 | `10000` ms / `10s` | Worker lease 续约周期。 |
| `clientInstanceId` / `client-instance-id` | `TIKEO_WORKER_CLIENT_INSTANCE_ID` | 核心 SDK 必填；Boot 可空 | Boot 为空时生成并持久化 | 稳定客户端 hint。 |
| `state-dir` | `TIKEO_WORKER_STATE_DIR` | 否 | `~/.tikeo/workers` | client instance id 和沙箱工具缓存目录。 |
| `namespace` | `TIKEO_WORKER_NAMESPACE` | 否 | `default` | 命名空间。 |
| `app` | `TIKEO_WORKER_APP` | 否 | `default` | 应用 scope。 |
| `cluster` | `TIKEO_WORKER_CLUSTER` | 否 | Java Boot `default`；其他 helper `local` | Worker 集群/环境分片。 |
| `region` | `TIKEO_WORKER_REGION` | 否 | Java Boot `default`；其他 helper `local` | Worker region/zone。 |
| `name` | `TIKEO_WORKER_NAME` | 否 | 通常为 client instance id | 运维可见 worker 名。 |
| `version` | `TIKEO_WORKER_VERSION` | 否 | `dev` | Worker/应用构建版本。 |
| `capabilities` | `TIKEO_WORKER_CAPABILITIES` | 否 | `[]` | 旧式/运维 metadata。 |
| `labels` | `TIKEO_WORKER_LABELS` | 否 | `{}` | demo 用逗号分隔 `key=value`；Boot 用 map。 |
| `structured.normalProcessors` | `TIKEO_WORKER_NORMAL_PROCESSORS`（兼容环境变量名） | 否 | 随 demo 而定 | 可派发 normal processor。 |
| `structured.scriptRunners` | `TIKEO_WORKER_SCRIPT_LANGUAGES` / SDK API | 否 | 随 demo 而定 | 脚本语言与沙箱 backend。 |
| `election.*` | `TIKEO_WORKER_ELECTION_*` | 否 | enabled `true`、priority `100` | Worker-cluster master election 配置。 |
| `wasm.*` | `TIKEO_WORKER_WASM_*` | 否 | 后台预热、`latest`、`120000` | Wasmtime 自动安装配置。 |
| `scripts.*` | `TIKEO_WORKER_SCRIPTS_*` / `TIKEO_WORKER_SCRIPT_*` | 否 | 见 SDK 默认值 | 动态脚本、容器 runner、工具安装、镜像配置。 |

## 示例启动

```bash
cp config/tikeo.yml ./tikeo.yml
./target/release/tikeo serve --config ./tikeo.yml
```

Docker Compose 部署中，Tikeo 服务行为改 `config/tikeo.yml`，不要放到 Compose `environment`。
## 前置条件

- 先确认当前进程是 Server 还是 Worker；下面两个表格刻意分开。
- Server 部署要先选择 SQLite、PostgreSQL、MySQL 或 CockroachDB，并准备对应 `storage.database.*` 字段。
- Worker 部署把 SDK 配置放在业务应用配置中，不写入 Server `config/tikeo.yml`。

## 验收

配置变更后启动进程：Server 检查 `/readyz`，Worker 检查注册、心跳和能力声明。确认实际存储、TLS、日志、通知配置与目标环境一致。

## 故障排查

配置未生效时先检查加载顺序：默认值、配置文件、`TIKEO__...` 环境变量覆盖。`cluster.peers`、`storage.database.params` 这类数组或 map 优先写在文件里，避免 shell 转义错误。

## 生产检查清单

- [ ] 敏感值来自平台 Secret 或 secret reference，没有复制进公开示例。
- [ ] 使用结构化数据库字段，而不是手写带凭据的 URL。
- [ ] 启用 TLS/mTLS 时，证书路径指向 `/config/tls` 下的挂载文件。
- [ ] 已复核 Worker SDK 默认值，包括 endpoint、namespace、app、state-dir 和声明能力。

### Sandbox 工具后台预热策略

所有 SDK 对 SRT、Deno、ripgrep、Rhai、PowerShell、Wasmtime 等 sandbox 工具采用同一规则：

- 自动安装只是**后台预热**，不会阻塞 Worker 启动、Spring Boot Context 启动或 SDK Client 构造。
- 默认模式可以复用宿主 `PATH` 中可用的工具，但任务执行时仍使用 sandbox 的 `cwd`、`HOME`、`TMPDIR`、`DENO_DIR` 以及 PowerShell/.NET 缓存目录。
- 如需更强隔离，设置 `TIKEO_SANDBOX_STRICT_ISOLATION=1`（Java Boot：`tikeo.worker.scripts.strict-sandbox-isolation=true`）。开启后 SDK 会忽略宿主 `PATH` 工具和解释器，只使用 `TIKEO_SANDBOX_TOOLS_DIR` / `~/.tikeo/sandbox-tools` 中的托管二进制，未就绪时 fail-closed。
- 工具缺失时不会提前上报对应脚本能力；如果任务仍命中不可用运行器，会以 fail-closed 方式返回清晰诊断，而不是让业务进程崩溃。
- 后台安装失败只记录日志。可以手动填充 `TIKEO_SANDBOX_TOOLS_DIR` 或 `~/.tikeo/sandbox-tools/<tool>`，也可以重启 Worker 触发重试。
- 生产环境建议把所需工具预装到镜像，或挂载持久/只读工具缓存；自动安装主要用于本地 demo、CI smoke test 和受控镜像源。
- 生产宿主机和 Dockerfile 示例见 [Worker 沙箱工具与 Dockerfile](../deployment/worker-sandbox-tools)。
