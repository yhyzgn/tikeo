# 会话日志

## 2026-05-19 — 设计阶段收尾与开发交接协议初始化

Agent:
- Codex

Work:
- 根据用户要求创建开发阶段总提示词 `prompt.md`。
- 初始化 `.memory/` 记忆库。
- 初始化 `.prompt/` 阶段提示词目录。
- 写入首个开发阶段提示词 `.prompt/001-bootstrap.md`。

Verification:
- 文件已创建并可读取。
- 尚未进行 Rust 编译/测试，因为代码工程尚未初始化。

Git:
- 本次任务结束前应提交并推送这些文档变更。


## 2026-05-19 — 固化 Rust workspace 与 Web 工程约束

Agent:
- Codex

Work:
- 将用户新增约束写入 `prompt.md`、`.memory`、`.prompt` 与设计文档。
- 明确 Rust 必须 workspace + `./crates/` 多 crate 解耦。
- 明确 Web 必须位于 `./web/`，使用 React + TypeScript + Ant Design + Bun。
- 强化每次代码改动后编译、测试、运行、提交、推送要求。

Verification:
- 文档约束 grep 校验。
- 本次为约束文档更新，尚无 Rust/Web 工程可编译。


## 2026-05-19 — 固化依赖最新版策略

Agent:
- Codex

Work:
- 将“各种依赖库尽量使用最新版”的约束写入总提示词、设计文档、记忆库和阶段提示词。
- 明确默认选择当前最新稳定版，不能使用最新版时需记录原因、锁定版本、风险与升级条件。

Verification:
- 文档约束 grep 校验。
- 本次为约束文档更新，尚无 Rust/Web 工程可编译。


## 2026-05-19 — 001-bootstrap Rust workspace 骨架完成

Agent:
- Codex

Work:
- 初始化 Cargo workspace，workspace members 限定在 `crates/*`。
- 新增 `scheduler-core`、`scheduler-config`、`scheduler-server` 三个 crate。
- 实现 `scheduler serve --config config/dev.toml`。
- 实现 Axum `/healthz` 与 `/readyz`。
- 增加配置加载、health handler 单元测试。
- 增加 `config/dev.toml`、`rustfmt.toml`、GitHub Actions CI。
- 更新下一阶段提示词 `.prompt/002-http-api-and-openapi.md`，新增 `.prompt/003-worker-tunnel.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` ✅
- `curl -fsS http://127.0.0.1:9090/healthz` ✅ returned `{"status":"ok","uptime_seconds":0}`
- `curl -fsS http://127.0.0.1:9090/readyz` ✅ returned `{"status":"ok","uptime_seconds":0}`

Git:
- 待提交并推送。


## 2026-05-19 — 调整后端主程序入口到根 src/main.rs

Agent:
- Codex

Work:
- 根据用户要求将后端主程序入口从 `crates/scheduler-server/src/main.rs` 移到根 `src/main.rs`。
- 根 package `scheduler` 只保留 binary entrypoint，实际 server 逻辑仍委托 `scheduler-server` crate。
- 更新 prompt、design、memory 与阶段提示词中的入口位置约束。

Verification:
- 调整后继续执行当前阶段完整验证。


## 2026-05-19 — 002-http-api-and-openapi 完成

Agent:
- Codex

Work:
- 增加 HTTP API 分层：DTO、error、OpenAPI、routes。
- 使用 `utoipa` 生成 OpenAPI JSON；禁止 API 文档 UI 禁用。
- 实现 `/api/v1/system/info`、`/api/v1/cluster`、`GET /api/v1/jobs`、`POST /api/v1/jobs` placeholder。
- 实现 Problem Details JSON 错误响应。
- 暴露 `/api-docs/openapi.json`；不提供文档 UI。
- 根据用户要求将后端主程序入口保留在根 `src/main.rs`，`crates/*` 继续承载解耦模块。
- 更新 `.prompt/003-worker-tunnel.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` ✅
- `GET /healthz` ✅
- `GET /readyz` ✅
- `GET /api-docs/openapi.json` ✅ contains `/api/v1/system/info` and `/api/v1/jobs`
- `GET /api/v1/system/info` ✅ returned scheduler metadata
- `GET /api/v1/cluster` ✅ returned standalone leader placeholder
- `GET /api/v1/jobs` ✅ returned empty page
- `POST /api/v1/jobs` ✅ returned 501 Problem Details placeholder

Git:
- 待提交并推送。


## 2026-05-19 — 固化并实现 HTTP 统一响应体

Agent:
- Codex

