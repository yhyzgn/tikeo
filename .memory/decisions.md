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
- 根 package `tikeo` 只负责 binary entrypoint 和启动委托，不承载业务模块。

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
- 每个开发工作项完成后，必须更新 `design/tikeo-architecture-design.md` 的开发路线图。
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
- Java Worker 仍必须主动连接 tikeo，不得要求业务应用暴露入站端口。


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
- `tikeo-storage` 同时启用 `sqlx-sqlite` 与 `sqlx-mysql` feature；开发默认 SQLite，生产 MySQL migration 能复用通用 SeaORM schema。

Rationale:
- 用户要求依赖尽量使用最新版，但调度平台存储层属于核心基础设施，不应以 RC 版本作为初始长期基线。
- `1.1.20` 是当前 1.x 稳定线可用版本，兼顾最新稳定与生态可靠性。

Constraint:
- SeaORM 2.0 发布稳定版后，应在独立阶段评估升级，不得在业务功能提交中顺手升级核心 ORM。


## 2026-05-19 — Worker Tunnel RPC 命名避免客户端冲突

Decision:
- Worker Tunnel protobuf RPC 从 `Connect` 调整为 `OpenTunnel`。
- Rust proto crate 开启 tonic client 生成，供 `tikeo` 使用。

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
- 浏览器端只能访问 tikeo HTTP/OpenAPI，不直接访问 Worker Tunnel。


## 2026-05-19 — 容器化部署基础形态

Decision:
- 后端使用仓库根 `Dockerfile` 产出 `tikeo` server 镜像，保持根 binary 入口，不把主程序入口迁入 `crates/`。
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


## 2026-05-19 — Tikeo tick loop dependency baseline

Decision:
- CRON expression parsing uses `cron 0.16.0`; Fixed Rate duration parsing uses `humantime 2.3.0`.
- 010 keeps trigger cursor in memory and creates pending instances through the existing `JobInstanceRepository`.

Rationale:
- These are current stable crates from crates.io search, and the project needs a minimal automatic trigger loop before persistent tikeo metadata is designed.

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

- 012 阶段采用开发管理员 token 作为最小认证闭环，默认 `tikeo_init/Tikeo@2026!` -> `tikeo-init-token`，允许环境变量覆盖。
- 写操作先保护 Job 创建与手动触发，读接口、health、ready、OpenAPI 暂保持开放，便于开发和部署烟测。
- Web token 暂存在 `localStorage`，后续正式 RBAC/OIDC 阶段必须替换为更完整的会话、安全刷新与权限模型。


## 2026-05-19 — Docker 网络验证基线

- Docker 构建与运行验证不得使用 `--network host` 作为捷径。
- 最低验收基线是 Docker 默认 bridge 网络与 `docker compose` bridge 网络；Web 容器必须通过 Compose 服务名 / bridge DNS 代理访问 tikeo。
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
- Built-in initialization credentials are `tikeo_init` / `Tikeo@2026!` / `tikeo-init-token`; they are development-only and remain overrideable through `TIKEO_DEV_ADMIN_*`.


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
- tikeo server 后续可能多节点部署，需要能够替换为 Redis 分布式 session 存储而不重写 auth/RBAC handler。
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

- 删除 `tikeo-init-token` 静态 Bearer 后门；初始化账号仅通过 `/api/v1/auth/login` 获取 `atk_` session token。
- 审计日志不得保存明文 Bearer token；session 相关审计只能保存脱敏标识或不可逆摘要。
- 出站告警 Webhook 默认只允许 HTTPS，并拒绝 localhost/私网/link-local/metadata 目标；后续如需内网 webhook，必须显式 allowlist。

## 2026-05-20 — 021 RBAC 使用软关联 permission/resource/action

- RBAC 从单字符串 role check 升级为 `resource/action` 权限检查。
- 数据库新增 roles、permissions、role_permissions，但不创建任何外键；角色名仍保留在 users.role 以兼容现有用户模型。
- admin 角色保留全权限短路和全量默认权限绑定，operator/viewer 通过 seed 权限控制能力边界。
- HTTP 层统一走 `require_permission`，Web 使用同一权限模型隐藏菜单和显示 403。

## 2026-05-21 — SDK/examples 语言目录规范与 demo 自主创建

Decision:
- `sdks/` 是 SDK 总目录，其下必须按 `sdks/<language>/<sdk-name>/` 组织，例如 `sdks/rust/tikeo`、`sdks/java/tikeo-spring-boot-starter`。
- Java SDK 必须使用 Gradle（优先 Kotlin DSL）而不是 Maven，且 Java toolchain / source / target 必须支持 JDK 21+。
- `examples/` 是 demo 总目录，必须按 `examples/<language>/<demo-name>/` 组织，并与 `sdks/` 语言结构对应。
- `examples/` 不再用于存放运行配置；运行配置仍属于 `config/`。
- 后续开发者/AI agent 在实现或验证 SDK、Worker、任务执行、工作流或跨语言集成时，需要自行判断并主动创建/更新相应 demo 项目用于调试，不必等待用户显式要求。

