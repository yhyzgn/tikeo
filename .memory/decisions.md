# 决策记录

## 2026-05-19 — 开发交接协议

Decision:
- 使用 `prompt.md` 作为跨 AI 智能体开发总提示词。
- 使用 `.memory/` 保存长期项目记忆。
- 使用 `.prompt/` 保存有序阶段提示词。

Rationale:
- 项目周期长，可能由 Codex、Claude、Gemini、OpenCode 等不同智能体接手。
- 需要保证上下文、验证证据、下一步任务和提交状态持续可追溯。

Constraint:
- 每次推进后必须更新记忆库和后续阶段 prompt。


## 2026-05-19 — Rust workspace 与 Web 技术栈约束

Decision:
- Rust 项目必须使用 Cargo workspace。
- 所有 Rust 模块抽取为独立 crate，统一放在 `./crates/` 下。
- Web 管理端必须独立放在 `./web/` 下。
- Web 技术栈固定为 React + TypeScript + Ant Design。
- Web 包管理工具固定使用 Bun。
- 每段代码改动后必须编译、运行、测试，通过后自动提交并推送远程 git。

Rationale:
- 保持后端模块完全解耦，便于长期演进和多智能体并行开发。
- Web 端独立工程能降低前后端耦合，并明确 UI 技术栈。

Constraint:
- 后续不得新建 `webui/` 作为前端工程目录；设计文档中的旧 `webui` 表述应统一迁移为 `web`。


## 2026-05-19 — 依赖版本策略

Decision:
- Rust crate、Web 依赖、构建工具、测试工具和运行时依赖默认使用当前最新稳定版。
- 若因为兼容性、生态稳定性、许可证、安全策略或框架约束不能使用最新版，必须记录原因、锁定版本、风险和未来升级条件。

Rationale:
- 新项目无历史包袱，优先使用最新稳定依赖可以降低后续迁移成本并获得安全修复。

Constraint:
- 禁止随意引入维护停滞、漏洞未修复或生态风险高的依赖。


## 2026-05-19 — 后端主程序入口位置

Decision:
- 后端主程序入口不放在 `crates/` 内，而是放在仓库根 `src/main.rs`。
- `crates/` 只承载解耦后的库模块 crate，例如 server、config、core、proto、storage 等。
- 根 package `scheduler` 只负责 binary entrypoint 和启动委托，不承载业务模块。

Rationale:
- 符合用户对主程序入口位置的明确要求，同时保留 workspace + crates 模块解耦。

Constraint:
- 后续不得在根 `src/` 下堆业务模块；业务能力必须继续进入 `crates/*`。


## 2026-05-19 — OpenAPI 生成库选择

Decision:
- HTTP/OpenAPI 阶段选择 `utoipa`；禁止 API 文档 UI 依赖。
- OpenAPI JSON 暴露路径使用 `GET /api-docs/openapi.json`；不提供文档 UI。

Rationale:
- `utoipa` 当前稳定、Axum 集成成熟，适合从 Rust handler/DTO 生成 OpenAPI。
- 已移除 API 文档 UI 依赖；后端手写暴露 `/api-docs/openapi.json`；不提供浏览器文档 UI。

Constraint:
- 后续 Web API client 应以该 OpenAPI 文档为输入生成或校验。


## 2026-05-19 — HTTP 业务接口统一响应体

Decision:
- 所有 HTTP 业务接口必须返回 `{code, message, data}` 三个字段。
- `code` 是业务成功判断标准，int `0` 表示成功，非 `0` 表示失败。
- `message` 是响应信息。
- `data` 是响应数据，必须显式返回；无数据时返回 `null`。
- HTTP 状态码仍保留协议语义，但客户端业务判断以响应体 `code` 为准。

Rationale:
- 提供稳定、统一、便于前端和外部系统接入的 API 契约。

Constraint:
- 后续不得新增裸 JSON DTO、Problem Details 顶层格式或只依赖 HTTP status 的业务接口。