Work:
- 根据用户要求固化 HTTP 响应体规范：`code`、`message`、`data` 必须存在。
- 成功响应使用 `code=0`；失败响应使用非 0 code。
- 将现有 `/api/v1/system/info`、`/api/v1/cluster`、`/api/v1/jobs` 包装为统一 envelope。
- 将 `POST /api/v1/jobs` placeholder 错误改为统一 envelope，code=`10001`。
- 更新 OpenAPI schema、设计文档、总提示词和记忆库。

Verification:
- 继续执行当前阶段完整验证。


## 2026-05-19 — 路线图完成项回写

Agent:
- Codex

Work:
- 根据用户要求，将已完成工作项在 `design/scheduler-architecture-design.md` 开发路线图中标记为 `[x] ✅`。
- 固化后续开发完成项必须同步更新路线图的规则。

Verification:
- 继续执行当前阶段完整验证。


## 2026-05-19 — 调整路线图标记与 Java Starter SDK 规划

Agent:
- Codex

Work:
- 移除 `design` 开发路线图已完成项中的 ✅ 图标，仅保留 `[x]`。
- 固化后续路线图完成项只用 `[x]` 标记的规则。
- 补充 Java SDK 规划，优先支持 Spring Boot Starter 模式。

Verification:
- 继续执行文档和代码完整验证。


## 2026-05-19 — 003-worker-tunnel 完成

Agent:
- Codex

Work:
- 新增 `crates/scheduler-proto`，使用 tonic/prost 生成 Worker Tunnel gRPC bindings。
- 新增 `proto/scheduler/worker/v1/worker.proto` 作为仓库级协议源。
- 定义最小 Worker Tunnel 消息：RegisterWorker、Heartbeat、WorkerRegistered、Ping。
- 实现 server 侧 `WorkerTunnelService::Connect` skeleton。
- 实现内存 `WorkerRegistry`，记录 worker id、app、namespace、cluster、region、capabilities、labels 和 heartbeat sequence。
- server 启动时同时监听 HTTP `9090` 与 Worker Tunnel gRPC `9091`。
- 设计路线图中将 “gRPC 协议定义与代码生成” 标记为完成 `[x]`。
- 新增 `.prompt/004-storage-and-scheduler.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` ✅
- HTTP `/healthz` ✅
- OpenAPI `/api-docs/openapi.json` ✅
- Worker Tunnel TCP listener `127.0.0.1:9091` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 005-basic-scheduler

- `scheduler-core` 新增调度领域模型：`ScheduleType`、`TriggerType`、`InstanceStatus`、`DispatchDecision`。
- `scheduler-storage` 新增 `JobInstanceRepository`，支持创建 pending job instance、按 job 查询实例、按 id 查询实例。
- HTTP 新增 `POST /api/v1/jobs/{job}:trigger`，实现 API 手动触发并返回统一 `{code,message,data}` envelope。
- HTTP 新增 `GET /api/v1/jobs/{job}/instances` 与 `GET /api/v1/instances/{instance}`，支持实例列表与详情查询。
- OpenAPI schema 已补充 TriggerJobRequest、JobInstanceSummary、JobInstancePage。
- 设计路线图已将 API 手动触发实例链路作为基础调度器子项标记完成；CRON / Fixed Rate tick loop 仍待后续阶段。


## 2026-05-19 — 006-worker-sdk-rust-and-java-starter

- Worker Tunnel proto RPC 从 `Connect` 改为 `OpenTunnel`，解决 tonic client 生成方法名冲突。
- `scheduler-proto` 开启 tonic client 生成。
- 新增 `crates/scheduler-worker-sdk`，实现 Rust Worker SDK 最小主动连接、注册、心跳客户端。
- Rust Worker SDK 增加 `TaskProcessor` / `TaskContext` / `TaskOutcome` 基础处理器接口，为后续任务分发做准备。
- Rust Worker SDK 集成测试启动真实 tonic Worker Tunnel server，验证 register ack 与 heartbeat ping。
- 新增 `java/` Maven 多模块 SDK 骨架：`scheduler-java-core`、`scheduler-spring-boot-autoconfigure`、`scheduler-spring-boot-starter`。
- Java core 提供 `@SchedulerProcessor`、`WorkerRegistration`、`SchedulerWorkerClient`、`NoopSchedulerWorkerClient`。
- Spring Boot autoconfigure 提供 `scheduler.worker.*` 配置、auto-configuration imports 和注解扫描 registry。


## 2026-05-19 — 007-web-ui-foundation

