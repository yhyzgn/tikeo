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
- 新增 `tikee-core`、`tikee-config`、`tikee-server` 三个 crate。
- 实现 `tikee serve --config config/dev.toml`。
- 实现 Axum `/healthz` 与 `/readyz`。
- 增加配置加载、health handler 单元测试。
- 增加 `config/dev.toml`、`rustfmt.toml`、GitHub Actions CI。
- 更新下一阶段提示词 `.prompt/002-http-api-and-openapi.md`，新增 `.prompt/003-worker-tunnel.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikee -- serve --config config/dev.toml` ✅
- `curl -fsS http://0.0.0.0:9090/healthz` ✅ returned `{"status":"ok","uptime_seconds":0}`
- `curl -fsS http://0.0.0.0:9090/readyz` ✅ returned `{"status":"ok","uptime_seconds":0}`

Git:
- 待提交并推送。


## 2026-05-19 — 调整后端主程序入口到根 src/main.rs

Agent:
- Codex

Work:
- 根据用户要求将后端主程序入口从 `crates/tikee-server/src/main.rs` 移到根 `src/main.rs`。
- 根 package `tikee` 只保留 binary entrypoint，实际 server 逻辑仍委托 `tikee-server` crate。
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
- `cargo run --bin tikee -- serve --config config/dev.toml` ✅
- `GET /healthz` ✅
- `GET /readyz` ✅
- `GET /api-docs/openapi.json` ✅ contains `/api/v1/system/info` and `/api/v1/jobs`
- `GET /api/v1/system/info` ✅ returned tikee metadata
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
- 根据用户要求，将已完成工作项在 `design/tikee-architecture-design.md` 开发路线图中标记为 `[x] ✅`。
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
- 新增 `crates/tikee-proto`，使用 tonic/prost 生成 Worker Tunnel gRPC bindings。
- 新增 `proto/tikee/worker/v1/worker.proto` 作为仓库级协议源。
- 定义最小 Worker Tunnel 消息：RegisterWorker、Heartbeat、WorkerRegistered、Ping。
- 实现 server 侧 `WorkerTunnelService::Connect` skeleton。
- 实现内存 `WorkerRegistry`，记录 worker id、app、namespace、cluster、region、capabilities、labels 和 heartbeat sequence。
- server 启动时同时监听 HTTP `9090` 与 Worker Tunnel gRPC `9998`。
- 设计路线图中将 “gRPC 协议定义与代码生成” 标记为完成 `[x]`。
- 新增 `.prompt/004-storage-and-tikee.md`。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikee -- serve --config config/dev.toml` ✅
- HTTP `/healthz` ✅
- OpenAPI `/api-docs/openapi.json` ✅
- Worker Tunnel TCP listener `0.0.0.0:9998` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 005-basic-tikee

- `tikee-core` 新增调度领域模型：`ScheduleType`、`TriggerType`、`InstanceStatus`、`DispatchDecision`。
- `tikee-storage` 新增 `JobInstanceRepository`，支持创建 pending job instance、按 job 查询实例、按 id 查询实例。
- HTTP 新增 `POST /api/v1/jobs/{job}:trigger`，实现 API 手动触发并返回统一 `{code,message,data}` envelope。
- HTTP 新增 `GET /api/v1/jobs/{job}/instances` 与 `GET /api/v1/instances/{instance}`，支持实例列表与详情查询。
- OpenAPI schema 已补充 TriggerJobRequest、JobInstanceSummary、JobInstancePage。
- 设计路线图已将 API 手动触发实例链路作为基础调度器子项标记完成；CRON / Fixed Rate tick loop 仍待后续阶段。


## 2026-05-19 — 006-worker-sdk-rust-and-java-starter

- Worker Tunnel proto RPC 从 `Connect` 改为 `OpenTunnel`，解决 tonic client 生成方法名冲突。
- `tikee-proto` 开启 tonic client 生成。
- 新增 `sdks/rust`，实现 Rust Worker SDK 最小主动连接、注册、心跳客户端。
- Rust Worker SDK 增加 `TaskProcessor` / `TaskContext` / `TaskOutcome` 基础处理器接口，为后续任务分发做准备。
- Rust Worker SDK 集成测试启动真实 tonic Worker Tunnel server，验证 register ack 与 heartbeat ping。
- 新增 `sdks/java/` Gradle 多模块 SDK 骨架：`tikee`、`tikee-spring`、`tikee-spring-boot-starter`。
- Java core 提供 `@TikeeProcessor`、`WorkerRegistration`、`TikeeWorkerClient`、`NoopTikeeWorkerClient`。
- Spring Boot autoconfigure 提供 `tikee.worker.*` 配置、auto-configuration imports 和注解扫描 registry。


## 2026-05-19 — 007-web-ui-foundation

- 新增 `web/` Bun 工程，技术栈为 React 19、TypeScript 6、Vite 8、Ant Design 6。
- 建立 AppShell、Dashboard、Jobs、Instances 页面骨架。
- Jobs 页面支持调用 API 创建 Job 与 API trigger；Instances 页面展示实例列表。
- 新增 typed API client，统一解析 `{code,message,data}` envelope。
- 新增 Bun test API client 单元测试，覆盖成功与业务失败分支。
- 建立 `lint`、`typecheck`、`test`、`build` 脚本并验证通过。


## 2026-05-19 — 008-container-deployment

- 新增后端多阶段 Dockerfile：Rust release builder + Debian slim runtime，默认运行 `tikee serve --config /app/config/container.toml`。
- 新增 `config/container.toml`，容器内 HTTP `0.0.0.0:9090`、Worker Tunnel `0.0.0.0:9998`、SQLite dev 数据落 `/data/tikee.db`。
- 新增 Web Dockerfile：Bun 构建 React/Ant Design 静态资源，nginx 托管并代理 `/api/`、`/api-docs/` 到 tikee HTTP 服务。
- 新增 `docker-compose.yml`，包含 tikee server 与 web 两个服务；Worker Tunnel 只暴露为 worker 主动出站连接入口。
- 新增 `deploy/k8s/tikee.yaml` 与 README，包含 Namespace、ConfigMap、SQLite dev PVC、server Deployment/Service、worker tunnel Service、web Deployment/Service。
- 新增 Docker ignore 规则，避免 target、node_modules、dist 进入镜像构建上下文。
- 设计路线图已将 Docker 镜像构建标记完成；后续 Helm Chart 仍保留在 Phase 3。

Verification:
- `docker compose config` ✅
- `docker build -t tikee:dev .` ✅
- `docker build -t tikee-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web HTML + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅
- `deploy/k8s/tikee.yaml` PyYAML 结构解析 ✅（8 documents；当前环境无 `kubectl`）
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
- `tikee` 新增 `WorkerSession::process_next`，接收 dispatch、构造 `TaskContext`、调用 `TaskProcessor`、回传 `TaskOutcome`。
- Storage 新增 pending instance 查询与 status update repository 方法。
- 测试覆盖 repository 状态流转、server dispatch、SDK dispatch -> processor -> result 回传。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikee -- serve --config config/dev.toml` + `/healthz` + `/api/v1/jobs` smoke ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t tikee:dev .` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅
- `docker build -t tikee-web:dev ./web` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 010-tikee-tick-loop

