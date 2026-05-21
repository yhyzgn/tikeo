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
- `curl -fsS http://0.0.0.0:9090/healthz` ✅ returned `{"status":"ok","uptime_seconds":0}`
- `curl -fsS http://0.0.0.0:9090/readyz` ✅ returned `{"status":"ok","uptime_seconds":0}`

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
- server 启动时同时监听 HTTP `9090` 与 Worker Tunnel gRPC `9998`。
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
- Worker Tunnel TCP listener `0.0.0.0:9998` ✅

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
- 新增 `sdks/rust`，实现 Rust Worker SDK 最小主动连接、注册、心跳客户端。
- Rust Worker SDK 增加 `TaskProcessor` / `TaskContext` / `TaskOutcome` 基础处理器接口，为后续任务分发做准备。
- Rust Worker SDK 集成测试启动真实 tonic Worker Tunnel server，验证 register ack 与 heartbeat ping。
- 新增 `sdks/java/` Gradle 多模块 SDK 骨架：`scheduler-java`、`scheduler-spring`、`scheduler-spring-boot-starter`。
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
- 新增 `config/container.toml`，容器内 HTTP `0.0.0.0:9090`、Worker Tunnel `0.0.0.0:9998`、SQLite dev 数据落 `/data/scheduler.db`。
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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
- `./sdks/java/gradlew -p sdks/java test` ✅
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

## 2026-05-20 — 017-script-versioning-and-diff

Agent:
- Claude (GLM-5.1)

Work:
- Storage 新增 `script_versions` 表（id/script_id/version_number/content/language/status/timeout_seconds/max_memory_bytes/allow_network/allowed_env_vars/created_by/created_at），无外键。
- Storage migration 新增 `create_script_versions()` + script_id + version_number 索引，SQLite 兼容补表。
- Storage 新增 `ScriptVersionRepository`（create_version/list_versions/get_version）。
- `ScriptRepository::update_script` 更新前自动将当前行快照写入 `script_versions`。
- HTTP 新增 `GET /api/v1/scripts/{id}/versions`（版本历史列表）和 `GET /api/v1/scripts/{id}/diff?v1=&v2=`（diff 对比）。
- Diff 返回 unified content diff + policy 字段对比（FieldChange[]）。
- Web 新增 `ScriptVersionSummary`/`FieldChange`/`ScriptDiffResult` 类型和 `listScriptVersions`/`diffScriptVersions` API。
- Web ScriptsPage 新增版本历史 Drawer、v1/v2 选择、diff 对比视图（content diff +/- 颜色编码 + policy diff 表格）。
- Web 新增 `CodeEditor` 组件（CodeMirror 6），支持 Shell/Python/Node 语法高亮。
- ScriptsPage 创建 Modal 使用 CodeEditor 替代 textarea。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅（5 tests）
- `bun run --cwd web build` ✅

Git:
- 后端提交 `bb51a12`，Web 提交 `4921a30`。
- 设计路线图待更新，018 prompt 待创建。

## 2026-05-20 — 020-review-remediation 善后启动

- Review 015-019 后确认需要善后：静态 admin bearer 后门、明文 token 审计、Webhook SSRF、脚本版本语义、空指标端点、审计静默失败、Web lint/fmt 失败等。
- 新增 `.prompt/020-review-remediation.md` 作为阶段提示词，逐项列出问题、风险与整改方案。
- 020 阶段目标：先修安全阻断和质量门禁，再修脚本版本核心语义，补最小业务指标，并把设计路线图中“骨架完成”和“完整平台能力”区分清楚。

## 2026-05-20 — 020-review-remediation 善后完成

- 删除 `scheduler-init-token` 静态 admin bearer 后门；后端测试改为先通过初始化账号登录获取真实 `atk_` session token。
- login/logout 审计改为 token 脱敏标识，避免明文 Bearer token 写入 `audit_logs`；审计写入失败改为 `warn!`。
- Alert Webhook 增加 HTTPS-only、localhost/私网/link-local/metadata 拒绝和 5s timeout，降低 SSRF 风险。
- 脚本创建时写入初始版本；脚本更新在事务内写入更新后的不可变版本快照；diff API 改为按 `(script_id, version_number)` 精确查询；diff 输出改为带 header/hunk 的 LCS 结果。
- `/metrics` 接入最小 HTTP request count/latency 与 Worker connected/dispatch 指标。
- 修复 Rust fmt、Web lint/typecheck 问题；更新 README、020 prompt、路线图和记忆库。