## 2026-05-19 — 开发路线图完成状态回写

Decision:
- 每个开发工作项完成后，必须更新 `design/scheduler-architecture-design.md` 的开发路线图。
- 已完成项使用 `[x]` 标记，并可补充实际完成范围说明；不额外添加 ✅ 图标。

Rationale:
- 设计文档是项目进度和范围共识源，路线图必须随实现推进同步更新。

Constraint:
- 后续提交若完成开发项但未更新路线图，视为交接上下文不完整。


## 2026-05-19 — Java SDK 优先支持 Spring Boot Starter

Decision:
- 除规划中的 Rust/Go/Python/Node 等 SDK 外，Java 端 SDK 必须支持。
- Java SDK 优先实现 Spring Boot Starter 模式，包括 starter、autoconfigure、annotation processor adapter 和 lifecycle 集成。

Rationale:
- 企业 Java/Spring Boot 业务接入调度平台时需要最小改造和自动配置体验。

Constraint:
- Java Worker 仍必须主动连接 scheduler，不得要求业务应用暴露入站端口。


## 2026-05-19 — Worker Tunnel 最小监听端口

Decision:
- 当前阶段使用独立 gRPC Worker Tunnel 监听地址，默认 `0.0.0.0:9998`。
- HTTP 管理 API 继续使用 `0.0.0.0:9090`。

Rationale:
- 先用最小可验证切片落地 Worker 主动连接协议和 server skeleton；后续可再评估单端口 h2/multiplex。

Constraint:
- 即使使用独立端口，Worker 仍必须主动出站连接，Server 不直连 Worker。


## 2026-05-19 — SeaORM 稳定线版本选择

Decision:
- 存储层使用 SeaORM / sea-orm-migration `1.1.20` 稳定线。
- 不采用 crates.io 当前标记为 latest 的 `2.0.0-rc.38`。
- `scheduler-storage` 同时启用 `sqlx-sqlite` 与 `sqlx-mysql` feature；开发默认 SQLite，生产 MySQL migration 能复用通用 SeaORM schema。

Rationale:
- 用户要求依赖尽量使用最新版，但调度平台存储层属于核心基础设施，不应以 RC 版本作为初始长期基线。
- `1.1.20` 是当前 1.x 稳定线可用版本，兼顾最新稳定与生态可靠性。

Constraint:
- SeaORM 2.0 发布稳定版后，应在独立阶段评估升级，不得在业务功能提交中顺手升级核心 ORM。


## 2026-05-19 — Worker Tunnel RPC 命名避免客户端冲突

Decision:
- Worker Tunnel protobuf RPC 从 `Connect` 调整为 `OpenTunnel`。
- Rust proto crate 开启 tonic client 生成，供 `scheduler-worker-sdk` 使用。

Rationale:
- tonic client 生成器会为客户端类型生成关联函数 `connect`；RPC 名称也叫 `Connect` 时会产生重复方法名。
- 使用 `OpenTunnel` 保留语义并解除 SDK client 生成阻塞。

Constraint:
- 后续 SDK 和文档应统一使用 `OpenTunnel` 作为 gRPC 方法名；概念层仍可称 Worker Tunnel connect/register。


## 2026-05-19 — Java SDK 初始依赖版本

Decision:
- Java SDK 使用 Spring Boot `4.0.6` 作为当前最新稳定基线。
- 不采用 Maven Central 当前 latest/release 指向的 Spring Boot `4.1.0-RC1`。
- gRPC Java 版本锁定为当前稳定 `1.81.0`，protobuf Java 预留 `4.34.1` 稳定线。

Rationale:
- 用户要求尽量使用最新版；SDK starter 属于企业接入面，应避免使用 RC 版本作为默认基线。

Constraint:
- Spring Boot 4.1 发布稳定版后，应单独评估升级。


## 2026-05-19 — Web 管理端依赖基线