- 新增 `web/` Bun 工程，技术栈为 React 19、TypeScript 6、Vite 8、Ant Design 6。
- 建立 AppShell、Dashboard、Jobs、Instances 页面骨架。
- Jobs 页面支持调用 API 创建 Job 与 API trigger；Instances 页面展示实例列表。
- 新增 typed API client，统一解析 `{code,message,data}` envelope。
- 新增 Bun test API client 单元测试，覆盖成功与业务失败分支。
- 建立 `lint`、`typecheck`、`test`、`build` 脚本并验证通过。


## 2026-05-19 — 008-container-deployment

- 新增后端多阶段 Dockerfile：Rust release builder + Debian slim runtime，默认运行 `scheduler serve --config /app/config/container.toml`。
- 新增 `config/container.toml`，容器内 HTTP `0.0.0.0:9090`、Worker Tunnel `0.0.0.0:9091`、SQLite dev 数据落 `/data/scheduler.db`。
- 新增 Web Dockerfile：Bun 构建 React/Ant Design 静态资源，nginx 托管并代理 `/api/`、`/api-docs/` 到 scheduler HTTP 服务。
- 新增 `docker-compose.yml`，包含 scheduler server 与 web 两个服务；Worker Tunnel 只暴露为 worker 主动出站连接入口。
- 新增 `deploy/k8s/scheduler.yaml` 与 README，包含 Namespace、ConfigMap、SQLite dev PVC、server Deployment/Service、worker tunnel Service、web Deployment/Service。
- 新增 Docker ignore 规则，避免 target、node_modules、dist 进入镜像构建上下文。
- 设计路线图已将 Docker 镜像构建标记完成；后续 Helm Chart 仍保留在 Phase 3。

Verification:
- `docker compose config` ✅
- `docker build -t scheduler:dev .` ✅
- `docker build -t scheduler-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web HTML + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅
- `deploy/k8s/scheduler.yaml` PyYAML 结构解析 ✅（8 documents；当前环境无 `kubectl`）
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 009-worker-dispatch

- Worker Tunnel proto 新增 `DispatchTask` 与 `TaskResult`，保留 `OpenTunnel` 双向流模型。
- Server registry 记录每个 worker 的 outbound stream sender，可向在线 worker 下发任务。
- 新增最小 dispatch loop：定期查询 pending job_instance，选择 first available worker，下发任务并把实例置为 running。
- Worker Tunnel service 接收 `TaskResult`，将实例状态更新为 succeeded 或 failed。
- `scheduler-worker-sdk` 新增 `WorkerSession::process_next`，接收 dispatch、构造 `TaskContext`、调用 `TaskProcessor`、回传 `TaskOutcome`。
- Storage 新增 pending instance 查询与 status update repository 方法。
- 测试覆盖 repository 状态流转、server dispatch、SDK dispatch -> processor -> result 回传。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` + `/healthz` + `/api/v1/jobs` smoke ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t scheduler:dev .` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅
- `docker build -t scheduler-web:dev ./web` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 010-scheduler-tick-loop

- 新增 `scheduler-server::scheduler` 自动调度 tick loop。
- Storage 新增 `list_enabled_scheduled_jobs`，只查询 enabled 的 `cron` / `fixed_rate` jobs。
- CRON 使用 `cron 0.16.0` 解析表达式，Fixed Rate 使用 `humantime 2.3.0` 解析持续时间表达式。
- Tick loop 使用内存 cursor 避免同一 tick 重复触发；到期时创建 pending job_instance，并复用 009 dispatch loop。
- Server 启动时同时运行 HTTP、Worker Tunnel、自动 scheduler tick loop 和 Worker dispatch loop。
- 测试覆盖 fixed_rate 到期触发、cron 到期触发、disabled scheduled job 不触发。
- 设计路线图已标记基础调度器 CRON/Fixed Rate/API 子项完成，Rust SDK 任务执行子项完成。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` + fixed_rate job 自动创建 pending instance smoke ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t scheduler:dev .` ✅
- `docker build -t scheduler-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 011-instance-logs

- Storage 新增 `job_instance_logs` 表、SeaORM entity、migration、append/list repository。
- Worker Tunnel proto 新增 `TaskLog`，Worker 继续通过主动建立的 `OpenTunnel` 回传日志。
- Server Tunnel service 持久化 Worker `TaskLog`，不引入 Worker 入站端口。
- Rust Worker SDK 新增 `WorkerSession::emit_log`。
- HTTP 新增 `GET /api/v1/instances/{instance}/logs`，返回统一 `{code,message,data}` envelope。
- OpenAPI 增加实例日志路径与 schema。
- Web API client 增加 `listInstanceLogs`；Instances 页面增加日志 Drawer 查看。
- 设计路线图已标记 Web 实例日志查看子项完成。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t scheduler:dev .` ✅
- `docker build -t scheduler-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 012-auth-rbac-foundation

