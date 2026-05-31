# tikee

`tikee` 是一个 Rust workspace 模式开发的分布式任务调度平台。后端主入口在根目录 `src/main.rs`，核心模块拆分在 `crates/*`；Web 管理端在 `web/`，使用 React + Ant Design + Bun。

## 本地开发一键启动

```bash
./scripts/dev.sh
```

脚本会自动：

1. 使用 `config/dev.toml` 启动后端 HTTP API 与 Worker Tunnel。
2. 如 `web/node_modules` 不存在，自动执行 `bun install`。
3. 启动 Web dev server，并通过 Vite proxy 访问后端 API。
4. 在 `.dev/server.log` 与 `.dev/web.log` 写入运行日志。

默认访问地址：

- Web UI: <http://0.0.0.0:5173>
- Backend API: <http://0.0.0.0:9090>
- OpenAPI JSON: <http://0.0.0.0:9090/api-docs/openapi.json>

> 项目不提供浏览器 API 文档 UI；仅保留机器可读的 OpenAPI JSON。

## 初始化管理员

系统不再内置默认管理员账号。首次部署后打开 Web UI，会强制进入初始化管理员注册页；需填写用户名、邮箱、密码和确认密码。注册成功后会自动登录，并在 `users.bootstrap_admin` 中记录该账号来自一次性部署初始化流程。

初始化注册入口只在用户表为空时开放；创建首个管理员后立即关闭。后续管理员、操作员、查看者等账号只能由站内管理员在“用户管理”中手动创建。所有受保护 API 都必须先通过登录接口获取 `atk_` 会话 token。


## 开发联调数据

后端迁移完成后，可以一键写入本地联调样例数据：

```bash
./scripts/dev-seed.sh              # 默认写入 tikee-dev.db
./scripts/dev-seed.sh /path/to.db  # 或指定 SQLite 数据库文件
```

脚本会执行 `scripts/dev-seed.sql`，内容包括：

- `default` namespace 与 `observability-demo` app。
- `dev_operator` / `dev_viewer` 两个开发账号，密码为 `Tikee@2026!`；它们不会替代首次初始化管理员。
- API、fixed-rate、cron 三类任务样例。
- 一个 pending dispatch queue 样例、一个 succeeded instance 与日志样例。
- shell / python 两个 approved script 及 released version。
- 一个两节点 workflow 样例与审计日志。

SQL 使用稳定 id + upsert，可重复执行；仅用于本地开发联调，不作为生产初始化数据。

## 配置目录

配置文件统一放在 `config/`：

- `config/dev.toml`：本地开发配置，监听 `0.0.0.0:9090` / `0.0.0.0:9998`。
- `config/container.toml`：容器部署配置，监听 `0.0.0.0:9090` / `0.0.0.0:9998`。
- `config/postgres.toml`：PostgreSQL / CockroachDB 部署配置模板，默认使用 `postgres://...` URL。
- `config/raft.toml`：Raft 集群配置形状模板；当前只暴露 `mode/node_id/peers` 与不可调度状态，真实 Raft runtime 仍在后续阶段实现。

存储 URL 支持：

| 数据库 | URL 示例 | 说明 |
| --- | --- | --- |
| SQLite | `sqlite://tikee-dev.db?mode=rwc` | 本地开发默认 |
| MySQL | `mysql://user:pass@mysql:3306/tikee` | 生产可用 |
| PostgreSQL | `postgres://user:pass@postgres:5432/tikee` | Phase2 已启用 sqlx-postgres |
| CockroachDB | `postgres://root@cockroach:26257/tikee?sslmode=disable` | 复用 PostgreSQL wire protocol |

环境变量覆盖示例：

```bash
export TIKEE__STORAGE__DATABASE_URL="postgres://tikee:tikee@postgres:5432/tikee"
./target/debug/tikee serve --config config/postgres.toml
```

## Cluster / Raft 配置状态

当前已支持 `cluster` 配置段：

```toml
[cluster]
mode = "standalone" # 或 "raft"
node_id = "tikee-0"
peers = []
```

`mode = "raft"` 目前是安全前置形状：`/api/v1/cluster` 会返回 `mode=raft`、`role=unknown`、`can_schedule=false`，不会伪装 leader，也不会运行调度 ownership loop。真实 Raft membership / leader election / fencing token 将在后续阶段接入。

## 常用验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
(cd sdks/java && ./gradlew test)
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
```

## SDK 与 Demo 目录规范

- SDK 总目录：`sdks/<language>/<sdk-name>/`，例如 `sdks/rust/tikee/`、`sdks/java/tikee-spring-boot-starter/`。
- Demo 总目录：`examples/<language>/<demo-name>/`，例如 `examples/rust/worker-demo/`、`examples/java/spring-worker-demo/`。
- 每个已实现 SDK / Demo 都必须能在自身目录或通过显式 `-p` / `--manifest-path` 单独构建、测试、运行。
- 根 `Dockerfile` 只构建 tikee 服务端镜像，绝不复制、缓存或构建 `sdks/` 与 `examples/`。

当前独立验证示例：

```bash
cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
(cd sdks/java && ./gradlew test)
(cd sdks/java && ./gradlew :tikee-spring-boot-starter:test)
(cd examples/java/spring-worker-demo && ./gradlew test)
(cd examples/java/spring-worker-demo && TIKEE_WORKER_DRY_RUN=false TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 ./gradlew bootRun)
```

## Worker ID 注册约束

Worker 客户端不得自行指定权威 `worker_id`。客户端只能上报可选的 `client_instance_id` / labels / capabilities 等元信息；服务端注册成功后下发唯一 `worker_id`，后续 heartbeat、日志、结果上报都必须使用服务端下发的 ID。

生产部署中的 `client_instance_id` 应按 `namespace/app/cluster/region/client_instance_id` 形成稳定 Logical Worker：K8s StatefulSet 推荐 Pod 名/ordinal，Deployment 推荐 Pod UID 或 Pod 名，systemd/裸机推荐 `${service}@${host_id}#${slot}`。详细模板见 `docs/operations/worker-identity-bootstrap.md` 与 `deploy/worker/identity.env.example`。

## SDK 发布约束

每个语言 SDK 都必须能作为独立包发布：Rust SDK 不得依赖仓库内 `crates/*` path dependency；Java SDK 不得依赖服务端模块；后续 Go/Python/NodeJS SDK 也必须遵循各语言包管理器的独立发布规范。

## Docker / Compose

```bash
DOCKER_BUILDKIT=1 docker build -t tikee:dev .
DOCKER_BUILDKIT=1 docker build -t tikee-web:dev ./web
cp deploy/compose/tikee.env.example .env
docker compose --env-file .env up -d --no-build
./deploy/smoke/worker-bootstrap-smoke.sh
```

Docker/Compose 验证必须使用默认 bridge 网络，不使用 host 网络规避真实网络层问题。更多 Compose、systemd、裸机/VM bootstrap 步骤见 `deploy/README.md`。