验证：
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅（仍有 Vite 大 chunk 警告，后续路由级拆包处理）
- `docker compose config` ✅

## 2026-05-20 — 021 RBAC/service hardening 与模块拆分

- 用户确认 021 先做 RBAC/service hardening，Phase2 工作流与分布式顺延到 022。
- 新增 RBAC 软关联表设计与实体：`roles`、`permissions`、`role_permissions`，继续禁止数据库外键。
- session principal 增加 `permissions`，HTTP 鉴权新增 `require_permission(resource, action)`。
- Web 改为基于 `permissions` 的菜单/路由权限判断，并新增 403 页面。
- 后端大文件拆分：`crates/scheduler-storage/src/repository.rs` 拆成 `repository/*`；`crates/scheduler-server/src/http/routes.rs` 拆成 `routes/*`。

## 2026-05-20 — 022 Phase2 workflow/queue/event foundation

- 删除未跟踪根目录 `AGENTS.md`。
- 新增 workflow / workflow_node / workflow_edge / workflow_instance / workflow_node_instance 存储模型，继续无外键软关联。
- 新增 dispatch_queue 与 instance_events 表，提供 priority/run_after/status 与 SSE 事件流基础。
- 新增 Workflow API：create/list/detail/validate/run/instance detail/SSE stream。
- Web 新增 Workflows 菜单与 JSON 定义创建、校验、运行入口。

## 2026-05-20 — 023 Phase2 workflow visual/mapreduce continuation

- 完成 workflow executor 最小推进器：`advance_workflow` 根据节点 `succeeded/failed/skipped` 与边条件 `always/on_success/on_failure` 推进后继节点，更新 workflow instance 终态并写入 instance_events。
- Workflow definition 扩展 `job_id` / `child_workflow_id` / `map_items` 约束：job 必须显式 job_id，map/map_reduce 必须有 map_items，sub_workflow 必须有 child_workflow_id。
- HTTP 增加 `POST /api/v1/workflows/dry-run` 与 `POST /api/v1/workflow-instances/{id}/advance`，继续统一返回 `code/message/data`。
- Web Workflows 页面升级为浅色现代化编排台：JSON 创建、YAML 预览、dry-run、基础 DAG 可视化、实例节点状态着色、SSE 事件流、手动推进 queued 节点。
- SSE endpoint now also accepts `?token=` by translating it into a Bearer header server-side because browser EventSource cannot set custom Authorization headers; normal APIs still use Authorization.

## 2026-05-20 — 024 Phase2 distributed worker/recovery slice

- Workflow queued node 与执行链路打通：`materialize_next_queued_node` 可把 job 节点生成 job_instance + dispatch_queue，把 map/map_reduce 节点生成 workflow_shards，把 sub_workflow 节点生成 child workflow_instance 软关联。
- 新增 workflow_shards 表；workflow_node_instances 增加 child_workflow_instance_id，继续无外键，仅软关联。
- 新增恢复 API：`POST /api/v1/workflow-instances/{id}/recover`，支持 retry/skip/fail/succeed 最小语义。
- 新增 Worker/队列管理 API：`GET /api/v1/workers`、`GET /api/v1/dispatch-queue`，Web 新增 Worker 集群页面。
- Dispatcher loop 每轮尝试 materialize 一个 queued workflow node，再走既有 job/broadcast dispatch。

## 2026-05-20 — Workflow visual drag editor quick upgrade

- Workflows page now defaults to visual mode and supports drag-and-drop node reordering, quick add Job/Map/MapReduce/SubWorkflow nodes, edge editing via selects, edge/node deletion, and live sync back to JSON definition.
- No heavy canvas dependency was added; this is a lightweight Ant Design + native HTML5 drag/drop editor suitable as an immediate usability improvement before a full graph canvas library.