Decision:
- Web 管理端使用 React `19.2.6`、Vite `8.0.13`、TypeScript `6.0.3`、Ant Design `6.4.3`、Bun `1.3.13`。
- 测试使用 Bun test，API client 单元测试覆盖 `{code,message,data}` envelope 成功与失败分支。

Rationale:
- 新项目无历史包袱，按用户要求使用当前 latest stable 依赖。
- Bun test 与项目包管理器一致，减少前端工具链分裂。

Constraint:
- 浏览器端只能访问 scheduler HTTP/OpenAPI，不直接访问 Worker Tunnel。


## 2026-05-19 — 容器化部署基础形态

Decision:
- 后端使用仓库根 `Dockerfile` 产出 `scheduler` server 镜像，保持根 binary 入口，不把主程序入口迁入 `crates/`。
- Web 使用 `web/Dockerfile` 产出独立 nginx 静态资源镜像，并通过同源 `/api/`、`/api-docs/` 反向代理访问后端 HTTP API。
- Compose 与 K8s baseline 均使用独立 Worker Tunnel 服务入口；Worker 只主动出站连接，不要求业务容器暴露入站端口。

Rationale:
- 用户要求平台必须绝对支持 K8s/Docker/容器部署，且 server/worker 可位于不同容器或集群。
- Web 与后端分镜像便于独立扩缩、缓存和后续 CDN/Ingress 接入。

Constraint:
- 后续 Worker/SDK/sidecar 示例不得引入 Server 直连 Worker 的反向入站模型；生产 Helm 需要继续保留 HTTP 与 Worker Tunnel 的清晰服务边界。


## 2026-05-19 — Worker Dispatch 最小链路

Decision:
- 009 阶段采用最小 first-available worker dispatch loop：pending instance 由 Server 通过 Worker Tunnel 下发，Worker SDK 调用 `TaskProcessor` 后回传 `TaskResult`，Server 更新实例状态。
- proto 继续维持单个 `OpenTunnel` 双向流，新增 server-to-worker `DispatchTask` 和 worker-to-server `TaskResult`。

Rationale:
- 先打通端到端执行闭环，再在后续阶段补能力匹配、租约、重试、日志和分布式队列。

Constraint:
- 即使存在 server-to-worker 指令，也必须经由 Worker 主动建立的 tunnel 返回；不得新增 Worker 入站 HTTP/gRPC 端口。


## 2026-05-19 — Backend runtime image uses Alpine

Decision:
- Backend Dockerfile uses a layered Rust builder with cargo dependency fetch caching and builds `x86_64-unknown-linux-musl`; final runtime image is `alpine:3.22`.

Rationale:
- Satisfies the project constraint that final runtime base image must be Alpine while retaining stable Rust build tooling and Docker layer caching.

Constraint:
- Future native dependencies must remain compatible with the musl static build path or explicitly document why the runtime image strategy changes.


## 2026-05-19 — Scheduler tick loop dependency baseline

Decision:
- CRON expression parsing uses `cron 0.16.0`; Fixed Rate duration parsing uses `humantime 2.3.0`.
- 010 keeps trigger cursor in memory and creates pending instances through the existing `JobInstanceRepository`.

Rationale:
- These are current stable crates from crates.io search, and the project needs a minimal automatic trigger loop before persistent scheduler metadata is designed.

Constraint:
- Before production deployment, schedule cursor and misfire handling must become durable and coordinated across server replicas.


## 2026-05-19 — Instance logs over Worker Tunnel

Decision:
- Worker 执行日志使用 Worker -> Server `TaskLog` tunnel 消息传输，Server 写入 `job_instance_logs`，HTTP/Web 从 Server 查询。

Rationale:
- 保持 Worker outbound-only 网络模型，同时给管理端提供实例日志闭环。

Constraint:
- 后续实时日志也必须复用 Worker 主动连接或 Server 侧事件推送，不得要求 Worker 暴露日志读取端口。



## 2026-05-19 — 开发期认证基础