- 新增后端开发管理员认证模块：`POST /api/v1/auth/login`、`GET /api/v1/auth/me`、`POST /api/v1/auth/logout`。
- 支持 env 覆盖开发管理员用户名、密码与 token：`SCHEDULER_DEV_ADMIN_USERNAME`、`SCHEDULER_DEV_ADMIN_PASSWORD`、`SCHEDULER_DEV_ADMIN_TOKEN`。
- `POST /api/v1/jobs` 与 `POST /api/v1/jobs/{job}:trigger` 增加 bearer token 校验；失败返回 401 且保持 `{code,message,data}` envelope。
- OpenAPI 增加 auth paths 与 schema。
- Web 新增登录页、token localStorage 持久化、登录恢复、退出入口；创建 Job 与触发 Job 自动携带 Authorization header。
- Web API client 增加 `login`、`me`、`logout`、`setAuthToken`、`getAuthToken`，并新增鉴权 header 测试。
- 设计路线图已标记基础 Web UI 与“登录与权限感知操作”完成。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` + `/healthz` + `/api/v1/auth/login` + `/api/v1/auth/me` + protected `POST /api/v1/jobs` smoke ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t scheduler:dev .` ✅
- `docker build -t scheduler-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/` smoke ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 013-broadcast-execution + bridge-safe containers

- Core 新增 `ExecutionMode::{single,broadcast}` 与 `InstanceStatus::partial_failed`。
- Storage 新增 `job_instances.execution_mode` 与 `job_instance_attempts`，并提供 attempt repository。
- HTTP `POST /api/v1/jobs/{job}:trigger` 支持 `execution_mode`；广播触发基于在线 Worker 创建子执行，并先校验在线 Worker 避免孤儿实例。
- HTTP 新增 `GET /api/v1/instances/{instance}/attempts`，继续返回统一 `{code,message,data}` envelope。
- Tunnel registry/dispatcher 支持按 worker id 发送任务；Worker 回传 `TaskResult` 后更新 attempt 并聚合父实例状态。
- Web Job 页面支持 single/broadcast 触发；Instances 页面新增广播 attempt Drawer。
- 移除浏览器 API 文档 UI 与相关依赖，只保留 `/api-docs/openapi.json` 机器可读契约。
- Backend Dockerfile 改为分层缓存构建 + musl release binary + Alpine runtime。
- Web Dockerfile/nginx 按分层构建与 nginx runtime 调整；Compose 使用默认 bridge 网络，Web 通过服务名 `scheduler` 反向代理后端。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `DOCKER_BUILDKIT=1 docker build -t scheduler:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t scheduler-web:dev ./web` ✅
- `docker compose up -d --no-build` on default bridge ✅
- `curl /healthz`, Web `/`, proxied `/api/v1/system/info`, proxied/direct `/api-docs/openapi.json` ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — dev startup script + config directory

- Renamed runtime configuration directory from `examples/` to `config/` and updated Dockerfile, Compose, prompt, memory, and design references.
- Added `scripts/dev.sh` to start backend + Web dev server together, wait for health checks, print browser/API URLs, and write logs under `.dev/`.
- Added root `README.md` with local startup instructions, configuration directory contract, and initialization credentials.
- Updated built-in development initialization account defaults to `scheduler_init` / `Scheduler@2026!` / `scheduler-init-token`; env overrides remain available.
- Preserved the manually edited Dockerfile and committed the new `.cargo/config.toml` it references for rsproxy cargo source configuration.

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `./scripts/dev.sh` startup smoke: backend health + Web dev server ready ✅
- `docker compose config` ✅
- `DOCKER_BUILDKIT=1 docker build -t scheduler:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t scheduler-web:dev ./web` ✅
- Compose bridge smoke: `/healthz` + Web `/` ✅


## 2026-05-19 — Web UI modernization + SQLite compatibility fix