## 2026-05-20 — Blender-like workflow node canvas

- Workflow visual editor upgraded from list/card layout to node-canvas style: grid canvas, absolute-positioned nodes, left input/right output ports, SVG Bézier arrow edges, port-click connection flow, and free node dragging with coordinates stored under node.config.ui.
- JSON remains canonical; visual coordinates and edges are synced back into WorkflowDefinition.

## 2026-05-20 — Workflow canvas layout/connection fixes

- Removed the wide left definition panel from Workflows page; creation now only needs inline name + create button, and JSON/YAML/Dry-run are canvas card actions.
- Fixed YAML preview by deriving YAML from current parsed JSON without replacing canonical draft state.
- Fixed node port connection by stopping pointer propagation on ports, and added per-node-type in/out connection limits shown on each node.

## 2026-05-20 — Workflow port linking reliability fix

- Fixed workflow canvas linking reliability by triggering port actions on pointerdown as well as click, enlarging the invisible port hit area, cancelling drag state during linking, and highlighting the source node while linking.
- Temporarily relaxed per-type port limits to 8 in/out for all node types to avoid UX-blocking false negatives while the exact product semantics are still evolving.

### 2026-05-20 17:28 工作流节点画布连线交互
- 用户要求像 Blender 节点编辑器一样：鼠标靠近节点边缘显示输入/输出端口，从输出端口按住拖出箭头，释放到下一个节点输入端口完成连线。
- 已在 `web/src/pages/WorkflowsPage.tsx` 使用 canvas-local pointer 坐标实现拖拽节点与拖拽连线，避免滚动画布/视口坐标偏移。
- 已在 `web/src/styles.css` 隐藏默认端口，仅 hover/连线态显示端口，并增加临时连线虚线和阴影。
- 已在 `crates/scheduler-storage/src/repository/workflow.rs` 扩展 workflow 节点类型白名单，避免 Web 新节点保存时报 unsupported node kind。

### 2026-05-20 027 工作流节点业务语义补齐
- 用户指出工作流节点必须能绑定任务，且条件分支、并行、人工审批等节点类型需要真实出现在编辑器中。
- 已让 Workflows 页面加载 Job 列表，Job 节点可在属性面板绑定具体任务 ID。
- 画布节点点击后显示属性 Inspector，不同节点类型展示不同配置项；这些配置写回 workflow definition JSON。
- 注意：当前 runtime 对新语义节点仍以定义/校验/人工推进为主，后续需要继续补齐 condition/parallel/join/delay/approval/notification 的自动执行语义。

### 2026-05-20 028 工作流边编辑交互
- 用户要求删除“连接最后两个节点”按钮，并支持点击箭头线条选中、拖动两端调整连接对象。
- 已删除快捷连接按钮和未使用 addEdge 逻辑。
- 已为 SVG 边增加可点击透明命中层、选中态样式、两端可拖拽 handle，以及重连时的临时箭头预览。

### 2026-05-20 029 工作流边关系内联配置
- 用户询问“边关系”是否能直接在线条上配置。
- 已将边条件配置迁移到选中线条附近的浮层，删除原底部边关系列表 UI。
- 选中边仍显示端点 handle；浮层支持修改 condition 和删除边。

### 2026-05-20 030 工作流边条件语义化
- 用户要求边条件候选应取决于前置节点类型，并且有默认值；不同 item 线条颜色不同，空白点击关闭弹窗，线条弱显示关系文本。
- 已新增按 from 节点 kind 生成的边条件选项、默认条件和颜色 meta。
- 新增 SVG text label 显示当前 condition；选中边 label 更明显，未选中弱显示。
- 画布空白点击会清空 selectedEdgeIndex，关闭边关系浮层。

### 2026-05-20 031 边端点拖拽回归修复
- 用户反馈线条点击后只能编辑条件，无法再拖动两端。
- 根因是 SVG 边 handle 处在节点卡片/浮层下方或命中层不稳定。
- 已新增 `.workflow-edge-rehandle` 绝对定位按钮层，z-index 高于节点和弹窗，按选中边端点坐标显示，pointerdown 直接进入重连模式。