- 012 阶段采用开发管理员 token 作为最小认证闭环，默认 `scheduler_init/Scheduler@2026!` -> `scheduler-init-token`，允许环境变量覆盖。
- 写操作先保护 Job 创建与手动触发，读接口、health、ready、OpenAPI 暂保持开放，便于开发和部署烟测。
- Web token 暂存在 `localStorage`，后续正式 RBAC/OIDC 阶段必须替换为更完整的会话、安全刷新与权限模型。


## 2026-05-19 — Docker 网络验证基线

- Docker 构建与运行验证不得使用 `--network host` 作为捷径。
- 最低验收基线是 Docker 默认 bridge 网络与 `docker compose` bridge 网络；Web 容器必须通过 Compose 服务名 / bridge DNS 代理访问 scheduler。
- 该约束用于提前暴露 WAF / LB / 多层网络下的问题，避免把本地 host 网络当成线上可行性证明。


## 2026-05-19 — 禁止 API 文档 UI

- 按最新约束，项目不使用 API 文档 UI，仅保留机器可读的 `/api-docs/openapi.json`。
- 保留 `/api-docs/openapi.json` 作为机器可读 OpenAPI JSON，供 SDK / CI / 外部系统集成使用。
- Web nginx 只代理 `/api/` 与 `/api-docs/`，不提供文档 UI 代理。


## 2026-05-19 — Broadcast execution foundation

- `ExecutionMode` supports `single` and `broadcast`; broadcast trigger creates per-worker `job_instance_attempts` for online workers.
- Dispatcher sends broadcast attempts only through each Worker's existing outbound `OpenTunnel`; no Worker inbound port or Server direct callback is introduced.
- Worker `TaskResult` first updates child attempt status, then aggregates parent instance to `succeeded` or `partial_failed`.
- HTTP exposes `GET /api/v1/instances/{instance}/attempts`; Web can trigger broadcast and inspect child attempts.
- Broadcast trigger validates that at least one Worker is online before creating the parent instance, avoiding orphan pending broadcasts.


## 2026-05-19 — Development bootstrap ergonomics

- Local configuration files live under `config/`, not `examples/`, because they are operational configuration rather than sample code.
- `scripts/dev.sh` is the canonical local development launcher for backend + Web UI during the active development cycle.
- Built-in initialization credentials are `scheduler_init` / `Scheduler@2026!` / `scheduler-init-token`; they are development-only and remain overrideable through `SCHEDULER_DEV_ADMIN_*`.


## 2026-05-19 — SQLite schema compatibility pass

- SeaORM's initial migration is already marked applied in existing dev SQLite files, so adding fields to that migration is insufficient for local upgrades.
- `connect_and_migrate` now runs a SQLite-only compatibility pass after normal migrations to add broadcast execution schema pieces idempotently.
- This is a local/dev compatibility bridge; future production schema changes should be separate versioned migrations before first production release.

## 2026-05-19 — Web visual direction

- Web UI direction is light modern SaaS control plane: clean cards, generous spacing, blue accents, clear hierarchy, and disabled future entries for modules not implemented yet.
- Do not expose clickable menu pages for backend capabilities that do not exist yet; show them as planned/disabled to avoid misleading operators.

## 2026-05-20 — SessionStore 抽象与 DB+moka 当前实现

Decision:
- HTTP 认证层只依赖 `SessionStore` trait 和 `SessionManager`，不直接操作 HashMap、moka、DB 或未来 Redis。
- 当前实现为 `DbMokaSessionStore`：DB `auth_sessions` 是权威状态，moka 是短生命周期本地读缓存。
- Token 明文仅在登录响应中返回一次；持久化和缓存索引均使用 `SHA-256(token)`。

Rationale:
- scheduler server 后续可能多节点部署，需要能够替换为 Redis 分布式 session 存储而不重写 auth/RBAC handler。
- 用户角色、密码或账号删除后必须能主动撤销已有 session。