Constraint:
- Rust SDK 路径为 `sdks/rust/tikeo`；服务端 Dockerfile 不处理 SDK 构建或缓存。
- Java Maven `pom.xml` 骨架已要求迁移为 Gradle 多模块，验证命令统一为 `./sdks/java/gradlew -p sdks/java test`。

## 2026-05-21 — Server Dockerfile 与 SDK 解耦

Decision:
- 根 `Dockerfile` 只用于构建 tikeo server，不复制或缓存 `sdks/`。
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
- Java SDK/starter configuration must expose `clientInstanceId` / `tikeo.worker.client-instance-id`, not `workerId`, until the server returns the authoritative id during tunnel registration.

### Java Worker Tunnel real client boundary (2026-05-21)
- Java SDK core owns gRPC/protobuf generated bindings inside `sdks/java/tikeo`; it must remain independently publishable and must not depend on server crates/modules.
- Spring Boot starter may expose dry-run mode for local demos, but production default is the real `GrpcTikeoWorkerClient`.
- Remaining Java SDK gap is ergonomic `@TikeoProcessor` method adaptation; do not revert to no-op as the default live client.

### Java processor dispatch convention (2026-05-21)
- Until Worker Tunnel protocol gains an explicit processor/key field, Java Spring adapter treats `DispatchTask.job_id` as the processor name for `@TikeoProcessor` routing.
- Supported Java processor signatures are intentionally small and safe: zero args, `TaskContext`, UTF-8 `String`, or `byte[]`; return `TaskOutcome`, `String`, `boolean`, or `void`.
- Exceptions are mapped to failed `TaskOutcome` instead of escaping the Worker Tunnel processing thread.

### Java SDK Lombok and injection style (2026-05-21)
- Java SDK and Java demos should use Lombok where it meaningfully removes boilerplate, currently pinned to Lombok 1.18.46.
- Spring beans in SDK/demo code should prefer constructor injection over method/field injection.

### Java SDK three-layer Gradle layout (2026-05-21)
- Java SDK Gradle modules are now exactly three integration layers: `tikeo` (native Java), `tikeo-spring` (Spring Framework adapter), and `tikeo-spring-boot-starter` (Spring Boot autoconfiguration/starter).
- Do not reintroduce `tikeo-core`, `tikeo-spring-boot-autoconfigure`, or a non-starter `tikeo-spring-boot` module name; Spring Boot integration artifact is `tikeo-spring-boot-starter`.

### Java Spring Boot starter artifact naming (2026-05-21)
- The Spring Boot Java SDK module/artifact must be named `tikeo-spring-boot-starter` to match Spring Boot ecosystem conventions.
- The three Java Gradle modules remain: `tikeo`, `tikeo-spring`, and `tikeo-spring-boot-starter`.

### Worker dispatch processor key (2026-05-21)
- `DispatchTask.processor_name` is the explicit SDK routing key for Java/Spring `@TikeoProcessor` and future language SDK adapters.
- Server currently populates `processor_name` from `job_id` for compatibility until job definitions carry a distinct processor binding.
- SDKs may fallback to `job_id` only when `processor_name` is empty for backward compatibility.

## 2026-05-22 — Non-WASM script dispatch requires worker runtime capabilities

- Decision: Dynamic scripts dispatched over Worker Tunnel target workers advertising unified `script`; worker-side runtime selection uses binding language plus sandbox.backend. Legacy `script:<language>`, `script:wasm`, `script:*`, and `*` remain accepted for compatibility.
- Server remains a metadata dispatcher only. It sends released immutable `script_versions` bytes, hash, version metadata, and policy fields, but never runs user code.
- Rust SDK workers must explicitly register a matching `ScriptRunner`; missing runner support is a task failure, not a fallback to normal task processors.
- Java SDK intentionally rejects script bindings until Java-side runner abstractions are designed.

## 2026-05-22 — 默认按功能拆分代码文件

- 所有后续开发默认按职责/功能拆分文件和模块，适用于 Rust server/crates、各语言 SDK、Web 前端和示例代码。
- 禁止让单个文件持续膨胀；当文件体量明显变大时，应在同一阶段内顺手拆分到合理模块。
- Rust SDK 先以 `tikeo/src/lib.rs` 为整改对象：`lib.rs` 只保留模块声明和 public re-export，具体能力拆到 config/session/task/error/script/wasm/proto/tests 等模块。

## 2026-05-22 — Project rename to tikeo