### 2026-05-20 032 地址与端口统一
- 用户要求普通节点默认边关系为 always，并将项目中的 127.0.0.1 统一为 0.0.0.0、9091 统一为 9998。
- 已排除 `.git`、`target`、`node_modules`、`dist`、`.omx/logs`、`.dev` 日志后全项目替换并复查无残留。
- 注意：`0.0.0.0` 适合作为监听/容器绑定地址；脚本 smoke 也按用户约束使用该地址进行健康检查。

### 2026-05-20 033 工作流页面布局重构
- 用户要求一级页面只放工作流列表，运行视图和实例事件流默认不展示，改为 item 操作栏按钮触发手风琴展示；新增/编辑单独页面和路由。
- 已新增 WorkflowEditorPage，并在 App 路由中加入 `/workflows/new`、`/workflows/:id/edit`。
- WorkflowsPage 只负责列表、校验、运行、展开运行视图/事件流。
- 为编辑保存补齐后端 update_workflow repository + HTTP PATCH + client updateWorkflow/getWorkflow。

### 2026-05-20 034 工作流运行交互术语修正
- 用户询问“物化下一节点”含义，并反馈当前 404。
- 已把按钮改成产品化文案“准备下一节点执行”，404 空队列场景改为 info 提示。
- 当前后端语义仍是 queued workflow node -> job_instance/shards/subworkflow instance 的准备执行步骤。

### 2026-05-20 035 工作流运行视图内联化
- 用户指出“点击该条目的运行视图按钮展开详情”没有意义，且 `运行视图 · test` 这类 Collapse header 会在 item 多时制造混乱。
- 已移除 WorkflowsPage 的 AntD Collapse 依赖与全局折叠项，改为在 selected workflow list item 下直接渲染运行视图和事件流。
- 保留单条展开的 accordion-like 行为；切换条目时清理旧 activeInstance/events/shards，运行工作流后自动展开对应条目。

### 2026-05-20 036 工作流二级页面返回按钮
- 用户要求进入二级页面后增加返回按钮。
- 已在 WorkflowEditorPage hero 顶部新增“← 返回工作流列表”按钮，并移除 Card extra 中原有的重复返回入口。
- 验证通过前后端全量 lint/typecheck/test/build 与 dev.sh 启动烟测。

### 2026-05-20 037 列表运行视图禁止编辑
- 用户指出列表页面展开的运行视图应该禁止编辑节点和线条。
- 已让 DagPreview 在非 editable 模式下不渲染端口、不渲染 edge hit path、不显示线条配置弹窗和重连 handle，节点点击也不会进入选中编辑态。
- 编辑页继续传入 `editable`，不影响工作流创建/编辑画布。

### 2026-05-20 038 Worker TaskResult 自动推进 Workflow
- 用户要求继续下一阶段开发；按 `.prompt/025-phase2-workflow-worker-results-and-streaming.md` 先实现最关键的 worker result -> workflow node -> DAG advance 链路。
- `WorkflowRepository::complete_job_node_from_result` 通过 job_instance_id 软关联查找 workflow_node_instance，更新节点终态并自动 advance 后继节点，同时把对应 job dispatch_queue 标记 done/failed。
- Worker Tunnel 注入 WorkflowRepository，TaskResult 单实例分支会调用自动推进；broadcast parent 刷新路径保持原逻辑。
- dispatch_queue 增加 lease_owner/lease_until 最小 schema，后续 030 继续做原子 claim、shard dispatch 与 child workflow 回写。

### 2026-05-20 039 工作流审计日志补漏
- 用户指出工作流模块的操作没有记录到审计日志。
- 已在 workflow routes 中接入 common::audit，覆盖 create/update/validate/dry-run/run/advance/materialize/recover。
- 读操作 list/get/shards/events 暂不记录，避免高频读取刷屏；管理和执行类动作全部记录。