Constraint:
- 后续新增 Redis session 时必须实现同一 `SessionStore` trait；不能让 HTTP handler 直接依赖 Redis client。

## 2026-05-20 — 数据库全库禁止外键

Decision:
- 全库严禁创建数据库级外键（`FOREIGN KEY` / `REFERENCES`）。
- 所有跨表关系只能通过字段命名（如 `job_id`、`user_id`）和 repository/service 逻辑进行软关联。
- SeaORM entity 不再声明 `belongs_to` / `has_many` relation，migration 和 SQLite 兼容建表 SQL 不得生成外键。

Rationale:
- 平台后续需要跨数据库、在线迁移、批量导入和分布式部署，数据库级外键会增加迁移和运维耦合。

Constraint:
- 后续新增表或字段时，必须用索引和业务校验维护关系完整性，不允许加外键。

## 2026-05-20 — Users 密码字段命名

Decision:
- `users` 表密码列命名为 `password`，不使用 `password_hash`。
- 字段内容仍必须保存 `BCrypt` hash，严禁保存明文密码。

Rationale:
- 用户要求字段名简化为 `password`，但安全语义不变。

Constraint:
- API 入参可以继续叫 `password` 表示明文输入；DB `users.password` 表示 hash 后的存储值。

## 2026-05-20 — 脚本管理必须支持 diff 对比

Decision:
- 所有脚本管理操作必须支持 diff 对比能力。
- 每次 content 或 policy 变更自动产生版本记录（存入 `script_versions` 表），不得覆盖历史版本。
- 提供 `GET /api/v1/scripts/{id}/versions` 查看版本历史，`GET /api/v1/scripts/{id}/diff?v1=&v2=` 任意两版本 diff。
- Web 脚本管理页面必须提供版本历史查看和 diff 视图（content diff + policy diff）。

Rationale:
- 动态脚本是高风险操作载体，版本可追溯和变更 diff 对比是安全审计和变更管理的基础要求。

Constraint:
- 后续脚本相关功能开发不得跳过版本历史和 diff 对比；任何 content/policy 更新必须写入版本表。
- 脚本编辑器必须支持语法检查（Shell/Python/Node 等），根据 language 实时校验，语法错误标红提示但不阻止保存。

## 2026-05-20 — 020 安全善后决策

- 删除 `scheduler-init-token` 静态 Bearer 后门；初始化账号仅通过 `/api/v1/auth/login` 获取 `atk_` session token。
- 审计日志不得保存明文 Bearer token；session 相关审计只能保存脱敏标识或不可逆摘要。
- 出站告警 Webhook 默认只允许 HTTPS，并拒绝 localhost/私网/link-local/metadata 目标；后续如需内网 webhook，必须显式 allowlist。

## 2026-05-20 — 021 RBAC 使用软关联 permission/resource/action

- RBAC 从单字符串 role check 升级为 `resource/action` 权限检查。
- 数据库新增 roles、permissions、role_permissions，但不创建任何外键；角色名仍保留在 users.role 以兼容现有用户模型。
- admin 角色保留全权限短路和全量默认权限绑定，operator/viewer 通过 seed 权限控制能力边界。
- HTTP 层统一走 `require_permission`，Web 使用同一权限模型隐藏菜单和显示 403。

## 2026-05-21 — SDK/examples 语言目录规范与 demo 自主创建

Decision:
- `sdks/` 是 SDK 总目录，其下必须按 `sdks/<language>/<sdk-name>/` 组织，例如 `sdks/rust/scheduler-worker-sdk`、`sdks/java/scheduler-spring-boot`。
- Java SDK 必须使用 Gradle（优先 Kotlin DSL）而不是 Maven，且 Java toolchain / source / target 必须支持 JDK 21+。
- `examples/` 是 demo 总目录，必须按 `examples/<language>/<demo-name>/` 组织，并与 `sdks/` 语言结构对应。
- `examples/` 不再用于存放运行配置；运行配置仍属于 `config/`。
- 后续开发者/AI agent 在实现或验证 SDK、Worker、任务执行、工作流或跨语言集成时，需要自行判断并主动创建/更新相应 demo 项目用于调试，不必等待用户显式要求。

