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
- 新增 `tikeo-core`、`tikeo-config`、`tikeo-server` 三个 crate。
- 实现 `tikeo serve --config config/dev.toml`。
- 实现 Axum `/healthz` 与 `/readyz`。
- 增加配置加载、health handler 单元测试。
- 增加 `config/dev.toml`、`rustfmt.toml`、GitHub Actions CI。
- 更新下一阶段提示词 `.prompt/002-http-api-and-openapi.md`，新增 `.prompt/003-worker-tunnel.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikeo -- serve --config config/dev.toml` ✅
- `curl -fsS http://0.0.0.0:9090/healthz` ✅ returned `{"status":"ok","uptime_seconds":0}`
- `curl -fsS http://0.0.0.0:9090/readyz` ✅ returned `{"status":"ok","uptime_seconds":0}`

Git:
- 待提交并推送。


## 2026-05-19 — 调整后端主程序入口到根 src/main.rs

Agent:
- Codex

Work:
- 根据用户要求将后端主程序入口从 `crates/tikeo-server/src/main.rs` 移到根 `src/main.rs`。
- 根 package `tikeo` 只保留 binary entrypoint，实际 server 逻辑仍委托 `tikeo-server` crate。
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
- `cargo run --bin tikeo -- serve --config config/dev.toml` ✅
- `GET /healthz` ✅
- `GET /readyz` ✅
- `GET /api-docs/openapi.json` ✅ contains `/api/v1/system/info` and `/api/v1/jobs`
- `GET /api/v1/system/info` ✅ returned tikeo metadata
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
- 根据用户要求，将已完成工作项在 `design/tikeo-architecture-design.md` 开发路线图中标记为 `[x] ✅`。
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
- 新增 `crates/tikeo-proto`，使用 tonic/prost 生成 Worker Tunnel gRPC bindings。
- 新增 `proto/tikeo/worker/v1/worker.proto` 作为仓库级协议源。
- 定义最小 Worker Tunnel 消息：RegisterWorker、Heartbeat、WorkerRegistered、Ping。
- 实现 server 侧 `WorkerTunnelService::Connect` skeleton。
- 实现内存 `WorkerRegistry`，记录 worker id、app、namespace、cluster、region、capabilities、labels 和 heartbeat sequence。
- server 启动时同时监听 HTTP `9090` 与 Worker Tunnel gRPC `9998`。
- 设计路线图中将 “gRPC 协议定义与代码生成” 标记为完成 `[x]`。
- 新增 `.prompt/004-storage-and-tikeo.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikeo -- serve --config config/dev.toml` ✅
- HTTP `/healthz` ✅
- OpenAPI `/api-docs/openapi.json` ✅
- Worker Tunnel TCP listener `0.0.0.0:9998` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 005-basic-tikeo

- `tikeo-core` 新增调度领域模型：`ScheduleType`、`TriggerType`、`InstanceStatus`、`DispatchDecision`。
- `tikeo-storage` 新增 `JobInstanceRepository`，支持创建 pending job instance、按 job 查询实例、按 id 查询实例。
- HTTP 新增 `POST /api/v1/jobs/{job}:trigger`，实现 API 手动触发并返回统一 `{code,message,data}` envelope。
- HTTP 新增 `GET /api/v1/jobs/{job}/instances` 与 `GET /api/v1/instances/{instance}`，支持实例列表与详情查询。
- OpenAPI schema 已补充 TriggerJobRequest、JobInstanceSummary、JobInstancePage。
- 设计路线图已将 API 手动触发实例链路作为基础调度器子项标记完成；CRON / Fixed Rate tick loop 仍待后续阶段。


## 2026-05-19 — 006-worker-sdk-rust-and-java-starter

- Worker Tunnel proto RPC 从 `Connect` 改为 `OpenTunnel`，解决 tonic client 生成方法名冲突。
- `tikeo-proto` 开启 tonic client 生成。
- 新增 `sdks/rust`，实现 Rust Worker SDK 最小主动连接、注册、心跳客户端。
- Rust Worker SDK 增加 `TaskProcessor` / `TaskContext` / `TaskOutcome` 基础处理器接口，为后续任务分发做准备。
- Rust Worker SDK 集成测试启动真实 tonic Worker Tunnel server，验证 register ack 与 heartbeat ping。
- 新增 `sdks/java/` Gradle 多模块 SDK 骨架：`tikeo`、`tikeo-spring`、`tikeo-spring-boot-starter`。
- Java core 提供 `@TikeoProcessor`、`WorkerRegistration`、`TikeoWorkerClient`、`NoopTikeoWorkerClient`。
- Spring Boot autoconfigure 提供 `tikeo.worker.*` 配置、auto-configuration imports 和注解扫描 registry。


## 2026-05-19 — 007-web-ui-foundation

- 新增 `web/` Bun 工程，技术栈为 React 19、TypeScript 6、Vite 8、Ant Design 6。
- 建立 AppShell、Dashboard、Jobs、Instances 页面骨架。
- Jobs 页面支持调用 API 创建 Job 与 API trigger；Instances 页面展示实例列表。
- 新增 typed API client，统一解析 `{code,message,data}` envelope。
- 新增 Bun test API client 单元测试，覆盖成功与业务失败分支。
- 建立 `lint`、`typecheck`、`test`、`build` 脚本并验证通过。


## 2026-05-19 — 008-container-deployment

- 新增后端多阶段 Dockerfile：Rust release builder + Debian slim runtime，默认运行 `tikeo serve --config /app/config/container.toml`。
- 新增 `config/container.toml`，容器内 HTTP `0.0.0.0:9090`、Worker Tunnel `0.0.0.0:9998`、SQLite dev 数据落 `/data/tikeo.db`。
- 新增 Web Dockerfile：Bun 构建 React/Ant Design 静态资源，nginx 托管并代理 `/api/`、`/api-docs/` 到 tikeo HTTP 服务。
- 新增 `docker-compose.yml`，包含 tikeo server 与 web 两个服务；Worker Tunnel 只暴露为 worker 主动出站连接入口。
- 新增 `deploy/k8s/tikeo.yaml` 与 README，包含 Namespace、ConfigMap、SQLite dev PVC、server Deployment/Service、worker tunnel Service、web Deployment/Service。
- 新增 Docker ignore 规则，避免 target、node_modules、dist 进入镜像构建上下文。
- 设计路线图已将 Docker 镜像构建标记完成；后续 Helm Chart 仍保留在 Phase 3。

Verification:
- `docker compose config` ✅
- `docker build -t tikeo:dev .` ✅
- `docker build -t tikeo-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web HTML + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅
- `deploy/k8s/tikeo.yaml` PyYAML 结构解析 ✅（8 documents；当前环境无 `kubectl`）
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
- `tikeo` 新增 `WorkerSession::process_next`，接收 dispatch、构造 `TaskContext`、调用 `TaskProcessor`、回传 `TaskOutcome`。
- Storage 新增 pending instance 查询与 status update repository 方法。
- 测试覆盖 repository 状态流转、server dispatch、SDK dispatch -> processor -> result 回传。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikeo -- serve --config config/dev.toml` + `/healthz` + `/api/v1/jobs` smoke ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t tikeo:dev .` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅
- `docker build -t tikeo-web:dev ./web` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 010-tikeo-tick-loop

- 新增 `tikeo-server::tikeo` 自动调度 tick loop。
- Storage 新增 `list_enabled_scheduled_jobs`，只查询 enabled 的 `cron` / `fixed_rate` jobs。
- CRON 使用 `cron 0.16.0` 解析表达式，Fixed Rate 使用 `humantime 2.3.0` 解析持续时间表达式。
- Tick loop 使用内存 cursor 避免同一 tick 重复触发；到期时创建 pending job_instance，并复用 009 dispatch loop。
- Server 启动时同时运行 HTTP、Worker Tunnel、自动 tikeo tick loop 和 Worker dispatch loop。
- 测试覆盖 fixed_rate 到期触发、cron 到期触发、disabled scheduled job 不触发。
- 设计路线图已标记基础调度器 CRON/Fixed Rate/API 子项完成，Rust SDK 任务执行子项完成。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikeo -- serve --config config/dev.toml` + fixed_rate job 自动创建 pending instance smoke ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t tikeo:dev .` ✅
- `docker build -t tikeo-web:dev ./web` ✅
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
- `docker build -t tikeo:dev .` ✅
- `docker build -t tikeo-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 012-auth-rbac-foundation