### 2026-05-20 040 SDK 目录统一
- 用户要求所有 SDK 包统一放到 `./sdks`。
- 已迁移 Rust Worker SDK 到 `sdks/rust/scheduler-worker-sdk`，Java 多模块 SDK 到 `sdks/java`。
- 根 Cargo workspace 已恢复为仅包含服务端与 `crates/*`；Rust SDK 独立于根 workspace 构建发布。
- Dockerfile 分层缓存、README、.gitignore、历史 prompt/memory 验证命令和设计文档结构图已同步到新目录。

### 2026-05-21 041 Dispatch Queue Claim/Lease
- 按 030 阶段继续推进队列多节点竞争基础能力。
- `WorkflowRepository` 新增 `claim_next_dispatch_queue_item`、`claim_dispatch_queue_item`、`release_dispatch_queue_item`，使用 lease_owner / lease_until 控制可占用性。
- HTTP 新增 `POST /api/v1/dispatch-queue:claim`，记录 `dispatch_queue` 的 `claim` 审计日志。
- 当前仍是最小实现；下一步建议继续把 dispatcher 的 materialize/dispatch 流程切到 claim API/原子条件更新路径，并补 visibility-timeout 回收。

### 2026-05-21 042 dev.sh 本地访问 URL 调整
- 用户手动修改 `scripts/dev.sh` 后要求代提交。
- 变更保留容器/服务绑定可配置性，默认 API_URL 改为 `http://localhost:$SCHEDULER_API_PORT`，WEB_URL 改为独立可覆盖的 `SCHEDULER_WEB_URL`。
- 烟测显示前后端均可启动：Web `http://localhost:5173`，Backend `http://localhost:9090`。

### 2026-05-21 043 dispatch queue 原子 claim 善后
- 按 030 阶段继续强化 dispatch_queue：补齐 repository 条件更新 claim、job queue claim、workflow-node queue claim、mark running、expired lease cleanup。
- 单实例 CreateJobInstance 现在事务内同时写入 dispatch_queue；broadcast 仍走 attempts 旧路径。
- Worker Tunnel dispatcher 已切到 dispatch_queue claim/lease 路径，避免直接 list pending instances 带来的多 server 重复派发风险。
- 已更新设计路线图 Phase 2，新增并完成“Dispatch queue 原子 claim 与 dispatcher 接入”。

### 2026-05-21 044 workflow shard / child workflow callback
- 继续 032 阶段：补齐 shard job_instance 软关联、shard complete API、shard 聚合推进、worker TaskResult -> shard 回写，以及 child workflow terminal -> parent node 回写。
- SQLite 兼容迁移会为既有 workflow_shards 增加 `job_instance_id` 列；全库仍无外键。
- 后续建议继续做 shard retry 策略、reduce 节点输入汇总以及 UI 上的 shard 完成/输出查看交互。

### 2026-05-21 045 SDK/examples 规范规划
- 用户要求 `sdks` 下按语言子目录存放 SDK，Java 改用 Gradle 且支持 JDK21+，并新增与 SDK 语言结构对齐的 `examples` demo 项目目录。
- 用户进一步要求后续开发过程中由 AI 在适当时候自行创建 demo 调试；已写入 design 与 memory 决策。
- 当前仅规划，不迁移代码；下一阶段应执行目录迁移、Java Gradle 化、examples 骨架创建和验证命令替换。

### 2026-05-21 046 SDK layout / Gradle migration
- 执行用户要求的 SDK 目录整改：`sdks/rust/scheduler-worker-sdk`、`sdks/java/<sdk-name>`。
- Java 构建从 Maven 切换到 Gradle Kotlin DSL + JDK21+；根 `gradlew` 会按需下载 Gradle。
- 创建 examples `<language>/<demo-name>` 目录骨架，仅放 demo/README，不放运行配置。
### 2026-05-21 SDK layout correction follow-up
- 用户明确根 `Dockerfile` 只构建 scheduler 服务端；已约束不得复制/缓存/构建 `sdks/` 或 `examples/`。
- SDK 路径规范固定为 `sdks/<language>/<sdk-name>/`，Demo 路径规范固定为 `examples/<language>/<demo-name>/`。
- Rust SDK 路径为 `sdks/rust/scheduler-worker-sdk`；现已移除 repo-local path dependencies，满足独立发布约束。
- 已补齐可独立运行的 Rust demo（`examples/rust/worker-demo`）与 Java Spring Boot demo（`examples/java/spring-worker-demo`）基础。