Constraint:
- Rust SDK 路径为 `sdks/rust/scheduler-worker-sdk`；服务端 Dockerfile 不处理 SDK 构建或缓存。
- Java Maven `pom.xml` 骨架已要求迁移为 Gradle 多模块，验证命令统一为 `./sdks/java/gradlew -p sdks/java test`。

## 2026-05-21 — Server Dockerfile 与 SDK 解耦

Decision:
- 根 `Dockerfile` 只用于构建 scheduler server，不复制或缓存 `sdks/`。
- SDK 与 examples 必须通过各自目录内的语言生态命令独立构建/运行。

Constraint:
- 后续修改 SDK 不应影响服务端 Dockerfile 分层缓存，除非显式构建 SDK 镜像。
### SDK / Demo independent artifact rule (2026-05-21)
- Root `Dockerfile` is server-only and must not process `sdks/` or `examples/`; SDKs/demos are independent artifacts with their own build/run commands.
- SDK packages must use `sdks/<language>/<sdk-name>/`; demos must use `examples/<language>/<demo-name>/`.
- Java SDK is Gradle + JDK 21+ only; Maven `pom.xml` must not be reintroduced for Java SDK builds.

### SDK independent publishing rule (2026-05-21)
- Every language SDK must be independently publishable through its native package ecosystem.
- Rust SDK crates must not depend on repo-local `crates/*` path dependencies; protocol definitions must be bundled/generated locally or provided by a published crate.

### Worker identity assignment rule (2026-05-21)
- Authoritative `worker_id` is server-assigned during Worker Tunnel registration and returned in `WorkerRegistered`.
- Clients may only send optional `client_instance_id` hints plus metadata; heartbeats/logs/results must use the assigned worker id.
- Java SDK/starter configuration must expose `clientInstanceId` / `scheduler.worker.client-instance-id`, not `workerId`, until the server returns the authoritative id during tunnel registration.

### Java Worker Tunnel real client boundary (2026-05-21)
- Java SDK core owns gRPC/protobuf generated bindings inside `sdks/java/scheduler-java`; it must remain independently publishable and must not depend on server crates/modules.
- Spring Boot starter may expose dry-run mode for local demos, but production default is the real `GrpcSchedulerWorkerClient`.
- Remaining Java SDK gap is ergonomic `@SchedulerProcessor` method adaptation; do not revert to no-op as the default live client.

### Java processor dispatch convention (2026-05-21)
- Until Worker Tunnel protocol gains an explicit processor/key field, Java Spring adapter treats `DispatchTask.job_id` as the processor name for `@SchedulerProcessor` routing.
- Supported Java processor signatures are intentionally small and safe: zero args, `TaskContext`, UTF-8 `String`, or `byte[]`; return `TaskOutcome`, `String`, `boolean`, or `void`.
- Exceptions are mapped to failed `TaskOutcome` instead of escaping the Worker Tunnel processing thread.

### Java SDK Lombok and injection style (2026-05-21)
- Java SDK and Java demos should use Lombok where it meaningfully removes boilerplate, currently pinned to Lombok 1.18.46.
- Spring beans in SDK/demo code should prefer constructor injection over method/field injection.

### Java SDK three-layer Gradle layout (2026-05-21)
- Java SDK Gradle modules are now exactly three integration layers: `scheduler-java` (native Java), `scheduler-spring` (Spring Framework adapter), and `scheduler-spring-boot` (Spring Boot autoconfiguration/starter).
- Do not reintroduce `scheduler-java-core`, `scheduler-spring-boot-autoconfigure`, or separate `scheduler-spring-boot-starter` module names; Spring Boot starter behavior belongs in `scheduler-spring-boot`.