- Decision: Rename the product/repository identity from the previous project identity to `tikeo` across source, docs, SDKs, protocol namespaces, Docker/K8s names, and operational environment variables.
- Java SDK/API package prefix is now `net.tikeo`; Java Gradle modules are `tikeo`, `tikeo-spring`, and `tikeo-spring-boot-starter`.
- Rust crates/binary now use `tikeo-*` and root binary `tikeo`; Rust Worker SDK path/crate is `sdks/rust/tikeo` / `tikeo`.
- Protobuf package is now `tikeo.worker.v1`; internal Raft transport token header is `x-tikeo-raft-token`.
- Git remote is expected to be `https://github.com/yhyzgn/tikeo.git`; local git identity is `Neo <yhyzgn@gmail.com>`.

## 2026-05-22 — SDK package names contract

- Decision: The Rust Worker SDK crate and path are `tikeo` / `sdks/rust/tikeo`, not the previous Rust Worker SDK name.
- Decision: The Java core SDK Gradle module/artifact is `tikeo`, not the previous Java core SDK name; companion modules remain `tikeo-spring` and `tikeo-spring-boot-starter`.
- Constraint: The repository root binary crate is also named `tikeo`; the Rust SDK is intentionally kept outside the root Cargo workspace so both packages can coexist.

## 2026-05-25 — Source file size and module-entry rule

- Decision: Source files must stay at or below 1500 lines. This applies to Rust, Web, SDK, and example source files in normal development paths.
- Decision: `mod.rs`, `lib.rs`, and equivalent module-entry files should declare modules and re-export entry points only; they must not accumulate route handlers, tests, storage logic, migrations, or business implementation bodies.
- Constraint: When a change would push a file near the limit, split by responsibility in the same change before adding more behavior.

## 2026-05-25 — Script release signature local boundary

- Decision: Script release signature verification is default-disabled and enabled only when `script_governance.release_signature_secret_ref` is configured.
- Decision: The first local verification boundary supports `env:NAME` secret refs and a deterministic `sha256:<hex>` signature over script id, immutable version number, content SHA-256, and approval ticket.
- Constraint: This is not a replacement for future KMS/PKI/multi-level approval; it is a local, testable boundary that prevents silently accepting approval/signature metadata.
- Rejected: Accepting arbitrary approval tickets without a matching signature | would create governance theater instead of a real gate.

## 2026-06-05 — Worker visibility persistence and structured parity rule

- Decision: Worker list visibility must merge live registry state with persisted `worker_sessions` snapshots; important Worker observability state is not allowed to be memory-only.
- Decision: Persisted worker snapshot fields include capabilities, structuredCapabilities, labels, and master state; UI/API filtering must use structured fields/labels/capabilities, not clientInstanceId or job naming conventions.
- Decision: Go and Rust SDK/demo parity is evaluated against the Java demo feature surface where feasible: live Worker Tunnel, structured scope, processor names, assignment-token task logs, script runner capabilities, and reconnect behavior.
- Constraint: Any future worker_pool/sandbox/processor matching fallback based only on naming convention is rejected unless documented as explicit legacy compatibility with tests proving structured matching remains primary.

## 2026-06-05 — Script runner capability truthfulness

- Decision: A Worker may advertise a script runner only when the corresponding sandbox/runtime boundary is executable in that process. Unavailable/Unsupported adapters may exist as fail-closed handlers but must not be included in structured `scriptRunners`.
- Decision: Go/Rust demos default to no script runner advertisement unless explicitly configured with a real container/runtime or a clearly development-only local runner advertising `custom`.
- Rejected: Advertising Java parity sandbox names such as `srt`, `deno`, `v8`, `wasmtime`, or `wasmedge` from Go/Rust without an executable implementation | this creates fake scheduling capacity and breaks manual/automated验收.

## 2026-06-05 — CI language coverage rule

- Decision: Main CI must include every implemented SDK/demo family, not just server/web and Java/Rust SDKs. Go SDK/demo, Java demos, Rust demo, deploy Go tooling, and cross-language smoke are quality gates.
- Rejected: leaving Go or demo validation as manual-only | it allows parity regressions and fake capability advertising to bypass GitHub checks.


## 2026-06-05 — Storage migration versioning rule

Decision:
- Runtime schema changes must be represented by explicit SeaORM migrations in `tikeo-storage::migration::Migrator::migrations` and persisted in `seaql_migrations`.
- `connect_and_migrate` may configure/connect and run `Migrator::up`, but must not append hidden post-migrate `ensure_*` schema patches.
- SQLite legacy/dev compatibility remains allowed only as a named, idempotent migration module such as `sqlite_compat`, with regression tests covering old DB shapes.

Rejected:
- Keeping compatibility DDL as an untracked startup hook | it makes production upgrades unauditable and can diverge across SQLite/MySQL/PostgreSQL/CockroachDB.