- 新增后端开发管理员认证模块：`POST /api/v1/auth/login`、`GET /api/v1/auth/me`、`POST /api/v1/auth/logout`。
- 支持 env 覆盖开发管理员用户名、密码与 token：`TIKEO_DEV_ADMIN_USERNAME`、`TIKEO_DEV_ADMIN_PASSWORD`、`TIKEO_DEV_ADMIN_TOKEN`。
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
- `cargo run --bin tikeo -- serve --config config/dev.toml` + `/healthz` + `/api/v1/auth/login` + `/api/v1/auth/me` + protected `POST /api/v1/jobs` smoke ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t tikeo:dev .` ✅
- `docker build -t tikeo-web:dev ./web` ✅
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
- Web Dockerfile/nginx 按分层构建与 nginx runtime 调整；Compose 使用默认 bridge 网络，Web 通过服务名 `tikeo` 反向代理后端。

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
- `DOCKER_BUILDKIT=1 docker build -t tikeo:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikeo-web:dev ./web` ✅
- `docker compose up -d --no-build` on default bridge ✅
- `curl /healthz`, Web `/`, proxied `/api/v1/system/info`, proxied/direct `/api-docs/openapi.json` ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — dev startup script + config directory

- Renamed runtime configuration directory from `examples/` to `config/` and updated Dockerfile, Compose, prompt, memory, and design references.
- Added `scripts/dev.sh` to start backend + Web dev server together, wait for health checks, print browser/API URLs, and write logs under `.dev/`.
- Added root `README.md` with local startup instructions, configuration directory contract, and initialization credentials.
- Updated built-in development initialization account defaults to `tikeo_init` / `Tikeo@2026!` / `tikeo-init-token`; env overrides remain available.
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
- `DOCKER_BUILDKIT=1 docker build -t tikeo:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikeo-web:dev ./web` ✅
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
- `cargo run --bin tikeo -- serve --config config/dev.toml` + healthz/jobs/reported instances endpoint smoke ✅
- `./scripts/dev.sh` backend + Web startup smoke ✅

## 2026-05-20 — 接手用户管理并抽象 SessionStore

Agent:
- Codex

Work:
- 接手他人已开发的用户管理/RBAC 模块。
- 新增 `crates/tikeo-server/src/http/session.rs`，定义 `SessionStore` trait、`SessionManager` 和当前 `DbMokaSessionStore`。
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
- `cargo run --bin tikeo -- serve --config config/dev.toml` + `/healthz` + `/auth/login` 冒烟 ✅，登录返回 `atk_` opaque token。

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
- `tikeo-core` 新增 `ScriptLanguage`（Shell/Python/Node/PowerShell/Rhai/Wasm）和 `ScriptStatus`（Draft/Approved/Disabled）枚举，含 `FromStr`/`Display`/`as_str()` 及测试。
- `tikeo-storage` 新增 `scripts` 表 SeaORM entity（id/name/language/version/content/status/timeout_seconds/max_memory_bytes/allow_network/allowed_env_vars/created_by/created_at/updated_at），无外键。
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

- 删除 `tikeo-init-token` 静态 admin bearer 后门；后端测试改为先通过初始化账号登录获取真实 `atk_` session token。
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
- 后端大文件拆分：`crates/tikeo-storage/src/repository.rs` 拆成 `repository/*`；`crates/tikeo-server/src/http/routes.rs` 拆成 `routes/*`。

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
- 已在 `crates/tikeo-storage/src/repository/workflow.rs` 扩展 workflow 节点类型白名单，避免 Web 新节点保存时报 unsupported node kind。

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
- 已迁移 Rust Worker SDK 到 `sdks/rust/tikeo`，Java 多模块 SDK 到 `sdks/java`。
- 根 Cargo workspace 已恢复为仅包含服务端与 `crates/*`；Rust SDK 独立于根 workspace 构建发布。
- Dockerfile 分层缓存、README、.gitignore、历史 prompt/memory 验证命令和设计文档结构图已同步到新目录。

### 2026-05-21 041 Dispatch Queue Claim/Lease
- 按 030 阶段继续推进队列多节点竞争基础能力。
- `WorkflowRepository` 新增 `claim_next_dispatch_queue_item`、`claim_dispatch_queue_item`、`release_dispatch_queue_item`，使用 lease_owner / lease_until 控制可占用性。
- HTTP 新增 `POST /api/v1/dispatch-queue:claim`，记录 `dispatch_queue` 的 `claim` 审计日志。
- 当前仍是最小实现；下一步建议继续把 dispatcher 的 materialize/dispatch 流程切到 claim API/原子条件更新路径，并补 visibility-timeout 回收。

### 2026-05-21 042 dev.sh 本地访问 URL 调整
- 用户手动修改 `scripts/dev.sh` 后要求代提交。
- 变更保留容器/服务绑定可配置性，默认 API_URL 改为 `http://localhost:$TIKEO_API_PORT`，WEB_URL 改为独立可覆盖的 `TIKEO_WEB_URL`。
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
- 执行用户要求的 SDK 目录整改：`sdks/rust/tikeo`、`sdks/java/<sdk-name>`。
- Java 构建从 Maven 切换到 Gradle Kotlin DSL + JDK21+；根 `gradlew` 会按需下载 Gradle。
- 创建 examples `<language>/<demo-name>` 目录骨架，仅放 demo/README，不放运行配置。
### 2026-05-21 SDK layout correction follow-up
- 用户明确根 `Dockerfile` 只构建 tikeo 服务端；已约束不得复制/缓存/构建 `sdks/` 或 `examples/`。
- SDK 路径规范固定为 `sdks/<language>/<sdk-name>/`，Demo 路径规范固定为 `examples/<language>/<demo-name>/`。
- Rust SDK 路径为 `sdks/rust/tikeo`；现已移除 repo-local path dependencies，满足独立发布约束。
- 已补齐可独立运行的 Rust demo（`examples/rust/worker-demo`）与 Java Spring Boot demo（`examples/java/spring-worker-demo`）基础。

### 2026-05-21 verification — SDK layout correction
- `./sdks/java/gradlew -p sdks/java test` ✅
- `./sdks/java/gradlew -p examples/java/spring-worker-demo test` ✅
- `cargo fmt --all -- --check` ✅
- `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features` ✅
- `cargo run --manifest-path examples/rust/worker-demo/Cargo.toml` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web lint && bun run --cwd web typecheck && bun test --cwd web && bun run --cwd web build` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikeo:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikeo:dev .` ✅ after switching builder/runtime flow to Alpine-compatible server-only image build.
- `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features` ✅ rerun after Dockerfile/dependency adjustments.

### 2026-05-21 Rust SDK independent publishing cleanup
- Removed `sdks/rust/tikeo` from root Cargo workspace and removed Dockerfile rewrite workaround.
- Made Rust SDK self-contained by bundling `proto/worker.proto`, local `build.rs`, and removing all `../../../crates/*` path dependencies.
- Replaced SDK integration tests with an in-crate mock Worker Tunnel server.
- `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `cargo package --manifest-path sdks/rust/tikeo/Cargo.toml --allow-dirty` ✅ proves Rust SDK package has no repo-local path dependencies.

### 2026-05-21 Worker identity assignment cleanup
- Changed Worker Tunnel RegisterWorker payload from client-supplied `worker_id` to optional `client_instance_id`.
- Server registry now generates authoritative `wrk-*` worker ids and returns them in `WorkerRegistered`.
- Rust SDK stores server-assigned worker id after connect and uses it for heartbeat/log/result messages.

### 2026-05-21 verification — worker identity assignment cleanup
- `cargo test --workspace --all-features` ✅
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features` ✅
- `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `cargo package --manifest-path sdks/rust/tikeo/Cargo.toml --allow-dirty` ✅
- `./sdks/java/gradlew -p sdks/java test` attempted first but Gradle distribution download hit `curl: (56) OpenSSL SSL_read ... unexpected eof while reading`.
- `~/.gradle/wrapper/dists/gradle-8.14-bin/.../bin/gradle -p sdks/java test` ✅ using cached Gradle.
- `~/.gradle/wrapper/dists/gradle-8.14-bin/.../bin/gradle -p examples/java/spring-worker-demo test` ✅ using cached Gradle.

### 2026-05-21 Java SDK Worker Tunnel implementation
- Implemented `GrpcTikeoWorkerClient` with server-assigned worker id registration semantics, heartbeat, task log emission, and dispatch result reporting.
- Added Java protobuf generation from bundled `worker.proto` in Java core SDK.
- Wired Spring Boot auto-configuration to create real gRPC client unless `tikeo.worker.dry-run=true`.
- Updated Java Spring demo to default dry-run and smoke-run without live tikeo.

### 2026-05-21 Java TikeoProcessor adapter
- Implemented invocable Spring processor handlers and `SpringTikeoTaskProcessor`.
- Wired Spring Boot autoconfiguration so live Java gRPC Worker Tunnel dispatches route to annotated processor methods.
- Added tests for context/string method invocation, exception failure mapping, duplicate processor rejection, route-by-job-id convention, and autoconfig registry wiring.

### 2026-05-21 Java Lombok/style adjustment
- Added Lombok to Java SDK and Java demo builds.
- Converted demo runner to constructor-injected component and simplified Spring worker properties / dry-run client boilerplate with Lombok.

### 2026-05-21 Java SDK three-module restructure
- Renamed Java native SDK module to `tikeo`.
- Split Spring Framework adapter into `tikeo-spring`.
- Moved Spring Boot auto-configuration/properties into `tikeo-spring-boot-starter` and updated AutoConfiguration imports.
- Updated Java demo and docs to use `tikeo-spring-boot-starter`.

### 2026-05-21 Java Spring Boot starter naming correction
- Renamed `sdks/java/tikeo-spring-boot` to `sdks/java/tikeo-spring-boot-starter`.
- Updated Gradle settings/build, Java demo dependency, README/design/prompt/memory references.

### 2026-05-21 Worker processor key protocol
- Added explicit `processor_name` to DispatchTask across server/Rust SDK/Java SDK protocol copies.
- Updated server dispatch task construction and tests to assert processor key population.
- Updated Rust SDK TaskContext and Java TaskContext to carry processor name.
- Updated SpringTikeoTaskProcessor to route by explicit processor name instead of job id fallback-only convention.

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
- Enabled `sqlx-postgres` on `tikeo-storage` and migrations so PostgreSQL URLs compile through SeaORM/sqlx.
- Added `config/postgres.toml` with PostgreSQL and CockroachDB URL examples; CockroachDB uses PostgreSQL wire protocol.
- Roadmap marks PostgreSQL + CockroachDB storage support complete at driver/config/template level; live DB smoke remains environment-dependent.

### 2026-05-21 Phase2 cluster coordinator foundation
- Added `tikeo-server::cluster` with ClusterCoordinator trait, explicit ClusterMode/ClusterRole, and StandaloneCoordinator.
- `/api/v1/cluster` now reports `role=standalone` with node_id/can_schedule/detail instead of fake `leader`.
- Design now records Raft implementation boundaries: leader ownership gate, follower fencing, DB claim as final idempotency guard, and container-friendly networking.

### 2026-05-21 Phase2 cluster ownership gates
- Tikeo tick loop and Worker dispatcher loop now consult `ClusterCoordinator` status before ownership-sensitive work.
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
- Kept current storage-backed no-op coordinator in `tikeo-server::cluster`; no new `tikeo-cluster` crate yet because runtime boundaries are not stable enough.

### 2026-05-21 Phase2 cluster diagnostics
- Added `/api/v1/cluster/diagnostics` for operator-visible cluster readiness: current status, scheduling gate, persisted Raft metadata, members, transport placeholder, and runtime boundary.
- Chose a separate diagnostics endpoint instead of bloating `/api/v1/cluster`; the lightweight status endpoint stays stable for UI polling.
- Kept cluster runtime in `tikeo-server::cluster` for now; no `tikeo-cluster` crate until consensus/runtime traits stabilize.

### 2026-05-21 Phase2 dispatch queue fencing token
- Reviewed Phase2: only full Raft runtime remains incomplete; Go/Python SDK stays Phase4.
- Added `dispatch_queue.fencing_token` shape and SQLite compatibility migration; claim responses now include a fencing token.
- Dispatcher now derives a fencing token from ClusterCoordinator status (`standalone:<node>:tikeo-dispatcher` today, future `raft:<node>:<leader-token>` when real consensus exists).

### 2026-05-21 Phase2 closeout / Phase3 audit paging
- Consensus dependency direction corrected to TiKV raft-rs (`raft` 0.7.0); full Raft scheduling still stays gated until event-loop/transport/persistence/fencing are real.
- Phase2 distributed safety foundations are documented as complete except real Raft runtime/membership.
- Started Phase3 audit governance by adding server-side audit filters and pagination plus Web UI filter controls.

### 2026-05-21 Phase2 raft-rs correction
- User corrected the OpenRaft direction; project now targets TiKV raft-rs (`raft` crate 0.7.0, Apache-2.0) instead of OpenRaft.
- Added `tikeo-server::cluster::raft_rs` bootstrap validation: deterministic string `node_id` -> non-zero u64 raft id, peer voters, `MemStorage + RawNode` construction. This proves dependency/API integration only; no tick loop, campaign, leader token, or scheduling grant exists yet.
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
- Runtime does not campaign, does not wire outbound transport, and still keeps `can_schedule=false` and `leader_fencing_token=null`; tikeo ownership remains fenced.
- Next slice: connect validated inbound HTTP messages to the runtime inbox, then implement Ready apply/outbound transport and real leader fencing.

### 2026-05-21 Phase2 raft-rs inbound runtime inbox
- Added a `ClusterCoordinator::submit_raft_message` boundary and wired `RaftRuntimeCoordinator` to enqueue validated `eraftpb::Message` values through a bounded mpsc inbox.
- `/api/v1/raft/append-entries` now returns `accepted=true` only when a running raft-rs runtime inbox accepts the message; standalone or stopped runtimes return `accepted=false` with a clear reason. This still does not grant scheduling ownership or a leader fencing token.
- Next slice: implement outbound peer HTTP transport and Ready apply/state-machine bookkeeping before enabling any leader fencing token.

### 2026-05-21 Phase2 raft-rs outbound transport skeleton
- Added optional `cluster.transport_token` config and `x-tikeo-raft-token` support so internal Raft HTTP transport can bypass human session auth without committing production secrets.
- Wired Ready outbound messages through a `RaftPeerTransport` skeleton: raft-rs `Message` values serialize to the existing HTTP wire DTO, base64 payloads are preserved, peer URLs append `/api/v1/raft/append-entries`, and delivery runs asynchronously through reqwest.
- Tikeo ownership remains fenced: no campaign, no leader token, no `can_schedule=true`. Next slice is committed-entry apply bookkeeping and fencing-token lifecycle.

### 2026-05-21 End-of-day handoff checkpoint
- User paused work for the day. Current pushed HEAD before this checkpoint: `222b1d6 Send raft-rs outbound messages through peer HTTP skeleton 📡`; working tree was clean before writing this memory checkpoint.
- Completed today: `fc67f13` runtime ticker + Ready persistence order, `dea7528` inbound runtime inbox, `222b1d6` outbound peer HTTP skeleton and optional internal Raft transport token.
- Verification evidence from last code slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` all passed.
- Tomorrow resume at `.prompt/053-phase2-raft-rs-apply-and-fencing.md`. Key safety rule: never set `can_schedule=true` or emit `leader_fencing_token` until real raft-rs leader state has generated and persisted a fencing token and dispatch/tikeo gates consume it.
- Key files for resume: `crates/tikeo-server/src/cluster/raft_rs.rs`, `crates/tikeo-storage/src/repository/raft.rs`, `crates/tikeo-server/src/cluster.rs`, `design/tikeo-architecture-design.md`, `.prompt/053-phase2-raft-rs-apply-and-fencing.md`.

### 2026-05-22 Phase2 raft-rs apply bookkeeping and fencing lifecycle
- Resumed from `.prompt/053-phase2-raft-rs-apply-and-fencing.md`.
- Implemented Ready committed-entry apply bookkeeping using `advance_append` / `advance_apply_to` instead of blindly advancing without state-machine acknowledgement.
- Committed `EntryNormal` entries now monotonically update `raft_metadata.applied_index`; `EntryConfChange` / `EntryConfChangeV2` are explicitly gated and stop apply progress before silent membership mutation.
- Added leader fencing-token lifecycle: only a real raft-rs `Leader` with term > 0 derives `raft:term:<term>:node:<node_id>`, persists it first, then reports `can_schedule=true`; non-leaders clear the token. Tikeo/dispatcher gates remain driven by `can_schedule` and dispatcher uses the persisted token.
- Targeted verification run so far: `cargo fmt --all`; `cargo test -p tikeo-server raft --all-features`; `cargo test -p tikeo-storage raft --all-features`.
- Next slice after commit: `.prompt/054-phase2-raft-rs-business-apply-membership.md`.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs business command envelope foundation
- Continued into `.prompt/054-phase2-raft-rs-business-apply-membership.md` after 053 commit.
- Added `raft_applied_commands` no-FK table/entity/repository for idempotent state-machine apply records keyed by `(node_id, log_index)` with `(cluster_id, command_id)` reserved for replay idempotency.
- `EntryNormal` payloads now parse as tikeo command envelopes (`command_id`, `command_type`, `payload`). `noop` is applied, unknown command types are recorded as `deferred_unsupported`, invalid JSON is recorded as `rejected`, and apply index still advances deliberately.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-storage raft --all-features`; `cargo test -p tikeo-server raft --all-features`.
- Next slice prompt: `.prompt/055-phase2-raft-rs-real-business-commands-and-membership.md`.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs real member command apply
- Resumed from `.prompt/055-phase2-raft-rs-real-business-commands-and-membership.md` and pushed previous local commit `7f82709`.
- Added `raft_member_upsert` as the first real state-machine command. Scope is intentionally limited to member catalog metadata, so it is safe before dynamic ConfChange support.
- Added duplicate `command_id` replay guard before side effects. Replayed commands advance Raft apply bookkeeping but do not reapply member mutations or violate the unique `(cluster_id, command_id)` index.
- Updated design to document the dynamic membership two-layer flow: member catalog command first; future proposal API + raft-rs `propose_conf_change` + committed ConfState apply before changing voters/learners.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_apply_committed_entries --all-features`; `cargo test -p tikeo-storage raft_tables_keep_soft_relationships_without_foreign_keys --all-features`.
- Full verification passed for 055: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs membership proposal intent API
- Continued automatically into `.prompt/056-phase2-raft-rs-membership-proposal-api.md` after committing 055.
- Implemented no-FK `raft_membership_proposals` storage and idempotent repository insert by `(cluster_id, proposal_id)`.
- Implemented `POST /api/v1/raft/members:propose` with `{ code, message, data }` envelope, `cluster:manage` RBAC, real-leader/fencing guard, http/https endpoint validation, self-removal block, and quorum-reduction block for unsafe remove proposals.
- Tests added for non-leader rejection, invalid endpoint rejection, and duplicate proposal idempotency.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_membership_proposal --all-features`; `cargo test -p tikeo-storage raft_tables_keep_soft_relationships_without_foreign_keys --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Full verification passed for 056: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs committed ConfChange apply
- Continued `.prompt/057-phase2-raft-rs-confchange-apply.md`.
- Added `RaftMembershipProposal` and `RaftMembershipProposalSubmission` to the cluster trait boundary plus runtime command handling in `RaftRuntimeCoordinator`.
- Added `raft_metadata.conf_state` persistence with SQLite compatibility migration and diagnostics exposure.
- Implemented committed ConfChange handling: decode v1/v2, require runtime node to apply real membership changes, persist `ConfState` before updating `raft_members`, and update proposal status to `applied`/`rejected`.
- Added targeted tests for committed add-member happy path, malformed ConfChange handling, and no-runtime gating.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Full verification passed for 057: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs multi-node in-process E2E
- Resumed from `.prompt/058-phase2-raft-rs-multinode-e2e.md`.
- Added `TestRaftCluster` / `TestRaftNode` harness in `crates/tikeo-server/src/cluster/raft_rs.rs` for deterministic in-process message routing between three raft-rs RawNodes.
- Added tests `raft_inprocess_harness_elects_real_leader_and_persists_fencing` and `raft_inprocess_membership_proposal_commits_and_applies_member`.
- Updated design roadmap item to completed and created `.prompt/059-phase2-raft-rs-http-transport-e2e-or-persistence-hardening.md` for HTTP transport/restart hardening.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_inprocess --all-features`; `cargo test -p tikeo-server raft --all-features`.
- Full verification passed for 058: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs restart recovery hardening
- Resumed into `.prompt/059-phase2-raft-rs-http-transport-e2e-or-persistence-hardening.md`.
- Added `build_runtime_from_repository` and `restore_persisted_storage` in `crates/tikeo-server/src/cluster/raft_rs.rs` to restore HardState/log entries into `MemStorage` on startup.
- Changed initial role metadata persistence to preserve existing raft term/log/applied/conf_state rows and only clear stale leader fencing.
- Added `.prompt/060-phase2-raft-rs-http-transport-smoke.md` for the next transport E2E/smoke slice.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_runtime_restore --all-features`; `cargo test -p tikeo-server raft --all-features`.
- Full verification passed for 059: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs HTTP transport token smoke
- Resumed into `.prompt/060-phase2-raft-rs-http-transport-smoke.md`.
- Added `http::tests::raft_append_entries_internal_token_bypasses_human_session_only_for_transport` in `crates/tikeo-server/src/http/mod.rs`.
- Created `.prompt/061-phase2-raft-rs-docker-bridge-e2e-script.md` for the remaining no-host-network Docker bridge E2E work.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server raft_append_entries_internal_token --all-features`.
- Full verification passed for 060: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs Docker bridge E2E script
- Implemented `scripts/raft-bridge-e2e.sh` for no-host-network Docker bridge verification with 3 tikeo containers and container-DNS raft peer endpoints.
- Fixed Dockerfile alpine build dependency gap for raft-proto by adding `protobuf-dev gcompat` to the builder stage; runtime remains alpine.
- Observed that bridge E2E may elect a real leader; script now accepts zero-or-one schedulable leader and requires any schedulable node to be `role=leader` with a fencing token.
- Created `.prompt/062-phase3-audit-before-after-trace-export.md` as the next roadmap slice.
- E2E verification passed: `./scripts/raft-bridge-e2e.sh`.
- Full verification passed for 061: `./scripts/raft-bridge-e2e.sh`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 audit before/after trace result foundation
- Resumed into `.prompt/062-phase3-audit-before-after-trace-export.md`.
- Updated audit storage/model/API/Web for before/after/trace_id/result/failure_reason.
- Updated design SQL sketch and roadmap: before/after trace/failure foundation complete; export governance remains `.prompt/063-phase3-audit-export-governance.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cargo test -p tikeo-storage migration_creates_metadata_tables --all-features`.
- Full verification passed for 062: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 governed audit JSON export
- Resumed into `.prompt/063-phase3-audit-export-governance.md`.
- Implemented `export_audit_logs` route and DTOs for governed JSON audit export; routed `/api/v1/audit-logs:export` and added OpenAPI registration.
- Updated Web audit client/page to download current-filter JSON exports.
- Created `.prompt/064-phase3-web-danger-confirm-permission-actions.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cd web && bun run typecheck`.
- Full verification passed for 063: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 Web dangerous confirmations and permission-aware actions
- Resumed into `.prompt/064-phase3-web-danger-confirm-permission-actions.md`.
- Added permission-aware frontend helper components and applied them to Jobs, Users, Scripts, and Workflows pages.
- Updated design roadmap item to completed and created `.prompt/065-phase3-route-meta-lazy-401-403-url-governance.md`.
- Targeted verification so far: `cd web && bun run typecheck`.
- Full verification passed for 064: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun run build` (Vite chunk-size warning only).


### 2026-05-22 Phase3 route meta, lazy loading, 401/403, URL query governance
- Started `.prompt/065-phase3-route-meta-lazy-401-403-url-governance.md`.
- Implemented route metadata table, App/AppShell lazy-route wiring, shared route fallback/forbidden page, API auth error handler, URL query state hook, and query persistence on audit/jobs/scripts/workflows.
- Updated design roadmap and created `.prompt/066-phase3-wasm-sandbox-processor-spike.md`.
- Targeted verification so far: `cd web && bun run typecheck`; `cd web && bun test`.
- Full verification passed for 065: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning remains for Scripts/main chunks).

### 2026-05-22 Phase3 WASM sandbox processor boundary
- Started `.prompt/066-phase3-wasm-sandbox-processor-spike.md`.
- Verified current `wasmtime = 45.0.0` with `cargo search --registry crates-io`; used upstream Wasmtime docs as policy evidence for fuel/epoch/resource limiting.
- Implemented stable `tikeo-core` WASM processor spec and default-deny validation for network/filesystem capabilities.
- Updated design roadmap and created `.prompt/067-phase3-wasm-worker-runtime-executor.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-core --all-features`.
- Full verification passed for 066: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM worker runtime executor
- Started `.prompt/067-phase3-wasm-worker-runtime-executor.md`.
- Added `tikeo-wasm` crate with Wasmtime executor and policy tests; no server HTTP/storage coupling.
- Updated design roadmap and created `.prompt/068-phase3-wasm-script-binding-and-dispatch.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-wasm --all-features`; `cargo clippy -p tikeo-wasm --all-targets --all-features -- -D warnings`.
- Full verification passed for 067: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM script binding and dispatch metadata
- Started `.prompt/068-phase3-wasm-script-binding-and-dispatch.md`.
- Added worker proto dynamic WASM binding metadata across server/Rust SDK/Java SDK proto files.
- Dispatcher attaches `WasmProcessorBinding` only for approved, policy-safe `script:<id>` WASM scripts and leaves regular SDK processor dispatch unchanged.
- Updated design roadmap and created `.prompt/069-phase3-wasm-sdk-execution-adapters.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikeo-server tunnel::dispatcher --all-features`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features`; Java Gradle test attempted but stopped due slow first distribution download.
- Full verification passed for 068: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features`. Java SDK Gradle test was attempted but not completed because the first Gradle distribution download was too slow; rerun once cached.
- Re-ran final 068 verification after proto boxing/clippy fixes: all listed Rust/backend/web/Rust-SDK checks passed again. Java SDK Gradle remains not completed due slow first distribution download.

### 2026-05-22 Java SDK Gradle verification补齐
- User corrected Gradle latest-version requirement; updated `sdks/java/gradlew` from 8.14.3 to Gradle 9.5.1 and defaulted distribution download to Huawei Cloud mirror with override support.
- Fixed wrapper cwd from repo root to `sdks/java`.
- Verification passed: `cd sdks/java && ./gradlew --version --no-daemon`; `cd sdks/java && ./gradlew test --no-daemon` (BUILD SUCCESSFUL). Warning: deprecated Gradle features need future Gradle 10 cleanup.


### 2026-05-22 Phase3 WASM SDK execution adapters
- Resumed `.prompt/069-phase3-wasm-sdk-execution-adapters.md`.
- Implemented Rust SDK WASM binding path behind explicit `wasm` feature and preserved normal `TaskProcessor` routing otherwise.
- Implemented Java SDK explicit unsupported result for WASM bindings with regression test that the normal processor is not called.
- Updated design roadmap and created `.prompt/070-phase3-wasm-distribution-integrity-and-gradle10-cleanup.md`.
- Verification passed: backend fmt/clippy/tests/help, web typecheck/tests/build, Rust SDK no-feature + wasm-feature tests + clippy, Java Gradle tests. Gradle 10 deprecation warning remains tracked for next slice.


### 2026-05-22 Phase3 WASM distribution integrity and Gradle 10 cleanup
- Resumed `.prompt/070-phase3-wasm-distribution-integrity-and-gradle10-cleanup.md`.
- Added WASM binding integrity/version metadata, script-version SHA-256 persistence, Rust SDK digest validation, Web sandbox-policy visibility, and Gradle 10 deprecation cleanup.
- Created `.prompt/071-phase3-script-release-pointer-and-worker-version-binding.md` for release-pointer/version-binding follow-up.
- Verification passed: backend fmt/clippy/tests/help, web typecheck/tests/build, Rust SDK no-feature + wasm-feature tests + clippy, Java Gradle tests with `--warning-mode all` and no deprecation warning output.

### 2026-05-22 — Phase 071 script release pointer / immutable dispatch binding
- Implemented release pointer columns on `scripts` (`released_version_id`, `released_version_number`) and compatibility migration logic; relationships remain soft only.
- Repository `create_script`/`create_version` now avoid SeaORM SQLite NULL aggregate decode issue by flattening `MAX(version_number)` and returning constructed summaries after inserts.
- Added `publish_version` / `rollback_release` repository methods plus targeted storage test covering create -> update -> versions -> publish -> rollback.
- Added HTTP publish/rollback routes and test asserting envelope response and pointer movement from latest version back to version 1.
- Updated dispatch path so `script:<id>` WASM binding is built only from released immutable version snapshots; no release/missing snapshot leaves dispatch pending instead of sending mutable content.
- Updated Web script management with released version metadata, released row tags, publish and rollback guarded actions.
- Updated architecture roadmap and created `.prompt/072-phase3-script-policy-engine-and-sandbox-runners.md`.

### 2026-05-22 — Phase 072 policy metadata / runner abstraction / Web chunk split
- Added default-deny script policy model to `tikeo-core` and validation tests.
- Added `policy_json` to scripts and script_versions, threaded through repository summaries/version snapshots, and included policy in version diff.
- Added HTTP policy validation for script create/update; dangerous network/filesystem/secret grants return bad-request envelopes.
- Added Web policy types, safe policy editing fields, detail display, policy diff fields, and Vite/Rolldown vendor chunk groups to eliminate large chunk warning.
- Added Rust SDK non-WASM runner abstraction and unsupported runner test as the handoff point for concrete sandbox runner implementation.
- Verification passed: Rust workspace fmt/clippy/test/help, Web typecheck/test/build without large chunk warning, Rust SDK native+wasm tests/clippy, Java SDK Gradle test.

### 2026-05-22 — Audit page loading loop fix
- Fixed Web audit log page infinite request/loading loop by hoisting `useUrlQueryState` defaults to a stable module-level object instead of recreating defaults on every render.
- Root cause: unstable defaults changed the memoized URL query object, which changed `fetchLogs`, which retriggered the effect continuously and made the app feel unclickable.
- Verification passed: `cd web && bun run typecheck && bun test && bun run build`.

### 2026-05-22 — Phase 073 local subprocess script runner foundation
- Implemented Rust SDK `LocalSubprocessScriptRunner` behind explicit worker opt-in for non-WASM dynamic scripts.
- The runner requires immutable released script version metadata and content digest verification before spawning a child process; it denies network/filesystem/secrets through `ScriptRunnerPolicy` and uses stdin rather than writing script files.
- Added timeout, output cap, unavailable executable, digest mismatch, missing release snapshot, and successful shell smoke tests.
- Verification passed across Rust workspace, Web build/test, Rust SDK native+wasm+clippy, and Java Gradle tests.

### 2026-05-22 — Phase 074 non-WASM script protocol binding
- Finished the handoff slice for non-WASM dynamic script dispatch.
- Added protocol-level `ScriptProcessorBinding` and synchronized server/root/Rust SDK/Java SDK proto files.
- Dispatcher binds only approved released immutable script snapshots, validates the released snapshot policy, and routes to workers advertising unified `script`, while remaining compatible with `script:<language>`, `script:wasm`, `script:*`, or `*` capability.
- Rust SDK now has explicit runner registry routing for script bindings; Java SDK refuses script bindings until Java runner support is intentionally designed.
- Updated architecture roadmap and prepared `.prompt/075-script-runner-container-and-execution-governance.md`.
- Verification passed across Rust workspace, tikeo-proto, dispatcher tests, Rust SDK native+wasm+clippy, Web typecheck/test/build, and Java Gradle tests.

### 2026-05-22 — Phase 075 container script runner foundation
- Added Rust SDK `ContainerScriptRunner` for Worker-side opt-in non-WASM script execution through a Docker-compatible CLI boundary.
- Honored the new modularity constraint by splitting `tikeo/src/lib.rs` into focused Rust modules; future work should follow this pattern across server, web, and all SDK languages.
- Runner command boundary is default-deny: stdin script content, no container network, read-only rootfs, no host mounts, explicit tikeo metadata env, and whitelisted env only.
- Added tests for Docker arg construction and dangerous policy rejection before runtime spawn.
- Prepared `.prompt/076-script-execution-governance-and-live-runner-smoke.md` for result/audit visibility and optional live runtime smoke.
- Verification passed after the Rust SDK module split: Rust workspace fmt/clippy/test/help, Rust SDK native+wasm+clippy, Web typecheck/test/build, and Java Gradle tests.

### 2026-05-22 — Project rename to tikeo
- Renamed project identity from the previous project identity to tikeo across source tree, crate/package names, SDKs, protocol namespaces, Docker/K8s/Compose config, docs, memory, and phase prompts.
- Java package prefix changed to `net.tikeo`; Java SDK modules changed to `tikeo`, `tikeo-spring`, and `tikeo-spring-boot-starter`.
- Rust SDK path/crate changed to `sdks/rust/tikeo` / `tikeo`; root binary changed to `tikeo`.
- Protobuf package changed to `tikeo.worker.v1`; internal Raft transport header changed to `x-tikeo-raft-token`; environment variables changed to `TIKEO_*` / `TIKEO__*`.
- Updated repository metadata target to `https://github.com/yhyzgn/tikeo.git` and prepared git identity target `Neo <yhyzgn@gmail.com>`.
- Created `.prompt/077-script-execution-governance-after-tikeo-rename.md` so future work resumes after the rename.
- Verification in progress: `cargo check --workspace --all-features` passed; `cargo fmt --all` completed. Full verification and git push status to be recorded before final response.

### 2026-05-22 — SDK naming contraction
- User requested Java SDK previous Java core SDK name -> `tikeo` and Rust SDK previous Rust Worker SDK name -> `tikeo`.
- Renamed directories to `sdks/java/tikeo` and `sdks/rust/tikeo`, updated Gradle project dependencies, Rust crate metadata, examples, docs, memory, and prompts.
- Verification after this additional rename is in progress.
- Fixed verification regression caused by renamed default admin password: regenerated the seeded BCrypt hash for `Tikeo@2026!`.
- Full verification passed after SDK naming contraction:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`
  - `cargo build --workspace --all-features`
  - `cargo run -- --help`
  - `cd web && bun run typecheck && bun test && bun run build`
  - `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`
  - `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`
  - `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`
  - `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`
### 2026-05-23 — Phase 077 script execution governance visibility
- Continued after context reload and RTK activation.
- Implemented dispatcher governance logging for script fail-closed dispatch and no eligible script worker capability.
- Implemented Rust SDK failure class detection for missing runner, policy rejection, digest mismatch, timeout, output limit, and runtime unavailable; Worker task result messages now carry JSON failure metadata for recognized script runner failures.
- Server Worker Tunnel persists recognized JSON failure metadata as `script_execution_governance` instance logs.
- Documented script-capable Worker Pool deployment constraints and `ContainerScriptRunner` usage in `design/tikeo-architecture-design.md` and `sdks/rust/tikeo/README.md`.
- Created `.prompt/078-script-governance-audit-alerting.md`.
- Targeted verification passed: `cargo test -p tikeo-server tunnel::dispatcher --all-features`; `cargo test -p tikeo-server tunnel::service --all-features`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml script`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml worker_session_rejects_script_binding_without_registered_runner`.
- Full verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cd web && bun install && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.
### 2026-05-23 — Phase 078 script governance query/UI/alert foundation
- Continued from `.prompt/078-script-governance-audit-alerting.md`.
- Added structured parsing of `script_execution_governance` instance logs into API fields and added governance-only filtering via `page_token=script_execution_governance`.
- Updated Web instance log drawer to highlight governance failure classes instead of showing only raw JSON.
- Added `AlertCondition::ScriptGovernanceFailure` and tests for alert condition serialization/noop dispatch path.
- Created `.prompt/079-script-governance-audit-materialization.md`.
- Targeted verification passed: `cargo test -p tikeo-server trigger_job_creates_pending_instance --all-features`; `cargo test -p tikeo-server alert --all-features`; `cd web && bun test src/api/client.test.ts`.
- Full verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-23 — Workflow edge condition legacy seed/frontend normalization fix
- Investigated `PATCH /api/v1/workflows/wf-dev-basic-pipeline` 400 with payload edge condition `success` after user clarified the visible Web selector already uses `on_success`.
- Root cause: development seed persisted the old alias `success`; the Web editor loaded that stale definition into JSON draft and node-position edits preserved the stale edge condition through `updateWorkflow`, where backend validation correctly rejects non-canonical conditions.
- Fixed Web API boundary by normalizing legacy workflow edge condition aliases before create/update/dry-run: `success`/`succeeded` -> `on_success`, `failure`/`failed` -> `on_failure`; unknown values still pass through so backend remains authoritative.
- Fixed editor load path to stringify the normalized definition, so the page draft no longer keeps stale aliases after opening an existing workflow.
- Updated `scripts/dev-seed.sql` to seed `wf-dev-basic-pipeline` and its edge row with `on_success`.
- Added Bun regression tests covering mutation and dry-run serialization of legacy aliases.
- Verification passed: `git diff --check -- web/src/api/client.ts web/src/api/client.test.ts web/src/pages/WorkflowsPage.tsx scripts/dev-seed.sql`; `cd web && bun run typecheck && bun test && bun run build`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-23 — Script editor secondary page and workflow canvas fullscreen
- Replaced the cramped script edit modal with a guarded secondary route `/scripts/:id/edit` and `ScriptEditorPage`; list edit actions now navigate to the page.
- The script editor page keeps the existing diff-before-save governance flow while giving the CodeMirror editor a wider layout and separating basic metadata, runtime limits, and policy controls into side cards.
- Added editable workflow DAG canvas fullscreen toggle with Escape-to-exit and body scroll lock; the existing DAG data model, node editing, edge editing, JSON/YAML, and dry-run flows remain unchanged.
- Added source-level Web tests for the new script edit route/page contract and workflow fullscreen affordance.
- Verification passed: `cd web && bun run typecheck`; `cd web && bun test && bun run build`; `git diff --check` on changed files; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-23 — Script editor back button compact style
- Fixed the secondary script editor back button being stretched by the hero flex-column container by constraining `.workflow-back-button.ant-btn` to `align-self: flex-start` and `width: auto`.
- Verification passed: `cd web && bun run typecheck && bun test && bun run build`.


### 2026-05-23 — Roadmap adjustment: migration and deployment scope
- User requested deferring Node.js SDK, K8s Helm Chart, and PowerJob migration tool from Phase 3 to Phase 4.
- Updated `design/tikeo-architecture-design.md` Phase 3/4 roadmap accordingly.
- Added XXL-JOB migration tool as a Phase 4 roadmap item and clarified migration CLI/report expectations.
- Updated `.memory/next.md` and `.memory/progress.md` so future handoff resumes Phase 3 without accidentally picking those deferred items.
### 2026-05-23 — Phase 079 script governance audit materialization
- Continued `.prompt/079-script-governance-audit-materialization.md`.
- Added `tunnel::governance` helper so dispatcher-side fail-closed script governance events and Worker result failure classes share the same canonical `script_execution_governance` payload.
- Materialized governance failures into durable `audit_logs` rows with `action=script_governance_failure`, `resource_type=script_execution_governance`, soft `resource_id=<instance_id>`, `result=failed`, and `failure_reason=<failure_class>`; no database foreign keys were added.
- Added audit repository/API filtering by `failure_reason`; Web audit page now keeps the filter in URL state and export uses the same filter.
- Added regression coverage proving governance audit rows can be queried by failure class.
### 2026-05-23 — Phase 080 alert rule API and event history
- Continued `.prompt/080-alert-rule-event-history.md`.
- Added persistent `alert_rules` and `alert_events` metadata tables plus repository support for rule creation/listing and event history queries.
- Exposed `/api/v1/alert-rules` and `/api/v1/alert-events` HTTP APIs behind existing admin/audit permissions and kept responses in the standard `{ code, message, data }` envelope.
- Wired script governance materialization to append alert history entries alongside audit rows, including basic threshold/dedupe/silence handling for `script_governance_failure` rules.
- Added regression coverage for alert rule creation, governance event ingestion, and alert event history queries.
### 2026-05-23 — Phase 081 alert recovery and notification history
- Continued `.prompt/081-alert-recovery-and-notifications.md`.
- Added deterministic alert recovery transitions by appending `script_governance_recovery` history rows with `status=recovered` instead of mutating prior events.
- Exposed a recovery HTTP endpoint for alert events and kept list/history queries stable for operators.
- Added regression coverage proving a firing governance alert can be resolved into a recovery history entry while preserving the original firing event.
### 2026-05-23 — Phase 082 alert notification summary
- Continued `.prompt/082-alert-notification-summary.md`.
- Added `/api/v1/alert-events:summary` to roll up alert event history by rule, resource, and failure class while preserving list filters.
- Summary rows include latest status/type/message, first/last seen timestamps, and firing/suppressed/silenced/recovered counts for operator notification history review.
- Added regression coverage proving firing, suppressed, and recovered history rows collapse into a single deterministic summary without external webhook smoke.
### 2026-05-23 — Phase 083 metrics summary and SLO API
- Continued `.prompt/083-metrics-summary-and-slo.md`.
- Added deterministic `/api/v1/metrics/summary` for operator dashboards without requiring external Prometheus/Grafana services in tests.
- Summary includes online worker count, job instance status counts, alert event status counts, and script governance failure counts by failure class.
- Added targeted regression coverage for storage/registry/alert count aggregation and standard HTTP envelope behavior.
### 2026-05-23 — Phase 084 OpenTelemetry tracing foundation
- Continued `.prompt/084-opentelemetry-tracing-foundation.md`.
- Added HTTP trace-id middleware that accepts `x-request-id`, `x-trace-id`, or W3C `traceparent`, generates `trc-*` when missing, and writes `x-trace-id` on API responses.
- Added local tracing span fields for method/path/trace_id without requiring an OTLP collector in tests; audit helpers now resolve the same generated/propagated trace id from request headers.
- Added targeted regression coverage for explicit and generated trace-id response behavior plus traceparent parsing.
### 2026-05-23 — Phase 085 OIDC/SSO foundation
- Continued `.prompt/085-oidc-sso-foundation.md`.
- Added `auth` / `auth.oidc` configuration shapes with local login enabled and OIDC disabled by default.
- Added public `GET /api/v1/auth/status` so clients can distinguish local vs OIDC-ready auth mode and see redacted provider metadata without a live IdP.
- Preserved existing local admin login/session/RBAC behavior and added regression coverage for local and configured OIDC status responses.
### 2026-05-23 — Phase 086 mTLS transport foundation
- Continued `.prompt/086-mtls-transport-foundation.md`.
- Added `transport_security` configuration shapes for HTTP and Worker Tunnel TLS/mTLS while keeping local development plaintext by default.
- Added `GET /api/v1/security/transport` diagnostics that redacts paths but reports TLS/mTLS readiness and partial-config issues.
- Added targeted regression coverage for default plaintext readiness and partial Worker Tunnel mTLS configuration diagnostics without certificate/network smoke.
### 2026-05-23 — Phase 087 script approval policy gates
- Continued `.prompt/087-script-approval-policy-gates.md`.
- Added publish/rollback policy gate checks that re-validate immutable script version snapshots before changing the release pointer.
- Dangerous legacy/imported versions that request network/filesystem/secret grants are blocked with a standard bad-request envelope and `failure_reason=script_policy_approval_required` audit rows.
- Safe script publish and rollback behavior remains unchanged; added regression coverage for blocked dangerous publish/rollback and queryable failed audit entries.

### 2026-05-23 — Phase 088 Grafana dashboard template foundation
- Continued `.prompt/088-phase3-remaining-hardening.md` with the smallest locally verifiable observability hardening slice.
- Added `observability/grafana/tikeo-phase3-dashboard.json` as a deterministic Grafana dashboard template for existing Prometheus metrics: HTTP request rate, HTTP p95 latency, connected workers, worker dispatch outcomes, and an HTTP error-ratio SLO placeholder.
- Added a Rust integration test that parses the dashboard JSON, asserts the expected title/panels shape, and verifies the required metric query strings stay present without needing a live Grafana/Prometheus service.
- Roadmap now marks the Grafana template foundation complete while leaving richer scheduling-latency/business SLO metrics open.

### 2026-05-23 — Phase 089 dispatch queue SLO summary
- Continued `.prompt/089-phase3-business-slo-metrics.md` with a locally verifiable business SLO metric slice.
- Added `DispatchQueueSloSummary` over existing `dispatch_queue` rows: total/by_status, pending/running counts, oldest pending age seconds, and average pending age seconds.
- Extended `GET /api/v1/metrics/summary` with the queue SLO summary while preserving the standard HTTP envelope and avoiding external Prometheus/Grafana dependencies in tests.
- Updated the Grafana template with a dispatch queue pending-age SLO query placeholder and kept JSON/metric-reference validation deterministic.

### 2026-05-23 — Phase 090 OTLP exporter status foundation
- Continued `.prompt/090-phase3-otel-exporter-foundation.md` with configuration/readiness plumbing before adding network exporter side effects.
- Added `observability.tracing` config with disabled-by-default OTLP export, optional endpoint, and header-name metadata; local dev and container configs keep export disabled.
- Added `GET /api/v1/observability/status` behind `system:read` permission to report tracing exporter readiness while redacting endpoint values and header secrets.
- Added regression coverage for default no-collector mode and configured OTLP readiness without requiring a live collector.

### 2026-05-23 — Phase 091 alert delivery readiness foundation
- Continued `.prompt/091-phase3-alert-provider-delivery-foundation.md` with a no-network notification hardening slice.
- Added `GET /api/v1/alert-rules/{id}/delivery-status` behind existing audit read permission.
- Delivery status parses persisted rule channels, reports provider/target/secret readiness, and redacts URLs/tokens/secrets from the response.
- Added regression coverage for webhook/email channel readiness and redaction without sending real external notifications.

### 2026-05-23 — Phase 092 OIDC authorize/callback skeleton
- Continued `.prompt/092-phase3-oidc-callback-skeleton.md` with a no-IdP local SSO shape slice.
- Added `GET /api/v1/auth/oidc/authorize` to build a redacted authorization URL from configured issuer/client/scopes without contacting the provider.
- Added `GET /api/v1/auth/oidc/callback` as a safe callback contract that validates code/state shape but refuses to create sessions until real token exchange/external identity mapping exists.
- Added regression coverage for disabled default behavior, configured authorize URL shape, secret redaction, and callback fail-closed behavior.

### 2026-05-23 — Phase 093 script approval/signature fail-closed skeleton
- Continued `.prompt/093-phase3-script-approval-signature-skeleton.md` with release metadata gates instead of runtime behavior.
- Added `approval_ticket` and `signature` fields to `ScriptReleaseRequest` so clients cannot silently send ignored approval/signature data.
- Publish/rollback now fail closed when those fields are present until real signature verification exists, and materialize `failure_reason=script_signature_verification_required` audit rows.
- Existing safe publish/rollback and dangerous policy gate behavior remain unchanged; Server still never executes user code.

### 2026-05-23 — Phase 094 transport listener boundary
- Continued `.prompt/094-phase3-transport-listener-boundary.md` with a fail-closed TLS readiness boundary.
- `GET /api/v1/security/transport` now reports `listener_mode` per endpoint: plaintext by default, `tls_pending_listener` when TLS is configured but listener wiring is not implemented.
- TLS/mTLS-enabled configs are no longer considered ready solely because cert/key paths are present; status issues explicitly call out pending listener wiring while keeping paths redacted.
- Added regression coverage for default plaintext, partial mTLS config, and fully path-configured HTTP TLS still failing closed until real TLS serving exists.

### 2026-05-23 — Phase 3 closeout review
- Completed `.prompt/095-phase3-closeout-review.md` as an honest roadmap closeout pass after the Phase 088-094 hardening run.
- Confirmed Phase 3 top-level items that still require external systems or larger production wiring remain unchecked: real OIDC token exchange and external identity mapping, real TLS/mTLS listeners, full script approval/signing/grants, real alert provider delivery, complete business SLO metrics, and real OTLP exporter smoke.
- Added Phase 3 closeout notes to `design/tikeo-architecture-design.md` summarizing completed local foundations vs remaining production gaps.
- Deferred Phase 4 scope remains unchanged: Node.js SDK, K8s Helm, PowerJob migration tooling, and XXL-JOB migration tooling.

### 2026-05-23 — Phase 096 dispatch queue Prometheus SLO metric
- Continued Phase 3 observability hardening by making the Grafana dispatch queue pending-age query backed by a real server-emitted Prometheus histogram instead of a dashboard-only placeholder.
- `GET /api/v1/metrics/summary` now records `tikeo_dispatch_queue_pending_age_seconds{stat="oldest|average"}` and `tikeo_dispatch_queue_items_total{status="pending|running"}` into the same local Prometheus recorder exposed by `/metrics`.
- Added regression coverage that calls the summary endpoint, then scrapes `/metrics` and asserts the dispatch queue pending-age metric is present.
- Full business SLO coverage remains open for broader dispatch latency, instance success-rate, workflow SLA, and map-reduce metrics.

### 2026-05-23 — Phase 097 business SLO Prometheus snapshots
- Continued `.prompt/097-phase3-business-slo-prometheus-snapshots.md` by promoting more existing metrics summary data into real Prometheus series.
- `GET /api/v1/metrics/summary` now records worker online, job instance status, job instance success ratio, alert status, and script governance failure gauges into the router-local recorder exposed by `/metrics`.
- Extended regression coverage so the summary-then-scrape path proves the new instance success and script governance SLO metric names are emitted.
- Kept full business SLO coverage open for end-to-end dispatch latency histograms, workflow/map-reduce SLA, and live Prometheus recording-rule validation.

### 2026-05-23 — Phase 098 API token lifecycle foundation
- Continued Phase 3 auth/RBAC hardening with a durable API token lifecycle slice.
- Added authenticated `POST /api/v1/auth/api-tokens`, `GET /api/v1/auth/api-tokens`, and `DELETE /api/v1/auth/api-tokens/{id}` endpoints.
- API tokens reuse the DB-backed session store, persist only SHA-256 token hashes, return the raw bearer token only at creation time, hide `token_hash` from list responses, and invalidate bearer access immediately on revoke.
- Added audit entries for API token create/revoke; fine-grained token scopes, rotation policy, and multi-tenant scope binding remain future work.

### 2026-05-23 — Phase 099 scoped API token permissions
- Continued `.prompt/099-phase3-api-token-scopes.md` by adding fine-grained API token scope allow-lists.
- `POST /api/v1/auth/api-tokens` now accepts optional `scopes` in `resource:action` form, validates every requested scope against the current principal permissions, stores the scope metadata with the hashed token session, and returns scopes in token metadata.
- Scoped API tokens now resolve to narrowed effective permissions; an `admin` role no longer bypasses scoped-token limits, so a `users:read` token can list users but cannot create users.
- Multi-tenant namespace/app/worker-pool scope binding and token rotation/expiry policy remain future work.

### 2026-05-23 — Phase 100 Worker cluster page interaction/layout refresh
- Responded to the user-requested Worker cluster page UX fix by replacing the two-list layout with an operations dashboard.
- Split `WorkersPage` into focused components under `web/src/pages/workers/`: cluster overview/queue stats, filterable worker table, dispatch queue status panel, and pure page-model helpers.
- Added worker search, namespace filtering, capability filtering, queue status drill-down, queue pressure/health affordances, responsive CSS, and static regression coverage for the new interaction contracts.
- No API contract changes; the page still uses `GET /api/v1/workers` and `GET /api/v1/dispatch-queue`.

### 2026-05-23 — Phase 101 Java Spring worker demo runtime fix
- Reproduced the Java Spring worker demo exiting immediately: `DemoRunner` called `client.close()` directly after `client.start()`, and the README command used the SDK wrapper without selecting the demo project.
- Updated the demo runner to stay alive on a `CountDownLatch` until shutdown and close the worker client from `@PreDestroy`; dry-run bootRun now remains running instead of exiting immediately.
- Changed the demo default Worker Tunnel endpoint to `http://127.0.0.1:9998`, added/committed a local demo `gradlew`, ignored demo `.gradle/`, and fixed README/root verification commands.
- Started tikeo with `config/dev.toml`, started the Java demo with `TIKEO_WORKER_DRY_RUN=false`, and verified `/api/v1/workers` reports one online worker with `java` and `spring-boot` capabilities.
### 2026-05-24 — Phase 102 API token expiry and rotation policy
- Continued `.prompt/102-phase3-api-token-expiry-rotation.md` with the remaining API token governance gap from Phase 3.
- Added `auth.api_tokens` policy defaults for token default/min/max TTL and exposed the dev config values explicitly.
- `POST /api/v1/auth/api-tokens` now accepts bounded `expires_in_seconds`; out-of-policy TTL requests fail with a standard bad-request envelope.
- Added `POST /api/v1/auth/api-tokens/{id}/rotate` to preserve existing scopes, issue a replacement token, revoke the old token immediately, and audit the rotation.
- Multi-tenant namespace/app/worker-pool scope binding remains open.
Verification evidence:
- `rtk cargo test -p tikeo-server api_token_policy --all-features` failed before implementation for ignored TTL and missing TTL bound rejection, then passed after implementation.
- `rtk cargo test -p tikeo-server api_token --all-features` passed.
- `rtk cargo test -p tikeo-config default_auth_config --all-features` passed.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 120 tests.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint'` passed.
- `rtk bash -lc 'cd web && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.
- Runtime smoke on temporary 127.0.0.1:19090/19998 server verified healthz, login, scoped token creation with 900s TTL, rotation, old-token 401, and new-token scoped users read.
### 2026-05-24 — Web login bypass and root dashboard route
- Responded to user UX feedback that `/login` should not stay visible while a session token exists and the bare domain should have a default page.
- Added an explicit `/` route redirecting to `ROUTE_META.dashboard.path`, so direct domain access lands on the overview route before protected-route auth handling.
- `LoginPage` now checks `getAuthToken()` on mount and replace-navigates to the dashboard when a token is present; successful login still returns to the originally requested protected path when available.
- Added a source-level route regression test for the login bypass and root default route.
- Verification passed: targeted RED/green route test and full Web `lint`, `typecheck`, `bun test`, `build`.
### 2026-05-24 — Phase 104 API token namespace/app/worker-pool scope bindings
- Continued `.prompt/104-phase3-api-token-scope-bindings.md` by closing the remaining API-token multi-tenant binding foundation gap.
- Added `AccessScopeBinding` metadata with optional namespace/app/worker_pool fields; API token create/list, rotate, and `/auth/me` now preserve and expose bindings without plaintext token storage.
- Enforced namespace/app bindings for job list/create/trigger: bound tokens only see matching jobs and cannot create/trigger outside their binding.
- Enforced worker-pool visibility for `/api/v1/workers` using `worker_pool` / `worker-pool` worker labels in addition to namespace/app.
- Full tenant/app/worker-pool CRUD/UI and OIDC identity-to-tenant mapping remain open.
Verification evidence:
- `rtk cargo test -p tikeo-server api_token_scope_bindings --all-features` failed before implementation because bindings were ignored, then passed with 2 tests.
- `rtk cargo test -p tikeo-server api_token --all-features` passed with 6 tests.
- `rtk cargo fmt --all -- --check` passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `rtk cargo test --workspace --all-features` passed: 122 tests.
- `rtk cargo build --workspace --all-features` passed.
- `rtk cargo run -- --help` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml` passed.
- `rtk cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm` passed.
- `rtk cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` passed.
- `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test && bun run build'` passed.
- `rtk bash -lc 'cd sdks/java && ./gradlew test --warning-mode all --no-daemon'` passed.

### 2026-05-24 — Web route-level login bypass hardening
- Tightened the login-session UX fix after user feedback: `/login` now uses a route-level guard that redirects to the dashboard before rendering `LoginPage` when a client auth token exists.
- Kept the bare `/` default route pointing at the dashboard overview route.
- Added/updated route regression coverage so the login route must use the bypass wrapper instead of rendering the login page directly.
Verification evidence:
- `rtk bash -lc "cd web && bun test src/pages/__tests__/RouteAuth.test.tsx"` failed before the route-level guard, then passed after implementation.
- `rtk bash -lc "cd web && bun run lint && bun run typecheck && bun test src/pages/__tests__/RouteAuth.test.tsx"` passed.
- `rtk bash -lc "cd web && bun run build"` passed.


### 2026-05-24 — Phase 108 alert delivery attempt history
- Continued `.prompt/108-phase3-alert-delivery-attempt-history.md` by adding durable alert notification delivery attempt records.
- Added `alert_delivery_attempts` storage/entity/migration compatibility without database foreign keys, including provider, redacted target, delivered flag, status code, error, attempt number, retry state, next retry time, and created time.
- Script governance firing alert dispatch now persists one attempt per channel delivery result, including production-policy rejections as failed `retry_pending` attempts.
- Added `GET /api/v1/alert-delivery-attempts` with event/rule/provider/retry_state filters and OpenAPI schema coverage.
- Retry/backoff workers, DLQ processing, email/SMTP, and live external provider smoke remain future alert delivery hardening.
Verification evidence:
- `rtk cargo test -p tikeo-server alert_rules_api_records_script_governance_event_history --all-features` passed after fixing storage/root exports and route re-export.
- `rtk bash -lc "cargo test -p tikeo-server alert --all-features && cargo test -p tikeo-server openapi_json_contains_management_paths --all-features && cargo test -p tikeo-storage migration_creates_metadata_tables --all-features"` passed.
- `rtk bash -lc "cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings"` passed after test-size allow annotation.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.


### 2026-05-24 — Phase 109 dispatch latency metrics
- Continued `.prompt/109-phase3-dispatch-latency-metrics.md` by closing the local dispatch latency histogram gap.
- `DispatchQueueSloSummary` now includes completed dispatch count plus average/longest dispatch latency seconds calculated from terminal dispatch queue rows.
- `/api/v1/metrics/summary` records `tikeo_dispatch_queue_dispatch_latency_seconds` histogram snapshots and `tikeo_dispatch_queue_completed_total` gauge into the Prometheus recorder.
- Updated the Phase 3 Grafana dashboard template and dashboard regression coverage for the dispatch latency metric.
- Remaining observability hardening: live Prometheus/Grafana recording-rule validation and real OTLP collector/export smoke.
Verification evidence:
- `rtk cargo test -p tikeo-server metrics_summary_reports_storage_registry_and_alert_counts --all-features` failed before implementation because dispatch latency fields were missing, then passed after implementation.
- `rtk bash -lc "cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test -p tikeo-server metrics_summary_reports_storage_registry_and_alert_counts --all-features && cargo test -p tikeo-server --test grafana_dashboard --all-features"` passed.


### 2026-05-24 — Phase 110 email SMTP delivery foundation
- Continued `.prompt/110-phase3-email-smtp-delivery.md` by replacing the explicitly unsupported email branch with a local SMTP delivery foundation.
- `NotificationChannel::Email` now accepts recipients plus optional `smtp_url`/`from`, with `to`/`url` aliases for simple JSON channel configs.
- Email delivery sends a plain SMTP message to loopback `smtp://` endpoints only under explicit local policy; missing recipients/SMTP endpoint or non-loopback production policy fails closed with structured delivery errors.
- Alert delivery readiness now requires email recipients and an SMTP endpoint.
- Remaining alert hardening: production SMTP TLS/auth/secret handling, retry/backoff/DLQ processing, and live external provider smoke.
Verification evidence:
- `rtk cargo test -p tikeo-server email_dispatch_sends_plain_smtp_to_allowed_local_receiver --all-features` failed before implementation because email fields/delivery were missing, then passed.
- `rtk bash -lc "cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test -p tikeo-server email_dispatch_sends_plain_smtp_to_allowed_local_receiver --all-features && cargo test -p tikeo-server alert_rule_delivery_status_redacts_channel_targets_and_reports_readiness --all-features"` passed.


### 2026-05-24 — Phase 111 alert retry/DLQ foundation
- Continued `.prompt/111-phase3-alert-retry-dlq-foundation.md` by adding bounded retry processing for persisted alert delivery attempts.
- Storage can list due `retry_pending` attempts by `next_retry_at` and update retry state to `retry_consumed` / `dead_letter`.
- Retry processing reconstructs alert payloads from event/rule history, matches the persisted provider/redacted target back to current notification channels, appends a new delivery attempt, applies backoff, and dead-letters exhausted or unmatchable attempts.
- Added `POST /api/v1/alert-delivery-attempts:retry-due` returning scanned/retried/dead_lettered/skipped counts while keeping production-safe delivery policy by default.
- Remaining alert hardening: production SMTP TLS/auth/secret handling, continuous background retry worker scheduling, and live external provider smoke.
Verification evidence:
- `rtk cargo test -p tikeo-server retry_processor_delivers_due_attempt_and_marks_previous_consumed --all-features` failed before storage retry methods existed, then passed after implementation.
- `rtk bash -lc "cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test -p tikeo-server alert --all-features"` passed.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 112 alert retry background worker
- Added enabled-by-default `alert_retry` config for interval, batch size, max attempts, and retry backoff.
- Server startup now runs an ownership-gated background retry worker alongside HTTP, Worker Tunnel, schedule tick, and dispatch loops.
- Retry scans skip automatically when cluster status cannot schedule, keeping Raft followers from processing shared retry state.
- Remaining alert gap: production SMTP TLS/auth/secret handling and live external provider smoke.
Verification evidence:
- RED config test failed before `AlertRetryConfig` existed, then passed.
- RED ownership-gate test failed before `retry_once_if_owner` existed, then passed.
- Targeted fmt and clippy for `tikeo-server` / `tikeo-config` passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 113 tenant scope management API foundation
- Added persistent worker-pool metadata using soft namespace/app links and no database foreign keys.
- Added scope repository operations and authenticated `/api/v1/namespaces`, `/api/v1/apps`, and `/api/v1/worker-pools` create/list routes.
- Seeded tenant read/manage RBAC permissions and added OpenAPI coverage for the new management API.
- Remaining tenant gap: full web UI, destructive lifecycle/cascade policy, and OIDC identity-to-tenant mapping.
Verification evidence:
- RED management API test failed before the routes existed, then passed after implementation.
- Targeted fmt, clippy for storage/server, migration, OpenAPI, and management API tests passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 114 tenant scope management UI
- Added Web client methods for namespace, app, and Worker Pool create/list APIs.
- Added governed `/scopes` route and menu entry for `tenants:read`.
- Added `ScopesPage` with focused create cards guarded by `tenants:manage` plus metadata tables for namespace/app/Worker Pool visibility.
- Remaining tenant gap: destructive lifecycle/cascade policy and OIDC identity-to-tenant mapping.
Verification evidence:
- RED Web client/page tests failed before the API exports and page existed, then passed after implementation.
- Web lint, typecheck, targeted tests, and production build passed via RTK.
- Full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.

### 2026-05-24 — Phase 115 tenant scope lifecycle policy
- Added guarded DELETE routes for namespaces, apps, and Worker Pool metadata.
- Namespace deletion now rejects non-empty scopes with apps, Worker Pools, or jobs; app deletion rejects remaining Worker Pools or jobs.
- Worker Pool metadata can be deleted without affecting online Worker sessions or job records.
- Added Web console delete actions with confirmation copy that documents the non-empty rejection policy.
- Remaining tenant gap: OIDC identity-to-tenant mapping and advanced tenant isolation policy UI.
Verification evidence:
- RED backend delete lifecycle test failed with 404 before DELETE routes existed, then passed after implementation.
- RED Web page test required delete client/actions/confirm copy and passed after implementation.
- Targeted storage/server clippy, backend lifecycle/OpenAPI tests, Web lint/typecheck/targeted test/build passed via RTK.

- Phase115 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'

### 2026-05-24 — Phase 116 OIDC token exchange boundary
- Added an OIDC callback token-exchange boundary that posts authorization codes to the configured provider token endpoint with client credentials.
- Callback now requires an `access_token` response but still fails closed before session issuance until external identity mapping and user mapping land.
- Split OIDC network exchange helpers into `crates/tikeo-server/src/http/oidc.rs` to keep auth routing focused.
- Remaining OIDC gap: OIDC user-info subject mapping, nonce/state persistence, user/role/tenant mapping, and opaque session issuance.
Verification evidence:
- RED/green mock IdP test covers code exchange and proves the callback does not create a session from an unverified ID token.
- Targeted OIDC tests and tikeo-server clippy passed via RTK.

- Phase116 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'

### 2026-05-24 — Phase 117 OIDC UserInfo discovery boundary
- Added OIDC provider discovery and UserInfo retrieval after authorization-code token exchange.
- Callback now requires provider discovery `userinfo_endpoint` plus a non-empty key set, but still fails closed before trusting `access_token` signatures or creating sessions.
- Extended the mock IdP regression test to prove token, discovery, and UserInfo endpoints are each reached once while preserving the `{ code, message, data }` failure envelope.
- Remaining OIDC gap: OIDC user-info subject mapping, role/tenant mapping, nonce/state hardening, and opaque session issuance.
Verification evidence:
- Targeted OIDC tests and tikeo-server clippy passed via RTK.

- Phase117 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'

### 2026-05-24 — Phase 118 OIDC state/UserInfo opaque-session correction
- Corrected the OIDC direction: tikeo login state remains opaque session tokens in `auth_sessions` plus moka cache; provider tokens are never used as local session state.
- Added persisted hashed OIDC authorization states with one-time callback consumption and replay rejection.
- Replaced the current provider-token-as-session path with token exchange + provider UserInfo fetch, then fail-closed until external subject mapping creates a local opaque tikeo session.
- Added `oidc_auth_states` storage/entity/repository support with soft, standalone metadata and no foreign keys.
- Remaining OIDC gap: external subject to local user/role/tenant mapping, nonce/state hardening, and opaque session issuance from mapped identity.
Verification evidence:
- Targeted OIDC tests cover generated state, one-time state consumption, token exchange, UserInfo fetch, and fail-closed local session mapping.
- Storage and server clippy passed via RTK after the correction.

- Phase118 full verification passed: rtk bash -lc 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features && cargo run -- --help >/tmp/tikeo-help.out && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml && cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm && cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings && cd web && bun run lint && bun run typecheck && bun test && bun run build && cd ../sdks/java && ./gradlew test --warning-mode all --no-daemon'


### 2026-05-24 — Phase 119 real OTLP exporter smoke
- Completed the OpenTelemetry distributed tracing Phase 3 item with real OTLP HTTP exporter startup wiring.
- Added focused `observability::tracing::TracingRuntime` using OpenTelemetry SDK plus `tracing-opentelemetry`, keeping local tracing disabled-by-default unless configured.
- Added local collector smoke coverage proving exported spans POST a non-empty OTLP protobuf payload to `/v1/traces` and carry configured exporter headers.
- Remaining observability gap is Prometheus/Grafana recording-rule validation, not OTLP tracing.
Verification evidence:
- `rtk cargo test -p tikeo-server --test otel_exporter_smoke --all-features` passed.
- `rtk cargo test -p tikeo-server observability_status_reports_default_and_configured_otlp_without_collector --all-features` passed.
- Targeted `rtk cargo fmt --all -- --check` and `rtk cargo clippy -p tikeo-server --all-targets --all-features -- -D warnings` passed.
- Phase119 full verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.
- SDK/Web verification passed: `rtk bash -lc 'set -euo pipefail; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml; cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm; cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings; cd web; bun run lint; bun run typecheck; bun test; bun run build; cd ../sdks/java; ./gradlew test --warning-mode all --no-daemon'`.


### 2026-05-24 — Phase 120 Java Spring Boot Starter lifecycle completion
- Completed the Java Spring Boot Starter SDK runtime behavior with a `TikeoWorkerLifecycle` SmartLifecycle bridge.
- Starter now auto-starts and stops the configured `TikeoWorkerClient` with the Spring application lifecycle while preserving processor scanning.
- Added `tikeo.worker.enabled` and `tikeo.worker.auto-startup` controls for disabling worker beans or manual startup.
- Updated the Spring worker demo so lifecycle ownership lives in the starter instead of the demo runner.
Verification evidence:
- RED starter test failed before `TikeoWorkerLifecycle` existed, then passed after implementation.
- `rtk bash -lc 'cd sdks/java && ./gradlew :tikeo-spring-boot-starter:test --warning-mode all --no-daemon'` passed.
- Phase120 Java verification passed: `rtk bash -lc 'set -euo pipefail; cd sdks/java; ./gradlew test --warning-mode all --no-daemon; cd ../../examples/java/spring-worker-demo; ./gradlew build --warning-mode all --no-daemon'`.

### 2026-05-24 — Worker identity lifecycle design
- Added `design/worker-identity-lifecycle-design.md` covering Worker identity, session lifecycle, lost-worker classification, generation/fencing token, UI/history behavior, and staged implementation slices.
- Updated `design/tikeo-architecture-design.md` roadmap with a Phase 4 Worker identity/session lifecycle governance item.
Verification evidence:
- Documentation-only change reviewed with `rtk git diff --check`.


### 2026-05-24 — Worker identity lifecycle design: bare metal support
- Extended `design/worker-identity-lifecycle-design.md` so Worker identity/session lifecycle governance explicitly supports bare metal, VM, systemd, Supervisor, and Windows Service deployments in addition to K8s/Docker.
- Added host-id + instance-slot identity guidance, auto identity-mode precedence, and route-map updates in the architecture roadmap wording.
Verification evidence:
- Documentation-only change reviewed with `rtk git diff --check`.

### 2026-05-24 — Phase3/Phase4 service-priority rebalance
- Updated `design/tikeo-architecture-design.md`, `.memory/next.md`, and `.prompt/121-phase3-phase4-service-priority-rebalance.md` with P0/P1/P2 ordering.
- P0 now prioritizes features that directly affect service use: OIDC mapped opaque login, real TLS/mTLS, Worker lifecycle identity, deployment bootstrap, and production alert delivery.
Verification evidence:
- Documentation-only change reviewed with `rtk git diff --check`.

### 2026-05-25 — P0 OIDC mapped opaque session issuance
- Completed P0 OIDC external-subject mapping: `oidc_identities` maps `(issuer, subject)` to a local username plus optional namespace/app/worker_pool bindings without database foreign keys.
- OIDC callback now uses provider tokens only for token exchange/UserInfo, then issues a local opaque `atk_` session from `auth_sessions` + moka after mapping succeeds; unmapped identities still fail closed.
- Split new code by responsibility: storage identity repository, OIDC callback completion, and session metadata encoding are separate modules; no new clippy allow was added.
Verification evidence:
- RED mapped-subject callback test failed before `OidcIdentityRepository` existed, then passed.
- Targeted OIDC tests, storage/server clippy, and fmt check passed via RTK.
- P0 OIDC mapped opaque session full backend verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.

### 2026-05-25 — P0 real TLS/mTLS listeners
- Added real HTTP HTTPS serving with rustls and a TLS smoke test that reaches an axum route over `https://127.0.0.1`.
- Added shared TLS material loading for rustls/tonic, Worker Tunnel TLS/mTLS startup wiring, and a `WorkerTunnelRuntime` dependency bundle to avoid adding clippy argument-count allowances.
- HTTP TLS rebuilds the acceptor from configured files for each new connection so certificate/key/CA file rotation is picked up without process restart.
- Transport security diagnostics now report `plaintext`, `tls`, `mtls`, or `tls_config_error` and check certificate/key/CA file readability instead of claiming `tls_pending_listener`.
Verification evidence:
- `rtk cargo test -p tikeo-server http_tls_listener_serves_https_when_configured --all-features` passed.
- `rtk cargo test -p tikeo-server transport_security_status_reports_defaults_and_partial_mtls_config --all-features` passed.
- Full backend verification passed: `rtk bash -lc 'set -euo pipefail; cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo test --workspace --all-features; cargo build --workspace --all-features; cargo run -- --help >/tmp/tikeo-help.out'`.

### 2026-05-25 — P0 Worker lifecycle Slice A: generation/fencing baseline
- Reviewed `design/worker-identity-lifecycle-design.md` before implementation.
- Added Worker session generation/fencing to proto, server registry/service, `/workers` summary, Rust SDK, and Java SDK.
- Same logical worker registrations now replace old generations; stale heartbeat/log/result messages are fenced from writes; `/workers` shows latest online generation only.
Verification evidence:
- Server worker tests, Rust SDK tests/clippy, Java SDK tests, and full backend verification passed via RTK.

## 2026-05-25 — HTTP/mod.rs cleanup and source-size gate

Task:
- User required splitting `crates/tikeo-server/src/http/mod.rs` and enforcing that single source files stay under 1500 lines; module entry files such as `mod.rs` / `lib.rs` should not hold implementation bodies.

Changes:
- Split `http/mod.rs` into focused `state`, `router`, `server`, `health`, and test shard modules; `http/mod.rs` now only declares/re-exports module entry points.
- Split oversized `cluster/raft_rs.rs` tests, storage migration identifiers/index/column helpers, and workflow repository types/conversions/validation/queue/events so all checked source files are <= 1500 lines.
- Added script release-gate preview API: `GET /api/v1/scripts/{id}/release-gate`, including OpenAPI DTO/path and tests.

Verification:
- Max source line count check: 1495.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.

Commit/push:
- Commit: `4395b77` (Make signed script releases auditable)
- Push: succeeded to `origin/main` (`8c2ae07..4395b77`).
- Commit: `9405b7d` (Keep module boundaries from hiding oversized implementation)
- Push: succeeded to `origin/main` (`6925be3..9405b7d`).

## 2026-05-25 — P1 script signature local verification boundary

Task:
- Continue P1 script production governance after source-size cleanup.

Changes:
- Added default-disabled `script_governance.release_signature_secret_ref` config.
- Added local env-secret-backed release signature verification for script publish/rollback; signatures bind script id, immutable version number, content digest, and approval ticket.
- Release-gate preview now reports whether signature verification is configured.

Verification:
- Max source line count check: 1495.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.

Commit/push:
- Commit: `4395b77` (Make signed script releases auditable)
- Push: succeeded to `origin/main` (`8c2ae07..4395b77`).
- Commit: `00a895e` (Make script release approvals verify a local signature)
- Push: failed after two attempts.
  - Attempt 1: `OpenSSL SSL_read: unexpected eof while reading`.
  - Attempt 2: `Failed to connect to github.com port 443 after 136460 ms`.
- Next step: retry `git push` when GitHub/network connectivity is available.
- Push retry: succeeded to `origin/main` (`28d84d1..2fe92b7`).

## 2026-05-25 — P1 script release signature metadata persistence

Task:
- Continue P1 script production governance by persisting/displaying successful signed release metadata after local signature verification.

Changes:
- Added nullable script release signature metadata columns and SQLite compatibility migration.
- `ScriptSummary` now includes verified release signature evidence.
- Publish/rollback persist approval ticket, signature, verifier, and verification timestamp after configured local verification succeeds.
- Web Scripts page displays signed/unsigned release state plus signature detail fields.

Verification:
- Max source line count check excluding `web/node_modules` and `web/dist`: 1495.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

Commit/push:
- Commit: `4395b77` (Make signed script releases auditable)
- Push: succeeded to `origin/main` (`8c2ae07..4395b77`).

## 2026-05-25 — P1 script release grant payload boundary

Task:
- Continue P1 script production governance by designing URL/File/Secret grant payloads that remain fail-closed until verified.

Changes:
- Added core `ScriptReleaseGrantSet` with explicit URL/File/Secret grant categories.
- Added HTTP/OpenAPI `ScriptReleaseRequest.grants` DTO shape and Web API client types.
- Publish/rollback reject any non-empty grant payload before release pointer movement because verified grant enforcement is not implemented yet.

Verification:
- Max source line count check excluding `web/node_modules` and `web/dist`: 1495.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

Commit/push:
- Commit: `11633a0` (Make script release grants explicit and fail closed)
- Push: succeeded to `origin/main` (`472b34d..11633a0`).

## 2026-05-25 — P1 script release grant evidence persistence

Task:
- Continue P1 script production governance by adding persistence/display boundaries for verified URL/File/Secret grant evidence without enabling execution access.

Changes:
- Added release-pointer grant evidence columns and SQLite compatibility migration.
- Added storage DTO and repository plumbing for verified grant evidence.
- Web Scripts detail displays grant evidence if a future verifier produces it.
- HTTP publish/rollback still do not produce verified grant evidence; non-empty grants remain fail-closed.

Verification:
- Max source line count check excluding `web/node_modules` and `web/dist`: 1495.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

Commit/push:
- Commit: `66a348a` (Persist script release grant evidence before enforcement)
- Push: first attempt failed with GitHub connection timeout; retry succeeded to `origin/main` (`df35944..66a348a`).

## 2026-05-25 — P1 local signed release grants

Task:
- Move faster and complete at least one P1 task item by closing the local script release governance loop for signed URL/File/Secret grants.

Changes:
- Local env-secret release signatures now bind canonical grants JSON.
- Publish/rollback persist grant evidence after local signature verification succeeds.
- Unconfigured deployments remain fail-closed for grants and signature metadata.
- Worker-side URL/File/Secret access remains disabled; release governance evidence only.
- Roadmap marks the P1 script approval/signature/grants release-gate item complete.

Verification:
- Max source line count check excluding `web/node_modules` and `web/dist`: 1495.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `cargo run -- --help >/tmp/tikeo-help.out` passed.
- `cd web && bun run typecheck && bun run lint && bun test && bun run build` passed.

Commit/push:
- Commit: `6f63564` (Complete local signed script release grants)
- Push: first attempt failed with TLS EOF; retry succeeded to `origin/main` (`cc3b146..6f63564`).

## 2026-05-25 — Correct P1 script governance roadmap checkboxes

Task:
- User pointed out the P1 script governance task was not visibly marked complete in the design roadmap and asked what Worker-side URL/File/Secret access disabled meant.

Clarification:
- The completed P1 item is the release-governance gate: policy/signature/grants are verified before the release pointer moves, and evidence is persisted.
- “Worker-side URL/File/Secret access remains disabled” means runtime execution still does not open network, mount host paths, or inject secrets into scripts. That is an intentional execution-sandbox safety boundary, not a failure of the release gate.

Changes:
- Marked the script strategy/governance roadmap entries complete in all matching design sections.
- Clarified that external KMS/PKI is future enhancement beyond the local env-secret verifier closure.

Verification:
- Documentation-only correction; `git diff --check` passed.

Commit/push:
- Pending at time of log entry.

## 2026-05-25 16:xx — P1 Worker runtime grant enforcement
- User flagged missing Worker-side closure and Java SDK omission. Implemented server/Rust/Java SDK protocol and runtime boundary updates.
- Verification run: `cargo check -p tikeo-server --all-features`; targeted server dispatch grant test; Rust SDK container grant tests + `cargo check --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`; Java `./gradlew :tikeo:generateProto :tikeo:test --tests net.tikeo.core.GrpcTikeoWorkerClientTest`.
- Constraint check: source files excluding generated/build artifacts remain under 1500 lines; largest touched server dispatcher is 1429 lines.

## 2026-05-25 16:xx — P1 OIDC tenant scope mapping
- Implemented list/upsert/delete OIDC identity mapping API guarded by `tenants:read/manage`, storage list/delete helpers, OpenAPI/router wiring, AuthSession scope metadata, and Scopes page OIDC mapping card.
- Verification run: `cargo check -p tikeo-server --all-features`; `cargo test -p tikeo-server oidc --all-features`; `cd web && bun run typecheck && bun test src/pages/__tests__/ScopesPage.test.tsx`.
- Source-size check remains under 1500 lines; largest source is `crates/tikeo-storage/src/repository/workflow.rs` at 1495 lines.

## 2026-05-25 17:xx — P1 Prometheus/Grafana recording-rule validation
- User clarified `.prompt` files are next-chapter prompts; updated `.prompt/README.md` and created next prompt `132-p1-prometheus-grafana-recording-rules.md` before closing observability slice.
- Implemented Prometheus recording rules/config, Compose observability profile, Grafana recording-query updates, and runbook.
- Verification run: `cargo test -p tikeo-server grafana --all-features`; `cargo check -p tikeo-server --all-features`; source-size check excluding generated/build artifacts.

## 2026-05-25 17:xx — P1 Go SDK dry-run foundation
- Started P1 Go/Python SDK work with Go because Go toolchain is present; `protoc` is absent, so this slice uses official `google.golang.org/grpc` / `google.golang.org/protobuf` dependencies and vendors the proto contract while deferring generated bindings.
- Verification run: `(cd sdks/go/tikeo && go test ./...)`; `(cd examples/go/worker-demo && go test ./...)`.

### 2026-05-25 P1 Go SDK official gRPC/protobuf foundation
- Added Go SDK foundation under `sdks/go/tikeo` with official `google.golang.org/grpc` ClientConn creation, official protobuf/grpc generated Worker Tunnel bindings, endpoint normalization, dry-run registration/heartbeat/task interfaces, and standalone demo tests.
- User explicitly deferred Python and Node.js SDKs; next SDK slice should stay on Go Worker Tunnel run-loop ergonomics.

### 2026-05-25 Phase4 P0 Worker lifecycle transport evidence
- Continued Worker identity/session lifecycle governance strictly against `design/worker-identity-lifecycle-design.md`. Added transport-error evidence path so gRPC stream errors or non-graceful stream end mark the current session offline/degraded with `transport_error` instead of waiting for lease timeout.
- Confirmed Python/Node SDK and remaining Go SDK run-loop work are deferred; next Phase4 P0 slice is deployment/operations bootstrap.

### 2026-05-25 Phase4 P0 deployment bootstrap
- Completed Compose/systemd/bare-metal deployment bootstrap docs and templates, including stable Worker identity env guidance, systemd Rust worker demo unit/env, and `deploy/smoke/worker-bootstrap-smoke.sh` readiness + dry-run worker check.
- Go run-loop/Python/Node SDK work remains deferred; Helm stays deferred until external DB, secrets, gateway, and TLS parameters stabilize.

- 2026-05-27 09:38: Locked script language identifiers to full JavaScript / TypeScript values so the web script editor can select explicit CodeMirror JavaScript vs TypeScript linting modes. Legacy js / ts aliases remain parse-compatible; dispatch and Java worker capabilities now canonicalize to script:javascript / script:typescript. Verified with targeted Rust, Web, and Java tests.

- 2026-05-27 09:51: Removed raw WASM from Web script create/edit language options. Direct language=wasm remains documented as a historical/low-level compatibility path, while normal scripts use sandbox.backend auto/wasmtime/wasmedge/srt/deno/v8/docker/podman/custom instead of WASM as a script type.

- 2026-05-27 09:54: Added local dev seed script examples and API jobs for every Web script language enum: shell, python, javascript, typescript, powershell, and rhai. Applied scripts/dev-seed.sh to tikeo-dev.db and verified six script_language_examples plus six script_jobs.

- 2026-05-27 12:55: Changed script dispatch matching to unified worker capability `script` so Python/JavaScript/TypeScript/etc. are dispatched to script-capable workers instead of being blocked by missing `script:<language>` capability. Legacy `script:<language>`, `script:*`, and `*` remain compatible for normal scripts; direct WASM modules still require `script:wasm`. Worker-side sandbox selection remains based on binding language plus sandbox.backend.

- 2026-05-27 14:10: Fixed Java demo shell script execution path: sandbox=auto now resolves native scripts to srt/native-script semantics, Spring starter registers a development-only local shell subprocess runner instead of sending real shell scripts through the limited bundled WASI shell micro-runtime, and the demo image config now uses JavaScript/TypeScript keys. Verification: Java SDK script tests, Spring starter auto-configuration tests, and spring-worker-demo tests passed.

### 2026-05-31 — Service Account upgraded to first-class SDK identity
- Upgraded SDK Management API-Key identity model from implicit service_account_name input to managed Service Account resources. Admins can create/list/update/disable Service Accounts, API-Key creation must choose an existing active Service Account, and disabling a Service Account revokes bound active keys.
- API-Key authentication now checks the bound Service Account remains active and uses the current Service Account namespace/app as the authorization boundary.
- Web `/api-keys` now loads/manages Service Accounts and signs API keys against selected identities; smoke and API client tests use the new flow.
- Verification: `cargo check -p tikeo-server`, `cargo test -p tikeo-server sdk_api_key -- --nocapture`, `cargo test -p tikeo-server disabling_service_account_revokes_bound_sdk_keys -- --nocapture`, `cd web && bun run typecheck`, `cd web && bun test --run client.test.ts`.

### 2026-05-31 — Fix tenant Secret creation drawer
- Fixed Scopes/Tenant page "新建 Secret" buttons: they now open a Secret drawer, submit `createSecret`, reset the form, close the drawer, show success, and refresh the list.
- Added source regression assertions so the tenant page must keep `handleSecretCreate` and `drawer === 'secret'` rendering.
- Verification: `cd web && rtk bun test --run src/pages/__tests__/ScopesPage.test.tsx`; `cd web && rtk bun run typecheck`; `rtk git diff --check -- web/src/pages/ScopesPage.tsx web/src/pages/__tests__/ScopesPage.test.tsx`.

### 2026-05-31 — Tenant Secret references made structured
- Replaced Secret creation `valueRef` prefix-string input with typed `reference` payload variants: env, vault, and external secret provider. Server now validates per-kind fields and stores normalized JSON in the existing `value_ref` storage column instead of `env:/vault:/secret:` protocol strings.
- Web tenant Secret drawer now uses reference type selection plus scoped fields, and the list renders parsed structured references as tags/fields rather than exposing a raw "Value Ref" string contract.
- Verification: `rtk cargo test -p tikeo-server tenant_secret -- --nocapture`; `rtk cargo check -p tikeo-server`; `cd web && rtk bun run typecheck`; `cd web && rtk bun test --run src/pages/__tests__/ScopesPage.test.tsx`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — Web dark-mode custom surface coverage
- Submitted the current local dev DB snapshot as requested before starting the theme fix.
- Reworked Web custom styles to use shared light/dark CSS variables for shell, cards, heroes, workflow canvases, topology fullscreen, API-Key panels, worker stats, and scheduling advice modules instead of hard-coded light surfaces.
- Added `ThemeDarkMode.test.tsx` coverage to lock the custom module dark-mode variable contract; also cleaned existing lint blockers in Calendar and Jobs pages so Web lint is green.
- Verification: `cd web && rtk bun test`; `cd web && rtk bun run lint`; `cd web && rtk bun run typecheck`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — Modern animated tikeo logo
- Added a repo-native `TikeoLogo` SVG React component with task-flow track, three orchestration nodes, arrow/tick motion, pulse animation, and dark-mode-compatible styling.
- Replaced the sidebar placeholder brand mark and added wordmark logo treatment to login and bootstrap setup pages.
- Verification: `cd web && rtk bun test --run src/pages/__tests__/TikeoLogo.test.tsx`; `cd web && rtk bun run typecheck`; `cd web && rtk bun run lint`; `cd web && rtk bun test`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — Refine tikeo logo away from worm-like curve
- Reworked the animated logo from a curved route into a harder-edged hexagonal control-plane mark with T-shaped dispatch trunk, three task nodes, and forward arrow motion.
- Added `web/src/assets/tikeo-logo.svg` as a static README-friendly logo asset and embedded it at the top of `README.md`.
- Verification: `cd web && rtk bun test --run src/pages/__tests__/TikeoLogo.test.tsx`; `cd web && rtk bun run typecheck`; `cd web && rtk bun run lint`; `cd web && rtk bun test`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — Enlarge and theme-adapt animated tikeo logo
- Increased the in-app sidebar logo from 44px to 52px and auth/setup logo from 64px to 76px with a larger wordmark.
- Moved animated logo colors to theme-aware CSS variables so primary color and dark mode affect shell gradient, accent, track highlights, node fill, and inner panel contrast.
- Verification: `cd web && rtk bun test --run src/pages/__tests__/TikeoLogo.test.tsx`; `cd web && rtk bun run typecheck`; `cd web && rtk bun run lint`; `cd web && rtk bun test`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — Make in-site tikeo logo visibly larger
- Investigated why the previous logo resize was not visible: the change was small and the SVG viewBox still had enough padding that the rendered mark looked nearly unchanged.
- Increased the sidebar logo to 64px and auth/setup logo to 96px, tightened the animated SVG viewBox, added explicit CSS sizing, and wired the static logo as the web favicon.
- Verification: `cd web && rtk bun test --run src/pages/__tests__/TikeoLogo.test.tsx`; `cd web && rtk bun run typecheck`; `cd web && rtk bun run lint`; `cd web && rtk bun test`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — Add browser/system theme following
- Extended Web theme preference from light/dark to light/dark/system, with system as the default for invalid or missing stored values.
- App now resolves the active theme from `prefers-color-scheme`, listens for browser/OS theme changes, writes `data-theme` from the resolved mode, and keeps `data-theme-preference` for diagnostics.
- Replaced the binary theme switch with a three-option selector: 跟随系统 / 亮色 / 暗色.
- Verification: `cd web && rtk bun test --run src/pages/__tests__/ThemeMode.test.ts`; `cd web && rtk bun run typecheck`; `cd web && rtk bun run lint`; `cd web && rtk bun test`; `rtk git diff --check -- . ':!.omx'`.

### 2026-05-31 — GitHub Actions CI and tag-only release pipelines
- Expanded `.github/workflows/ci.yml` so push/PR validates server Rust workspace, Web lint/typecheck/test/build, Java SDK test/jar/source jar, Rust SDK fmt/clippy/test/package, and Docker server/web image builds without pushing.
- Added `.github/workflows/release.yml` for `v*` tags only: cross-platform server archives, Web dist archive, Java SDK archive, Rust SDK crate archive, GitHub Release upload, and Docker Hub image push for `tikeo-server` and `tikeo-web`.
- Added `.github/RELEASE_SETUP.md` with required placeholders/secrets: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`, and Docker Hub repositories.
- Fixed Rust SDK test/proto drift and clippy blockers so the new SDK CI lane is actually green.
- Verification: workflow contract unittest; `cargo check --workspace --all-features`; `cd web && rtk bun run build`; `(cd sdks/java && ./gradlew test jar sourcesJar)`; Rust SDK clippy/test; `rtk git diff --check -- . ':!.omx'`.

### 2026-06-01 — Fix Terraform provider and K8s CRD test verification
- Resolved Go dependency errors by executing `go mod tidy` in `deploy/terraform/provider/` and `deploy/k8s/operator/` sub-modules, restoring missing `go.sum` lock files.
- Executed local verification tests: ran `deploy/smoke/terraform-provider-smoke.sh` and `deploy/smoke/k8s-operator-dry-run-smoke.sh` successfully.
- Validated K8s CustomResourceDefinition YAML using `kubeconform` dry-run, confirming the CRD schema parses correctly.
- Patched status in `design/server-web-java-joint-executable-test-status-plan.md` and `design/server-web-java-joint-automation-test-plan.md` for `G-TF-001` (Terraform provider build/test), `G-K8S-001` (CRD schema check), and `G-K8S-002` (Operator reconcile dry-run) to `✅ 通过`.
- Verification: local executions of all three dry-run scripts passed successfully.

### 2026-06-02 — 联合自动化测试方案/状态复核
- 重新核对 `design/server-web-java-joint-executable-test-status-plan.md` 与 `design/server-web-java-joint-automation-test-plan.md` 中的测试项状态。
- 执行状态计划当前 P0-A/P0-B/P0-C/P0-D/P1-E/P1-F/P2-G/数据库专项均为 `✅ 通过`，总览为 80/80 通过、0 待执行/失败/阻塞/跳过。
- 修正测试方案中残留的旧 `⏳ 待执行`：环境、端口、功能预期断言、C-WEB、D-WEB、E-KEY、G-TF、CI 分层与排障清单均已按现有证据同步为 `✅ 通过` / `✅ 已配置` / `✅ 已沉淀`。
- 当前项目状态：Server + Web + Java SDK/Demo 联合自动化测试闭环已完成；后续增强仅剩将部分 DOM/JSON 截图证据升级为真实浏览器 screenshot/video CI 产物，不阻塞当前联调验收。
- Verification: 文档状态 grep 确认无测试项级 `⏳/❌/🚧/⏭️/🔄` 残留（仅状态口径说明和总览表头保留图标）。

### 2026-06-02 — Java SDK Java 17+ 与 Spring Boot 2/3/4 starter 分层修正
- 修正上一版错误方向：主 `tikeo-spring-boot-starter` 不再降级，恢复/保持 Spring Boot 4.x + Spring Framework 7.x 依赖。
- 新增 `tikeo-spring5` / `tikeo-spring6` 兼容 Spring adapter 模块，复用 `tikeo-spring` 源码但分别依赖 Spring Framework 5.3 / 6.2。
- 新增 `tikeo-spring-boot2-starter` / `tikeo-spring-boot3-starter`，复用主 starter 源码与测试，分别依赖 Spring Boot 2.7 / 3.5；demo 改用 Boot3 compat starter。
- Java SDK 全模块保持 `--release 17`，修掉 Java 21 `List.getFirst()` API 残留，保证 Java 17+ 消费者可用。
- Verification: `cd sdks/java && ./gradlew projects --no-daemon`; `cd sdks/java && ./gradlew clean test --no-daemon`; `cd sdks/java && ./gradlew :tikeo-spring-boot-starter:test :tikeo-spring-boot2-starter:test :tikeo-spring-boot3-starter:test --no-daemon`; `cd examples/java/spring-worker-demo && ./gradlew clean test --no-daemon`; dependency matrix grep confirmed Boot4/Spring7, Boot2/Spring5, Boot3/Spring6.

### 2026-06-02 — Java compat modules now contain explicit src trees
- Reworked the Spring Boot 2/3 compatibility modules from Gradle source-set indirection into real modules with their own `src/main` and `src/test` trees.
- Confirmed `tikeo-spring5`, `tikeo-spring6`, `tikeo-spring-boot2-starter`, and `tikeo-spring-boot3-starter` each contain concrete Java sources/resources/tests where applicable.
- Verification: explicit source-count checks for all four modules; `cd sdks/java && ./gradlew clean test --no-daemon`; `cd examples/java/spring-worker-demo && ./gradlew clean test --no-daemon`; source line check max Java file 1043 lines; `git diff --check`.

### 2026-06-02 — Java demo covers Spring starter compatibility use cases
- Added a Java demo regression/use-case test for the Spring Boot starter compatibility matrix: the demo is explicitly validated as a Spring Boot 3.x application using `tikeo-spring-boot3-starter`.
- The demo test now also verifies the Java SDK publishes separate Boot 2, Boot 3, and Boot 4 starter modules plus Spring 5/6 adapters with concrete `src/main` and `src/test` trees, not Gradle-only aliases.
- Updated the Java demo README with the compatibility matrix and test-suite entry so the expected starter choice is visible to users.
- Verification: `cd examples/java/spring-worker-demo && ./gradlew clean test --no-daemon`; `cd sdks/java && ./gradlew clean test --no-daemon`; source-count checks for compat modules; `git diff --check -- examples/java/spring-worker-demo sdks/java .memory`.

### 2026-06-02 — Split Java Spring Boot demos by major version
- Added independent Java demo projects under `examples/java/spring-boot2-worker-demo`, `examples/java/spring-boot3-worker-demo`, and `examples/java/spring-boot4-worker-demo` instead of relying on a single Boot3 demo plus matrix assertions.
- Each demo has its own Gradle wrapper/project files, source tree, tests, README, default port, worker labels, processor examples, management API examples, and matching tikeo starter dependency.
- Boot2 demo intentionally uses the Spring Boot 2.7 BOM without applying the Boot Gradle plugin because the Boot 2 plugin is not compatible with the current Gradle 9.5.1 API; the application remains a standard `@SpringBootApplication` with Boot2 web/test dependencies.
- Verification: Boot2/Boot3/Boot4 demo `./gradlew clean test --no-daemon` all passed from their own directories.

### 2026-06-02 — Removed obsolete generic Java Spring demo
- Removed `examples/java/spring-worker-demo` after adding explicit Spring Boot 2/3/4 demos; keeping it would create duplicate Boot3 maintenance and path ambiguity.
- Updated current README, smoke scripts, integration docs, test plans, and architecture docs to point at `spring-boot3-worker-demo` for default Java joint testing and document the three-version demo split.
- Verification pending in this turn: Boot2/Boot3/Boot4 demo tests, Java SDK tests, shell syntax, and diff check.

### 2026-06-02 — Java SDK Gradle ownership split per module
- Reworked `sdks/java` from a root build script that injected all module plugins/dependencies into a root aggregator plus per-module `build.gradle.kts` files.
- Root `sdks/java/build.gradle.kts` now owns only aggregation and shared group/version; each SDK module owns `java-library`, `maven-publish`, dependencies, tests, and framework/plugin constraints.
- `tikeo` owns the protobuf plugin and gRPC/protobuf dependencies; Spring adapters own their Spring Framework major version; Boot starters own their Boot BOM and starter/autoconfigure dependencies.
- Verification: `cd sdks/java && ./gradlew projects --no-daemon`; `cd sdks/java && ./gradlew clean test publishToMavenLocal --no-daemon`; Boot2/Boot3/Boot4 demo `./gradlew clean test --no-daemon`.

### 2026-06-05 — 同步 2026-06-04 Worker/SDK parity 文档与下一阶段提示词
- 按 `prompt.md` 接手协议读取 prompt、memory、最新 `.prompt/146`、昨日提交与相关 design/docs。
- 将 2026-06-04 已完成状态同步到架构、Worker 生命周期、Java 多 worker 联调报告、联合自动化测试状态/方案和 integration docs：Worker 可见性快照持久化、Web Worker 分组/调度队列二级页、Go/Rust SDK demo 默认 live、assignment-token 日志、script runner capability 对齐、CI run `26947829951` success。
- 新增 `.prompt/147-phase4-cross-language-worker-parity-and-persistence-hardening.md`，下一步聚焦把当前手动 Java/Go/Rust Worker parity 与 server restart persistence 验收固化为 executable harness。
- Verification: 文档轻量检查与 git diff check 在本轮执行后记录。

### 2026-06-05 — 反伪实现审计与跨语言 Worker harness 闭环
- 按生产级宪法复查 server/web/sdks/demo，修复 schedule cursor 内存状态、Raft unknown command deferred 语义、Go/Rust/Java 不可用 script runner 假能力广告、Rust success outcome 消息、Web i18n 机械翻译残留。
- 新增 `deploy/smoke/cross-language-worker-parity-smoke.sh`，一键启动临时 server/web + Java Boot2/Boot3/Boot4 + Go + Rust worker，覆盖结构化 worker parity、Go/Rust 实例日志、server restart persisted snapshot、worker_pool scoped filtering 与 Web worker route smoke。
- 最新证据：`.dev/reports/cross-language-workers-20260605T032108Z-202626/cross-language-workers-20260605T032108Z-202626.json`。
- Verification: `cargo test -p tikeo-storage -- --nocapture`; `cargo test -p tikeo-server -- --nocapture`; Go SDK/demo tests; Rust SDK/demo tests; `cd sdks/java && ./gradlew test --no-daemon`; `cd web && bun run typecheck && bun test --run src/i18n/i18n.test.ts`; `deploy/smoke/cross-language-worker-parity-smoke.sh`.

### 2026-06-05 — GitHub CI coverage completed for Go and demos
- Audited `.github/workflows/ci.yml`: previous main CI covered Server/Web/Java SDK/Rust SDK/Docker only; it missed Go SDK/demo, Go deploy tooling, Java Boot2/3/4 demos, Rust demo, and the new cross-language smoke harness.
- Added CI jobs: `Go SDK and demo`, `Go deploy tooling`, `Java worker demos`, `Rust worker demo`, and `Cross-language worker smoke`; Docker build validation now depends on all quality gates.
- Verification: workflow YAML parse; `git diff --check`; Go SDK/demo/deploy `go test ./... -count=1`; Rust demo fmt/clippy/test; Java Boot2/3/4 demo `./gradlew test --no-daemon`.


### 2026-06-05 — 数据库迁移版本化专项
- 用户要求继续生产化风险清理；选择数据库 migration/versioning 作为下一步 P0。
- 审计发现 `tikeo-storage::connect_and_migrate` 在 SeaORM Migrator 后追加 SQLite-only `ensure_*_schema_compatibility`，升级补丁没有进入 `seaql_migrations`，属于不可审计/不可复盘的启动隐式 patch。
- 先写失败测试 `sqlite_schema_compatibility_upgrade_is_tracked_as_versioned_migration`，确认当前 migration history 只有 `['mod']`。
- 实现：新增 `migration/sqlite_compat.rs` 显式 SeaORM migration，迁入原 SQLite legacy schema compatibility；`connect_and_migrate` 删除未记录 post-hook；foreign-key soft-link rebuild 拆为子模块。
- 验证：targeted migration tests、`cargo test -p tikeo-storage -- --nocapture`、`cargo test -p tikeo-server -- --nocapture`、`cargo clippy -p tikeo-storage -p tikeo-server --all-targets --all-features -- -D warnings`、`scripts/db-compat-smoke.sh`、`cargo build -p tikeo-server --all-features`、`git diff --check` 均通过。


### 2026-06-05 — CI 分组重排，不等待远端 CI
- 用户要求 CI 按 server、web、java sdk+demo、rust sdk+demo、go sdk+demo、python sdk+demo、nodejs sdk+demo、其他分组展示。
- 先补 workflow contract RED，确认旧 workflow 仍拆成 `java-sdk/java-demos/rust-sdk/rust-demo/go-deploy-tools/cross-language-smoke/docker-build-*` 不符合目标分组。
- 改造 `.github/workflows/ci.yml`：合并语言 SDK+demo job，新增 Python/Node.js deferred fail-closed gate，其他类 job 统一 `Other / ...` 命名。
- 本地验证：contract test、YAML parse、Node runtime policy、diff check 通过。用户明确要求下班前先跳过远端 CI 结果调试，因此本轮提交后不继续等待 GitHub Actions 完成。

## 2026-06-08 — Promotional browser recording evidence

Agent:
- Codex

Work:
- Started a real local Tikeo demo stack for promotional browser recording.
- Initialized bootstrap admin with user-provided credentials `admin / admin@qq.com / qqqqqq` in an isolated throwaway SQLite DB under `.dev/reports/`.
- Started Server, Web, and demo workers: Java Boot2, Java Boot3, Java Boot4, Go, Rust, Python, and Node.js.
- Recorded a Playwright browser walkthrough covering dashboard, workers, dispatch queue, jobs, topology, workflows, scripts, roles, API-Key, audit, and alert delivery pages.
- Exported WebM and MP4 promotional video artifacts under `.dev/reports/promo-showcase-20260608T030355Z-111791/`.

Verification:
- `ffprobe` confirmed the WebM is 1440x960, 25 fps, duration 65.280 seconds.
- `ffmpeg` converted the WebM to `tikeo-promo-showcase.mp4` successfully.
- Browser login state confirmed URL `http://127.0.0.1:15174/dashboard`, token present, and visible console text for admin.
- Worker API evidence confirmed all seven demo workers online: `java-boot2-orders-blue`, `java-boot3-orders-blue`, `java-boot4-billing-green`, `go-worker-demo-local`, `rust-worker-demo-local`, `python-worker-demo-local`, `nodejs-worker-demo-local`.
- Visual thumbnail inspection confirmed the recording reached the authenticated console and final alert delivery page.
- Critical log scan found no proxy/auth/module-resolution/Playwright timeout errors in the final run.

Git:
- This commit records session evidence only; generated video artifacts remain local under ignored `.dev/`.

## 2026-06-08 — Enhanced bilingual promotional video artifact

Agent:
- Codex

Work:
- Re-ran the promotional demo as a slower, richer Playwright walkthrough using an isolated local stack and throwaway SQLite DB under `.dev/reports/`.
- Seeded denser promotional data across jobs, workers, dispatch queue, topology, workflows, scripts, RBAC roles, service accounts/API keys, audit, and alerts.
- Reused the user-provided bootstrap admin identity for the local demo recording only.
- Recorded a 12-segment browser walkthrough covering intro, dashboard, worker fleet, dispatch queue/leases, jobs, topology, workflows, scripts/sandbox policy, RBAC roles, service accounts/API keys, audit trail, and alerts.
- Generated English narration with `edge-tts`, generated Chinese narration, soft subtitle tracks for English and Simplified Chinese, and burned-in bilingual Chinese/English subtitles.
- Final local artifact: `.dev/reports/promo-rich-showcase-20260608T032701Z-133036/tikeo-rich-promo-bilingual.mp4`.

Verification:
- `ffprobe` confirmed final MP4 duration `279.680000` seconds at `1440x960`, 25 fps.
- `ffprobe` confirmed two AAC audio tracks: English narration (`eng`) is default, Chinese narration (`zho`) is second.
- `ffprobe` confirmed two soft subtitle tracks: English (`eng`, default) and Simplified Chinese (`zho`).
- Timeline evidence confirmed 12 slower segments ending at `278.743` seconds.
- Worker API evidence confirmed seven demo workers online in the enhanced run: Java Boot2, Java Boot3, Java Boot4, Go, Rust, Python, and Node.js demos.
- Visual frame inspection at 60s and 210s confirmed the authenticated console, rich demo data, feature callout card, and readable burned-in bilingual subtitles.
- Critical log scan found no proxy/auth/module-resolution/Playwright timeout errors in the enhanced run.

Git:
- This commit records enhanced video evidence only; generated media, TTS cache, and Playwright artifacts remain local under ignored `.dev/`.

## 2026-06-08 — Cinematic soft-subtitle promotional video and audit date polish

Agent:
- Codex

Work:
- Reworked the promotional browser recording into a cinematic long-form walkthrough with continuous mouse movement, scroll/pan motion on content-heavy pages, slower feature pacing, and expanded narration about Tikeo architecture, cloud-native Worker Tunnel design, dispatch leases/fencing, unified job lifecycle, topology, workflow state machine, script governance, RBAC, service-account API keys, audit evidence, and alert delivery.
- Re-recorded the real local stack at 1920x1080 using Server, Web, and seven live demo workers: Java Boot2, Java Boot3, Java Boot4, Go, Rust, Python, and Node.js.
- Regenerated English narration and Chinese narration with `edge-tts`; validated each segment duration covers both English and Chinese TTS without truncation.
- Changed the final media packaging to high-quality H.264 (`libx264`, `preset=slow`, `crf=16`) with no burned-in subtitles; English and Simplified Chinese subtitles are embedded as soft subtitle tracks, with standalone `.srt` files preserved for platform CC uploads.
- Fixed Web audit log date rendering to accept both `createdAt` and backend `created_at`, preventing `Invalid Date` from appearing in the promotional audit segment.
- Final local artifact: `.dev/reports/promo-cinematic-showcase-20260608T041919Z-187012/tikeo-cinematic-promo-hq-softsubs.mp4`.
- Standalone subtitle files: `subtitles.en.srt`, `subtitles.zh-CN.srt`, and `subtitles.bilingual.srt` in the same report directory.

Verification:
- `ffprobe` confirmed final MP4 duration `495.320000` seconds, `1920x1080`, H.264 video, English AAC narration default track, Chinese AAC narration second track, English soft subtitle default track, and Chinese soft subtitle second track.
- Final media summary records `burnedInSubtitles: false` and CRF 16 source-resolution encode.
- TTS fit script confirmed all 12 segments cover both English and Chinese generated audio without overrun.
- Visual frame inspection confirmed no burned-in subtitles, clean 1080p UI, and audit dates rendered as concrete dates/times instead of `Invalid Date`.
- Critical final run log scan found no proxy/auth/module-resolution/Playwright timeout errors.
- Web verification: `cd web && bun run lint`; `cd web && bun run typecheck`; explicit src unit-test file list via `bun test $(find src -type f \( -name '*.test.ts' -o -name '*.test.tsx' \) | sort)` passed 117 tests; `cd web && bun run build` passed.
- Known verification note: plain `cd web && bun test` still fails because Bun test runner loads Playwright e2e specs (`e2e/rbac-role-management.spec.ts`) and hits the existing Playwright `test()` runner conflict; this is unrelated to the audit-date fix and was bypassed with an explicit src test file list.

Git:
- Generated media remains local under ignored `.dev/`; source commit includes only the Web audit date compatibility fix and memory evidence.


## 2026-06-08 — English-site cinematic promotional video refresh

Agent:
- Codex

Work:
- Re-recorded the cinematic promotional browser walkthrough with the Web UI forced to `en-US` through Playwright context locale and `tikeo.locale` localStorage initialization, while preserving the previous rich narration, dynamic scrolling/mouse motion, live Server/Web stack, and seven demo workers.
- Fixed Worker cluster page English-mode copy gaps that were still hardcoded in Chinese: cluster overview hero, refresh/dispatch actions, app cluster node labels, master/follower tags, generation/election/term labels, logical/client instance metadata, processor/script/plugin capability tags, filters, and empty states.
- Rebuilt the high-quality MP4 with English default narration, Chinese secondary narration, English/Chinese soft subtitle tracks, standalone `.srt` files, no burned-in subtitles, and 1920x1080 H.264 CRF 16 packaging.
- Final local English-site artifact: `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-softsubs.mp4`.
- Standalone subtitle files: `subtitles.en.srt`, `subtitles.zh-CN.srt`, and `subtitles.bilingual.srt` in the same report directory.

Verification:
- `ffprobe` confirmed final MP4 duration `496.520000` seconds, `1920x1080`, H.264 video, English AAC narration default track, Chinese AAC narration second track, English soft subtitle default track, and Chinese soft subtitle second track.
- Browser login state body text confirmed English UI indicators: Overview, Task orchestration, Execution resources, Governance configuration, English, and Sign out, with no matching Chinese navigation indicators.
- Extracted final MP4 frames for dashboard, workers, and API-Key segments; visual inspection confirmed no burned-in subtitles and Worker page labels are English.
- Critical final run log scan found no proxy/auth/module-resolution/Playwright timeout errors after excluding expected end-of-recording worker EOF/SIGTERM shutdown lines.
- Web verification after Worker i18n fix: `cd web && bun run lint`; `cd web && bun run typecheck`; explicit src unit-test file list via `bun test $(find src -type f \( -name '*.test.ts' -o -name '*.test.tsx' \) | sort)` passed 117 tests; `cd web && bun run build` passed.
- `git diff --check` passed before commit.

Git:
- Generated media remains local under ignored `.dev/`; source commit includes only Worker page English-mode copy hardening and memory evidence.


## 2026-06-08 — Sentence-level promotional subtitles

Agent:
- Codex

Work:
- Refined the final English-site promotional subtitle files from coarse chapter-level captions into sentence/phrase-level CC-style subtitles.
- Used Edge TTS subtitle timing boundaries per narration segment, then split overly long narration sentences at natural punctuation to prevent full-screen subtitle blocks.
- Replaced the standalone SRT files in the final report directory: `subtitles.en.srt`, `subtitles.zh-CN.srt`, and `subtitles.bilingual.srt`.
- Remuxed the final MP4 without re-encoding video/audio so the embedded English and Chinese soft subtitle tracks use the refined sentence-level timing.
- Final local artifact with refined captions: `.dev/reports/promo-cinematic-showcase-20260608T050247Z-231970/tikeo-cinematic-promo-hq-sentence-subs.mp4`.

Verification:
- Subtitle readability summary: English 72 cues, Chinese 57 cues, bilingual 72 cues; max English cue length 118 chars, max Chinese cue length 54 chars; max English cue duration 8.112s and max Chinese cue duration 11.225s.
- `ffprobe` confirmed remuxed MP4 duration `496.520000` seconds, `1920x1080`, H.264 video, English AAC default audio, Chinese AAC secondary audio, English default soft subtitle track, and Chinese secondary soft subtitle track.
- Extracted embedded English subtitle stream from the remuxed MP4 and confirmed it contains the refined sentence/phrase-level timing rather than 12 long chapter captions.
- `git diff --check` passed before commit.

Git:
- Generated media/subtitle artifacts remain local under ignored `.dev/`; source commit records updated memory evidence only.

## 2026-06-08 — Open-source discovery and GitHub first-fold polish

Agent:
- Codex

Work:
- Added a compact README console tour GIF derived from the final English promotional walkthrough: `docs/assets/tikeo-console-tour.gif`.
- Updated English and Chinese README first-fold messaging with a sharper value hook, demo GIF, quick evaluation links, support note, and Star History chart.
- Added open-source project hygiene files: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, and `ROADMAP.md`.
- Added GitHub pull request and issue templates for bug reports and feature requests.
- Updated GitHub repository description and topics via `gh repo edit`: concise Rust-native orchestration description; topics now include `job-scheduler` and `workflow-engine` while retaining Tikeo-specific discovery terms.

Verification:
- Demo GIF size check passed: 1.58 MB, under the 5 MB README target.
- README anchor/text checks passed for demo asset, support/star-history sections, quick-start anchors, and comparison anchors in both English and Chinese README files.
- GitHub issue template YAML parsed successfully.
- `git diff --check` passed.

Git:
- Generated temporary video-processing files remain local under ignored `.dev/`; committed asset is the optimized README GIF only.


## 2026-06-08 — Docs site build plan

Agent:
- Codex

Work:
- Added `design/docs-site-build-plan.md` as a plan-only document for a future standalone Tikeo documentation site.
- Used the Hermes Agent docs site as an information-architecture reference and mapped the reusable structure to Tikeo: Getting Started, Concepts, User Guide, SDKs, Deployment, Integrations, Guides, Developer Guide, and Reference.
- Recommended Docusaurus 3, bilingual docs, LLM-readable exports, search/SEO rules, implementation phases, and first-launch page priorities.

Verification:
- Markdown file exists and is readable.
- Checked that the plan is scoped as documentation-only and does not scaffold or deploy the site.
- `git diff --check` to be run before commit.

Git:
- Pending commit/push at the time of this memory update.


## 2026-06-08 — README badges and SDK runtime requirements

Agent:
- Codex

Work:
- Replaced the README/Chinese README CI Shields workflow badge with the native GitHub Actions workflow badge because the Shields workflow endpoint can time out and render as a broken image.
- Removed the hardcoded `coverage: report pending` badge until CI publishes real coverage data to a badge provider.
- Added SDK runtime requirements to `README.md`, `README.zh-CN.md`, `sdks/README.md`, and each language SDK README.
- Added `engines.node >=24.0.0` to the Node.js SDK package metadata to match the documented runtime baseline.

Coverage note:
- A normal percentage badge requires a CI coverage job to generate LCOV/Cobertura/JaCoCo-style reports and upload them to Codecov or another badge source. Until that upload exists, Codecov returns unknown and a static `report pending` badge is only cosmetic.

Verification:
- README/runtime requirement text checks and JSON parse to be run before commit.
- `git diff --check` to be run before commit.

Git:
- Pending commit/push at the time of this memory update.


## 2026-06-08 — Static CI and SDK runtime badges

Agent:
- Codex

Work:
- Replaced the native GitHub workflow SVG with a static Shields `CI / GitHub Actions` badge linked to the CI workflow, avoiding dynamic status endpoints that can render broken.
- Added runtime requirement Shields badges for Java 17+, Rust 1.95+, Go 1.26+, Python 3.11+, and Node.js 24+ to `README.md`, `README.zh-CN.md`, and `sdks/README.md`.

Verification:
- Static badge URL HTTP checks to be run before commit.
- README badge/text checks and `git diff --check` to be run before commit.

Git:
- Pending commit/push at the time of this memory update.


## 2026-06-08 — Codecov Rust coverage workflow

Agent:
- Codex

Work:
- Moved SDK runtime requirement badge rows above all SDK version badge rows in both README files.
- Added `.github/workflows/coverage.yml` for Rust workspace coverage using `cargo llvm-cov` and Codecov upload with the repository `CODECOV_TOKEN` secret.
- Added a Codecov Rust coverage badge to README and Chinese README; it will become meaningful after the first successful main-branch coverage upload.

Verification:
- Workflow runtime policy, YAML parse, README badge ordering checks, badge URL checks, and `git diff --check` to be run before commit.

Git:
- Pending commit/push at the time of this memory update.


## 2026-06-08 — Codecov upload workflow repair

Agent:
- Codex

Work:
- Replaced `codecov/codecov-action@v5` with direct `codecov-cli` installation/upload in `.github/workflows/coverage.yml`.

Reason:
- Remote run `27120846387` proved Rust LCOV generation succeeded and `CODECOV_TOKEN` was present, but the Codecov action failed during CLI GPG signature validation and emitted an internal Node20 `actions/github-script` warning.

Verification:
- Workflow policy and YAML checks to be rerun before commit.
- Remote workflow to be rerun after push.

Git:
- Pending commit/push at the time of this memory update.


## 2026-06-08 — Codecov upload verified

Agent:
- Codex

Verification:
- Remote Coverage workflow run `27121393205` completed successfully for commit `6a2bf48`.
- Rust LCOV generation, Codecov CLI install, and Codecov upload all passed.
- Codecov badge endpoint `https://codecov.io/gh/yhyzgn/tikeo/branch/main/graph/badge.svg?flag=rust` returned `84%` after upload.

Result:
- The remote `CODECOV_TOKEN` configuration is valid for Rust workspace coverage uploads.
- Current badge represents the Rust coverage flag, not full monorepo coverage.


## 2026-06-08 — Full coverage workflow and animated README logo

Agent:
- Codex

Work:
- Expanded `.github/workflows/coverage.yml` from Rust-only upload to a multi-surface Coverage workflow: Rust workspace LCOV, Web LCOV, Java SDK JaCoCo XML, Go SDK coverprofile, Python SDK/demo XML, and Node.js SDK LCOV.
- Kept direct `codecov-cli upload-process` for all uploads because the earlier Codecov GitHub Action path failed remotely during GPG validation while the token itself was already verified.
- Enabled Java SDK JaCoCo XML generation in `sdks/java/build.gradle.kts` for every Java library submodule.
- Changed README/Chinese README coverage badge from the Rust flag badge to the overall Codecov branch badge, matching the new full-project upload intent.
- Generated `docs/assets/tikeo-logo-breathe.gif` from the Web breathing/logo motion treatment and replaced the static README logo in both README files.

Verification:
- YAML parse passed for `.github/workflows/coverage.yml` with jobs `rust, web, java, go, python, nodejs`.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed: 16 external actions checked, no runtime below Node 24.
- README checks passed: animated logo path present, general Codecov badge present, Rust flag badge removed, runtime requirement badges remain above SDK version badges.
- GIF check passed: `docs/assets/tikeo-logo-breathe.gif` is GIF89a, 220x220, 404314 bytes.
- `git diff --check` passed before memory update.
- Web coverage generation passed: `cd web && bun test src --coverage --coverage-reporter=lcov --coverage-dir=../coverage/web` with 117 tests passing and `coverage/web/lcov.info` produced.
- Node.js SDK coverage generation passed: `cd sdks/nodejs/tikeo && bun test --coverage --coverage-reporter=lcov --coverage-dir=../../../coverage/nodejs-sdk` with 14 tests passing and `coverage/nodejs-sdk/lcov.info` produced.
- Go SDK coverage generation passed: `cd sdks/go/tikeo && go test ./... -covermode=atomic -coverprofile=../../../coverage/go-sdk.out -count=1` and `coverage/go-sdk.out` produced.
- Java SDK JaCoCo generation passed: `cd sdks/java && ./gradlew test jacocoTestReport --no-daemon`; seven `jacocoTestReport.xml` files produced.
- Python SDK/demo coverage generation passed after clearing the local temporary venv: `python -m pytest sdks/python/tikeo/tests examples/python/worker-demo/tests --cov=tikeo --cov=tikeo_python_worker_demo --cov-report=xml:coverage/python.xml -q` with 19 tests passing.

Git:
- Pending commit/push at the time of this memory update.
- Remote full Coverage workflow must be checked after push before claiming the overall Codecov badge is populated by all flags.


## 2026-06-08 — Full Coverage workflow remote verification

Agent:
- Codex

Verification:
- Remote Coverage workflow run `27125171618` completed successfully for commit `5beb036380c8fbb54f54a0ed60a01b6c366b286d`.
- Successful remote jobs: Node.js SDK coverage, Rust workspace coverage, Python SDK coverage, Java SDK coverage, Go SDK coverage, and Web coverage.
- Overall Codecov branch badge endpoint `https://codecov.io/gh/yhyzgn/tikeo/branch/main/graph/badge.svg` returned `79%` after the multi-surface upload.

Result:
- The README coverage badge now has a real full-project Codecov source instead of a static pending badge or Rust-only flag badge.


## 2026-06-08 — Main CI remote verification for README motion / coverage commit

Agent:
- Codex

Verification:
- Remote main CI run `27125171526` completed successfully for commit `5beb036380c8fbb54f54a0ed60a01b6c366b286d`.
- Successful CI groups included workflow policy, Server, Web, Java SDK + demo, Rust SDK + demo, Go SDK + demo, Python SDK + demo, Node.js SDK + demo, deploy tooling, cross-language worker smoke, and Docker build validation for Web and Server.

Result:
- No known remote CI failures remain for the README animated logo / full coverage workflow commit.

## 2026-06-08 — Helm production deployment hardening

Agent:
- Codex

Work:
- Added a failing deployment contract test for Helm production hardening, then implemented the chart changes to make it pass.
- Expanded `deploy/helm/tikeo/values.yaml` and templates for external DB Secret injection, conditional SQLite PVC, service account, tunable probes/resources/security contexts, server/web ingress, HTTP TLS Secret mounts, Worker Tunnel TLS/mTLS Secret mounts, and generated transport security config.
- Added Helm example values for SQLite dev, external PostgreSQL, ingress/listener TLS, and worker identity shape.
- Updated Helm/deploy docs with production DB Secret usage, TLS/mTLS boundaries, worker outbound-only identity guidance, and rollback steps.
- Updated `scripts/verify-deploy-bootstrap.sh` to verify Helm production artifacts instead of the old deferred-Helm statement.
- Updated design roadmap and `.prompt/150-phase4-production-helm-followup.md` for the next deployment maturity slice.

Verification:
- RED: `python3 -m unittest deploy.tests.iac_artifacts_test.IacArtifactsTest.test_helm_chart_exposes_production_hardening_contracts` failed against the old chart.
- GREEN: the same test passed after implementation.
- `python3 -m unittest deploy.tests.iac_artifacts_test deploy.tests.smoke_assertions_test` passed.
- `scripts/verify-deploy-bootstrap.sh` passed.
- `.dev/tools/helm lint deploy/helm/tikeo` passed with only the optional chart icon recommendation.
- `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo` passed.
- `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo -f deploy/helm/tikeo/examples/values-external-postgres.yaml` passed and rendered no SQLite PVC while injecting `TIKEO__STORAGE__DATABASE_URL` from `tikeo-database`.
- `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo -f deploy/helm/tikeo/examples/values-external-postgres.yaml -f deploy/helm/tikeo/examples/values-ingress-tls.yaml` passed and rendered HTTP/Worker Tunnel TLS/mTLS paths.

Git:
- Pending final full verification, commit, push, and remote CI check at the time of this memory update.

Final local verification before commit:
- `git diff --check` passed.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `bun run --cwd web lint` passed.
- `bun run --cwd web typecheck` passed.
- `bun run --cwd web test` passed with 117 tests.
- `bun run --cwd web build` passed; Vite reported the existing large chunk warning for bundled vendor assets.

Remote verification after push:
- CI run `27128044956` completed successfully for source commit `c90b44177a692946ad4cd000f16e6653ddc508e9`.
- Successful CI groups: workflow policy, Web, deploy tooling, Go SDK + demo, Rust SDK + demo, Python SDK + demo, Java SDK + demo, Node.js SDK + demo, Server, cross-language worker smoke, Docker build validation / web, and Docker build validation / server.
- Coverage run `27128044845` completed successfully for source commit `c90b44177a692946ad4cd000f16e6653ddc508e9`; Rust workspace, Web, Java SDK, Go SDK, Python SDK, and Node.js SDK coverage jobs all passed.

## 2026-06-08 — Helm operations maturity overlay

Agent:
- Codex

Work:
- Added a failing Helm operations maturity contract test, then implemented optional PodDisruptionBudget, NetworkPolicy, ServiceMonitor, Gateway API `GRPCRoute`, and `values.schema.json` support.
- Added `values-ops-hardening.yaml` and `values-gateway-api-worker-tunnel.yaml` examples.
- Updated Helm README and root deploy README to describe PDB, NetworkPolicy, ServiceMonitor, Gateway API, and schema validation boundaries.
- Updated deployment bootstrap verification, design roadmap, memory, and `.prompt/151-phase4-helm-ops-and-source-size-followup.md`.

Verification so far:
- RED: `python3 -m unittest deploy.tests.iac_artifacts_test.IacArtifactsTest.test_helm_chart_exposes_operational_maturity_contracts` failed against the previous chart because `values.schema.json` and ops templates were missing.
- GREEN: the same test passed after implementation.
- `scripts/verify-deploy-bootstrap.sh` passed.
- `.dev/tools/helm lint deploy/helm/tikeo` passed with only the optional chart icon recommendation.
- `.dev/tools/helm lint` passed with external DB values and with external DB + TLS + ops hardening + Gateway API values.
- `.dev/tools/helm template` passed for default, external DB, TLS, and ops/Gateway overlays.

Git:
- Pending final full verification, commit, push, and remote CI check at the time of this memory update.

Final local verification before commit:
- `git diff --check` passed.
- `python3 -m unittest deploy.tests.iac_artifacts_test deploy.tests.smoke_assertions_test` passed with 8 tests.
- `scripts/verify-deploy-bootstrap.sh` passed.
- `.dev/tools/helm lint` passed for default, external DB, and external DB + TLS + ops + Gateway values; only optional chart icon recommendation was reported.
- `.dev/tools/helm template` passed for default, external DB, TLS, and ops/Gateway overlays; ops overlay rendered `PodDisruptionBudget`, `NetworkPolicy`, `ServiceMonitor`, `Gateway`, and `GRPCRoute` resources.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `bun run --cwd web lint` passed.
- `bun run --cwd web typecheck` passed.
- `bun run --cwd web test` passed with 117 tests.
- `bun run --cwd web build` passed; Vite reported the existing large chunk warning for bundled vendor assets.

Remote verification after push:
- CI run `27129836559` completed successfully for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`.
- Successful CI groups: workflow policy, Rust SDK + demo, Go SDK + demo, Java SDK + demo, Python SDK + demo, Server, Web, Node.js SDK + demo, deploy tooling, cross-language worker smoke, Docker build validation / server, and Docker build validation / web.
- Coverage run `27129836631` completed successfully for source commit `e98f6fd7395f1c104050ce8037db79ab5447aed6`; Rust workspace, Web, Java SDK, Go SDK, Python SDK, and Node.js SDK coverage jobs all passed.
### 2026-06-08 — Source-size debt cleanup
- Added `scripts/check-source-size.py` as a repo-wide source-size audit for normal Rust/TypeScript/TSX files, excluding generated/dependency/build output.
- Split all known historical >1500-line source files without behavior changes: storage repository tests, workflow runtime methods, migration RBAC role-management migration, server dispatcher processors/tests, registry tests, HTTP part_03 tests, and Web workflow API client functions.
- Current source-size gate is green for the whole repository; future source changes should run the audit before commit.
Verification evidence:
- `python3 scripts/check-source-size.py` passed.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test -p tikeo-storage --all-features` passed.
- `cargo test -p tikeo-server --all-features` passed.
- `cargo test --workspace --all-features` passed.
- `cargo build --workspace --all-features` passed.
- `bun run --cwd web lint` passed.
- `bun run --cwd web typecheck` passed.
- `bun run --cwd web test` passed with 117 tests.
- `bun run --cwd web build` passed with the existing large vendor chunk warning.
- Smoke: `cargo run --bin tikeo -- serve --config /tmp/tikeo-source-size-smoke.toml` plus `curl -fsS http://127.0.0.1:19090/healthz` returned `{"status":"ok","uptime_seconds":0}`.

## 2026-06-08 — Source-size audit CI gate

Agent:
- Codex

Work:
- Added `python3 scripts/check-source-size.py` to the main CI `workflow-policy` job so source-size violations fail before Server/Web/SDK runtime jobs start.
- Added a GitHub workflow contract test proving the CI policy job enforces the source-size gate.

Verification:
- RED: `python3 .github/tests/workflow_contract_test.py -k test_ci_enforces_source_size_before_runtime_jobs` failed before the workflow step was added.
- GREEN: the same targeted test passed after adding the workflow step.
- `python3 scripts/check-source-size.py` passed.
- `python3 .github/tests/workflow_contract_test.py` passed with 11 tests.
- YAML parse for all `.github/workflows/*.yml` passed.
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` passed with 16 external actions and no runtime below node24.
- `git diff --check` passed.

Git:
- Pending commit/push for this CI gate slice. Per user instruction, do not wait for remote Actions before continuing to the next work item.

## 2026-06-08 — Standalone docs site scaffold

Agent:
- Codex

Work:
- Created `website/` as a Docusaurus 3.10.1 TypeScript docs app using Bun.
- Replaced template content with Tikeo homepage, navbar/footer, sidebar IA, Phase A P0 English docs pages, starter `zh-CN` translations, a release-note blog entry, and static `llms.txt` / `llms-full.txt` entrypoints.
- Reused existing project assets: breathing logo GIF, architecture SVGs, and console tour GIF.
- Added `.github/tests/docs_site_contract_test.py` to lock the docs scaffold contract.
- Updated `design/docs-site-build-plan.md`, `.memory/commands.md`, `.memory/progress.md`, `.memory/next.md`, and `.prompt/153-docs-site-content-followup.md`.

Verification:
- RED: `python3 .github/tests/docs_site_contract_test.py` failed before `website/` existed.
- GREEN: `python3 .github/tests/docs_site_contract_test.py` passed after scaffold implementation.
- `python3 scripts/check-source-size.py` passed.
- `bun install --frozen-lockfile` passed in `website/`.
- `bun run docs:typecheck` passed in `website/`.
- `bun run docs:build` passed in `website/`, generating English and `zh-CN` static output.
- Docs serve smoke passed at port `13030` for `/`, `/docs/`, `/zh-CN/docs/`, `/docs/getting-started/quickstart`, and `/llms.txt`.

Git:
- Pending final verification, commit, and push for docs scaffold. Remote Actions should not be awaited unless the user asks.

### 2026-06-08 — Docs P0 content depth, full zh-CN route mirror, and complete SDK list
- Expanded the standalone docs site P0 pages beyond scaffold-level summaries: overview, installation, quickstart, seed demo data, Worker Tunnel, workflows, Rust/Go/Java SDKs, Docker Compose, Kubernetes/Helm, integrations, configuration, and troubleshooting now contain evaluation-oriented guidance tied to repository behavior.
- Added missing Python and Node.js SDK pages to `website/docs/sdks/` and the docs sidebar, so the docs list all current SDK families: Rust, Go, Java Spring Boot, Python, and Node.js.
- Filled zh-CN counterparts for every current P0 docs route, including all SDK pages, fixing the Chinese 404 gap caused by partial localization.
- Strengthened `.github/tests/docs_site_contract_test.py` so every P0 English route must have enough evaluation depth, every P0 route must have a zh-CN file, and zh-CN files must contain real localized depth instead of placeholder summaries.
- Updated `design/docs-site-build-plan.md` to mark Phase B P0 content and Phase C current-route localization as implemented; deployment target remains undecided.
Verification evidence:
- `python3 .github/tests/docs_site_contract_test.py` passed.
- `python3 scripts/check-source-size.py` passed.
- `cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed.
- Docs serve smoke on port `13031` passed for `/zh-CN/docs/`, `/zh-CN/docs/getting-started/installation`, `/zh-CN/docs/sdks/rust`, `/zh-CN/docs/sdks/python`, `/zh-CN/docs/sdks/nodejs`, `/zh-CN/docs/deployment/kubernetes`, and `/zh-CN/docs/reference/troubleshooting`.
- `python3 .github/tests/workflow_contract_test.py` passed; workflow YAML parse passed; `git diff --check` passed.
Verification gap:
- `python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24` timed out locally after 20s with no output during this docs-only slice; prior baseline had this policy green, and no workflow files were changed.

### 2026-06-08 — Docs language switch baseUrl fix and copy-paste deployment docs
- Fixed the likely zh-CN language-switch 404 for GitHub Pages/project-subpath hosting by making Docusaurus `url`/`baseUrl` environment-configurable and defaulting to standalone-root `https://tikeo.dev` + `/`; GitHub Pages project hosting can set `TIKEO_DOCS_URL=https://yhyzgn.github.io` and `TIKEO_DOCS_BASE_URL=/tikeo/`.
- Updated the homepage to use `useBaseUrl` for static assets, so logo/architecture images remain valid under `/tikeo/`.
- Expanded deployment docs from overview text into copy-paste runbooks: single binary/systemd, Compose SQLite/PostgreSQL/MySQL, Prometheus profile, Helm SQLite dev install, external database Secret install, TLS/mTLS install, operations hardening, Gateway API rendering, values reference, validation, cleanup, and rollback.
- Expanded configuration reference with committed config file map, ports, storage URLs, env override rules, auth/API token knobs, transport security, observability, alert retry/secrets, and script governance.
- Added docs contract coverage that requires deployment docs to keep concrete runbook snippets and baseUrl support.
Verification evidence:
- `python3 .github/tests/docs_site_contract_test.py` passed with 8 tests.
- `python3 scripts/check-source-size.py` passed.
- `cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed with default `/tikeo/` baseUrl.
- Generated HTML contains `/tikeo/zh-CN/...` language-switch links for docs root, deployment/kubernetes, and homepage.
- Default subpath serve smoke on port `13032` passed for `/tikeo/`, `/tikeo/docs/`, `/tikeo/zh-CN/`, `/tikeo/zh-CN/docs/`, and zh-CN deployment/config routes.
- Custom root build with `TIKEO_DOCS_URL=http://127.0.0.1:13033 TIKEO_DOCS_BASE_URL=/` passed; generated HTML contains `/zh-CN/...` links; root serve smoke on port `13033` passed.

### 2026-06-08 — Docs root-locale fix and full Compose YAML publication
- Reverted docs default baseUrl to standalone-root `/` so `/zh-CN/` and `/zh-CN/docs/` work by default for an independently deployed docs site.
- Kept GitHub Pages project hosting supported through explicit `TIKEO_DOCS_URL=https://yhyzgn.github.io TIKEO_DOCS_BASE_URL=/tikeo/` build variables.
- Expanded Docker Compose docs to include the full committed `docker-compose.yml`, `docker-compose.postgres.yml`, and `docker-compose.mysql.yml` contents in both English and zh-CN docs, plus commands, parameter table, Worker connectivity, Prometheus, and cleanup guidance.
- Updated docs contract checks so deployment docs must retain full Compose file headings and copy-paste runbook snippets.
Verification evidence:
- `python3 .github/tests/docs_site_contract_test.py` passed with 8 tests.
- `python3 scripts/check-source-size.py` passed.
- `cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed with default root baseUrl.
- Generated root HTML contains `/zh-CN/...` language links; root serve smoke on port `13036` passed for `/zh-CN/`, `/zh-CN/docs/`, `/zh-CN/docs/deployment/docker-compose`, and `/zh-CN/docs/deployment/kubernetes`.
- GitHub Pages subpath build with `TIKEO_DOCS_URL=https://yhyzgn.github.io TIKEO_DOCS_BASE_URL=/tikeo/` generated `/tikeo/zh-CN/...` links; subpath serve smoke on port `13037` passed.
- `python3 .github/tests/workflow_contract_test.py` passed; workflow YAML parse passed; `git diff --check` passed.

### 2026-06-08 — Docs locale separation fixed
- Generated and completed Docusaurus standard zh-CN translation resources for navbar, footer, docs sidebar categories, blog options, theme copy, blog author metadata, tags, and the first release post.
- Reworked the docs homepage to be locale-aware: the root/default English page renders English copy and the zh-CN page renders Chinese copy, while code commands remain shared.
- Confirmed default root route remains English because Docusaurus `defaultLocale` is `en`; Chinese content is under `/zh-CN/...`.
- Added docs contract coverage to prevent zh-CN navbar/sidebar/footer/blog translations from falling back to English strings.
Verification evidence:
- `python3 .github/tests/docs_site_contract_test.py` passed with 10 tests.
- `python3 scripts/check-source-size.py` passed.
- `cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` passed.
- Generated HTML grep confirmed root homepage has English headline and no Chinese headline; zh-CN homepage has Chinese headline and no English headline; English docs sidebar remains English; zh-CN docs sidebar/footer are localized; English release remains English; zh-CN release is Chinese.
- Serve smoke on port `13039` confirmed `/`, `/zh-CN/`, `/zh-CN/docs/deployment/docker-compose`, and `/zh-CN/releases/docs-site-scaffold` render with expected locale isolation.

## 2026-06-08 — 0.2.0 正式版发布准备

Agent:
- Codex

Work:
- 将 Rust workspace、独立 Rust SDK/demo、Java SDK/demo、Python SDK/demo、Node SDK/demo、Web、Docs 站点和 Helm chart 版本统一提升到 `0.2.0`。
- 更新 README / README.zh-CN / Helm README 中 SDK、镜像、Helm 安装示例到 `0.2.0`。
- 在 `CHANGELOG.md` 新增 `0.2.0` 正式版条目，概括 docs 站点、SDK 文档、Helm/Compose、README 宣传资产、CI/coverage/source-size 等发布内容。
- 对 Node/Python worker demo 的常驻 dry-run 启动改用 `timeout` 烟测模型记录：进程可启动并注册本地 sandbox runner，超时退出为预期。

Verification:
- `python3 .github/tests/docs_site_contract_test.py` ✅
- `python3 scripts/check-source-size.py` ✅
- `python3 .github/tests/workflow_contract_test.py` ✅
- `.github/workflows/*.yml` YAML parse ✅
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web lint` / `typecheck` / `test` / `build` ✅
- `cd website && bun install --frozen-lockfile && bun run docs:typecheck && bun run docs:build` ✅
- `cd sdks/java && ./gradlew test jar sourcesJar --no-daemon` ✅
- `cd sdks/rust/tikeo && cargo check --all-features && cargo test --all-features` ✅
- `cd sdks/go/tikeo && go test ./... -count=1` ✅
- `cd sdks/nodejs/tikeo && bun install --frozen-lockfile && bun test && bun run build` ✅
- `cd sdks/python/tikeo && uv run --extra test python -m pytest` ✅
- `cd examples/rust/worker-demo && cargo check && cargo test` ✅
- `cd examples/go/worker-demo && go test ./... -count=1` ✅
- `cd examples/nodejs/worker-demo && timeout 8s env TIKEO_WORKER_DRY_RUN=1 bun start && bun test` with timeout `124` accepted for long-running worker ✅
- `cd examples/python/worker-demo && timeout 8s uv run --with '../../../sdks/python/tikeo[test]' --extra test python -m tikeo_python_worker_demo && uv run --with '../../../sdks/python/tikeo[test]' --extra test python -m pytest` with timeout `124` accepted for long-running worker ✅
- Java Spring Boot 2/3/4 worker demos `./gradlew test --no-daemon` ✅

Notes:
- Python system interpreter lacks `pip`; Python verification used `uv` isolated environments instead.
- Java demo verification performed real sandbox runtime installation/download paths and completed successfully after network waits.

Git:
- Pending commit, push, annotated tag `v0.2.0`, and GitHub Release creation.

## 2026-06-08 — 0.2.0 Docker web publish follow-up

Agent:
- Codex

Work:
- Investigated tag-triggered `Publish / Docker web` failure for `v0.2.0`.
- First run failed during `bun install` extracting `antd`; rerun failed under `oven/bun:latest` / Bun `1.3.14` with tarball extraction/integrity errors for `@rolldown/binding-linux-x64-gnu` and `@ant-design/icons`.
- Pinned `web/Dockerfile` builder image to `oven/bun:1.3.13`, matching the verified local/CI Bun baseline used by Web tests.
- Added optional `ref` input to `publish-docker-web.yml` so the existing `v0.2.0` image tag can be manually built from the release-fix commit on `main` without moving the already-pushed release tag or re-publishing successful SDK artifacts.
- Cleaned duplicate `CHANGELOG.md` 0.2.0 subsections.

Verification:
- `.github/workflows/*.yml` YAML parse ✅
- `python3 .github/tests/workflow_contract_test.py` ✅
- `bun run --cwd web build` ✅
- `docker build -f web/Dockerfile web -t tikeo-web:0.2.0-local` ✅

Git:
- Pending commit and push, then manual rerun of `Publish / Docker web` with `tag=v0.2.0` and `ref=main`.

## 2026-06-08 — 0.2.0 正式发布完成

Agent:
- Codex

Remote release status:
- `main` release-prep commit pushed: `00c2927`.
- Annotated tag pushed: `v0.2.0` -> `00c2927`.
- GitHub Release published as formal release, not draft and not prerelease: https://github.com/yhyzgn/tikeo/releases/tag/v0.2.0
- Tag-triggered publish workflows succeeded:
  - Publish / Rust SDK: run `27146185208` ✅
  - Publish / Python SDK: run `27146186151` ✅
  - Publish / Node.js SDK: run `27146186189` ✅
  - Publish / Java SDK: run `27146186292` ✅
  - Publish / Go SDK: run `27146186464` ✅
  - Publish / Docker server: run `27146186396` ✅
  - Release / GitHub assets: run `27146186259` ✅
- Tag-triggered Publish / Docker web run `27146185121` failed under `oven/bun:latest` / Bun `1.3.14`; fixed by commit `0eaf04f` and manually re-published `tikeo-web:v0.2.0` from `main` using workflow_dispatch run `27147150174` ✅.
- Main CI and Coverage after release prep succeeded:
  - Release-prep CI: run `27146163208` ✅
  - Release-prep Coverage: run `27146163363` ✅
  - Docker-web-fix CI: run `27147138165` ✅
  - Docker-web-fix Coverage: run `27147138007` ✅

Notes:
- The `v0.2.0` source tag was not moved after successful SDK publishes; only the Docker web image workflow was repaired and rerun with the same image tag from the follow-up `main` fix commit.
- User restored the shutdown instruction after briefly cancelling it; execute shutdown after this final memory commit/push.

### 2026-06-09 — Cross-language SDK Management API trigger parity
- Audited SDK Management API trigger support across Java, Rust, Go, Python, and Node.js.
- Confirmed Java already exposed `TikeoJobClient.triggerJob(...)` / `HttpTikeoJobClient.triggerJob(...)` and Spring Boot 2/3/4 demo controller endpoints for create+trigger use cases.
- Added trigger support to Rust, Go, Python, and Node.js management SDKs, including returned `JobInstance` models and `api` trigger helpers.
- Completed trigger request parity with the server/Web contract by supporting default `executionMode=single` and explicit broadcast helpers/selectors in Rust, Go, Python, and Node.js.
- Updated Rust/Go/Python/Node worker demos so `TIKEO_MANAGEMENT_CREATE_EXAMPLES=1` creates example API jobs and immediately calls `POST /api/v1/jobs/{job}:trigger`, printing returned instance evidence.
- Updated demo README files, including Java Spring Boot 2/3/4 create+trigger endpoint documentation.
- Updated the architecture roadmap to mark Python and Node.js SDKs as complete for the current Worker/demo/management-trigger scope.

Verification:
- `go test ./... -count=1` in `sdks/go/tikeo` ✅
- `bun test && bun run build` in `sdks/nodejs/tikeo` ✅
- `uv run --extra test python -m pytest` in `sdks/python/tikeo` ✅
- `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features` ✅
- `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `go test ./... -count=1` in `examples/go/worker-demo` ✅
- `bun test` in `examples/nodejs/worker-demo` ✅
- `uv run --with '../../../sdks/python/tikeo[test]' --extra test python -m pytest` in `examples/python/worker-demo` ✅
- `cargo test --manifest-path examples/rust/worker-demo/Cargo.toml` ✅
- Java Spring Boot 2/3/4 demos `./gradlew test --no-daemon` ✅
- `git diff --check` ✅
- `python3 scripts/check-source-size.py` ✅
- `cargo fmt --all -- --check` ✅

### 2026-06-09 — Job edit namespace/app migration

Agent:
- Codex

Work:
- Enabled `PATCH /api/v1/jobs/{job}` to accept `namespace` and `app` updates instead of silently preserving the original job scope.
- Persisted job scope moves in storage by updating `namespace_id` / `app_id` and creating a new immutable job version when the scope changes.
- Added source and target scope authorization checks so moving a job requires permission on both the current and destination namespace/app.
- Added canary target validation for job updates so a moved job cannot retain or set a canary target outside the destination namespace/app.
- Updated the Web Jobs edit drawer to select namespace/app from tenant scope management and to filter/clear canary targets by the selected scope.
- Updated README-protocol-required design roadmap and follow-up prompt for this scope-edit slice.

Verification:
- RED observed before implementation: `cargo test -p tikeo-server job_management_update_can_move_namespace_and_app -- --nocapture` failed because the update still returned `default`; `cd web && bun test src/api/client.test.ts src/pages/__tests__/JobsPage.test.tsx` failed because the edit drawer still had disabled namespace/app fields.
- `cargo test -p tikeo-server job_management_update_can_move_namespace_and_app -- --nocapture` ✅
- `cd web && bun test src/api/client.test.ts src/pages/__tests__/JobsPage.test.tsx` ✅
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cd web && bun run typecheck && bun run test && bun run build` ✅ (`vite build` only emitted the existing large vendor chunk warning)
- `python3 scripts/check-source-size.py` ✅
- `git diff --check` ✅

Notes:
- Direct `cd web && bun test` is not the project test command; it also loads Playwright e2e specs and fails with the known Playwright runner boundary. The configured project script `bun run test` correctly runs `bun test src` and passed.
- Existing local uncommitted files not owned by this slice remain unstaged: `examples/java/spring-boot2-worker-demo/src/main/resources/application.yml` and `tikeo-dev.db`.

Git:
- Pending commit and push for the job scope edit slice.