- 新增 `tikee-server::tikee` 自动调度 tick loop。
- Storage 新增 `list_enabled_scheduled_jobs`，只查询 enabled 的 `cron` / `fixed_rate` jobs。
- CRON 使用 `cron 0.16.0` 解析表达式，Fixed Rate 使用 `humantime 2.3.0` 解析持续时间表达式。
- Tick loop 使用内存 cursor 避免同一 tick 重复触发；到期时创建 pending job_instance，并复用 009 dispatch loop。
- Server 启动时同时运行 HTTP、Worker Tunnel、自动 tikee tick loop 和 Worker dispatch loop。
- 测试覆盖 fixed_rate 到期触发、cron 到期触发、disabled scheduled job 不触发。
- 设计路线图已标记基础调度器 CRON/Fixed Rate/API 子项完成，Rust SDK 任务执行子项完成。

Verification:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `cargo run --bin tikee -- serve --config config/dev.toml` + fixed_rate job 自动创建 pending instance smoke ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t tikee:dev .` ✅
- `docker build -t tikee-web:dev ./web` ✅
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
- `docker build -t tikee:dev .` ✅
- `docker build -t tikee-web:dev ./web` ✅
- `docker compose up -d --no-build` + `/healthz` + Web nginx `/api/v1/jobs` 代理 ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — 012-auth-rbac-foundation

- 新增后端开发管理员认证模块：`POST /api/v1/auth/login`、`GET /api/v1/auth/me`、`POST /api/v1/auth/logout`。
- 支持 env 覆盖开发管理员用户名、密码与 token：`TIKEE_DEV_ADMIN_USERNAME`、`TIKEE_DEV_ADMIN_PASSWORD`、`TIKEE_DEV_ADMIN_TOKEN`。
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
- `cargo run --bin tikee -- serve --config config/dev.toml` + `/healthz` + `/api/v1/auth/login` + `/api/v1/auth/me` + protected `POST /api/v1/jobs` smoke ✅
- `./sdks/java/gradlew -p sdks/java test` ✅
- `bun install --cwd web` ✅
- `bun run --cwd web lint` ✅
- `bun run --cwd web typecheck` ✅
- `bun test --cwd web` ✅
- `bun run --cwd web build` ✅
- `docker compose config` ✅
- `docker build -t tikee:dev .` ✅
- `docker build -t tikee-web:dev ./web` ✅
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
- Web Dockerfile/nginx 按分层构建与 nginx runtime 调整；Compose 使用默认 bridge 网络，Web 通过服务名 `tikee` 反向代理后端。

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
- `DOCKER_BUILDKIT=1 docker build -t tikee:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikee-web:dev ./web` ✅
- `docker compose up -d --no-build` on default bridge ✅
- `curl /healthz`, Web `/`, proxied `/api/v1/system/info`, proxied/direct `/api-docs/openapi.json` ✅
- `docker compose down` ✅

Git:
- 待提交并推送。


## 2026-05-19 — dev startup script + config directory

- Renamed runtime configuration directory from `examples/` to `config/` and updated Dockerfile, Compose, prompt, memory, and design references.
- Added `scripts/dev.sh` to start backend + Web dev server together, wait for health checks, print browser/API URLs, and write logs under `.dev/`.
- Added root `README.md` with local startup instructions, configuration directory contract, and initialization credentials.
- Updated built-in development initialization account defaults to `tikee_init` / `Tikee@2026!` / `tikee-init-token`; env overrides remain available.
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
- `DOCKER_BUILDKIT=1 docker build -t tikee:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikee-web:dev ./web` ✅
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
- `cargo run --bin tikee -- serve --config config/dev.toml` + healthz/jobs/reported instances endpoint smoke ✅
- `./scripts/dev.sh` backend + Web startup smoke ✅

## 2026-05-20 — 接手用户管理并抽象 SessionStore

Agent:
- Codex

Work:
- 接手他人已开发的用户管理/RBAC 模块。
- 新增 `crates/tikee-server/src/http/session.rs`，定义 `SessionStore` trait、`SessionManager` 和当前 `DbMokaSessionStore`。
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
- `cargo run --bin tikee -- serve --config config/dev.toml` + `/healthz` + `/auth/login` 冒烟 ✅，登录返回 `atk_` opaque token。

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
- `tikee-core` 新增 `ScriptLanguage`（Shell/Python/Node/PowerShell/Rhai/Wasm）和 `ScriptStatus`（Draft/Approved/Disabled）枚举，含 `FromStr`/`Display`/`as_str()` 及测试。
- `tikee-storage` 新增 `scripts` 表 SeaORM entity（id/name/language/version/content/status/timeout_seconds/max_memory_bytes/allow_network/allowed_env_vars/created_by/created_at/updated_at），无外键。
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

- 删除 `tikee-init-token` 静态 admin bearer 后门；后端测试改为先通过初始化账号登录获取真实 `atk_` session token。
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
- 后端大文件拆分：`crates/tikee-storage/src/repository.rs` 拆成 `repository/*`；`crates/tikee-server/src/http/routes.rs` 拆成 `routes/*`。

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
- 已在 `crates/tikee-storage/src/repository/workflow.rs` 扩展 workflow 节点类型白名单，避免 Web 新节点保存时报 unsupported node kind。

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
- 已迁移 Rust Worker SDK 到 `sdks/rust/tikee`，Java 多模块 SDK 到 `sdks/java`。
- 根 Cargo workspace 已恢复为仅包含服务端与 `crates/*`；Rust SDK 独立于根 workspace 构建发布。
- Dockerfile 分层缓存、README、.gitignore、历史 prompt/memory 验证命令和设计文档结构图已同步到新目录。

### 2026-05-21 041 Dispatch Queue Claim/Lease
- 按 030 阶段继续推进队列多节点竞争基础能力。
- `WorkflowRepository` 新增 `claim_next_dispatch_queue_item`、`claim_dispatch_queue_item`、`release_dispatch_queue_item`，使用 lease_owner / lease_until 控制可占用性。
- HTTP 新增 `POST /api/v1/dispatch-queue:claim`，记录 `dispatch_queue` 的 `claim` 审计日志。
- 当前仍是最小实现；下一步建议继续把 dispatcher 的 materialize/dispatch 流程切到 claim API/原子条件更新路径，并补 visibility-timeout 回收。

### 2026-05-21 042 dev.sh 本地访问 URL 调整
- 用户手动修改 `scripts/dev.sh` 后要求代提交。
- 变更保留容器/服务绑定可配置性，默认 API_URL 改为 `http://localhost:$TIKEE_API_PORT`，WEB_URL 改为独立可覆盖的 `TIKEE_WEB_URL`。
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
- 执行用户要求的 SDK 目录整改：`sdks/rust/tikee`、`sdks/java/<sdk-name>`。
- Java 构建从 Maven 切换到 Gradle Kotlin DSL + JDK21+；根 `gradlew` 会按需下载 Gradle。
- 创建 examples `<language>/<demo-name>` 目录骨架，仅放 demo/README，不放运行配置。
### 2026-05-21 SDK layout correction follow-up
- 用户明确根 `Dockerfile` 只构建 tikee 服务端；已约束不得复制/缓存/构建 `sdks/` 或 `examples/`。
- SDK 路径规范固定为 `sdks/<language>/<sdk-name>/`，Demo 路径规范固定为 `examples/<language>/<demo-name>/`。
- Rust SDK 路径为 `sdks/rust/tikee`；现已移除 repo-local path dependencies，满足独立发布约束。
- 已补齐可独立运行的 Rust demo（`examples/rust/worker-demo`）与 Java Spring Boot demo（`examples/java/spring-worker-demo`）基础。

### 2026-05-21 verification — SDK layout correction
- `./sdks/java/gradlew -p sdks/java test` ✅
- `./sdks/java/gradlew -p examples/java/spring-worker-demo test` ✅
- `cargo fmt --all -- --check` ✅
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features` ✅
- `cargo run --manifest-path examples/rust/worker-demo/Cargo.toml` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace --all-features` ✅
- `cargo build --workspace --all-features` ✅
- `bun run --cwd web lint && bun run --cwd web typecheck && bun test --cwd web && bun run --cwd web build` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikee:dev .` ✅
- `DOCKER_BUILDKIT=1 docker build -t tikee:dev .` ✅ after switching builder/runtime flow to Alpine-compatible server-only image build.
- `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo build --workspace --all-features` ✅ rerun after Dockerfile/dependency adjustments.

### 2026-05-21 Rust SDK independent publishing cleanup
- Removed `sdks/rust/tikee` from root Cargo workspace and removed Dockerfile rewrite workaround.
- Made Rust SDK self-contained by bundling `proto/worker.proto`, local `build.rs`, and removing all `../../../crates/*` path dependencies.
- Replaced SDK integration tests with an in-crate mock Worker Tunnel server.
- `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `cargo package --manifest-path sdks/rust/tikee/Cargo.toml --allow-dirty` ✅ proves Rust SDK package has no repo-local path dependencies.

### 2026-05-21 Worker identity assignment cleanup
- Changed Worker Tunnel RegisterWorker payload from client-supplied `worker_id` to optional `client_instance_id`.
- Server registry now generates authoritative `wrk-*` worker ids and returns them in `WorkerRegistered`.
- Rust SDK stores server-assigned worker id after connect and uses it for heartbeat/log/result messages.

### 2026-05-21 verification — worker identity assignment cleanup
- `cargo test --workspace --all-features` ✅
- `cargo fmt --all -- --check` ✅
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features` ✅
- `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings` ✅
- `cargo package --manifest-path sdks/rust/tikee/Cargo.toml --allow-dirty` ✅
- `./sdks/java/gradlew -p sdks/java test` attempted first but Gradle distribution download hit `curl: (56) OpenSSL SSL_read ... unexpected eof while reading`.
- `~/.gradle/wrapper/dists/gradle-8.14-bin/.../bin/gradle -p sdks/java test` ✅ using cached Gradle.
- `~/.gradle/wrapper/dists/gradle-8.14-bin/.../bin/gradle -p examples/java/spring-worker-demo test` ✅ using cached Gradle.

### 2026-05-21 Java SDK Worker Tunnel implementation
- Implemented `GrpcTikeeWorkerClient` with server-assigned worker id registration semantics, heartbeat, task log emission, and dispatch result reporting.
- Added Java protobuf generation from bundled `worker.proto` in Java core SDK.
- Wired Spring Boot auto-configuration to create real gRPC client unless `tikee.worker.dry-run=true`.
- Updated Java Spring demo to default dry-run and smoke-run without live tikee.

### 2026-05-21 Java TikeeProcessor adapter
- Implemented invocable Spring processor handlers and `SpringTikeeTaskProcessor`.
- Wired Spring Boot autoconfiguration so live Java gRPC Worker Tunnel dispatches route to annotated processor methods.
- Added tests for context/string method invocation, exception failure mapping, duplicate processor rejection, route-by-job-id convention, and autoconfig registry wiring.

### 2026-05-21 Java Lombok/style adjustment
- Added Lombok to Java SDK and Java demo builds.
- Converted demo runner to constructor-injected component and simplified Spring worker properties / dry-run client boilerplate with Lombok.

### 2026-05-21 Java SDK three-module restructure
- Renamed Java native SDK module to `tikee`.
- Split Spring Framework adapter into `tikee-spring`.
- Moved Spring Boot auto-configuration/properties into `tikee-spring-boot-starter` and updated AutoConfiguration imports.
- Updated Java demo and docs to use `tikee-spring-boot-starter`.

### 2026-05-21 Java Spring Boot starter naming correction
- Renamed `sdks/java/tikee-spring-boot` to `sdks/java/tikee-spring-boot-starter`.
- Updated Gradle settings/build, Java demo dependency, README/design/prompt/memory references.

### 2026-05-21 Worker processor key protocol
- Added explicit `processor_name` to DispatchTask across server/Rust SDK/Java SDK protocol copies.
- Updated server dispatch task construction and tests to assert processor key population.
- Updated Rust SDK TaskContext and Java TaskContext to carry processor name.
- Updated SpringTikeeTaskProcessor to route by explicit processor name instead of job id fallback-only convention.

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
- Enabled `sqlx-postgres` on `tikee-storage` and migrations so PostgreSQL URLs compile through SeaORM/sqlx.
- Added `config/postgres.toml` with PostgreSQL and CockroachDB URL examples; CockroachDB uses PostgreSQL wire protocol.
- Roadmap marks PostgreSQL + CockroachDB storage support complete at driver/config/template level; live DB smoke remains environment-dependent.

### 2026-05-21 Phase2 cluster coordinator foundation
- Added `tikee-server::cluster` with ClusterCoordinator trait, explicit ClusterMode/ClusterRole, and StandaloneCoordinator.
- `/api/v1/cluster` now reports `role=standalone` with node_id/can_schedule/detail instead of fake `leader`.
- Design now records Raft implementation boundaries: leader ownership gate, follower fencing, DB claim as final idempotency guard, and container-friendly networking.

### 2026-05-21 Phase2 cluster ownership gates
- Tikee tick loop and Worker dispatcher loop now consult `ClusterCoordinator` status before ownership-sensitive work.
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
- Kept current storage-backed no-op coordinator in `tikee-server::cluster`; no new `tikee-cluster` crate yet because runtime boundaries are not stable enough.

### 2026-05-21 Phase2 cluster diagnostics
- Added `/api/v1/cluster/diagnostics` for operator-visible cluster readiness: current status, scheduling gate, persisted Raft metadata, members, transport placeholder, and runtime boundary.
- Chose a separate diagnostics endpoint instead of bloating `/api/v1/cluster`; the lightweight status endpoint stays stable for UI polling.
- Kept cluster runtime in `tikee-server::cluster` for now; no `tikee-cluster` crate until consensus/runtime traits stabilize.

### 2026-05-21 Phase2 dispatch queue fencing token
- Reviewed Phase2: only full Raft runtime remains incomplete; Go/Python SDK stays Phase4.
- Added `dispatch_queue.fencing_token` shape and SQLite compatibility migration; claim responses now include a fencing token.
- Dispatcher now derives a fencing token from ClusterCoordinator status (`standalone:<node>:tikee-dispatcher` today, future `raft:<node>:<leader-token>` when real consensus exists).

### 2026-05-21 Phase2 closeout / Phase3 audit paging
- Consensus dependency direction corrected to TiKV raft-rs (`raft` 0.7.0); full Raft scheduling still stays gated until event-loop/transport/persistence/fencing are real.
- Phase2 distributed safety foundations are documented as complete except real Raft runtime/membership.
- Started Phase3 audit governance by adding server-side audit filters and pagination plus Web UI filter controls.

### 2026-05-21 Phase2 raft-rs correction
- User corrected the OpenRaft direction; project now targets TiKV raft-rs (`raft` crate 0.7.0, Apache-2.0) instead of OpenRaft.
- Added `tikee-server::cluster::raft_rs` bootstrap validation: deterministic string `node_id` -> non-zero u64 raft id, peer voters, `MemStorage + RawNode` construction. This proves dependency/API integration only; no tick loop, campaign, leader token, or scheduling grant exists yet.
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
- Runtime does not campaign, does not wire outbound transport, and still keeps `can_schedule=false` and `leader_fencing_token=null`; tikee ownership remains fenced.
- Next slice: connect validated inbound HTTP messages to the runtime inbox, then implement Ready apply/outbound transport and real leader fencing.

### 2026-05-21 Phase2 raft-rs inbound runtime inbox
- Added a `ClusterCoordinator::submit_raft_message` boundary and wired `RaftRuntimeCoordinator` to enqueue validated `eraftpb::Message` values through a bounded mpsc inbox.
- `/api/v1/raft/append-entries` now returns `accepted=true` only when a running raft-rs runtime inbox accepts the message; standalone or stopped runtimes return `accepted=false` with a clear reason. This still does not grant scheduling ownership or a leader fencing token.
- Next slice: implement outbound peer HTTP transport and Ready apply/state-machine bookkeeping before enabling any leader fencing token.

### 2026-05-21 Phase2 raft-rs outbound transport skeleton
- Added optional `cluster.transport_token` config and `x-tikee-raft-token` support so internal Raft HTTP transport can bypass human session auth without committing production secrets.
- Wired Ready outbound messages through a `RaftPeerTransport` skeleton: raft-rs `Message` values serialize to the existing HTTP wire DTO, base64 payloads are preserved, peer URLs append `/api/v1/raft/append-entries`, and delivery runs asynchronously through reqwest.
- Tikee ownership remains fenced: no campaign, no leader token, no `can_schedule=true`. Next slice is committed-entry apply bookkeeping and fencing-token lifecycle.

### 2026-05-21 End-of-day handoff checkpoint
- User paused work for the day. Current pushed HEAD before this checkpoint: `222b1d6 Send raft-rs outbound messages through peer HTTP skeleton 📡`; working tree was clean before writing this memory checkpoint.
- Completed today: `fc67f13` runtime ticker + Ready persistence order, `dea7528` inbound runtime inbox, `222b1d6` outbound peer HTTP skeleton and optional internal Raft transport token.
- Verification evidence from last code slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` all passed.
- Tomorrow resume at `.prompt/053-phase2-raft-rs-apply-and-fencing.md`. Key safety rule: never set `can_schedule=true` or emit `leader_fencing_token` until real raft-rs leader state has generated and persisted a fencing token and dispatch/tikee gates consume it.
- Key files for resume: `crates/tikee-server/src/cluster/raft_rs.rs`, `crates/tikee-storage/src/repository/raft.rs`, `crates/tikee-server/src/cluster.rs`, `design/tikee-architecture-design.md`, `.prompt/053-phase2-raft-rs-apply-and-fencing.md`.

### 2026-05-22 Phase2 raft-rs apply bookkeeping and fencing lifecycle
- Resumed from `.prompt/053-phase2-raft-rs-apply-and-fencing.md`.
- Implemented Ready committed-entry apply bookkeeping using `advance_append` / `advance_apply_to` instead of blindly advancing without state-machine acknowledgement.
- Committed `EntryNormal` entries now monotonically update `raft_metadata.applied_index`; `EntryConfChange` / `EntryConfChangeV2` are explicitly gated and stop apply progress before silent membership mutation.
- Added leader fencing-token lifecycle: only a real raft-rs `Leader` with term > 0 derives `raft:term:<term>:node:<node_id>`, persists it first, then reports `can_schedule=true`; non-leaders clear the token. Tikee/dispatcher gates remain driven by `can_schedule` and dispatcher uses the persisted token.
- Targeted verification run so far: `cargo fmt --all`; `cargo test -p tikee-server raft --all-features`; `cargo test -p tikee-storage raft --all-features`.
- Next slice after commit: `.prompt/054-phase2-raft-rs-business-apply-membership.md`.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs business command envelope foundation
- Continued into `.prompt/054-phase2-raft-rs-business-apply-membership.md` after 053 commit.
- Added `raft_applied_commands` no-FK table/entity/repository for idempotent state-machine apply records keyed by `(node_id, log_index)` with `(cluster_id, command_id)` reserved for replay idempotency.
- `EntryNormal` payloads now parse as tikee command envelopes (`command_id`, `command_type`, `payload`). `noop` is applied, unknown command types are recorded as `deferred_unsupported`, invalid JSON is recorded as `rejected`, and apply index still advances deliberately.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-storage raft --all-features`; `cargo test -p tikee-server raft --all-features`.
- Next slice prompt: `.prompt/055-phase2-raft-rs-real-business-commands-and-membership.md`.
- Full verification passed for this slice: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build`.

### 2026-05-22 Phase2 raft-rs real member command apply
- Resumed from `.prompt/055-phase2-raft-rs-real-business-commands-and-membership.md` and pushed previous local commit `7f82709`.
- Added `raft_member_upsert` as the first real state-machine command. Scope is intentionally limited to member catalog metadata, so it is safe before dynamic ConfChange support.
- Added duplicate `command_id` replay guard before side effects. Replayed commands advance Raft apply bookkeeping but do not reapply member mutations or violate the unique `(cluster_id, command_id)` index.
- Updated design to document the dynamic membership two-layer flow: member catalog command first; future proposal API + raft-rs `propose_conf_change` + committed ConfState apply before changing voters/learners.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_apply_committed_entries --all-features`; `cargo test -p tikee-storage raft_tables_keep_soft_relationships_without_foreign_keys --all-features`.
- Full verification passed for 055: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs membership proposal intent API
- Continued automatically into `.prompt/056-phase2-raft-rs-membership-proposal-api.md` after committing 055.
- Implemented no-FK `raft_membership_proposals` storage and idempotent repository insert by `(cluster_id, proposal_id)`.
- Implemented `POST /api/v1/raft/members:propose` with `{ code, message, data }` envelope, `cluster:manage` RBAC, real-leader/fencing guard, http/https endpoint validation, self-removal block, and quorum-reduction block for unsafe remove proposals.
- Tests added for non-leader rejection, invalid endpoint rejection, and duplicate proposal idempotency.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_membership_proposal --all-features`; `cargo test -p tikee-storage raft_tables_keep_soft_relationships_without_foreign_keys --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Full verification passed for 056: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs committed ConfChange apply
- Continued `.prompt/057-phase2-raft-rs-confchange-apply.md`.
- Added `RaftMembershipProposal` and `RaftMembershipProposalSubmission` to the cluster trait boundary plus runtime command handling in `RaftRuntimeCoordinator`.
- Added `raft_metadata.conf_state` persistence with SQLite compatibility migration and diagnostics exposure.
- Implemented committed ConfChange handling: decode v1/v2, require runtime node to apply real membership changes, persist `ConfState` before updating `raft_members`, and update proposal status to `applied`/`rejected`.
- Added targeted tests for committed add-member happy path, malformed ConfChange handling, and no-runtime gating.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Full verification passed for 057: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs multi-node in-process E2E
- Resumed from `.prompt/058-phase2-raft-rs-multinode-e2e.md`.
- Added `TestRaftCluster` / `TestRaftNode` harness in `crates/tikee-server/src/cluster/raft_rs.rs` for deterministic in-process message routing between three raft-rs RawNodes.
- Added tests `raft_inprocess_harness_elects_real_leader_and_persists_fencing` and `raft_inprocess_membership_proposal_commits_and_applies_member`.
- Updated design roadmap item to completed and created `.prompt/059-phase2-raft-rs-http-transport-e2e-or-persistence-hardening.md` for HTTP transport/restart hardening.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_inprocess --all-features`; `cargo test -p tikee-server raft --all-features`.
- Full verification passed for 058: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs restart recovery hardening
- Resumed into `.prompt/059-phase2-raft-rs-http-transport-e2e-or-persistence-hardening.md`.
- Added `build_runtime_from_repository` and `restore_persisted_storage` in `crates/tikee-server/src/cluster/raft_rs.rs` to restore HardState/log entries into `MemStorage` on startup.
- Changed initial role metadata persistence to preserve existing raft term/log/applied/conf_state rows and only clear stale leader fencing.
- Added `.prompt/060-phase2-raft-rs-http-transport-smoke.md` for the next transport E2E/smoke slice.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_runtime_restore --all-features`; `cargo test -p tikee-server raft --all-features`.
- Full verification passed for 059: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs HTTP transport token smoke
- Resumed into `.prompt/060-phase2-raft-rs-http-transport-smoke.md`.
- Added `http::tests::raft_append_entries_internal_token_bypasses_human_session_only_for_transport` in `crates/tikee-server/src/http/mod.rs`.
- Created `.prompt/061-phase2-raft-rs-docker-bridge-e2e-script.md` for the remaining no-host-network Docker bridge E2E work.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server raft_append_entries_internal_token --all-features`.
- Full verification passed for 060: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase2 raft-rs Docker bridge E2E script
- Implemented `scripts/raft-bridge-e2e.sh` for no-host-network Docker bridge verification with 3 tikee containers and container-DNS raft peer endpoints.
- Fixed Dockerfile alpine build dependency gap for raft-proto by adding `protobuf-dev gcompat` to the builder stage; runtime remains alpine.
- Observed that bridge E2E may elect a real leader; script now accepts zero-or-one schedulable leader and requires any schedulable node to be `role=leader` with a fencing token.
- Created `.prompt/062-phase3-audit-before-after-trace-export.md` as the next roadmap slice.
- E2E verification passed: `./scripts/raft-bridge-e2e.sh`.
- Full verification passed for 061: `./scripts/raft-bridge-e2e.sh`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 audit before/after trace result foundation
- Resumed into `.prompt/062-phase3-audit-before-after-trace-export.md`.
- Updated audit storage/model/API/Web for before/after/trace_id/result/failure_reason.
- Updated design SQL sketch and roadmap: before/after trace/failure foundation complete; export governance remains `.prompt/063-phase3-audit-export-governance.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cargo test -p tikee-storage migration_creates_metadata_tables --all-features`.
- Full verification passed for 062: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun run build` (Vite chunk-size warning only).

### 2026-05-22 Phase3 governed audit JSON export
- Resumed into `.prompt/063-phase3-audit-export-governance.md`.
- Implemented `export_audit_logs` route and DTOs for governed JSON audit export; routed `/api/v1/audit-logs:export` and added OpenAPI registration.
- Updated Web audit client/page to download current-filter JSON exports.
- Created `.prompt/064-phase3-web-danger-confirm-permission-actions.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server audit_logs_support_server_side_filters_and_pagination --all-features`; `cd web && bun run typecheck`.
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
- Implemented stable `tikee-core` WASM processor spec and default-deny validation for network/filesystem capabilities.
- Updated design roadmap and created `.prompt/067-phase3-wasm-worker-runtime-executor.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-core --all-features`.
- Full verification passed for 066: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM worker runtime executor
- Started `.prompt/067-phase3-wasm-worker-runtime-executor.md`.
- Added `tikee-wasm` crate with Wasmtime executor and policy tests; no server HTTP/storage coupling.
- Updated design roadmap and created `.prompt/068-phase3-wasm-script-binding-and-dispatch.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-wasm --all-features`; `cargo clippy -p tikee-wasm --all-targets --all-features -- -D warnings`.
- Full verification passed for 067: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build` (Vite chunk-size warning unchanged).

### 2026-05-22 Phase3 WASM script binding and dispatch metadata
- Started `.prompt/068-phase3-wasm-script-binding-and-dispatch.md`.
- Added worker proto dynamic WASM binding metadata across server/Rust SDK/Java SDK proto files.
- Dispatcher attaches `WasmProcessorBinding` only for approved, policy-safe `script:<id>` WASM scripts and leaves regular SDK processor dispatch unchanged.
- Updated design roadmap and created `.prompt/069-phase3-wasm-sdk-execution-adapters.md`.
- Targeted verification so far: `cargo fmt --all`; `cargo test -p tikee-server tunnel::dispatcher --all-features`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features`; Java Gradle test attempted but stopped due slow first distribution download.
- Full verification passed for 068: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck`; `cd web && bun test`; `cd web && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features`. Java SDK Gradle test was attempted but not completed because the first Gradle distribution download was too slow; rerun once cached.
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
- Added default-deny script policy model to `tikee-core` and validation tests.
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
- Dispatcher binds only approved released immutable script snapshots, validates the released snapshot policy, and routes only to workers advertising `script:<language>`, `script:*`, or `*` capability.
- Rust SDK now has explicit runner registry routing for script bindings; Java SDK refuses script bindings until Java runner support is intentionally designed.
- Updated architecture roadmap and prepared `.prompt/075-script-runner-container-and-execution-governance.md`.
- Verification passed across Rust workspace, tikee-proto, dispatcher tests, Rust SDK native+wasm+clippy, Web typecheck/test/build, and Java Gradle tests.

### 2026-05-22 — Phase 075 container script runner foundation
- Added Rust SDK `ContainerScriptRunner` for Worker-side opt-in non-WASM script execution through a Docker-compatible CLI boundary.
- Honored the new modularity constraint by splitting `tikee/src/lib.rs` into focused Rust modules; future work should follow this pattern across server, web, and all SDK languages.
- Runner command boundary is default-deny: stdin script content, no container network, read-only rootfs, no host mounts, explicit tikee metadata env, and whitelisted env only.
- Added tests for Docker arg construction and dangerous policy rejection before runtime spawn.
- Prepared `.prompt/076-script-execution-governance-and-live-runner-smoke.md` for result/audit visibility and optional live runtime smoke.
- Verification passed after the Rust SDK module split: Rust workspace fmt/clippy/test/help, Rust SDK native+wasm+clippy, Web typecheck/test/build, and Java Gradle tests.

### 2026-05-22 — Project rename to tikee
- Renamed project identity from the previous project identity to tikee across source tree, crate/package names, SDKs, protocol namespaces, Docker/K8s/Compose config, docs, memory, and phase prompts.
- Java package prefix changed to `com.yhyzgn.tikee`; Java SDK modules changed to `tikee`, `tikee-spring`, and `tikee-spring-boot-starter`.
- Rust SDK path/crate changed to `sdks/rust/tikee` / `tikee`; root binary changed to `tikee`.
- Protobuf package changed to `tikee.worker.v1`; internal Raft transport header changed to `x-tikee-raft-token`; environment variables changed to `TIKEE_*` / `TIKEE__*`.
- Updated repository metadata target to `https://github.com/yhyzgn/tikee.git` and prepared git identity target `Neo <yhyzgn@gmail.com>`.
- Created `.prompt/077-script-execution-governance-after-tikee-rename.md` so future work resumes after the rename.
- Verification in progress: `cargo check --workspace --all-features` passed; `cargo fmt --all` completed. Full verification and git push status to be recorded before final response.

### 2026-05-22 — SDK naming contraction
- User requested Java SDK previous Java core SDK name -> `tikee` and Rust SDK previous Rust Worker SDK name -> `tikee`.
- Renamed directories to `sdks/java/tikee` and `sdks/rust/tikee`, updated Gradle project dependencies, Rust crate metadata, examples, docs, memory, and prompts.
- Verification after this additional rename is in progress.
- Fixed verification regression caused by renamed default admin password: regenerated the seeded BCrypt hash for `Tikee@2026!`.
- Full verification passed after SDK naming contraction:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`
  - `cargo build --workspace --all-features`
  - `cargo run -- --help`
  - `cd web && bun run typecheck && bun test && bun run build`
  - `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`
  - `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`
  - `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`
  - `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`
### 2026-05-23 — Phase 077 script execution governance visibility
- Continued after context reload and RTK activation.
- Implemented dispatcher governance logging for script fail-closed dispatch and no eligible script worker capability.
- Implemented Rust SDK failure class detection for missing runner, policy rejection, digest mismatch, timeout, output limit, and runtime unavailable; Worker task result messages now carry JSON failure metadata for recognized script runner failures.
- Server Worker Tunnel persists recognized JSON failure metadata as `script_execution_governance` instance logs.
- Documented script-capable Worker Pool deployment constraints and `ContainerScriptRunner` usage in `design/tikee-architecture-design.md` and `sdks/rust/tikee/README.md`.
- Created `.prompt/078-script-governance-audit-alerting.md`.
- Targeted verification passed: `cargo test -p tikee-server tunnel::dispatcher --all-features`; `cargo test -p tikee-server tunnel::service --all-features`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml script`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml worker_session_rejects_script_binding_without_registered_runner`.
- Full verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cd web && bun install && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.
### 2026-05-23 — Phase 078 script governance query/UI/alert foundation
- Continued from `.prompt/078-script-governance-audit-alerting.md`.
- Added structured parsing of `script_execution_governance` instance logs into API fields and added governance-only filtering via `page_token=script_execution_governance`.
- Updated Web instance log drawer to highlight governance failure classes instead of showing only raw JSON.
- Added `AlertCondition::ScriptGovernanceFailure` and tests for alert condition serialization/noop dispatch path.
- Created `.prompt/079-script-governance-audit-materialization.md`.
- Targeted verification passed: `cargo test -p tikee-server trigger_job_creates_pending_instance --all-features`; `cargo test -p tikee-server alert --all-features`; `cd web && bun test src/api/client.test.ts`.
- Full verification passed: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cd web && bun run typecheck && bun test && bun run build`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-23 — Workflow edge condition legacy seed/frontend normalization fix
- Investigated `PATCH /api/v1/workflows/wf-dev-basic-pipeline` 400 with payload edge condition `success` after user clarified the visible Web selector already uses `on_success`.
- Root cause: development seed persisted the old alias `success`; the Web editor loaded that stale definition into JSON draft and node-position edits preserved the stale edge condition through `updateWorkflow`, where backend validation correctly rejects non-canonical conditions.
- Fixed Web API boundary by normalizing legacy workflow edge condition aliases before create/update/dry-run: `success`/`succeeded` -> `on_success`, `failure`/`failed` -> `on_failure`; unknown values still pass through so backend remains authoritative.
- Fixed editor load path to stringify the normalized definition, so the page draft no longer keeps stale aliases after opening an existing workflow.
- Updated `scripts/dev-seed.sql` to seed `wf-dev-basic-pipeline` and its edge row with `on_success`.
- Added Bun regression tests covering mutation and dry-run serialization of legacy aliases.
- Verification passed: `git diff --check -- web/src/api/client.ts web/src/api/client.test.ts web/src/pages/WorkflowsPage.tsx scripts/dev-seed.sql`; `cd web && bun run typecheck && bun test && bun run build`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-23 — Script editor secondary page and workflow canvas fullscreen
- Replaced the cramped script edit modal with a guarded secondary route `/scripts/:id/edit` and `ScriptEditorPage`; list edit actions now navigate to the page.
- The script editor page keeps the existing diff-before-save governance flow while giving the CodeMirror editor a wider layout and separating basic metadata, runtime limits, and policy controls into side cards.
- Added editable workflow DAG canvas fullscreen toggle with Escape-to-exit and body scroll lock; the existing DAG data model, node editing, edge editing, JSON/YAML, and dry-run flows remain unchanged.
- Added source-level Web tests for the new script edit route/page contract and workflow fullscreen affordance.
- Verification passed: `cd web && bun run typecheck`; `cd web && bun test && bun run build`; `git diff --check` on changed files; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo run -- --help`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`; `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`; `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`; `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`.

### 2026-05-23 — Script editor back button compact style
- Fixed the secondary script editor back button being stretched by the hero flex-column container by constraining `.workflow-back-button.ant-btn` to `align-self: flex-start` and `width: auto`.
- Verification passed: `cd web && bun run typecheck && bun test && bun run build`.


### 2026-05-23 — Roadmap adjustment: migration and deployment scope
- User requested deferring Node.js SDK, K8s Helm Chart, and PowerJob migration tool from Phase 3 to Phase 4.
- Updated `design/tikee-architecture-design.md` Phase 3/4 roadmap accordingly.
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
- Added `observability/grafana/tikee-phase3-dashboard.json` as a deterministic Grafana dashboard template for existing Prometheus metrics: HTTP request rate, HTTP p95 latency, connected workers, worker dispatch outcomes, and an HTTP error-ratio SLO placeholder.
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
- Added `GET /api/v1/auth/oidc/callback` as a safe callback contract that validates code/state shape but refuses to create sessions until real token exchange/JWKS verification exists.
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