- Fixed existing SQLite dev databases that had already applied the older initial migration before `job_instances.execution_mode` existed. Startup now runs a SQLite compatibility pass after SeaORM migrations to add `execution_mode` with default `single` and ensure `job_instance_attempts` plus indexes exist.
- Verified the reported endpoint now succeeds on the existing local DB: `GET /api/v1/jobs/job_019e3ec775b177b0bd1f804874c84f3c/instances` returns `{code:0,message:"success",data:{items:[],...}}`.
- Upgraded Web UI from plain Ant Design defaults to a light, simple, modern SaaS management style with clearer branding, hero overview, metric cards, intentional future-module menu entries, and cleaner table/card treatments.
- Menu note: only Dashboard / Jobs / Instances are active because those backend capabilities exist today; Worker 集群 / 安全策略 / 审计日志 are shown as disabled planning entries rather than fake pages.

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` + healthz/jobs/reported instances endpoint smoke ✅
- `./scripts/dev.sh` backend + Web startup smoke ✅

## 2026-05-20 — 接手用户管理并抽象 SessionStore

Agent:
- Codex

Work:
- 接手他人已开发的用户管理/RBAC 模块。
- 新增 `crates/scheduler-server/src/http/session.rs`，定义 `SessionStore` trait、`SessionManager` 和当前 `DbMokaSessionStore`。
- 将 HTTP auth/login/logout 与用户角色/密码变更 session 失效逻辑从内存 HashMap 改为 SessionStore。
- 新增/接入 `auth_sessions` 存储实体、repository 与 SQLite 兼容补表逻辑。
- 更新 `design/auth-session-design.md`，明确 DB+moka 当前方案与 Redis 分布式扩展方案。
- 更新开发路线图，标记用户管理/RBAC 与 SessionStore 抽象已完成。

Verification:
- `cargo fmt --all` ✅
- `cargo check --workspace --all-features` ✅
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅（保留 Vite 大 chunk warning）
- `docker compose config` ✅
- `cargo run --bin scheduler -- serve --config config/dev.toml` + `/healthz` + `/auth/login` 冒烟 ✅，登录返回 `atk_` opaque token。

## 2026-05-20 — 禁止外键与 session 过期物理删除

Agent:
- Codex

Work:
- 将“数据库全库禁止外键，只允许字段软关联”写入设计文档与记忆库。
- 移除 SeaORM migration 中所有 `foreign_key` 定义和 entity relation 声明。
- 为 SQLite 兼容层增加旧表重建逻辑，用无外键表结构替换已存在的外键表。
- 为 `AuthSessionRepository` 增加 `delete_expired`，并在 `DbMokaSessionStore` 创建/读取 session 前执行过期物理清理。
- 按用户要求将 `users.password_hash` 改为 `users.password`，字段内容仍保存 `BCrypt` hash，并为 SQLite 旧库增加列重命名兼容。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `mvn -f java/pom.xml -q test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅（保留 Vite 大 chunk warning）
- `docker compose config` ✅
- SQLite dev DB 外键检查 ✅ 无 `REFERENCES` 表定义
- SQLite `users` 表字段检查 ✅ 包含 `password`，不含 `password_hash`
- 本地 serve + `/healthz` + `/auth/login` 冒烟 ✅，登录返回 `atk_` token。

## 2026-05-20 — 016-dynamic-script-sandbox

Agent:
- Claude (GLM-5.1)

Work:
- `scheduler-core` 新增 `ScriptLanguage`（Shell/Python/Node/PowerShell/Rhai/Wasm）和 `ScriptStatus`（Draft/Approved/Disabled）枚举，含 `FromStr`/`Display`/`as_str()` 及测试。
- `scheduler-storage` 新增 `scripts` 表 SeaORM entity（id/name/language/version/content/status/timeout_seconds/max_memory_bytes/allow_network/allowed_env_vars/created_by/created_at/updated_at），无外键。
- Storage migration 新增 `create_scripts()` 与 status/name 索引，SQLite 兼容补表。
- Storage 新增 `ScriptRepository`（list/get/create/update/delete）与 `CreateScript`/`UpdateScript`/`ScriptSummary` 类型。
- HTTP 新增 5 个 Admin 权限保护端点：`GET /api/v1/scripts`、`POST /api/v1/scripts`、`GET /api/v1/scripts/{id}`、`PATCH /api/v1/scripts/{id}`、`DELETE /api/v1/scripts/{id}`。
- OpenAPI 补充 script paths 与 DTO schema。
- Web 新增脚本管理页面（ScriptsPage），含 Table、创建 Modal、行操作（approve/disable/delete）；AppShell 增加"脚本管理"菜单。
- Web API client 新增 `listScripts`/`createScript`/`getScript`/`updateScript`/`deleteScript`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅（5 tests）
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- 本地 serve + 登录 + Script CRUD 冒烟 ✅（Create/GET/PATCH status draft→approved/DELETE 全链路通过）

Git:
- 后端已提交为 `ff17519` 并推送。
- Web + memory 变更待本次一起提交推送。
