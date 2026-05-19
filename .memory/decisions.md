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
- HTTP/OpenAPI 阶段选择 `utoipa` + `utoipa-swagger-ui`。
- OpenAPI JSON 暴露路径使用 `GET /api-docs/openapi.json`，Swagger UI 使用 `/docs`。

Rationale:
- `utoipa` 当前稳定、Axum 集成成熟，适合从 Rust handler/DTO 生成 OpenAPI。
- `utoipa-swagger-ui` 会注册自身 OpenAPI JSON 路由，使用 `/api-docs/openapi.json` 可避免与手写 `/openapi.json` 路由冲突。

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
- 已完成项使用 `[x] ✅` 标记，并可补充实际完成范围说明。

Rationale:
- 设计文档是项目进度和范围共识源，路线图必须随实现推进同步更新。

Constraint:
- 后续提交若完成开发项但未更新路线图，视为交接上下文不完整。