Constraint:
- Future schema additions must add a migration entry or update a clearly named migration module with tests; do not silently patch tables after `Migrator::up`.

## 2026-06-08 — Helm production secret and worker boundary

Decision: The Helm chart may deploy Tikeo Server, Worker Tunnel service, and Web console, but it must not deploy business workers or expose worker inbound Services by default.

Rationale:
- Tikeo's core cloud-native advantage is that workers initiate outbound Worker Tunnel connections across namespaces, clusters, VPCs, and NAT boundaries.
- Production database URLs and TLS/mTLS materials must be injected through Kubernetes Secrets or platform secret managers, not committed in values files.

Implications:
- `server.storage.mode=external` uses `TIKEO__STORAGE__DATABASE_URL` from `server.storage.existingSecret`.
- Listener TLS/mTLS secrets are mounted and rendered into `transport_security` config; ingress TLS remains a separate edge termination boundary.
- Future chart work can add PDB/NetworkPolicy/ServiceMonitor/Gateway API, but must preserve worker outbound-only semantics.

## 2026-06-08 — Helm ops hardening remains optional and CRD-gated

Decision: PodDisruptionBudget, NetworkPolicy, ServiceMonitor, and Gateway API manifests are chart-supported but disabled by default.

Rationale:
- Local and minimal Kubernetes installs should not require Prometheus Operator or Gateway API CRDs.
- NetworkPolicy behavior depends on the cluster CNI and must be explicitly enabled by operators.
- Gateway API `GRPCRoute` support depends on the installed controller and should be an opt-in example for Worker Tunnel h2/gRPC exposure.

Implications:
- The chart renders these resources only when corresponding values are enabled.
- Operations overlays must keep Worker networking outbound-only and must not create business Worker inbound Services.

## 2026-06-09 — Job scope edits require dual-scope authorization

Decision:
- Editing a Job's namespace/app is allowed through the normal job update path, but it is treated as a scope move rather than a cosmetic field edit.
- The API must authorize the caller against both the current job scope and the destination namespace/app scope before persisting the move.
- Existing job instances remain historical execution records; the moved job's future scheduling, triggering, Worker matching, and canary validation use the new namespace/app.
- Canary targets set or retained during a job update must belong to the target namespace/app.

Rejected:
- Frontend-only scope editing while the backend silently ignores namespace/app | would create false UI behavior and SDK/API drift.
- Allowing a move with only source-scope authorization | would let tokens write jobs into scopes they cannot otherwise manage.

## 2026-06-10 — SDK management docs must be source-backed

Decision:
- SDK documentation for Management API create+trigger flows must name only helpers that exist in committed SDK source.
- Broadcast API triggers remain explicit helper calls with selector payloads; default helper paths stay single-worker (`executionMode=single`).
- Java SDK now includes `BroadcastSelectorRequest` and `TriggerJobRequest.broadcastApi(...)` to match Rust/Go/Python/Node broadcast selector parity before documentation references it.

Rationale:
- The docs site should be usable as API reference without inventing SDK helpers or blurring app-scoped machine credentials with human session flows.

Constraint:
- Future SDK docs must keep `x-tikeo-api-key` / `TIKEO_MANAGEMENT_API_KEY`, `triggerType=api`, default `executionMode=single`, and explicit `broadcastSelector` wording/source tests aligned.

## 2026-06-10 — Acceptance-stage rigor and context freshness rule

Decision:
- During functional/module testing and acceptance phases, scope must not be silently reduced to make work look complete.
- If an agent finds any missing, incomplete, untested, or hallucinated behavior, it must fill the gap with production-grade implementation or explicitly record the real blocker.
- Source-backed facts, current context, memory, prompts, and verification evidence must be kept fresh so later agents do not inherit context rot.

Constraint:
- This rule is also recorded in `~/.codex/CONSTITUTION.md` and OMX project memory; future work must apply it as an acceptance-phase operating rule.

## 2026-06-10 — Docs module path and Docker image publishing

Decision:
- The Docusaurus documentation site module is now `docs/`, replacing the old `website/` directory name.
- Shared README/media assets that used to live under top-level `docs/assets/` are now under `assets/docs/`, keeping the `docs/` tree dedicated to the buildable docs-site module.
- The docs site has its own Docker image boundary and publish workflow targeting Docker Hub repository `yhyzgn/tikeo-docs` with `docs/Dockerfile` and nginx static serving config.

Rationale:
- The docs site is a first-class module that is built, validated, containerized, and published independently.
- Keeping assets outside `docs/` avoids mixing generic repository media with Docusaurus source.

Constraint:
- Future docs/frontend commands must use `bun`/`bunx`; docs module commands should run from `docs/`.
- Do not reintroduce `website/` as a build module or point CI/publishing contracts back to it.