### 2026-05-21 verification — SDK layout correction
- `./sdks/java/gradlew -p sdks/java test` ✅
- `./sdks/java/gradlew -p examples/java/spring-worker-demo test` ✅
- `cargo fmt --all -- --check` ✅
- `cargo test --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-features` ✅
- `cargo run --manifest-path examples/rust/worker-demo/Cargo.toml` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web lint && bun run --cwd web typecheck && bun test --cwd web && bun run --cwd web build` ✅
- `DOCKER_BUILDKIT=1 docker build -t scheduler:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t scheduler:dev .` ✅ after switching builder/runtime flow to Alpine-compatible server-only image build.
- `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features` ✅ rerun after Dockerfile/dependency adjustments.

### 2026-05-21 Rust SDK independent publishing cleanup
- Removed `sdks/rust/scheduler-worker-sdk` from root Cargo workspace and removed Dockerfile rewrite workaround.
- Made Rust SDK self-contained by bundling `proto/worker.proto`, local `build.rs`, and removing all `../../../crates/*` path dependencies.
- Replaced SDK integration tests with an in-crate mock Worker Tunnel server.
- `cargo clippy --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `cargo package --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --allow-dirty` ✅ proves Rust SDK package has no repo-local path dependencies.

### 2026-05-21 Worker identity assignment cleanup
- Changed Worker Tunnel RegisterWorker payload from client-supplied `worker_id` to optional `client_instance_id`.
- Server registry now generates authoritative `wrk-*` worker ids and returns them in `WorkerRegistered`.
- Rust SDK stores server-assigned worker id after connect and uses it for heartbeat/log/result messages.

### 2026-05-21 verification — worker identity assignment cleanup
- `cargo test --workspace --all-features` ✅
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-features` ✅
- `cargo clippy --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `cargo package --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --allow-dirty` ✅
- `./sdks/java/gradlew -p sdks/java test` attempted first but Gradle distribution download hit `curl: (56) OpenSSL SSL_read ... unexpected eof while reading`.
- `~/.gradle/wrapper/dists/gradle-8.14-bin/.../bin/gradle -p sdks/java test` ✅ using cached Gradle.
- `~/.gradle/wrapper/dists/gradle-8.14-bin/.../bin/gradle -p examples/java/spring-worker-demo test` ✅ using cached Gradle.

### 2026-05-21 Java SDK Worker Tunnel implementation
- Implemented `GrpcSchedulerWorkerClient` with server-assigned worker id registration semantics, heartbeat, task log emission, and dispatch result reporting.
- Added Java protobuf generation from bundled `worker.proto` in Java core SDK.
- Wired Spring Boot auto-configuration to create real gRPC client unless `scheduler.worker.dry-run=true`.
- Updated Java Spring demo to default dry-run and smoke-run without live scheduler.

### 2026-05-21 Java SchedulerProcessor adapter
- Implemented invocable Spring processor handlers and `SpringSchedulerTaskProcessor`.
- Wired Spring Boot autoconfiguration so live Java gRPC Worker Tunnel dispatches route to annotated processor methods.
- Added tests for context/string method invocation, exception failure mapping, duplicate processor rejection, route-by-job-id convention, and autoconfig registry wiring.

### 2026-05-21 Java Lombok/style adjustment
- Added Lombok to Java SDK and Java demo builds.
- Converted demo runner to constructor-injected component and simplified Spring worker properties / dry-run client boilerplate with Lombok.

### 2026-05-21 Java SDK three-module restructure
- Renamed Java native SDK module to `scheduler-java`.
- Split Spring Framework adapter into `scheduler-spring`.
- Moved Spring Boot auto-configuration/properties into `scheduler-spring-boot-starter` and updated AutoConfiguration imports.
- Updated Java demo and docs to use `scheduler-spring-boot-starter`.

### 2026-05-21 Java Spring Boot starter naming correction
- Renamed `sdks/java/scheduler-spring-boot` to `sdks/java/scheduler-spring-boot-starter`.
- Updated Gradle settings/build, Java demo dependency, README/design/prompt/memory references.

### 2026-05-21 Worker processor key protocol
- Added explicit `processor_name` to DispatchTask across server/Rust SDK/Java SDK protocol copies.
- Updated server dispatch task construction and tests to assert processor key population.
- Updated Rust SDK TaskContext and Java TaskContext to carry processor name.
- Updated SpringSchedulerTaskProcessor to route by explicit processor name instead of job id fallback-only convention.

### 2026-05-21 Job/Workflow processor binding model
- Implemented first-class `processor_name` on jobs and workflow node specs.
- Server dispatch now resolves processor key from workflow node override, then job binding, then legacy job id.
- Added HTTP test for job processor binding and dispatcher test for workflow node override.
- Web Jobs page and Workflow DAG inspector can configure processor names.

### 2026-05-21 Go/Python SDK deferral
- User explicitly moved Go SDK + Python SDK out of Phase 2 and into Phase 4.
- Current Phase 2 continuation target is realtime task log streaming over gRPC server stream.

### 2026-05-21 Phase2 realtime task log stream
- Worker Tunnel proto now exposes `SubscribeTaskLogs(SubscribeTaskLogsRequest) returns (stream TaskLog)`.
- Server replays persisted `job_instance_logs` after a requested sequence and then streams live TaskLog records via an in-memory broadcast fan-out after successful DB append.
- Go/Python SDK remains deferred to Phase4 per user instruction.

### 2026-05-21 Phase2 PostgreSQL/CockroachDB storage support
- Enabled `sqlx-postgres` on `scheduler-storage` and migrations so PostgreSQL URLs compile through SeaORM/sqlx.
- Added `config/postgres.toml` with PostgreSQL and CockroachDB URL examples; CockroachDB uses PostgreSQL wire protocol.
- Roadmap marks PostgreSQL + CockroachDB storage support complete at driver/config/template level; live DB smoke remains environment-dependent.

### 2026-05-21 Phase2 cluster coordinator foundation
- Added `scheduler-server::cluster` with ClusterCoordinator trait, explicit ClusterMode/ClusterRole, and StandaloneCoordinator.
- `/api/v1/cluster` now reports `role=standalone` with node_id/can_schedule/detail instead of fake `leader`.
- Design now records Raft implementation boundaries: leader ownership gate, follower fencing, DB claim as final idempotency guard, and container-friendly networking.

### 2026-05-21 Phase2 cluster ownership gates
- Scheduler tick loop and Worker dispatcher loop now consult `ClusterCoordinator` status before ownership-sensitive work.
- Standalone remains schedulable; mock Raft follower tests prove tick and dispatch skip work when `can_schedule=false`.
- dispatch_queue DB conditional claim remains in place as final idempotency/fencing guard.

### 2026-05-21 Phase2 Raft config shape
- Added `[cluster]` config with `mode`, `node_id`, and static `peers` shape.
- Server now builds ClusterCoordinator from config: standalone can schedule; raft mode reports unknown/not-schedulable until real consensus starts.
- Added `config/raft.toml` as a safe template; no fake leader behavior introduced.

### 2026-05-21 Phase2 Raft metadata persistence
- Checked crates.io on 2026-05-21: OpenRaft alpha/prerelease conclusion was superseded by user direction to use TiKV raft-rs; real runtime adoption remains gated on event-loop/transport/persistence/fencing work.
- Added `raft_metadata` and `raft_members` storage tables with no foreign keys; IDs remain soft-linked.
- Raft startup now persists local metadata and configured peers, but cluster status remains unknown/not-schedulable until real consensus produces leadership.

### 2026-05-21 Phase2 Raft transport/fencing shape
- Added leader fencing token field shape to cluster status and `raft_metadata`; placeholder/config paths keep it null.
- Added reserved `/api/v1/raft/append-entries` HTTP transport endpoint for Docker/K8s/LB-safe node-to-node wiring; it returns `accepted=false` until real consensus runtime exists.
- Kept current storage-backed no-op coordinator in `scheduler-server::cluster`; no new `scheduler-cluster` crate yet because runtime boundaries are not stable enough.

### 2026-05-21 Phase2 cluster diagnostics
- Added `/api/v1/cluster/diagnostics` for operator-visible cluster readiness: current status, scheduling gate, persisted Raft metadata, members, transport placeholder, and runtime boundary.
- Chose a separate diagnostics endpoint instead of bloating `/api/v1/cluster`; the lightweight status endpoint stays stable for UI polling.
- Kept cluster runtime in `scheduler-server::cluster` for now; no `scheduler-cluster` crate until consensus/runtime traits stabilize.

### 2026-05-21 Phase2 dispatch queue fencing token
- Reviewed Phase2: only full Raft runtime remains incomplete; Go/Python SDK stays Phase4.
- Added `dispatch_queue.fencing_token` shape and SQLite compatibility migration; claim responses now include a fencing token.
- Dispatcher now derives a fencing token from ClusterCoordinator status (`standalone:<node>:scheduler-dispatcher` today, future `raft:<node>:<leader-token>` when real consensus exists).

### 2026-05-21 Phase2 closeout / Phase3 audit paging
- Consensus dependency direction corrected to TiKV raft-rs (`raft` 0.7.0); full Raft scheduling still stays gated until event-loop/transport/persistence/fencing are real.
- Phase2 distributed safety foundations are documented as complete except real Raft runtime/membership.
- Started Phase3 audit governance by adding server-side audit filters and pagination plus Web UI filter controls.

### 2026-05-21 Phase2 raft-rs correction
- User corrected the OpenRaft direction; project now targets TiKV raft-rs (`raft` crate 0.7.0, Apache-2.0) instead of OpenRaft.
- Added `scheduler-server::cluster::raft_rs` bootstrap validation: deterministic string `node_id` -> non-zero u64 raft id, peer voters, `MemStorage + RawNode` construction. This proves dependency/API integration only; no tick loop, campaign, leader token, or scheduling grant exists yet.
- `mode=raft` remains `role=unknown`, `can_schedule=false`, `leader_fencing_token=null` until real raft-rs leadership/fencing is implemented.

### 2026-05-21 Phase2 raft-rs durable records and wire shape
- Added `raft_log_entries` and `raft_snapshots` tables/entities/repository helpers as no-FK durable foundations for future raft-rs Ready log/snapshot persistence.
- Updated reserved `/api/v1/raft/append-entries` request DTO to carry raft-rs message-like fields (`from/to/term/message_type/index/log_term/commit/entries/context/reject`) while still returning `accepted=false`; no consensus state mutation or leader grant yet.
- Next safe slice: implement event loop + Ready persistence/application and only derive scheduling ownership from real raft-rs leadership plus persisted fencing token.

### 2026-05-21 Phase2 raft-rs message conversion
- Added route-local conversion from the reserved Raft HTTP DTO into raft-rs `eraftpb::Message`, including message/entry type allowlists, non-negative term/index validation, and base64 decoding for message/entry payloads.
- Endpoint still returns `accepted=false` and does not enqueue/step the message; this only validates wire compatibility before the runtime loop exists.

### 2026-05-21 Phase2 raft-rs runtime ticker skeleton
- `coordinator_from_config_with_storage` now starts a `RaftRuntimeCoordinator` for `mode=raft` when bootstrap succeeds. It drives `RawNode::tick()` on a 100ms loop and processes Ready in safe order: HardState metadata, entries, snapshot, then `advance()`.
- Runtime does not campaign, does not wire outbound transport, and still keeps `can_schedule=false` and `leader_fencing_token=null`; scheduler ownership remains fenced.
- Next slice: connect validated inbound HTTP messages to the runtime inbox, then implement Ready apply/outbound transport and real leader fencing.
